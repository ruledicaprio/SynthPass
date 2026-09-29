//! Real-model check of the chargrid capture on the pass trace (ADR-0024,
//! amendment 2). Ignored by default (needs the two `.rten` files at the repo
//! root or in `SYNTHPASS_OCR_MODEL_DIR`); run explicitly with:
//!
//! ```sh
//! cargo test -p synthpass-ocr --test native_ocr_chargrid_trace --release -- --ignored
//! ```
//!
//! One `#[test]`, because it toggles `SYNTHPASS_OCR_CHARGRID` in the process
//! environment, which other tests in a shared binary would race with (the same
//! discipline as `synthpass-bench`'s `chargrid_repair_synthetic.rs`). On one
//! image, in sequence:
//!
//! - `on`: the traced page equals the untraced page; exactly the `Accepted`
//!   record carries a chargrid record, whose verdict is the page's; the record
//!   replays (below) on real output;
//! - `control`: the same, with the placebo's verdict, and the repair still
//!   recorded;
//! - unset: nothing is recorded, and the page carries no verdict.
//!
//! "Replays" means: from the capture alone, `fit_grid` reproduces the grid,
//! the column profile reproduces each cell's ink bit for bit, and
//! `repair_name_line` reproduces the repair. That is the property that makes
//! the record enough for an offline analysis.
//!
//! The specimen must validate on the general pass, so the test holds under
//! CI's `SYNTHPASS_OCR_MAX_PASSES=1`, which stops the retry loop before any
//! retry pass runs.

use std::path::{Path, PathBuf};
use synthpass_ocr::chargrid::{self, Grid};
use synthpass_ocr::{ChargridLineCapture, ChargridRecord, NativeOcr, PassOutcome, PassRecord};

/// Validates on the general pass, and the chargrid attempt fits a grid to its
/// name line (the 2026-09-18 chargrid A/B recorded it `repaired` under `on`).
const SAMPLE: &str = "Serbia_Passport_Specimen_P0_SRB_2012_mrz.jpg";

fn require_models() -> (PathBuf, PathBuf) {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut detection_path = repo_root.join("text-detection.rten");
    let mut recognition_path = repo_root.join("text-recognition.rten");
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

/// Cell `k`'s ink recomputed from the column profile: the sum over the cell's
/// columns divided by `columns * rows`, with `chargrid::cell_ink`'s own
/// rounding and clamping.
fn cell_ink_from_columns(line: &ChargridLineCapture, grid: &Grid) -> Vec<f32> {
    let columns = &line.column_ink;
    let w = columns.len() as u32;
    let (top, bottom) = (
        line.line_box.top.min(line.image_height),
        line.line_box.bottom.min(line.image_height),
    );
    let rows = bottom.saturating_sub(top);
    (0..grid.cells)
        .map(|k| {
            let x_lo = (grid.origin + k as f32 * grid.pitch).max(0.0).round() as u32;
            let x_hi = ((grid.origin + (k as f32 + 1.0) * grid.pitch)
                .max(0.0)
                .round() as u32)
                .min(w);
            if x_lo >= x_hi || rows == 0 {
                return 0.0;
            }
            let dark: u32 = columns[x_lo as usize..x_hi as usize].iter().sum();
            dark as f32 / ((x_hi - x_lo) * rows) as f32
        })
        .collect()
}

/// The record's own claims, then the record replayed from itself.
fn assert_line_replays(line: &ChargridLineCapture) {
    assert_eq!(
        line.column_ink.len(),
        line.image_width as usize,
        "one profile entry per column of the working image"
    );
    let b = line.line_box;
    assert!(
        b.left < b.right && b.top < b.bottom,
        "a non-empty line box: {b:?}"
    );
    assert!(
        b.right <= line.image_width && b.bottom <= line.image_height,
        "the line box lies inside the working image: {b:?} in {}x{}",
        line.image_width,
        line.image_height
    );
    assert_eq!(line.band_sha256.len(), 64);
    assert!(line.band_sha256.bytes().all(|c| c.is_ascii_hexdigit()));
    assert!(!line.glyphs.is_empty());

    let fit = line.fit.as_ref().expect("the specimen fits a grid");
    assert_eq!(fit.grid.cells, line.width);

    // Replay: the grid from the recorded glyphs and box.
    let grid = chargrid::fit_grid(&line.glyphs, b.left as f32, b.right as f32, line.width)
        .expect("the recorded inputs fit again");
    assert_eq!(grid, fit.grid, "fit_grid reproduces the grid");
    assert_eq!(
        chargrid::align(&line.glyphs, &grid),
        fit.cells,
        "align reproduces the cells"
    );

    // Replay: each cell's ink from the column profile alone.
    let ink = cell_ink_from_columns(line, &grid);
    assert_eq!(ink.len(), fit.cell_ink.len());
    for (k, (replayed, recorded)) in ink.iter().zip(&fit.cell_ink).enumerate() {
        assert_eq!(
            replayed.to_bits(),
            recorded.to_bits(),
            "cell {k}: the profile reproduces the ink the gate saw"
        );
    }

    // Replay: the repair from the recorded inputs.
    let repair = chargrid::repair_name_line(
        &line.raw_name_line,
        &line.glyphs,
        line.width,
        &ink,
        line.ink_floor,
        &grid,
    );
    assert_eq!(repair, fit.repair, "repair_name_line reproduces the repair");
}

/// The trace of `path` under the current environment: the traced page must be
/// the untraced page, and only an accepted pass may carry a chargrid record.
fn trace(ocr: &NativeOcr, path: &Path) -> (synthpass_ocr::OcrPage, Vec<PassRecord>) {
    let plain = ocr.recognize_detailed(path).expect("recognition succeeds");
    let (page, records) = ocr
        .recognize_detailed_traced(path)
        .expect("traced recognition succeeds");
    assert_eq!(page, plain, "tracing must not change the page");
    for record in &records {
        assert!(
            record.chargrid.is_none() || record.outcome == PassOutcome::Accepted,
            "{}: only the accepted pass carries a chargrid record",
            record.id
        );
    }
    (page, records)
}

/// The one record the run holds, on the accepted pass.
fn the_chargrid_record<'a>(
    page: &synthpass_ocr::OcrPage,
    records: &'a [PassRecord],
) -> &'a ChargridRecord {
    assert_eq!(
        page.retry_stop.as_deref(),
        Some("general_valid"),
        "the specimen must validate on the general pass"
    );
    let holders: Vec<&PassRecord> = records.iter().filter(|r| r.chargrid.is_some()).collect();
    assert_eq!(holders.len(), 1, "exactly one pass holds the attempt");
    assert_eq!(holders[0].outcome, PassOutcome::Accepted);
    assert_eq!(holders[0].id, "general");
    holders[0].chargrid.as_ref().expect("filtered on it")
}

#[test]
#[ignore]
fn the_chargrid_capture_follows_the_arm_and_replays() {
    let (detection_path, recognition_path) = require_models();
    let ocr = NativeOcr::load(&detection_path, &recognition_path).expect("models load");
    let path = find_sample(SAMPLE);

    // `on`.
    unsafe { std::env::set_var("SYNTHPASS_OCR_CHARGRID", "on") };
    let (page, records) = trace(&ocr, &path);
    let record = the_chargrid_record(&page, &records);
    assert_eq!(record.mode, "on");
    assert_eq!(
        Some(&record.verdict),
        page.chargrid.as_ref(),
        "the record's verdict is the page's, byte for byte"
    );
    assert_line_replays(record.line.as_ref().expect("a line matched"));

    // `control`: the placebo repairs nothing, but the attempt is recorded whole.
    unsafe { std::env::set_var("SYNTHPASS_OCR_CHARGRID", "control") };
    let (page, records) = trace(&ocr, &path);
    let record = the_chargrid_record(&page, &records);
    assert_eq!(record.mode, "control");
    assert_eq!(record.verdict, "control");
    assert_eq!(page.chargrid.as_deref(), Some("control"));
    assert_line_replays(record.line.as_ref().expect("a line matched"));

    // Unset: nothing is recorded.
    unsafe { std::env::remove_var("SYNTHPASS_OCR_CHARGRID") };
    let (page, records) = trace(&ocr, &path);
    assert_eq!(page.chargrid, None);
    assert!(
        records.iter().all(|r| r.chargrid.is_none()),
        "no pass carries a chargrid record while the arm is off"
    );
}
