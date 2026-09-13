//! Validates `samples/template_traits.jsonl` against the two inputs it was
//! generated from.
//!
//! # Why this duplicates the generator
//!
//! `crates/synthpass-ocr/examples/template_traits.rs` produces the tracked
//! file; this test re-derives the same rows from the same two tracked inputs
//! (`samples/corpus.jsonl` and `samples/ocr_fixtures/*.json`) using its own,
//! separately-written implementation, and asserts the two agree — the same
//! reasoning `crates/synthpass-bench/tests/corpus_manifest.rs` gives for not
//! sharing its filename parser with its own producer: a validator built on
//! the producer's own code agrees with the producer by construction and
//! cannot catch a bug in it. Nothing here imports the generator (an
//! `examples/` binary isn't importable from another crate anyway, so the
//! independence is structural, not just a style choice).
//!
//! # Why this runs without any images
//!
//! Like `corpus_manifest.rs`'s test, this reads only tracked text —
//! `samples/corpus.jsonl` and the hand-verified `samples/ocr_fixtures/*.json`
//! labels are tracked directly (see `.gitignore`), so this runs the same in a
//! fresh clone as anywhere else, with no OCR models and no specimen images.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: must be tracked and readable: {e}", path.display()));
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("every line is one JSON object"))
        .collect()
}

fn corpus_rows() -> Vec<Value> {
    read_jsonl(&repo_root().join("samples").join("corpus.jsonl"))
}

fn registry_rows() -> Vec<Value> {
    read_jsonl(&repo_root().join("samples").join("template_traits.jsonl"))
}

fn fixture_for_stem(stem: &str) -> Option<Value> {
    let path = repo_root()
        .join("samples")
        .join("ocr_fixtures")
        .join(format!("{stem}.json"));
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// One character per position: `A` for an ASCII letter, `9` for a digit,
/// anything else kept literally.
fn positional_shape(number: &str) -> String {
    number
        .chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                'A'
            } else if c.is_ascii_digit() {
                '9'
            } else {
                c
            }
        })
        .collect()
}

/// The broad document category for a corpus row: `passports`/`id_cards`/
/// `driving_licenses` map directly; anything else (`misc`, and the
/// old-convention `ocr_fixtures/` storage directory) is read off the
/// filename itself.
fn document_type_for(dir: &str, filename: &str) -> Option<&'static str> {
    match dir {
        "passports" => Some("passport"),
        "id_cards" => Some("id-card"),
        "driving_licenses" => Some("driving-license"),
        _ => {
            let lower = filename.to_ascii_lowercase();
            if lower.contains("passport") {
                Some("passport")
            } else if lower.contains("driving")
                || lower.contains("licence")
                || lower.contains("license")
            {
                Some("driving-license")
            } else if lower.contains("_id_") {
                Some("id-card")
            } else {
                None
            }
        }
    }
}

/// Pads or trims a trailing `<` filler run to `expected_len`, refusing when
/// the gap reaches into real content or is implausibly large. Mirrors the
/// generator's own tolerance for the measured hand-transcription miscount
/// (see that file's module doc comment) — re-derived independently here
/// rather than shared, which is the point of this test.
fn fit_trailing_filler(line: &str, expected_len: usize) -> Result<String, String> {
    let observed = line.chars().count();
    if observed == expected_len {
        return Ok(line.to_string());
    }
    let core = line.trim_end_matches('<');
    let core_len = core.chars().count();
    if core_len > expected_len || observed.abs_diff(expected_len) > 2 {
        return Err(format!(
            "{line:?}: {observed} characters (want {expected_len}), non-filler content is \
             {core_len} — not a plain trailing-filler miscount"
        ));
    }
    let mut fitted = core.to_string();
    while fitted.chars().count() < expected_len {
        fitted.push('<');
    }
    Ok(fitted)
}

/// Independently re-derives one registry row from a joined (corpus row,
/// fixture) pair, or explains why no row should exist for it.
fn derive_row(corpus_row: &Value, fixture: &Value) -> Result<Value, String> {
    let mrz_line = fixture
        .get("mrz_line")
        .and_then(Value::as_str)
        .ok_or("fixture has no mrz_line")?;
    let lines: Vec<&str> = mrz_line.split('\n').collect();
    let line1 = *lines.first().ok_or("mrz_line is empty")?;
    if line1.chars().count() < 5 {
        return Err(format!("line 1 {line1:?} shorter than 5 characters"));
    }
    let document_code = &line1[0..2];
    let issuing_state = &line1[2..5];
    if issuing_state.starts_with('<') {
        return Err(format!(
            "issuing-state field {issuing_state:?} on line 1 {line1:?} starts with a filler — \
             a shifted field boundary, not a real issuing state"
        ));
    }

    let expected_lens: &[usize] = match (lines.len(), document_code.as_bytes()[0]) {
        (3, _) => &[30, 30, 30],
        (2, b'P') => &[44, 44],
        (2, b'I' | b'A' | b'C') => &[36, 36],
        (n, code) => {
            return Err(format!(
                "{n} line(s), document code starting {:?} — no known ICAO format",
                code as char
            ));
        }
    };
    if lines.len() != expected_lens.len() {
        return Err("line count does not match the format its document code implies".to_string());
    }
    let mut fitted = Vec::with_capacity(lines.len());
    for (line, &want) in lines.iter().zip(expected_lens) {
        fitted.push(fit_trailing_filler(line, want)?);
    }

    let (data, td_format) = match fitted.len() {
        3 => (
            mrz::parse_td1(&fitted[0], &fitted[1], &fitted[2])
                .map_err(|e| format!("parse_td1: {e:?}"))?,
            "Td1",
        ),
        2 if expected_lens[0] == 44 => (
            mrz::parse_td3(&fitted[0], &fitted[1]).map_err(|e| format!("parse_td3: {e:?}"))?,
            "Td3",
        ),
        2 => (
            mrz::parse_td2(&fitted[0], &fitted[1]).map_err(|e| format!("parse_td2: {e:?}"))?,
            "Td2",
        ),
        _ => unreachable!("expected_lens is always length 2 or 3"),
    };

    let dir = corpus_row
        .get("dir")
        .and_then(Value::as_str)
        .ok_or("corpus row has no dir")?;
    let filename = corpus_row
        .get("filename")
        .and_then(Value::as_str)
        .ok_or("corpus row has no filename")?;
    let sha256 = corpus_row
        .get("sha256")
        .and_then(Value::as_str)
        .ok_or("corpus row has no sha256")?;
    let document_type = document_type_for(dir, filename).ok_or("cannot classify document_type")?;
    let series = corpus_row
        .get("year")
        .and_then(|y| y.get("value"))
        .and_then(Value::as_u64)
        .map(|y| y.to_string())
        .ok_or("no year.value to use as series")?;

    let nationality_field_is_issuer = data.nationality != data.issuing_country;

    let document_number = match fixture.get("document_number").and_then(Value::as_str) {
        Some(n) if !n.is_empty() => serde_json::json!({
            "shape": positional_shape(n),
            "length": n.chars().count(),
        }),
        _ => Value::Null,
    };

    let optional_data_present = data.personal_number.is_some();

    let checksums_valid = fixture
        .get("mrz_checksums_valid")
        .and_then(Value::as_bool)
        .ok_or("fixture has no mrz_checksums_valid")?;

    Ok(serde_json::json!({
        "issuing_state": issuing_state,
        "document_type": document_type,
        "document_code": document_code,
        "document_code_label": Value::Null,
        "td_format": td_format,
        "series": series,
        "nationality_field_is_issuer": nationality_field_is_issuer,
        "document_number": document_number,
        "optional_data_present": optional_data_present,
        "checksums_valid": checksums_valid,
        "source": [{ "filename": filename, "sha256": sha256 }],
    }))
}

/// Every ground-truth-backed corpus row this test's independent derivation
/// can build a trait row from, keyed by source filename (unique: a row's
/// `source` always names exactly the one corpus entry it came from in v0).
fn independently_derivable_rows() -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for row in &corpus_rows() {
        let Some(stem) = row.get("ground_truth_stem").and_then(Value::as_str) else {
            continue;
        };
        let Some(fixture) = fixture_for_stem(stem) else {
            continue;
        };
        if let Ok(derived) = derive_row(row, &fixture) {
            let filename = row["filename"].as_str().unwrap_or_default().to_string();
            out.insert(filename, derived);
        }
    }
    out
}

#[test]
fn every_tracked_row_matches_an_independent_rederivation() {
    let expected = independently_derivable_rows();
    let actual = registry_rows();
    assert!(
        !actual.is_empty(),
        "samples/template_traits.jsonl must not be empty"
    );

    for row in &actual {
        let filename = row["source"][0]["filename"]
            .as_str()
            .unwrap_or_else(|| panic!("row has no source[0].filename: {row}"));
        let want = expected.get(filename).unwrap_or_else(|| {
            panic!(
                "{filename}: template_traits.jsonl carries a row this test cannot independently \
                 re-derive from samples/corpus.jsonl + samples/ocr_fixtures/ — regenerate with \
                 `cargo run -p synthpass-ocr --example template_traits` and check its report"
            )
        });
        assert_eq!(
            row, want,
            "{filename}: tracked row disagrees with independent re-derivation"
        );
    }
}

#[test]
fn every_independently_derivable_row_is_tracked() {
    // The other direction: nothing this test can derive is missing from the
    // tracked file — a stale file after a fixture or corpus change fails
    // here the same way a stale corpus manifest fails its own test.
    let expected = independently_derivable_rows();
    let actual = registry_rows();
    let tracked_filenames: std::collections::BTreeSet<&str> = actual
        .iter()
        .map(|r| r["source"][0]["filename"].as_str().unwrap_or_default())
        .collect();
    for filename in expected.keys() {
        assert!(
            tracked_filenames.contains(filename.as_str()),
            "{filename}: independently derivable but missing from template_traits.jsonl — \
             regenerate with `cargo run -p synthpass-ocr --example template_traits`"
        );
    }
    assert_eq!(
        actual.len(),
        expected.len(),
        "template_traits.jsonl has a different row count than this test can independently derive"
    );
}

#[test]
fn every_row_has_the_required_shape() {
    for row in &registry_rows() {
        let source = row["source"][0]["filename"].as_str().unwrap_or("<unknown>");
        for key in [
            "issuing_state",
            "document_type",
            "document_code",
            "document_code_label",
            "td_format",
            "series",
            "nationality_field_is_issuer",
            "document_number",
            "optional_data_present",
            "checksums_valid",
            "source",
        ] {
            assert!(row.get(key).is_some(), "{source}: row is missing `{key}`");
        }
        assert_eq!(
            row["document_code"].as_str().map(|s| s.chars().count()),
            Some(2),
            "{source}: document_code must be exactly two characters (ICAO 9303 line 1 positions 1-2)"
        );
        assert_eq!(
            row["issuing_state"].as_str().map(|s| s.chars().count()),
            Some(3),
            "{source}: issuing_state must be exactly three characters (ICAO 9303 line 1 positions 3-5)"
        );
        assert!(
            matches!(
                row["td_format"].as_str(),
                Some("Td1") | Some("Td2") | Some("Td3")
            ),
            "{source}: td_format must be Td1, Td2 or Td3 (no visa fixture exists yet), got {:?}",
            row["td_format"]
        );
        assert!(
            row["checksums_valid"].is_boolean(),
            "{source}: checksums_valid must be a boolean"
        );
        assert!(
            row["nationality_field_is_issuer"].is_boolean(),
            "{source}: nationality_field_is_issuer must be a boolean"
        );
        assert!(
            row["optional_data_present"].is_boolean(),
            "{source}: optional_data_present must be a boolean"
        );
        assert_eq!(
            row["document_code_label"],
            Value::Null,
            "{source}: document_code_label must be null in v0 (no source-verified label exists yet)"
        );
        let sources = row["source"].as_array().expect("source is an array");
        assert_eq!(
            sources.len(),
            1,
            "{source}: v0 never merges more than one verified source into a row"
        );
        let sha = sources[0]["sha256"].as_str().unwrap_or_default();
        assert_eq!(
            sha.len(),
            64,
            "{source}: source sha256 must be 64 hex characters"
        );
        assert!(
            sha.chars().all(|c| c.is_ascii_hexdigit()),
            "{source}: source sha256 must be hex"
        );
        if let Some(number) = row["document_number"].as_object() {
            let shape = number["shape"].as_str().unwrap_or_default();
            let length = number["length"].as_u64();
            assert_eq!(
                Some(shape.chars().count() as u64),
                length,
                "{source}: document_number.length must equal the shape string's own length"
            );
            assert!(
                shape.chars().all(|c| c == 'A' || c == '9'),
                "{source}: document_number.shape must use only `A`/`9` for the shapes seen so far, \
                 got {shape:?} — if a document number really contains another character class, \
                 that's a new, legitimate shape token, not a bug"
            );
        }
    }
}

#[test]
fn rows_are_sorted_by_issuing_state_then_document_code_then_series() {
    let rows = registry_rows();
    let keys: Vec<(String, String, String, String)> = rows
        .iter()
        .map(|r| {
            (
                r["issuing_state"].as_str().unwrap_or_default().to_string(),
                r["document_code"].as_str().unwrap_or_default().to_string(),
                r["series"].as_str().unwrap_or_default().to_string(),
                r["source"][0]["filename"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            )
        })
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(
        keys, sorted,
        "template_traits.jsonl is not sorted by (issuing_state, document_code, series, source \
         filename) — regenerate with `cargo run -p synthpass-ocr --example template_traits`"
    );
}

#[test]
fn no_row_carries_a_holder_value() {
    // The schema itself has no field for a name, a raw document number, a
    // date, or the MRZ text — this is a belt-and-braces check that a future
    // edit of the generator never adds one back in.
    for row in &registry_rows() {
        let obj = row.as_object().expect("row is a JSON object");
        for key in obj.keys() {
            assert!(
                ![
                    "surname",
                    "given_names",
                    "name",
                    "date_of_birth",
                    "date_of_expiry",
                    "mrz_line",
                    "personal_number",
                    "document_number_value",
                ]
                .contains(&key.as_str()),
                "template_traits.jsonl row has a holder-value-shaped key `{key}` — this registry \
                 records shapes only"
            );
        }
    }
}
