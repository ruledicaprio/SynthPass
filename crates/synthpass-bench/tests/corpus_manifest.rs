//! Validates `samples/corpus.jsonl` against the filenames it describes.
//!
//! # Why this runs without any images
//!
//! `samples/` images are gitignored — they live on the orphan `samples-data`
//! branch — so a fresh clone has the manifest but none of the pictures. Every
//! assertion here therefore reads only the manifest's own `filename` field and
//! the values recorded beside it. That keeps the check always-on in CI instead
//! of quietly skipping wherever the corpus has not been synced.
//!
//! # Why the parsing is duplicated
//!
//! `crates/synthpass-ocr/examples/corpus_manifest.rs` has its own filename
//! parser, and this file deliberately does not share it. A validator built on
//! the producer's parser agrees with the producer by construction and cannot
//! catch a bug in it; an independent re-derivation can. The convention is small
//! enough that the duplication is cheaper than the blind spot.
//!
//! The convention, for passports:
//!
//! ```text
//! <country>_<type>_<Specimen|Private>_<code>_<state>_<year>[_redacted]_<mrz|no_mrz>[_variant…].<ext>
//! ```
//!
//! `id_cards/` and `driving_licenses/` predate the code/state slots and are
//! checked only for the claims their names actually make.

use std::path::{Path, PathBuf};

use serde_json::Value;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn manifest_rows() -> Vec<Value> {
    let path = repo_root().join("samples").join("corpus.jsonl");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("samples/corpus.jsonl is tracked and must exist: {e}"));
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("every manifest line is one JSON object"))
        .collect()
}

fn stem_of(filename: &str) -> &str {
    filename.rsplit_once('.').map_or(filename, |(s, _)| s)
}

/// Filenames substitute `0` for the MRZ filler `<`, which a path cannot hold.
fn to_mrz_chars(token: &str) -> String {
    token.replace('0', "<")
}

/// The code/state pair, when the name carries one: the two tokens immediately
/// after `Specimen`/`Private`, where the second is exactly three characters.
///
/// `XX`/`XXX` are the corpus's sentinels for "not knowable from this image"
/// (Algeria has no MRZ; Armenia is an all-`X` blank template), so they are
/// recognised but never treated as real MRZ characters.
fn claimed_code_and_state(stem: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = stem.split('_').collect();
    let i = parts
        .iter()
        .position(|p| p.eq_ignore_ascii_case("Specimen") || p.eq_ignore_ascii_case("Private"))?;
    let code = parts.get(i + 1)?;
    let state = parts.get(i + 2)?;
    let code_ok = (2..=3).contains(&code.len())
        && code
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    let state_ok = state.len() == 3
        && state
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    if code_ok && state_ok {
        Some((to_mrz_chars(code), to_mrz_chars(state)))
    } else {
        None
    }
}

fn claims_no_mrz(stem: &str) -> bool {
    stem.to_ascii_lowercase().contains("no_mrz")
}

#[test]
fn every_manifest_row_has_the_required_shape() {
    let rows = manifest_rows();
    assert!(!rows.is_empty(), "manifest must not be empty");
    for row in &rows {
        let name = row["filename"]
            .as_str()
            .expect("every row has a string filename");
        assert!(
            !name.is_empty() && !name.contains('/') && !name.contains('\\'),
            "{name}: filename must be a bare basename"
        );
        for key in ["dir", "sha256", "mrz", "year", "variants"] {
            assert!(
                row.get(key).is_some(),
                "{name}: manifest row is missing `{key}`"
            );
        }
        let sha = row["sha256"].as_str().unwrap_or_default();
        assert_eq!(sha.len(), 64, "{name}: sha256 must be 64 hex characters");
        assert!(
            sha.chars().all(|c| c.is_ascii_hexdigit()),
            "{name}: sha256 must be hex"
        );
    }
}

#[test]
fn basenames_are_unique_across_the_whole_corpus() {
    // Every fixture lookup in this repo resolves by basename through a
    // recursive walk of `samples/`, so two files sharing one anywhere in the
    // tree would make which image a test reads depend on directory order.
    let rows = manifest_rows();
    let mut seen = std::collections::BTreeMap::new();
    for row in &rows {
        let name = row["filename"].as_str().unwrap_or_default().to_string();
        let dir = row["dir"].as_str().unwrap_or_default().to_string();
        if let Some(first) = seen.insert(name.clone(), dir.clone()) {
            panic!("{name} appears in both {first}/ and {dir}/ — basenames must be unique");
        }
    }
}

#[test]
fn the_document_code_and_issuing_state_agree_with_the_filename() {
    for row in &manifest_rows() {
        let name = row["filename"].as_str().unwrap_or_default();
        let Some((code, state)) = claimed_code_and_state(stem_of(name)) else {
            continue;
        };
        // Sentinel names assert nothing about real MRZ characters.
        if code.starts_with('X') || state.starts_with('X') {
            continue;
        }
        let recorded_code = row["mrz"]["document_code"].as_str();
        let recorded_state = row["mrz"]["issuing_state"].as_str();
        assert_eq!(
            recorded_code,
            Some(code.as_str()),
            "{name}: filename claims document code {code:?}, manifest records {recorded_code:?}"
        );
        assert_eq!(
            recorded_state,
            Some(state.as_str()),
            "{name}: filename claims issuing state {state:?}, manifest records {recorded_state:?}"
        );
    }
}

#[test]
fn a_document_code_is_two_mrz_characters() {
    // `PE0_ARG` is the case this exists to catch: three characters in a
    // two-character field, which no positional parser can read correctly.
    for row in &manifest_rows() {
        let name = row["filename"].as_str().unwrap_or_default();
        let Some(code) = row["mrz"]["document_code"].as_str() else {
            continue;
        };
        assert_eq!(
            code.chars().count(),
            2,
            "{name}: document code {code:?} is not two characters — ICAO 9303 line 1 \
             positions 1-2. If the document really is non-conformant, record that in \
             `notes` and leave the field at the two characters actually printed."
        );
    }
}

#[test]
fn an_issuing_state_is_three_mrz_characters() {
    for row in &manifest_rows() {
        let name = row["filename"].as_str().unwrap_or_default();
        let Some(state) = row["mrz"]["issuing_state"].as_str() else {
            continue;
        };
        assert_eq!(
            state.chars().count(),
            3,
            "{name}: issuing state {state:?} is not three characters — ICAO 9303 line 1 \
             positions 3-5"
        );
    }
}

#[test]
fn no_mrz_images_do_not_claim_mrz_content() {
    // A name tagged `no_mrz` cannot also carry a document code read off the
    // MRZ: whatever the code says, it did not come from this image.
    for row in &manifest_rows() {
        let name = row["filename"].as_str().unwrap_or_default();
        if !claims_no_mrz(stem_of(name)) {
            continue;
        }
        assert_eq!(
            row["mrz"]["present"].as_bool(),
            Some(false),
            "{name}: filename is tagged no_mrz but the manifest records mrz.present = true"
        );
    }
}

#[test]
fn recorded_mrz_presence_matches_the_filename_tag() {
    for row in &manifest_rows() {
        let name = row["filename"].as_str().unwrap_or_default();
        let stem = stem_of(name);
        let lower = stem.to_ascii_lowercase();
        if !lower.contains("mrz") {
            continue;
        }
        let tagged_present = !claims_no_mrz(stem);
        assert_eq!(
            row["mrz"]["present"].as_bool(),
            Some(tagged_present),
            "{name}: filename tags mrz presence as {tagged_present}, manifest disagrees"
        );
    }
}

#[test]
fn the_year_recorded_matches_the_year_in_the_filename() {
    for row in &manifest_rows() {
        let name = row["filename"].as_str().unwrap_or_default();
        let claimed: Option<u64> = stem_of(name)
            .split('_')
            .find(|p| p.len() == 4 && p.chars().all(|c| c.is_ascii_digit()))
            .and_then(|p| p.parse().ok());
        let recorded = row["year"]["value"].as_u64();
        assert_eq!(
            recorded, claimed,
            "{name}: filename year {claimed:?}, manifest records {recorded:?}"
        );
    }
}

#[test]
fn every_year_kind_is_stated_explicitly() {
    // The convention says "issue year", but at least one specimen was named
    // for its expiry instead (Germany's BDR passport: issued 2008, expires
    // 2018, filed as 2018). The field exists so that ambiguity is recorded
    // rather than assumed.
    for row in &manifest_rows() {
        let name = row["filename"].as_str().unwrap_or_default();
        if row["year"]["value"].is_null() {
            continue;
        }
        let kind = row["year"]["kind"].as_str();
        assert!(
            matches!(kind, Some("issue") | Some("expiry")),
            "{name}: year.kind must be \"issue\" or \"expiry\", got {kind:?}"
        );
    }
}

#[test]
fn provenance_is_one_of_the_known_classes() {
    // `template` is the all-`X` blank form (Armenia); it tests robustness, not
    // extraction accuracy, and must not be confused with a filled specimen.
    for row in &manifest_rows() {
        let name = row["filename"].as_str().unwrap_or_default();
        let p = row["provenance"].as_str();
        assert!(
            matches!(p, Some("specimen") | Some("private") | Some("template")),
            "{name}: provenance must be specimen, private or template, got {p:?}"
        );
    }
}

#[test]
fn the_directory_agrees_with_the_document_type_in_the_name() {
    // `synthpass_bench::classify_specimen` reads the document class off the
    // immediate parent directory and never the filename, deliberately — a
    // directory is stable under a rename. That only holds while the two agree.
    // A reorganisation once left passport specimens under `id_cards/`, where
    // they classified as ID cards and made `--format passport` under-count.
    for row in &manifest_rows() {
        let name = row["filename"].as_str().unwrap_or_default();
        let dir = row["dir"].as_str().unwrap_or_default();
        let parts: Vec<&str> = stem_of(name).split('_').collect();
        let claimed = if parts.contains(&"Passport") {
            Some("passports")
        } else if parts.contains(&"ID") {
            Some("id_cards")
        } else {
            None
        };
        // `ocr_fixtures/` and `misc/` deliberately mix formats.
        if dir == "ocr_fixtures" || dir == "misc" {
            continue;
        }
        if let Some(expected) = claimed {
            assert_eq!(
                dir, expected,
                "{name}: the name says {expected}, but the file is filed under {dir}/ — \
                 classify_specimen reads the directory, so it would report the wrong class"
            );
        }
    }
}

#[test]
fn an_unrecognized_issuing_state_carries_a_note_explaining_why() {
    // Kosovo's RKS, Germany's specimen-only BDR, and the genuinely
    // non-conformant line-1 documents all land here. Each is legitimate, and
    // each needs a written reason — otherwise an unrecognized state is
    // indistinguishable from an unnoticed typo.
    for row in &manifest_rows() {
        let name = row["filename"].as_str().unwrap_or_default();
        if row["mrz"]["issuing_state"].is_null() {
            continue;
        }
        if row["mrz"]["issuing_state_recognized"].as_bool() != Some(false) {
            continue;
        }
        let note = row["notes"].as_str().unwrap_or_default();
        assert!(
            !note.trim().is_empty(),
            "{name}: issuing state {} does not resolve against mrz::country_name, so the \
             manifest must say why in `notes`",
            row["mrz"]["issuing_state"]
        );
    }
}

#[test]
fn a_no_mrz_specimen_never_records_a_checksum_valid_read() {
    // Passing every check digit by chance is vanishingly unlikely, so a
    // checksum-valid read off a page tagged `no_mrz` is near-proof the *tag* is
    // wrong -- not that the parser hallucinated.
    //
    // `Monaco_ID_Specimen_XXXX_front_no_mrz.png` is why this test exists. It
    // read `IC`/`MCO` with valid checksums, which is precisely what a correct
    // read of a Monaco identity card looks like; the file was a back side filed
    // under a front-side name. It was first reported as a Tier-1 false
    // positive, and it was not one. A structurally-parseable read whose check
    // digits *fail* is the opposite case and stays allowed -- 18 specimens do
    // that, because visual-zone text lands in an MRZ-shaped line routinely.
    for row in &manifest_rows() {
        let name = row["filename"].as_str().unwrap_or_default();
        if row["mrz"]["present"].as_bool() != Some(false) {
            continue;
        }
        assert_ne!(
            row["mrz"]["observed"]["checksums_valid"].as_bool(),
            Some(true),
            "{name}: tagged no_mrz but a checksum-valid MRZ was read from it — the file is              almost certainly the other side of the document, or otherwise mislabelled"
        );
    }
}
