//! Real-model end-to-end smoke test. Ignored by default (needs the two
//! `.rten` files at the repo root); run explicitly with:
//!
//! ```sh
//! cargo test -p synthpass-ocr --test native_ocr_e2e -- --ignored
//! ```

use std::path::{Path, PathBuf};
use synthpass_ocr::NativeOcr;

/// Extensions treated as corpus images — the same list
/// `examples/corpus_manifest.rs` uses for the same walk.
const IMAGE_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "gif"];

fn require_models() -> (PathBuf, PathBuf) {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let detection_path = repo_root.join("text-detection.rten");
    let recognition_path = repo_root.join("text-recognition.rten");
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

/// Every image file under `samples/`, recursively — whatever subset of the
/// full `samples-data` corpus happens to be checked out locally (a fresh
/// clone carries only the small tracked fixture set under
/// `samples/passports/`; CI's real-specimen job syncs the rest). Never
/// empty in CI or after `scripts/sync-samples.ps1`.
fn walk_sample_images() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.as_str()))
            {
                out.push(path);
            }
        }
    }
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut out = Vec::new();
    walk(&repo_root.join("samples"), &mut out);
    out
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

/// #508: `OcrPage::accepted_mrz_text`'s documented contract, checked against
/// the real retry loop over every image under `samples/` (whatever subset is
/// checked out locally — CI's real-specimen job syncs the full corpus; a
/// fresh clone still has the small tracked `samples/passports/` set, which is
/// never empty).
///
/// This is the real-model complement to
/// `crates/mrz/tests/issue_508_accepted_pass_regression.rs`, which pins the
/// same contract's *mechanism* (why the concatenation refuses and the
/// accepted text alone doesn't) against a captured synthetic text dump —
/// this test instead proves the field the retry loop actually populates
/// today satisfies that contract, on real images, without needing a captured
/// dump to stay in sync with the loop's own logic.
#[test]
#[ignore]
fn native_ocr_508_accepted_mrz_text_matches_the_retry_loops_own_decision() {
    let (detection_path, recognition_path) = require_models();
    let ocr = NativeOcr::load(&detection_path, &recognition_path).expect("models load");

    let images = walk_sample_images();
    assert!(
        !images.is_empty(),
        "no sample images found under samples/ — a fresh clone should still have \
         samples/passports/"
    );

    for path in &images {
        let name = path.display().to_string();
        let page = ocr
            .recognize_detailed(path)
            .unwrap_or_else(|e| panic!("{name}: recognition failed: {e}"));

        assert_eq!(
            page.accepted_mrz_text.is_some(),
            page.retry_variant_id.is_some(),
            "{name}: accepted_mrz_text.is_some() must equal retry_variant_id.is_some() \
             (retry_stop={:?})",
            page.retry_stop
        );

        match page.retry_stop.as_deref() {
            Some("general_valid") => {
                assert_eq!(
                    page.accepted_mrz_text.as_deref(),
                    Some(page.text.as_str()),
                    "{name}: general_valid must carry exactly the general pass's own text"
                );
            }
            Some("variant_valid") => {
                let accepted = page.accepted_mrz_text.as_deref().unwrap_or_else(|| {
                    panic!("{name}: variant_valid must carry accepted_mrz_text")
                });
                assert!(
                    page.text.ends_with(&format!("\n{accepted}")),
                    "{name}: the full text must end with exactly the accepted variant's \
                     own candidate lines"
                );
                assert!(
                    mrz::find_and_parse(accepted).is_ok_and(|d| d.valid()),
                    "{name}: the accepted variant's own text must parse to a valid MRZ \
                     on its own, without the rest of the retry loop's candidate lines"
                );
            }
            _ => {}
        }
    }
}
