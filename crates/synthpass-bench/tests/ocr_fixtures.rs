//! Every hand-verified ground-truth fixture in `samples/ocr_fixtures/` is
//! internally consistent: feeding its recorded `mrz_line` back through
//! `mrz::find_and_parse` reproduces the `mrz_checksums_valid` it claims.
//!
//! This is the guard the checksum_failed track's step 3 needs: the 23
//! specimens labelled in that pass carry the *true printed* MRZ, and 16 of
//! them are non-conforming by design (`mrz_checksums_valid: false`). A typo in
//! one of those zones would silently turn a "the specimen is broken" label
//! into a "the OCR misread it" one, which is exactly the distinction the
//! labels exist to draw. Reads only the tracked `.json` files — no images.

use std::fs;
use std::path::{Path, PathBuf};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .join("samples/ocr_fixtures")
}

#[test]
fn every_fixture_mrz_line_reproduces_its_recorded_checksum_verdict() {
    let dir = fixtures_dir();
    let mut checked = 0;
    let mut mismatches = Vec::new();

    for entry in fs::read_dir(&dir).expect("samples/ocr_fixtures must exist") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("{}: not valid JSON ({e})", path.display()));

        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let mrz_line = v["mrz_line"]
            .as_str()
            .unwrap_or_else(|| panic!("{stem}: fixture has no string `mrz_line`"));
        let claimed = v["mrz_checksums_valid"]
            .as_bool()
            .unwrap_or_else(|| panic!("{stem}: fixture has no bool `mrz_checksums_valid`"));

        let parsed_valid = mrz::find_and_parse(mrz_line)
            .map(|d| d.valid())
            .unwrap_or(false);

        checked += 1;
        if parsed_valid != claimed {
            mismatches.push(format!(
                "{stem}: mrz_checksums_valid = {claimed}, but find_and_parse(mrz_line).valid() = {parsed_valid}"
            ));
        }
    }

    assert!(
        checked >= 41,
        "expected at least 41 fixtures, found {checked}"
    );
    assert!(
        mismatches.is_empty(),
        "fixture(s) whose mrz_line disagrees with their recorded verdict:\n  {}",
        mismatches.join("\n  ")
    );
}

#[test]
fn every_fixture_json_has_a_paired_md() {
    let dir = fixtures_dir();
    let mut missing = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if !path.with_extension("md").is_file() {
            missing.push(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    assert!(
        missing.is_empty(),
        "fixture .json with no paired .md (parity.rs skips these):\n  {}",
        missing.join("\n  ")
    );
}
