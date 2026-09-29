//! Real-model end-to-end smoke test. Ignored by default (needs the two
//! `.rten` files at the repo root); run explicitly with:
//!
//! ```sh
//! cargo test -p synthpass-ocr --test native_ocr_e2e -- --ignored
//! ```

use std::path::{Path, PathBuf};
use synthpass_ocr::{NativeOcr, PassOutcome, PassRecord, PassTransform};

fn require_models() -> (PathBuf, PathBuf) {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut detection_path = repo_root.join("text-detection.rten");
    let mut recognition_path = repo_root.join("text-recognition.rten");
    // Fall back to `SYNTHPASS_OCR_MODEL_DIR` (the variable every other entry
    // point reads) when the models are not at the repo root, so a machine that
    // keeps them in one shared directory does not need a copy in each checkout.
    if !(detection_path.exists() && recognition_path.exists()) {
        if let Some(dir) = std::env::var_os("SYNTHPASS_OCR_MODEL_DIR") {
            let dir = PathBuf::from(dir);
            if dir.join("text-detection.rten").exists()
                && dir.join("text-recognition.rten").exists()
            {
                detection_path = dir.join("text-detection.rten");
                recognition_path = dir.join("text-recognition.rten");
            }
        }
    }
    assert!(
        detection_path.exists() && recognition_path.exists(),
        "model files not found at {} — download them first (see synthpass_ocr::download)",
        repo_root.display()
    );
    (detection_path, recognition_path)
}

/// Locates `name` (a bare filename, no path) anywhere under `samples/`,
/// searching recursively — so this survives `samples/` being reorganized
/// into continent/class subfolders without every call site needing the
/// exact subpath hardcoded.
fn find_sample(name: &str) -> PathBuf {
    fn search(dir: &Path, name: &str) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = search(&path, name) {
                    return Some(found);
                }
            } else if path.file_name().and_then(|f| f.to_str()) == Some(name) {
                return Some(path);
            }
        }
        None
    }
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    search(&repo_root.join("samples"), name)
        .unwrap_or_else(|| panic!("sample file not found anywhere under samples/: {name}"))
}

#[test]
#[ignore]
fn native_ocr_recognizes_mrz_fragment_from_sample_passport() {
    let (detection_path, recognition_path) = require_models();
    let ocr = NativeOcr::load(&detection_path, &recognition_path).expect("models load");

    let image_path = find_sample("Canada_Passport_Specimen_PP_CAN_2023_mrz_wide.jpg");
    let text = ocr.recognize(&image_path).expect("recognition succeeds");

    assert!(!text.is_empty(), "expected non-empty OCR output");
    // The sample is a published specimen passport; its MRZ line contains
    // "CAN" (Canada's ICAO nationality code) and surname "MARTIN".
    let upper = text.to_uppercase();
    assert!(
        upper.contains("SARAH") || upper.contains("CAN"),
        "expected a recognizable MRZ fragment in OCR output, got: {text}"
    );
}

/// M5 A3 tie-break: a page photographed/scanned fully upside-down (180°,
/// which `choose_rotation` alone cannot distinguish from upright — see its
/// doc comment) must still recover the same MRZ fragment as the upright
/// original, via the `mrz_band`-driven tie-break in `recognize_detailed`
/// (see `band_in_upper_third`). Ignored by default alongside the other
/// real-model test above; writes the rotated fixture next to the source
/// sample so this stays a single self-contained image-crate round trip, no
/// new dependency for temp-file handling.
#[test]
#[ignore]
fn native_ocr_recovers_mrz_from_a_180_degree_rotated_page() {
    let (detection_path, recognition_path) = require_models();
    let ocr = NativeOcr::load(&detection_path, &recognition_path).expect("models load");

    let source_path = find_sample("Canada_Passport_Specimen_PP_CAN_2023_mrz_wide.jpg");
    let upright = image::open(&source_path)
        .expect("sample image opens")
        .into_rgb8();
    let flipped = image::imageops::rotate180(&upright);

    let rotated_path = std::env::temp_dir().join("synthpass_ocr_test_canadian_passport_180.png");
    flipped.save(&rotated_path).expect("rotated fixture writes");

    let page = ocr
        .recognize_detailed(&rotated_path)
        .expect("recognition succeeds on the rotated fixture");

    let _ = std::fs::remove_file(&rotated_path);

    let upper = page.text.to_uppercase();
    assert!(
        upper.contains("SARAH") || upper.contains("CAN"),
        "expected the 180°-rotated fixture to recover the same MRZ fragment \
         as the upright original, got: {}",
        page.text
    );
    assert_eq!(
        page.rotation, 180,
        "expected the tie-break to report a 180° correction, got {}",
        page.rotation
    );
}

/// Which images the pass-trace tests read. Each `retry_stop` is the one
/// `knowledge/benchmarks/real-specimen-outcomes.jsonl` records for the specimen.
///
/// - China 2012: the general pass validates, so the trace is one accepted
///   `general` record.
/// - Canada 2023: a retry variant validates on the first retry (`pass-00`).
/// - UAE 2011: a retry variant validates late (`pass-03`), so three retry
///   passes were appended before the accepting one and the page-text tail is
///   made of several passes.
const GENERAL_VALID_SAMPLE: &str = "China_Passport_Specimen_P0_CHN_2012_mrz.png";
const VARIANT_VALID_SAMPLE: &str = "Canada_Passport_Specimen_PP_CAN_2023_mrz_wide.jpg";
const LATE_VARIANT_VALID_SAMPLE: &str =
    "United_Arab_Emirates_Passport_Specimen_P0_ARE_2011_mrz.jpg";

/// `recognize_detailed_traced` returns exactly the page `recognize_detailed`
/// does, and the records satisfy the trace's own invariants.
fn assert_trace_is_faithful(sample: &str) {
    let (detection_path, recognition_path) = require_models();
    let ocr = NativeOcr::load(&detection_path, &recognition_path).expect("models load");
    let path = find_sample(sample);

    let plain = ocr.recognize_detailed(&path).expect("recognition succeeds");
    let (page, records) = ocr
        .recognize_detailed_traced(&path)
        .expect("traced recognition succeeds");
    assert_eq!(
        page, plain,
        "{sample}: the traced entry point must return the same page"
    );
    assert_trace_invariants(sample, &page, &records);
}

fn assert_trace_invariants(sample: &str, page: &synthpass_ocr::OcrPage, records: &[PassRecord]) {
    // Contiguous from 0, with ids in the `retry_variant_id` vocabulary.
    assert!(
        !records.is_empty(),
        "{sample}: the general pass always runs"
    );
    for (n, record) in records.iter().enumerate() {
        assert_eq!(record.order, n, "{sample}: orders are contiguous from 0");
        let expected_id = if n == 0 {
            "general".to_string()
        } else {
            format!("pass-{:02}", n - 1)
        };
        assert_eq!(record.id, expected_id, "{sample}: id matches order");
        assert_eq!(
            record.transform == PassTransform::General,
            n == 0,
            "{sample}: only the general pass carries the general transform"
        );
        assert!(
            record.image_width > 0 && record.image_height > 0,
            "{sample}: every pass reads a non-empty image"
        );
        assert_eq!(
            record.outcome == PassOutcome::Failed
                || record.outcome == PassOutcome::NoMrzShapedLines,
            record.readings.is_empty(),
            "{sample}: {}: a pass that read nothing MRZ-shaped has no readings, and vice versa",
            record.id
        );
        for reading in &record.readings {
            assert_eq!(reading.text, reading.text.trim());
            assert!(reading.text.chars().filter(|c| !c.is_whitespace()).count() >= 20);
        }
    }

    // Exactly one accepted pass when the loop stopped on a valid reading, and
    // it is the last one, named by `retry_variant_id`; none otherwise.
    let accepted: Vec<&PassRecord> = records
        .iter()
        .filter(|r| r.outcome == PassOutcome::Accepted)
        .collect();
    match page.retry_stop.as_deref() {
        Some("general_valid") | Some("variant_valid") => {
            assert_eq!(accepted.len(), 1, "{sample}: exactly one accepted pass");
            assert_eq!(
                accepted[0].id,
                *page
                    .retry_variant_id
                    .as_ref()
                    .expect("a valid stop names its pass"),
                "{sample}: the accepted pass is the one retry_variant_id names"
            );
            assert_eq!(
                accepted[0].order,
                records.len() - 1,
                "{sample}: the accepted pass is the last one that ran"
            );
        }
        other => assert!(
            accepted.is_empty(),
            "{sample}: retry_stop {other:?} must have no accepted pass"
        ),
    }

    // The retry passes' lines are the tail of the page text, in order.
    let tail: String = records
        .iter()
        .skip(1)
        .filter(|r| matches!(r.outcome, PassOutcome::Appended | PassOutcome::Accepted))
        .map(|r| {
            let lines: Vec<&str> = r.readings.iter().map(|l| l.text.as_str()).collect();
            format!("\n{}", lines.join("\n"))
        })
        .collect();
    assert!(
        page.text.ends_with(&tail),
        "{sample}: the appended and accepted passes' readings must be the tail of the page text"
    );
}

#[test]
#[ignore]
fn traced_ocr_matches_untraced_on_a_general_valid_specimen() {
    assert_trace_is_faithful(GENERAL_VALID_SAMPLE);
}

#[test]
#[ignore]
fn traced_ocr_matches_untraced_on_a_variant_valid_specimen() {
    assert_trace_is_faithful(VARIANT_VALID_SAMPLE);
}

#[test]
#[ignore]
fn traced_ocr_matches_untraced_when_a_late_variant_validates() {
    assert_trace_is_faithful(LATE_VARIANT_VALID_SAMPLE);
}

/// A blank page has nothing to read on any pass: every retry pass that runs
/// records no MRZ-shaped line, no pass is accepted, and the page is still the
/// one the untraced entry point returns.
#[test]
#[ignore]
fn traced_ocr_on_a_blank_page_accepts_no_pass() {
    let (detection_path, recognition_path) = require_models();
    let ocr = NativeOcr::load(&detection_path, &recognition_path).expect("models load");
    let path = std::env::temp_dir().join(format!(
        "synthpass_ocr_test_blank_page_{}.png",
        std::process::id()
    ));
    image::RgbImage::from_pixel(800, 600, image::Rgb([255, 255, 255]))
        .save(&path)
        .expect("blank fixture writes");

    let plain = ocr.recognize_detailed(&path);
    let traced = ocr.recognize_detailed_traced(&path);
    let _ = std::fs::remove_file(&path);
    let plain = plain.expect("recognition succeeds");
    let (page, records) = traced.expect("traced recognition succeeds");

    assert_eq!(page, plain);
    assert_trace_invariants("blank page", &page, &records);
    assert!(records.len() > 1, "the retry passes ran");
    assert!(records.iter().all(|r| r.readings.is_empty()));
}
