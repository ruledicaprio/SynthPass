//! Real-model end-to-end smoke test. Ignored by default (needs the two
//! `.rten` files at the repo root); run explicitly with:
//!
//! ```sh
//! cargo test -p synthpass-ocr --test native_ocr_e2e -- --ignored
//! ```

use std::path::{Path, PathBuf};
use synthpass_ocr::NativeOcr;

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

#[test]
#[ignore]
fn native_ocr_recognizes_mrz_fragment_from_sample_passport() {
    let (detection_path, recognition_path) = require_models();
    let ocr = NativeOcr::load(&detection_path, &recognition_path).expect("models load");

    let image_path = find_sample("Canada_Passport_Specimen_2023_mrz.jpg");
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

    let source_path = find_sample("Canada_Passport_Specimen_2023_mrz.jpg");
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

/// ADR-0008 chunk 2, layer 1: the corpus specimens that are genuinely
/// photographed sideways must come back with a **checksum-valid** MRZ.
///
/// These three are the documents the chunk-2 sweep verified are actually
/// rotated — by opening them, not by trusting a `_rotated` filename, which on
/// this corpus is wrong at least once (`North_Macedonia_…_mrz_rotated` is
/// upright, and its MRZ reads at 0°). Angola is the awkward one on purpose: it
/// is a two-page spread whose facing page is horizontal while the biodata page
/// is turned, so no whole-page orientation vote can be right about it. That is
/// precisely why this passes now — nothing votes. Both quarter-turns are tried
/// late in the retry chain and an ICAO check digit decides.
///
/// Asserted on **validity, not content**: a checksum-valid parse is the
/// property that matters, and pinning MRZ characters in a non-fixture file
/// would put specimen data somewhere it does not belong. The reported rotation
/// is asserted as non-zero rather than as a specific angle — which turn wins is
/// evidence's business, and Pakistan already resolved at 270° where reading the
/// image predicted 90°.
#[test]
#[ignore]
fn native_ocr_reads_the_genuinely_sideways_specimens() {
    let (detection_path, recognition_path) = require_models();
    let ocr = NativeOcr::load(&detection_path, &recognition_path).expect("models load");

    for name in [
        "Indonesia_Passport_Specimen_P0_IDN_2024_mrz_rotated.png",
        "Pakistan_Passport_Specimen_P0_PAK_2024_mrz_rotated.png",
        "Angola_Passport_Specimen_PN_AGO_2026_mrz_rotated_counterclockwise_90_deg.png",
    ] {
        let page = ocr
            .recognize_detailed(&find_sample(name))
            .unwrap_or_else(|e| panic!("{name}: recognition failed: {e}"));

        assert!(
            mrz::find_and_parse(&page.text).is_ok_and(|d| d.valid()),
            "{name}: expected a checksum-valid MRZ once the page is turned; \
             reported rotation {}°",
            page.rotation
        );
        assert_ne!(
            page.rotation, 0,
            "{name}: a sideways page that reads must report the turn it took to read it"
        );
    }
}

/// ADR-0008 chunk 2, layer 1: a page handed to the engine a quarter-turn off
/// must come back with the same MRZ fragment as the upright original, and must
/// say how far it turned it to get there.
///
/// The synthetic counterpart to the corpus test above. That one proves three
/// real specimens read; this one proves the property holds for a document whose
/// upright answer is already known, in **both** directions — 90° and 270° are
/// separate entries in `ROTATION_RETRY_TURNS`, and a transposition bug that
/// swaps them still passes if only one is checked.
///
/// Asserted on the outcome, not the mechanism: whether the recovery comes from
/// the retry chain (today) or from a future layer is not this test's business,
/// which is what keeps it meaningful once layers 2 and 3 land.
#[test]
#[ignore]
fn native_ocr_recovers_mrz_from_a_quarter_turned_page() {
    let (detection_path, recognition_path) = require_models();
    let ocr = NativeOcr::load(&detection_path, &recognition_path).expect("models load");

    let source_path = find_sample("Canada_Passport_Specimen_2023_mrz.jpg");
    let upright = image::open(&source_path)
        .expect("sample image opens")
        .into_rgb8();

    for (turns, expected_correction) in [(1u8, 270u16), (3u8, 90u16)] {
        let turned = match turns {
            1 => image::imageops::rotate90(&upright),
            _ => image::imageops::rotate270(&upright),
        };
        let turned_path = std::env::temp_dir().join(format!(
            "synthpass_ocr_test_canadian_passport_{turns}turn.png"
        ));
        turned.save(&turned_path).expect("turned fixture writes");

        let page = ocr
            .recognize_detailed(&turned_path)
            .expect("recognition succeeds on the turned fixture");
        let _ = std::fs::remove_file(&turned_path);

        let upper = page.text.to_uppercase();
        assert!(
            upper.contains("SARAH") || upper.contains("CAN"),
            "a page turned {}° should still recover the upright MRZ fragment, got: {}",
            u16::from(turns) * 90,
            page.text
        );
        assert_eq!(
            page.rotation,
            expected_correction,
            "a page turned {}° needs a {expected_correction}° correction to read upright; \
             reported {}",
            u16::from(turns) * 90,
            page.rotation
        );
    }
}
