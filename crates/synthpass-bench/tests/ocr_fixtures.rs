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

use serde::Deserialize;

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

/// Every fixture whose printed zone validates (`mrz_checksums_valid: true`)
/// carries `surname`/`given_names` equal to what `mrz` itself parses from
/// that same fixture's `mrz_line` — using `mrz::find_and_parse`, the same
/// entry point every other consumer of a printed zone in this workspace
/// uses, never a hand-rolled re-split of the name.
///
/// This is ADR-0013's guardrail
/// (`knowledge/decisions/ADR-0013-names-are-scored-against-mrz-form-truth.md`):
/// name accuracy is scored only against MRZ-form truth (`mrz`'s own split at
/// `<<`, filler read as a space), never against the visual-zone (VIZ) form —
/// mixed case, native punctuation, a human's own idea of where the name
/// splits. A hand transcription is exactly where a VIZ-form split can creep
/// back into an MRZ-form fixture unnoticed: Argentina 2026's printed zone
/// has no `<<` at all, and its `surname`/`given_names` were transcribed the
/// way the visual zone splits them — but that zone also fails its own check
/// digits (`mrz_checksums_valid: false`), so it never reaches this test.
/// Skipping a non-validating zone is deliberate, not a gap: `mrz` cannot be
/// asked to split a name inside a zone it does not consider well-formed
/// enough to validate, and ADR-0013's own guardrail is scoped to the
/// fixtures the strict-name-hit-rate metric can actually score.
///
/// Model-free and fast: no OCR, no image, just the tracked `.json` files and
/// `mrz::find_and_parse`.
#[test]
fn every_validating_fixtures_names_equal_mrzs_own_parse() {
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
        let claimed_valid = v["mrz_checksums_valid"]
            .as_bool()
            .unwrap_or_else(|| panic!("{stem}: fixture has no bool `mrz_checksums_valid`"));
        // Only a validating zone is in scope — see this test's doc comment
        // for why a non-conforming zone (Argentina 2026) is excluded by
        // design rather than by exception-listing its filename.
        if !claimed_valid {
            continue;
        }

        let mrz_line = v["mrz_line"]
            .as_str()
            .unwrap_or_else(|| panic!("{stem}: fixture has no string `mrz_line`"));
        let fixture_surname = v["surname"]
            .as_str()
            .unwrap_or_else(|| panic!("{stem}: fixture has no string `surname`"));
        let fixture_given = v["given_names"]
            .as_str()
            .unwrap_or_else(|| panic!("{stem}: fixture has no string `given_names`"));

        let parsed = mrz::find_and_parse(mrz_line).unwrap_or_else(|e| {
            panic!("{stem}: mrz_checksums_valid is true but find_and_parse(mrz_line) failed: {e:?}")
        });

        checked += 1;
        if parsed.surname != fixture_surname || parsed.given_names != fixture_given {
            mismatches.push(format!(
                "{stem}: fixture surname={fixture_surname:?} given_names={fixture_given:?}, but \
                 mrz::find_and_parse(mrz_line) split surname={:?} given_names={:?}",
                parsed.surname, parsed.given_names
            ));
        }
    }

    assert!(
        checked >= 40,
        "expected at least 40 validating fixtures, found {checked}"
    );
    assert!(
        mismatches.is_empty(),
        "fixture(s) whose surname/given_names disagree with mrz's own parse of their mrz_line \
         (a VIZ-form split may have crept into an MRZ-form fixture — see ADR-0013):\n  {}",
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

#[derive(Deserialize)]
struct ManifestFixtureRow {
    dir: String,
    filename: String,
    #[serde(default)]
    ground_truth_stem: Option<String>,
    #[serde(default)]
    expected_document_number: Option<String>,
}

fn manifest_fixture_agreement() -> Result<(), String> {
    let fixtures = fixtures_dir();
    let corpus = fixtures
        .parent()
        .ok_or("samples directory unavailable")?
        .join("corpus.jsonl");
    let source = fs::read_to_string(&corpus)
        .map_err(|error| format!("read {}: {error}", corpus.display()))?;
    let mut verdicts = [0usize; 2];
    for (line_number, line) in source.lines().enumerate() {
        let row: ManifestFixtureRow = serde_json::from_str(line).map_err(|error| {
            format!(
                "parse {} line {}: {error}",
                corpus.display(),
                line_number + 1
            )
        })?;
        let Some(stem) = row.ground_truth_stem else {
            continue;
        };
        let fixture_path = fixtures.join(format!("{stem}.json"));
        let fixture_source = fs::read_to_string(&fixture_path).map_err(|error| {
            format!(
                "samples/corpus.jsonl disagrees with the fixture for {stem}: {error}. Regenerate the manifest (cargo run -p synthpass-ocr --example corpus_manifest) and commit the result."
            )
        })?;
        let fixture: serde_json::Value = serde_json::from_str(&fixture_source)
            .map_err(|error| format!("parse {}: {error}", fixture_path.display()))?;
        let valid = fixture["mrz_checksums_valid"].as_bool().ok_or_else(|| {
            format!(
                "{}: fixture has no boolean mrz_checksums_valid",
                fixture_path.display()
            )
        })?;
        let consistent = if valid {
            row.expected_document_number.as_deref()
                == fixture["document_number"]
                    .as_str()
                    .filter(|number| !number.is_empty())
        } else {
            row.expected_document_number.is_none()
        };
        if !consistent {
            return Err(format!(
                "samples/corpus.jsonl disagrees with the fixture for {stem}: a checksum-valid fixture implies a recorded expected_document_number (a non-conforming one implies null). Regenerate the manifest (cargo run -p synthpass-ocr --example corpus_manifest) and commit the result."
            ));
        }
        verdicts[usize::from(valid)] += 1;
    }
    if verdicts.into_iter().any(|count| count == 0) {
        return Err("expected both checksum-valid and non-conforming reviewed fixtures".into());
    }
    Ok(())
}

#[test]
fn manifest_records_fixture_document_number_contract() {
    match manifest_fixture_agreement() {
        Ok(()) => {}
        Err(error) => panic!("{error}"),
    }
}
