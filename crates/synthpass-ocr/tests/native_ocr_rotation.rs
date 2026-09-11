//! ADR-0008 chunk 2, layer 1 — page-orientation recovery against real models.
//!
//! Deliberately **not** in `native_ocr_e2e.rs`. That file is a *smoke* suite:
//! `ci.yml` runs it with `SYNTHPASS_OCR_MAX_PASSES=1` to assert the OCR stage
//! executes at all, which breaks the retry loop before any variant runs. Since
//! chunk 2 these assertions need the retry chain — see
//! `has_the_retry_budget_these_tests_need` below — so putting them there makes
//! them fail for a reason that has nothing to do with orientation.
//!
//! Ignored by default (needs the two `.rten` files at the repo root):
//!
//! ```sh
//! cargo test -p synthpass-ocr --release --test native_ocr_rotation -- --ignored
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

/// Locates `name` anywhere under `samples/`, or `None` when it is absent.
///
/// `None` is the ordinary case in CI and in a fresh clone, not an error: only
/// four fixture images under `samples/ocr_fixtures/` are tracked in git. The
/// rest of the corpus lives on the orphan `samples-data` branch and arrives via
/// `scripts/sync-samples.ps1`. A test that needs one of those must **skip**
/// rather than fail, or it reports "the corpus is missing" as "orientation
/// recovery is broken".
fn find_sample(name: &str) -> Option<PathBuf> {
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
}

/// Whether the ambient pass budget lets the retry chain reach its last tier.
///
/// **This encodes a real property of the chunk-2 design, and it is worth being
/// explicit about because it is a cost, not a detail.** Orientation used to be
/// corrected *before* the general pass, so it worked with retries switched off
/// entirely. It is now recovered *by* the retry chain's outermost tier, so a
/// constrained `SYNTHPASS_OCR_MAX_PASSES` — or an exhausted
/// `SYNTHPASS_OCR_MAX_SECONDS` — silently loses sideways pages. CI's smoke
/// configuration (`MAX_PASSES=1`) is exactly that case, and it is how this
/// property was discovered.
///
/// Rather than mutate a process-global env var (these tests run in threads, and
/// one test rewriting the budget under another is its own bug), the tests
/// detect a hostile setting and skip loudly.
fn has_the_retry_budget_these_tests_need() -> bool {
    match std::env::var("SYNTHPASS_OCR_MAX_PASSES") {
        Err(_) => true,
        Ok(raw) => {
            eprintln!(
                "SKIPPED: SYNTHPASS_OCR_MAX_PASSES is set to {raw:?}. Orientation recovery lives \
                 in the retry chain's last tier since ADR-0008 chunk 2, so any override here \
                 changes what is being tested. Unset it to run this."
            );
            false
        }
    }
}

/// The corpus specimens that are genuinely photographed sideways must come back
/// with a **checksum-valid** MRZ.
///
/// These three were verified as actually rotated by opening them, not by
/// trusting a `_rotated` filename — which on this corpus is wrong at least once
/// (`North_Macedonia_…_mrz_rotated` is upright and its MRZ reads at 0°). Angola
/// is the awkward one on purpose: a two-page spread whose facing page is
/// horizontal while the biodata page is turned, so no whole-page orientation
/// vote can be right about it. That is exactly why it passes now — nothing
/// votes; both quarter-turns are tried and an ICAO check digit decides.
///
/// Asserted on **validity, not content**: a checksum-valid parse is the
/// property that matters, and pinning MRZ characters in a non-fixture file
/// would put specimen data somewhere it does not belong. Rotation is asserted
/// non-zero rather than as a specific angle — which turn wins is evidence's
/// business, and Pakistan resolved at 270° where reading the image predicted
/// 90°.
#[test]
#[ignore]
fn native_ocr_reads_the_genuinely_sideways_specimens() {
    if !has_the_retry_budget_these_tests_need() {
        return;
    }
    let (detection_path, recognition_path) = require_models();
    let ocr = NativeOcr::load(&detection_path, &recognition_path).expect("models load");

    let mut checked = 0usize;
    for name in [
        "Indonesia_Passport_Specimen_P0_IDN_2024_mrz_rotated.png",
        "Pakistan_Passport_Specimen_P0_PAK_2024_mrz_rotated.png",
        "Angola_Passport_Specimen_PN_AGO_2026_mrz_rotated_counterclockwise_90_deg.png",
    ] {
        let Some(path) = find_sample(name) else {
            eprintln!("SKIPPED {name}: not present — run scripts/sync-samples.ps1 for the corpus");
            continue;
        };
        let page = ocr
            .recognize_detailed(&path)
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
        checked += 1;
    }

    if checked == 0 {
        eprintln!(
            "SKIPPED: none of the sideways specimens are present locally. This test asserts \
             nothing without the corpus — that is deliberate, but it means a green run here is \
             not evidence."
        );
    }
}

/// The upright documents ADR-0008 chunk 2 recovered must keep reading — and
/// keep reading **at 0°**.
///
/// Each of these was a real-specimen miss on the CI gate report before chunk 2
/// and a hit after it (per-document diff, 2026-09-10 → 2026-09-12: fifteen
/// flips, every one toward HIT). Ten are the documents `choose_rotation` used to
/// turn 90° on too little detected text, after which every crop presented the
/// MRZ sideways; India 2022 is the band the second deskew angle recovers.
///
/// Why name them when the real-specimen gate already asserts the hit count: that
/// gate compares **totals**, so a change that loses one of these and gains any
/// other document passes it at tolerance 0. This test does not.
///
/// `rotation == 0` is asserted alongside validity because the regression this
/// guards against is specifically a page being turned before it is read. A
/// quarter-turn tier that happened to recover one of these sideways would pass
/// a validity-only check while reintroducing exactly the behaviour chunk 2
/// removed. Failures are collected rather than stopping at the first, since a
/// change to orientation handling rarely costs just one document.
#[test]
#[ignore]
fn native_ocr_keeps_reading_the_upright_specimens_chunk_2_recovered() {
    if !has_the_retry_budget_these_tests_need() {
        return;
    }
    let (detection_path, recognition_path) = require_models();
    let ocr = NativeOcr::load(&detection_path, &recognition_path).expect("models load");

    let mut checked = 0usize;
    let mut failures = Vec::new();
    for name in [
        "Canada_Passport_Specimen_PP_CAN_2023_mrz.jpeg",
        "Dominican_Republic_Passport_Specimen_P0_DOM_2020_mrz.png",
        "Finland_Passport_Specimen_P0_FIN_2007_mrz.jpg",
        "Finland_Passport_Specimen_P0_FIN_2023_mrz.png",
        "India_Passport_Specimen_P0_IND_2022_mrz.png",
        "India_Passport_Specimen_P0_IND_2024_mrz.jpg",
        "Kuwait_Passport_Specimen_P0_KWT_2023_mrz.png",
        "Monaco_ID_Specimen_XXXX_back_mrz.png",
        "Nepal_Passport_Specimen_P0_NPL_2019_mrz.png",
        "Oman_Passport_Specimen_P0_OMN_2004_mrz.jpg",
        "Portugal_Passport_Specimen_PX_PRT_2017_mrz.png",
        "Russian_Federation_Passport_Specimen_P0_RUS_2014_mrz.jpg",
        "Vietnam_Passport_Specimen_P0_VNM_2023_mrz.webp",
    ] {
        let Some(path) = find_sample(name) else {
            eprintln!("SKIPPED {name}: not present — run scripts/sync-samples.ps1 for the corpus");
            continue;
        };
        let page = ocr
            .recognize_detailed(&path)
            .unwrap_or_else(|e| panic!("{name}: recognition failed: {e}"));

        let valid = mrz::find_and_parse(&page.text).is_ok_and(|d| d.valid());
        if !valid || page.rotation != 0 {
            failures.push(format!(
                "{name}: checksum-valid MRZ = {valid}, reported rotation = {}°",
                page.rotation
            ));
        }
        checked += 1;
    }

    assert!(
        failures.is_empty(),
        "{} of {checked} upright specimens recovered by ADR-0008 chunk 2 no longer read upright:\n{}",
        failures.len(),
        failures.join("\n")
    );
    if checked == 0 {
        eprintln!(
            "SKIPPED: none of the upright chunk-2 specimens are present locally. This test asserts \
             nothing without the corpus — that is deliberate, but it means a green run here is \
             not evidence."
        );
    }
}

/// A page handed to the engine a quarter-turn off must come back with the same
/// MRZ fragment as the upright original, and must say how far it turned it.
///
/// The synthetic counterpart to the corpus test above, built on the one
/// passport image that **is** tracked in git — so this half of the property
/// survives a fresh clone with no corpus. Both directions, because 90° and 270°
/// are separate entries in `ROTATION_RETRY_TURNS` and a transposition bug that
/// swaps them still passes if only one is checked.
///
/// Asserted on the outcome, not the mechanism: whether recovery comes from the
/// retry chain (today) or a future layer is not this test's business, which is
/// what keeps it meaningful once layers 2 and 3 land.
#[test]
#[ignore]
fn native_ocr_recovers_mrz_from_a_quarter_turned_page() {
    if !has_the_retry_budget_these_tests_need() {
        return;
    }
    let (detection_path, recognition_path) = require_models();
    let ocr = NativeOcr::load(&detection_path, &recognition_path).expect("models load");

    let source_path =
        find_sample("Canada_Passport_Specimen_2023_mrz.jpg").expect("a tracked fixture image");
    let upright = image::open(&source_path)
        .expect("sample image opens")
        .into_rgb8();

    for (turns, expected_correction) in [(1u8, 270u16), (3u8, 90u16)] {
        let turned = if turns == 1 {
            image::imageops::rotate90(&upright)
        } else {
            image::imageops::rotate270(&upright)
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
