//! Generates `samples/template_traits.jsonl` — a template-trait registry
//! recording, per verified specimen, the **shape** of MRZ its
//! code/document-type/series produces: never a holder value.
//!
//! # Why this exists
//!
//! The specimen loop is starting to add MRZ-format variety on purpose
//! (organisations, nationality-only codes, new series per code). Nothing else
//! in the repo records *what shape of MRZ a given code/document-type/series is
//! known to produce* — that knowledge lives only in a reviewer's head and in
//! the hand-verified fixtures under `samples/ocr_fixtures/`. This file turns
//! that tacit knowledge into a tracked, testable artifact. Full rationale in
//! `work/registry-v0-design.md` (task T8 of the specimen acquisition loop).
//!
//! # Ground truth, not OCR
//!
//! A row is emitted only for a `samples/corpus.jsonl` entry whose
//! `ground_truth_stem` names a fixture under `samples/ocr_fixtures/` — every
//! field here is read from that hand-verified fixture's own `mrz_line` or from
//! `corpus.jsonl`'s own human-reviewed `dir`/`year`/`sha256` fields. Nothing
//! here falls back to `corpus.jsonl`'s `mrz.observed.*`, which is a dated,
//! single-pass OCR read — 8 of the 59 fixtures disagree with a fresh OCR pass
//! over the same image, and all 8 fixture labels were independently
//! re-verified as correct (see the design note's TCR1 section). Better an
//! absent row than one built on the less reliable of two sources.
//!
//! # Why the strict per-format parsers, not [`mrz::find_and_parse`]
//!
//! `find_and_parse` is built to repair noisy OCR text of unknown shape: when
//! every check digit fails it tries confusable-character substitutions
//! (`5`↔`S`, filler-width shifts, ...) hunting for a candidate that scores
//! better, and returns whichever candidate scored best even when none
//! validates. That is exactly right for OCR output, and exactly wrong here:
//! several fixtures are ground truth for a document whose *printed* MRZ fails
//! its own checksum (`mrz_checksums_valid: false`, hand-transcribed verbatim
//! from a real or intentionally-non-conformant specimen), and feeding that
//! already-clean text through the repair pass can silently substitute a
//! character. Measured on this corpus: `Turkiye_Passport_Specimen_P0_TUR_2024_mrz`'s
//! line 2 reads `U12345678<5TUR...` verbatim, but `find_and_parse` returns a
//! best-effort candidate with `5` swapped for `S` (`<STUR...`), turning a
//! `nationality` of `TUR` into `STU` — and `Turkiye_ID_Specimen_2020_back_mrz`
//! (three 30-character TD1 lines) comes back re-classified as `Td2` entirely,
//! because two of that TD1 zone's own lines happen to be repair-scoreable as a
//! TD2 candidate once checksums are already known to fail. Neither is a fact
//! about the document; both are artifacts of a repair heuristic built for a
//! different job. The strict functions ([`mrz::parse_td1`], [`mrz::parse_td2`],
//! [`mrz::parse_td3`], [`mrz::parse_mrv_a`], [`mrz::parse_mrv_b`]) are the same
//! crate's same field-boundary logic with that repair pass never invoked:
//! given the right number of lines at the right length they slice and trim
//! and nothing else, so a checksum-failing zone still reports its fields
//! exactly as printed. Format is decided from the line count plus the
//! document code already read off line 1 (see `build_row`), not from the raw
//! line lengths — those turn out not to be reliable, see the next section.
//!
//! # A second, measured fixture defect: miscounted trailing fillers
//!
//! 19 of the 59 fixtures' first physical line is one or two characters off
//! the ICAO-exact length (44 for TD3, 36 for TD2, 30 for TD1) — some short
//! (`Canada_Passport_Specimen_2023_mrz`, 43 not 44), most long
//! (`Nigeria_Passport_Specimen_P0_NGA_2022_mrz`, 45). In every one of the 19,
//! the gap sits entirely in the run of trailing `<` filler after the real
//! name content, which is exactly the position a person counting fillers by
//! eye in a photo (several of these fixtures are explicitly rotated or
//! partly censored) most easily miscounts by one — a hand-transcription slip
//! in `samples/ocr_fixtures/*.json`, not a defect in the underlying document,
//! and out of this file's scope to fix (task boundary: read fixtures, never
//! edit them). [`normalize_trailing_filler`] re-pads or trims *only* that
//! trailing run to the exact length the detected format requires before
//! calling the strict parser, and refuses (skip, reported) whenever the gap
//! reaches into non-filler content instead — which is what distinguishes this
//! from `Mauritania_Passport_Specimen_P0_MRT_2010_mrz`'s line 2 (also one
//! character over, but the extra character sits *before* the nationality
//! field, not in trailing filler) and from
//! `India_Passport_Specimen_P0_IND_2013_mrz_boxed` (exact length already, but
//! a missing character mid-line still shifts the issuing-state field) — both
//! correctly still rejected. This normalization changes nothing before the
//! first filler position, so it can never move `document_code`,
//! `issuing_state`, `document_number`, or any other real field value.
//!
//! # No merge-by-series yet
//!
//! The registry's conceptual key is code × document type × series, and a few
//! series in the corpus already have more than one verified fixture (Canada
//! 2023 `PP`, Argentina 2026 `P<ARG`, India 2013 `P<IND` each have two). v0
//! does not merge those into one row with a multi-entry `source` list — each
//! fixture-backed corpus entry gets its own row, `source` always holding
//! exactly one entry. Collapsing same-tuple rows into one, picking a
//! canonical row when two sources disagree, is future work once the registry
//! has a real reader; the schema's `source` field is already a list so that
//! change never needs a shape migration.
//!
//! # Usage
//!
//! ```powershell
//! cargo run -p synthpass-ocr --example template_traits
//! ```
//!
//! Needs no OCR models and no specimen images — it reads only the two tracked
//! text inputs above, so it runs the same in a fresh clone as anywhere else.

use std::path::{Path, PathBuf};

fn main() {
    let root = repo_root();
    let samples = root.join("samples");
    let corpus_path = samples.join("corpus.jsonl");
    let fixtures_dir = samples.join("ocr_fixtures");
    let out_path = samples.join("template_traits.jsonl");

    let corpus_rows = load_jsonl(&corpus_path);
    println!("corpus rows: {}", corpus_rows.len());

    let mut rows: Vec<serde_json::Value> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut no_fixture = 0usize;

    for row in &corpus_rows {
        let Some(stem) = row.get("ground_truth_stem").and_then(|v| v.as_str()) else {
            continue;
        };
        let fixture_path = fixtures_dir.join(format!("{stem}.json"));
        let Ok(bytes) = std::fs::read(&fixture_path) else {
            // corpus.jsonl claims a ground-truth stem whose fixture is
            // missing from this checkout — count it, don't guess at it.
            no_fixture += 1;
            skipped.push(format!(
                "{stem}: ground_truth_stem set, but {fixture_path:?} is missing"
            ));
            continue;
        };
        let fixture: serde_json::Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(e) => {
                skipped.push(format!("{stem}: fixture is not valid JSON ({e})"));
                continue;
            }
        };

        match build_row(row, &fixture) {
            Ok(trait_row) => rows.push(trait_row),
            Err(reason) => skipped.push(format!("{stem}: {reason}")),
        }
    }

    rows.sort_by_key(sort_key);

    if skipped.is_empty() {
        println!("every ground-truth-backed corpus entry produced a row");
    } else {
        println!(
            "\n{} entr{} could not be classified — reviewed by hand, not guessed at:",
            skipped.len(),
            if skipped.len() == 1 { "y" } else { "ies" }
        );
        for s in &skipped {
            println!("  {s}");
        }
    }
    if no_fixture > 0 {
        println!("({no_fixture} of the above were a missing fixture file, not a derivation gap)");
    }

    let body: String = rows
        .iter()
        .map(|r| format!("{}\n", serde_json::to_string(r).expect("row serializes")))
        .collect();
    std::fs::write(&out_path, body).expect("write template_traits.jsonl");
    println!("\nwrote {} row(s) to {}", rows.len(), out_path.display());
}

/// The sort key used for the tracked file: `(issuing_state, document_code,
/// series, source filename)`. The first three match the registry's
/// conceptual key (code × document type × series); the filename is a
/// deterministic tie-break for the same-tuple case described above, so a
/// rerun never reorders two rows that share a key.
fn sort_key(row: &serde_json::Value) -> (String, String, String, String) {
    (
        row["issuing_state"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        row["document_code"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        row["series"].as_str().unwrap_or_default().to_string(),
        row["source"][0]["filename"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    )
}

/// Builds one registry row from a joined (corpus row, fixture) pair, or
/// explains why it could not.
fn build_row(
    corpus_row: &serde_json::Value,
    fixture: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mrz_line = fixture
        .get("mrz_line")
        .and_then(|v| v.as_str())
        .ok_or("fixture has no mrz_line")?;
    let lines: Vec<&str> = mrz_line.split('\n').collect();

    // Line 1's document code (positions 1-2) and issuing state (positions
    // 3-5) are read literally off the fixture's own first line, fillers
    // included, the same way `corpus_manifest.rs` does — not from
    // `MrzData::document_type`/`issuing_country`, which trim trailing `<`
    // and would silently turn AFG's `PO` into a different-looking value than
    // Germany's legacy `D<<` for no reason relevant to this file. Both real
    // positions, never touched by the trailing-filler normalization below.
    let line1 = *lines.first().ok_or("mrz_line is empty")?;
    if line1.chars().count() < 5 {
        return Err(format!("line 1 {line1:?} is shorter than 5 characters"));
    }
    let document_code = line1[0..2].to_string();
    let issuing_state = line1[2..5].to_string();

    // ICAO 9303's filler `<` only ever pads a field's *trailing* positions
    // (Germany's real `D<<` is padding after one real letter); a leading
    // filler means an earlier field lost a character and every position
    // after it shifted, not a genuine issuing-state code. That is exactly
    // what `India_Passport_Specimen_P0_IND_2013_mrz_boxed` does: its line 1
    // reads `P<<SPECIMEN<<...`, one filler short of the `P<IND` an Indian
    // passport prints, so positions 3-5 read `<SP` instead of `IND` — the
    // fixture's own `issuing_country: "IND"` is a human's contextual read of
    // the document, not what its MRZ zone literally encodes. Recording `<SP`
    // as a trait would be exactly the "assume OCR/transcription is correct"
    // mistake this repo's OCR philosophy rules out, so this is reported as a
    // skip rather than guessed at.
    if issuing_state.starts_with('<') {
        return Err(format!(
            "line 1 {line1:?} has a leading filler in the issuing-state field \
             (read {issuing_state:?}) — the field boundary is shifted, most likely by a \
             missing character earlier in the line, not a real issuing state"
        ));
    }

    // Which ICAO format this is, decided by line count plus the document
    // code already read above — not by the raw line lengths, which turn out
    // not to be reliable (see `normalize_trailing_filler`'s doc comment).
    let (expected_lens, parse): (&[usize], ParseFn) =
        match (lines.len(), document_code.as_bytes()[0]) {
            (3, _) => (&[30, 30, 30], ParseFn::Td1),
            (2, b'P') => (&[44, 44], ParseFn::Td3),
            (2, b'I' | b'A' | b'C') => (&[36, 36], ParseFn::Td2),
            (2, b'V') => {
                // No visa fixture exists in the corpus today; disambiguate MRV-A
                // (44) from MRV-B (36) by which canonical length the observed
                // lines sit closer to, same tolerance as the trailing-filler
                // normalization below.
                let total: usize = lines.iter().map(|l| l.chars().count()).sum();
                if total.abs_diff(88) <= total.abs_diff(72) {
                    (&[44, 44], ParseFn::MrvA)
                } else {
                    (&[36, 36], ParseFn::MrvB)
                }
            }
            (n, code) => {
                return Err(format!(
                    "{n} line(s) with document code starting {:?} matches no ICAO 9303 format",
                    code as char
                ));
            }
        };

    let mut normalized = Vec::with_capacity(lines.len());
    for (line, &expected) in lines.iter().zip(expected_lens) {
        normalized.push(normalize_trailing_filler(line, expected)?);
    }
    let data = match parse {
        ParseFn::Td1 => mrz::parse_td1(&normalized[0], &normalized[1], &normalized[2]),
        ParseFn::Td2 => mrz::parse_td2(&normalized[0], &normalized[1]),
        ParseFn::Td3 => mrz::parse_td3(&normalized[0], &normalized[1]),
        ParseFn::MrvA => mrz::parse_mrv_a(&normalized[0], &normalized[1]),
        ParseFn::MrvB => mrz::parse_mrv_b(&normalized[0], &normalized[1]),
    }
    .map_err(|e| format!("{parse:?} parse failed on {normalized:?}: {e:?}"))?;
    let td_format = format!("{:?}", data.format);

    let dir = corpus_row
        .get("dir")
        .and_then(|v| v.as_str())
        .ok_or("corpus row has no dir")?;
    let filename = corpus_row
        .get("filename")
        .and_then(|v| v.as_str())
        .ok_or("corpus row has no filename")?;
    let sha256 = corpus_row
        .get("sha256")
        .and_then(|v| v.as_str())
        .ok_or("corpus row has no sha256")?;
    let document_type = document_type_for(dir, filename)
        .ok_or_else(|| format!("cannot classify document_type (dir={dir:?})"))?;

    let series = corpus_row
        .get("year")
        .and_then(|y| y.get("value"))
        .and_then(|v| v.as_u64())
        .map(|y| y.to_string())
        .ok_or("corpus row has no year.value to use as series")?;

    // Mechanical comparison of the two MRZ-native fields, both trimmed the
    // same way by the same parser, so no separate normalization is needed
    // here. This is expected to read `false` for every specimen currently in
    // the corpus — the field exists for the nationality-only codes
    // (GBD/GBN/GBO/GBP/GBS/XXA/XXB/XXC/XXX) a later scouting pass will add —
    // and reads `true` for three rows today. Two are genuine: Belgium and
    // Sweden's ID-card fixtures print ICAO's own `UTO` placeholder nationality
    // on an otherwise real-issuer specimen, and both have `checksums_valid:
    // true` (a well-formed zone, not a transcription defect). The third,
    // `Turkiye_Passport_Specimen_P0_TUR_2024_mrz`, almost certainly is not:
    // its `checksums_valid` is already `false`, and line 2 carries the same
    // class of hand-transcription defect documented above (an extra
    // character inserted right after the document-number check digit) —
    // except here the line's *total* length still comes out exactly 44,
    // because the shift happens to be absorbed elsewhere in the same line, so
    // `normalize_trailing_filler`'s length-based safeguard cannot see it.
    // Detecting a content-preserving mid-line shift in general needs
    // per-field checksum cross-validation this generator does not attempt in
    // v0; this row is left as a known, reported false positive rather than
    // guessed away.
    let nationality_field_is_issuer = data.nationality != data.issuing_country;

    let document_number = match fixture.get("document_number").and_then(|v| v.as_str()) {
        Some(number) if !number.is_empty() => serde_json::json!({
            "shape": positional_shape(number),
            "length": number.chars().count(),
        }),
        _ => serde_json::Value::Null,
    };

    // `MrzData::personal_number` is already `None` exactly when the field's
    // content, after trimming trailing `<` filler, is empty — for TD1 it is
    // the two optional-data fields joined, for TD2/TD3 the one optional-data
    // field. That is exactly "is this format's optional-data element used at
    // all", so no offset math is duplicated here.
    let optional_data_present = data.personal_number.is_some();

    let checksums_valid = fixture
        .get("mrz_checksums_valid")
        .and_then(|v| v.as_bool())
        .ok_or("fixture has no mrz_checksums_valid")?;

    Ok(serde_json::json!({
        "issuing_state": issuing_state,
        "document_type": document_type,
        "document_code": document_code,
        "document_code_label": serde_json::Value::Null,
        "td_format": td_format,
        "series": series,
        "nationality_field_is_issuer": nationality_field_is_issuer,
        "document_number": document_number,
        "optional_data_present": optional_data_present,
        "checksums_valid": checksums_valid,
        "source": [{ "filename": filename, "sha256": sha256 }],
    }))
}

/// Which strict per-format parser [`build_row`] dispatches to.
#[derive(Debug, Clone, Copy)]
enum ParseFn {
    Td1,
    Td2,
    Td3,
    MrvA,
    MrvB,
}

/// Pads or trims *only* a trailing run of `<` filler so `line` reaches
/// exactly `expected_len` characters, or explains why it cannot.
///
/// Real content — anything before the last non-`<` character — is never
/// touched: this can lengthen or shorten the filler run, never move a
/// character that carries information. See the module doc comment ("A
/// second, measured fixture defect") for the corpus finding this exists to
/// tolerate, and for the two cases (a shifted field mid-line, real content
/// itself over width) it deliberately still rejects.
fn normalize_trailing_filler(line: &str, expected_len: usize) -> Result<String, String> {
    let observed_len = line.chars().count();
    if observed_len == expected_len {
        return Ok(line.to_string());
    }
    let core = line.trim_end_matches('<');
    let core_len = core.chars().count();
    if core_len > expected_len {
        return Err(format!(
            "line {line:?} is {observed_len} characters (expected {expected_len}), and its \
             non-filler content alone is {core_len} — too long to be a trailing-filler miscount"
        ));
    }
    let diff = observed_len.abs_diff(expected_len);
    if diff > 2 {
        return Err(format!(
            "line {line:?} is {observed_len} characters (expected {expected_len}), a \
             {diff}-character gap too large to attribute to a trailing-filler miscount"
        ));
    }
    let mut normalized = core.to_string();
    while normalized.chars().count() < expected_len {
        normalized.push('<');
    }
    Ok(normalized)
}

/// The broad document category for a corpus row.
///
/// `passports`/`id_cards`/`driving_licenses` map directly. Everything else —
/// `misc`, and the old-convention `ocr_fixtures/` directory that stores a
/// handful of specimens directly rather than under one of the three above —
/// falls back to reading the filename itself, since that is the only
/// remaining source a human already reviewed when naming the file. `None`
/// means neither the directory nor the filename says, and a row is skipped
/// rather than guessed.
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

/// A document number rewritten to its positional shape: `A` for an ASCII
/// letter, `9` for a digit, one token per position, matching the notation
/// `work/registry-v0-design.md` tabulated by hand (`A9999999`, `AA9999999`,
/// ...). Every document number seen in the corpus today is plain
/// alphanumeric; a character that is neither is kept literally rather than
/// forced into one of the two buckets, so an unexpected shape stays visible
/// instead of silently miscounted.
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

fn load_jsonl(path: &Path) -> Vec<serde_json::Value> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("every line is one JSON object"))
        .collect()
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/synthpass-ocr → repo root is two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}
