//! Opt-in reporting of per-pass OCR readings (ADR-0024, amendment 1).
//!
//! [`synthpass_ocr::NativeOcr::recognize_detailed_traced`] records which OCR
//! pass read each MRZ-shaped line. This module is the benchmarks' side of it:
//! the JSON shapes both binaries write, and the rules for where the readings
//! may be written.
//!
//! - `synthpass-bench --ocr-passes` adds `ocr_text` and `ocr_passes` to each
//!   `results[]` entry ([`pass_objects`]).
//! - `provider-bench --real-specimens --dump-ocr-passes` writes
//!   [`OCR_PASSES_FILENAME`], one [`OcrPassesRow`] per OCR'd document, from the
//!   corpus prep (which runs once, not once per provider).
//!
//! - When `SYNTHPASS_OCR_CHARGRID` is `on` or `control`, the pass that read the
//!   MRZ carries a `chargrid` object ([`ChargridObject`]): the name-line
//!   repair's matched line, glyph positions, fitted grid and ink, enough to
//!   replay it (ADR-0024, amendment 2). It is `null` on every other pass and
//!   whenever the arm is off. Each `provider-bench` row also carries
//!   `chargrid`, the verdict [`OcrPage::chargrid`] reports.
//!
//! Both are off by default, and the readings never enter the `--out` trend
//! report, the outcome ledger, stdout, stderr or a log: they are diagnostic
//! zone text, and for the real-specimen track they are a document's OCR. The
//! chargrid object's `raw_name_line`, `recognized_text`, glyph characters,
//! `repair.line` and `column_ink` are document content under the same rules.
//!
//! The file is an input as well as an output (ADR-0024, amendment 3):
//! [`read_rows`] is the one reader, and every type here derives `Deserialize`
//! with owned fields, so what was written reads back equal. `provider-bench
//! --replay-ocr-passes` replays Tier 1 from it without OCR, and a chargrid
//! replay reads the same rows. A change to the row's schema updates the reader
//! in the same change.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use synthpass_ocr::{ChargridFit, ChargridLineCapture, ChargridRecord, OcrPage, PassRecord};

/// The file `provider-bench --dump-ocr-passes` writes next to `--out`.
pub const OCR_PASSES_FILENAME: &str = "provider-bench-ocr-passes.jsonl";

/// A bounding box in the reading pass's own image space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoxObject {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// One MRZ-shaped line one pass read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadingObject {
    pub line_index: usize,
    pub bbox: BoxObject,
    pub text: String,
}

/// One executed OCR pass, as written to a report. Key set pinned by tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PassObject {
    pub order: usize,
    pub id: String,
    /// The `synthpass_ocr::PassTransform` wire label, e.g. `mrz_variants:2`.
    pub transform: String,
    pub turn: u16,
    pub image_width: u32,
    pub image_height: u32,
    /// `failed`, `no_mrz_shaped_lines`, `appended` or `accepted`.
    pub outcome: String,
    pub readings: Vec<ReadingObject>,
    /// The chargrid attempt that read this pass's pixels. Always present:
    /// `null` unless this is the accepted pass and the arm ran.
    pub chargrid: Option<ChargridObject>,
}

/// One chargrid attempt (`synthpass_ocr::ChargridRecord`). Key set pinned by
/// tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChargridObject {
    /// `on` or `control`.
    pub mode: String,
    /// Byte-identical to the page's `chargrid` verdict.
    pub verdict: String,
    /// `null` when the attempt stopped before a line matched.
    pub line: Option<ChargridLineObject>,
}

/// The matched line and what was measured on it, in the working image's pixel
/// space. See `synthpass_ocr::ChargridLineCapture` for each field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChargridLineObject {
    pub name_line_index: usize,
    pub width: usize,
    pub raw_name_line: String,
    pub recognized_line_index: usize,
    pub recognized_text: String,
    pub downscaled: bool,
    pub image_width: u32,
    pub image_height: u32,
    pub line_box: LineBoxObject,
    pub band_sha256: String,
    pub glyphs: Vec<GlyphObject>,
    pub column_ink: Vec<u32>,
    pub ink_floor: f32,
    /// `null` when no grid could be fitted.
    pub fit: Option<ChargridFitObject>,
}

/// A line box in edges, not [`BoxObject`]'s `{x, y, w, h}`: edges are what the
/// grid fit and the ink measurement consume, so a replay reads them as they are.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineBoxObject {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

/// One recognized character with its horizontal extent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlyphObject {
    /// A one-character string.
    pub ch: String,
    pub left: f32,
    pub right: f32,
}

/// The fitted grid and the repair's verdict on it. The grid's cell count is the
/// line's `width`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChargridFitObject {
    pub origin: f32,
    pub pitch: f32,
    /// Glyph `i` sits in cell `cells[i]`; `null` when the glyphs could not be
    /// aligned.
    pub cells: Option<Vec<usize>>,
    /// Exactly what the repair's ink gate received, one value per cell.
    pub cell_ink: Vec<f32>,
    pub repair: RepairObject,
}

/// What the repair returned, with a fixed key set: every value that does not
/// apply is `null`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairObject {
    /// `ok`, or a `synthpass_ocr::chargrid::Rejected::as_str` label.
    pub outcome: String,
    /// The repaired line; only on `ok`.
    pub line: Option<String>,
    /// How many fillers were placed; only on `ok`.
    pub fillers_placed: Option<usize>,
    /// The glyph deficit; only on `ink_mismatch`.
    pub expected: Option<usize>,
    /// How many empty cells met the ink floor; only on `ink_mismatch`.
    pub found: Option<usize>,
}

fn repair_object(
    repair: &Result<synthpass_ocr::chargrid::Repair, synthpass_ocr::chargrid::Rejected>,
) -> RepairObject {
    match repair {
        Ok(repair) => RepairObject {
            outcome: "ok".to_string(),
            line: Some(repair.line.clone()),
            fillers_placed: Some(repair.fillers_placed),
            expected: None,
            found: None,
        },
        Err(rejected) => {
            let (expected, found) = match rejected {
                synthpass_ocr::chargrid::Rejected::InkMismatch { expected, found } => {
                    (Some(*expected), Some(*found))
                }
                _ => (None, None),
            };
            RepairObject {
                outcome: rejected.as_str().to_string(),
                line: None,
                fillers_placed: None,
                expected,
                found,
            }
        }
    }
}

fn fit_object(fit: &ChargridFit) -> ChargridFitObject {
    ChargridFitObject {
        origin: fit.grid.origin,
        pitch: fit.grid.pitch,
        cells: fit.cells.clone(),
        cell_ink: fit.cell_ink.clone(),
        repair: repair_object(&fit.repair),
    }
}

fn line_object(line: &ChargridLineCapture) -> ChargridLineObject {
    ChargridLineObject {
        name_line_index: line.name_line_index,
        width: line.width,
        raw_name_line: line.raw_name_line.clone(),
        recognized_line_index: line.recognized_line_index,
        recognized_text: line.recognized_text.clone(),
        downscaled: line.downscaled,
        image_width: line.image_width,
        image_height: line.image_height,
        line_box: LineBoxObject {
            left: line.line_box.left,
            top: line.line_box.top,
            right: line.line_box.right,
            bottom: line.line_box.bottom,
        },
        band_sha256: line.band_sha256.clone(),
        glyphs: line
            .glyphs
            .iter()
            .map(|glyph| GlyphObject {
                ch: glyph.ch.to_string(),
                left: glyph.left,
                right: glyph.right,
            })
            .collect(),
        column_ink: line.column_ink.clone(),
        ink_floor: line.ink_floor,
        fit: line.fit.as_ref().map(fit_object),
    }
}

/// The report shape of one chargrid attempt.
pub fn chargrid_object(record: &ChargridRecord) -> ChargridObject {
    ChargridObject {
        mode: record.mode.to_string(),
        verdict: record.verdict.clone(),
        line: record.line.as_ref().map(line_object),
    }
}

/// The report shape of a run's pass records, in execution order.
pub fn pass_objects(records: &[PassRecord]) -> Vec<PassObject> {
    records
        .iter()
        .map(|record| PassObject {
            order: record.order,
            id: record.id.clone(),
            transform: record.transform.to_string(),
            turn: record.turn,
            image_width: record.image_width,
            image_height: record.image_height,
            outcome: record.outcome.as_str().to_string(),
            readings: record
                .readings
                .iter()
                .map(|reading| ReadingObject {
                    line_index: reading.line_index,
                    bbox: BoxObject {
                        x: reading.bbox.x,
                        y: reading.bbox.y,
                        w: reading.bbox.w,
                        h: reading.bbox.h,
                    },
                    text: reading.text.clone(),
                })
                .collect(),
            chargrid: record.chargrid.as_ref().map(chargrid_object),
        })
        .collect()
}

/// One row of `provider-bench-ocr-passes.jsonl`: one OCR'd real specimen,
/// whatever its outcome. Key set and key order pinned by tests.
///
/// A separate file rather than a widened `MissOcrDump`: ADR-0024 rejected
/// widening that row because `tools/classify_mrz_mechanisms.py` depends on its
/// shape.
///
/// **Additive only.** A key keeps its name, its position and its bytes; a new
/// key goes at the end. `retry_damaged_recovery` and `mrz_band_score` were
/// added at the end for replay (ADR-0024, amendment 3): they are the two
/// `OcrPage` values scoring reads that the row did not yet carry. Both are
/// text-free.
///
/// `Deserialize` with `#[serde(default)]`: a row written before a key existed
/// reads back with that key defaulted, and [`read_rows`] reports which keys
/// were absent, so a consumer that needs one can refuse rather than replay a
/// default.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OcrPassesRow {
    pub name: String,
    pub asset_id: Option<String>,
    pub source_sha256: Option<String>,
    /// The run manifest's file name, relative to this file's directory.
    pub run_manifest: Option<String>,
    pub rotation: u16,
    pub retry_variant_id: Option<String>,
    pub retry_stop: Option<String>,
    pub retry_budget_hit: bool,
    /// `OcrPage::chargrid` verbatim: the chargrid arm's verdict for this
    /// document, `null` when the arm is off. Present even when no pass holds a
    /// capture (a `skipped:` verdict reads no pixels).
    pub chargrid: Option<String>,
    /// The full `OcrPage::text` the providers were handed.
    pub ocr_text: String,
    pub ocr_passes: Vec<PassObject>,
    /// `OcrPage::retry_damaged_recovery` verbatim: `MrzData::damaged_recovery`
    /// of the reading the retry loop accepted. `Some` exactly when
    /// `retry_variant_id` is. Reported per document by `provider-bench`.
    pub retry_damaged_recovery: Option<bool>,
    /// `OcrPage::mrz_band_score` verbatim: the winning MRZ band's score in
    /// `[0, 1]`, `null` when no band was found. It is in the miss dump.
    pub mrz_band_score: Option<f64>,
}

/// Every key a row is written with, in the order it is written. What
/// [`PassesFile::missing_keys`] is computed against.
pub const ROW_KEYS: [&str; 13] = [
    "name",
    "asset_id",
    "source_sha256",
    "run_manifest",
    "rotation",
    "retry_variant_id",
    "retry_stop",
    "retry_budget_hit",
    "chargrid",
    "ocr_text",
    "ocr_passes",
    "retry_damaged_recovery",
    "mrz_band_score",
];

/// The rows of one prep, filled as each document is OCR'd.
#[derive(Debug, Default)]
pub struct OcrPassesCollector {
    run_manifest: Option<String>,
    rows: Vec<OcrPassesRow>,
}

impl OcrPassesCollector {
    /// `run_manifest` is copied into every row.
    pub fn new(run_manifest: Option<String>) -> Self {
        Self {
            run_manifest,
            rows: Vec::new(),
        }
    }

    /// Records one OCR'd document.
    pub fn push(
        &mut self,
        name: &str,
        asset_id: Option<&str>,
        source_sha256: Option<&str>,
        page: &OcrPage,
        records: &[PassRecord],
    ) {
        self.rows.push(OcrPassesRow {
            name: name.to_string(),
            asset_id: asset_id.map(str::to_string),
            source_sha256: source_sha256.map(str::to_string),
            run_manifest: self.run_manifest.clone(),
            rotation: page.rotation,
            retry_variant_id: page.retry_variant_id.clone(),
            retry_stop: page.retry_stop.clone(),
            retry_budget_hit: page.retry_budget_hit,
            chargrid: page.chargrid.clone(),
            ocr_text: page.text.clone(),
            ocr_passes: pass_objects(records),
            retry_damaged_recovery: page.retry_damaged_recovery,
            mrz_band_score: page.mrz_band_score,
        });
    }

    /// The rows collected so far, in document order.
    pub fn rows(&self) -> &[OcrPassesRow] {
        &self.rows
    }

    /// Writes [`OCR_PASSES_FILENAME`] in `dir` (created if missing),
    /// replacing any earlier file: one line per row, each ending in `\n`.
    /// Returns the path and the row count.
    pub fn write(&self, dir: &Path) -> Result<(PathBuf, usize), String> {
        let mut body = String::new();
        for row in &self.rows {
            body.push_str(&serde_json::to_string(row).map_err(|e| e.to_string())?);
            body.push('\n');
        }
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let path = dir.join(OCR_PASSES_FILENAME);
        std::fs::write(&path, body).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        Ok((path, self.rows.len()))
    }
}

/// The rows of a `provider-bench-ocr-passes.jsonl`, as read back.
#[derive(Debug, Clone, PartialEq)]
pub struct PassesFile {
    /// One per line, in file order.
    pub rows: Vec<OcrPassesRow>,
    /// The [`ROW_KEYS`] that at least one row does not carry, in [`ROW_KEYS`]
    /// order. Empty for a file this code wrote. A non-empty list means the file
    /// predates a key, and the row's value for it is a default, not a reading.
    pub missing_keys: Vec<&'static str>,
}

/// Parses the body of [`OCR_PASSES_FILENAME`]: one JSON object per non-blank
/// line.
///
/// Error messages carry the row number and a category, never a value: a row
/// holds document OCR, and a message must not quote it (this crate's rule for
/// everything that reaches stderr).
pub fn parse_rows(body: &str) -> Result<PassesFile, String> {
    let mut rows = Vec::new();
    let mut missing = [false; ROW_KEYS.len()];
    for (index, line) in body.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let number = index + 1;
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| format!("row {number} is not JSON ({:?})", e.classify()))?;
        let Some(object) = value.as_object() else {
            return Err(format!("row {number} is not a JSON object"));
        };
        for (slot, key) in missing.iter_mut().zip(ROW_KEYS) {
            *slot |= !object.contains_key(key);
        }
        let row = serde_json::from_value(value).map_err(|e| {
            format!(
                "row {number} does not match the pass-file schema ({:?})",
                e.classify()
            )
        })?;
        rows.push(row);
    }
    let missing_keys = ROW_KEYS
        .iter()
        .zip(missing)
        .filter_map(|(key, absent)| absent.then_some(*key))
        .collect();
    Ok(PassesFile { rows, missing_keys })
}

/// Reads [`OCR_PASSES_FILENAME`] from `dir`: the one reader of the file. See
/// [`parse_rows`].
pub fn read_rows(dir: &Path) -> Result<PassesFile, String> {
    let path = dir.join(OCR_PASSES_FILENAME);
    let body = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    parse_rows(&body).map_err(|e| format!("{}: {e}", path.display()))
}

/// Refuses a destination inside the working tree that git does not ignore.
///
/// The readings are document OCR. A file inside the checkout that git would
/// stage is one `git add -A` from a commit, so the file is only written where
/// it is either outside the tree or ignored (`artifacts/` is). `is_ignored`
/// answers for a path relative to `root`; production passes [`git_ignores`].
/// The same rule, and the same `git check-ignore` mechanism, as
/// `ground_truth`'s review-HTML output.
///
/// `dir` is where [`OCR_PASSES_FILENAME`] will be written; it need not exist
/// yet. A relative `dir` is taken from `cwd`. A `..` in the part of `dir` that
/// does not exist yet is refused rather than guessed at.
pub fn check_passes_destination(
    root: &Path,
    cwd: &Path,
    dir: &Path,
    is_ignored: impl Fn(&Path) -> Result<bool, String>,
) -> Result<(), String> {
    let absolute = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        cwd.join(dir)
    };
    let destination = resolve_existing_prefix(&absolute)?.join(OCR_PASSES_FILENAME);
    let root = std::fs::canonicalize(root)
        .map_err(|e| format!("cannot resolve the working tree {}: {e}", root.display()))?;
    let Ok(relative) = destination.strip_prefix(&root) else {
        // Outside the working tree: nothing here for git to stage.
        return Ok(());
    };
    if is_ignored(relative)? {
        Ok(())
    } else {
        Err(format!(
            "--dump-ocr-passes writes document OCR, and {} is inside the working tree and not \
             ignored by git; use an --out directory under artifacts/ or outside the checkout",
            relative.display()
        ))
    }
}

/// Canonicalizes the deepest existing ancestor of `path` and re-appends the
/// rest, so a not-yet-created output directory is judged by where it will be.
fn resolve_existing_prefix(path: &Path) -> Result<PathBuf, String> {
    let mut existing = path.to_path_buf();
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    while !existing.exists() {
        // `file_name` is `None` for a path ending in `..`: where that leads
        // depends on directories that do not exist yet, so it is refused.
        let name = existing.file_name().ok_or_else(|| {
            format!(
                "{} has a parent-directory component past its last existing directory",
                path.display()
            )
        })?;
        rest.push(name.to_os_string());
        if !existing.pop() {
            return Err(format!("cannot resolve {}", path.display()));
        }
    }
    let mut resolved = std::fs::canonicalize(&existing)
        .map_err(|e| format!("cannot resolve {}: {e}", existing.display()))?;
    resolved.extend(rest.into_iter().rev());
    Ok(resolved)
}

/// `git check-ignore --quiet -- <relative>` run in `root`: `Ok(true)` when git
/// ignores the path, `Ok(false)` when it does not, `Err` when git could not
/// answer (a failure to ask is a refusal, never a pass).
pub fn git_ignores(root: &Path, relative: &Path) -> Result<bool, String> {
    let status = std::process::Command::new("git")
        .current_dir(root)
        .args(["check-ignore", "--quiet", "--"])
        .arg(relative)
        .status()
        .map_err(|e| format!("cannot run git check-ignore: {e}"))?;
    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err("git check-ignore could not answer".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use synthpass_ocr::{BBox, LineReading, PassOutcome, PassTransform};

    fn record(order: usize, outcome: PassOutcome, readings: Vec<LineReading>) -> PassRecord {
        PassRecord {
            order,
            id: if order == 0 {
                "general".to_string()
            } else {
                format!("pass-{:02}", order - 1)
            },
            transform: if order == 0 {
                PassTransform::General
            } else {
                PassTransform::MrzVariant(order - 1)
            },
            turn: 0,
            image_width: 640,
            image_height: 480,
            outcome,
            readings,
            chargrid: None,
        }
    }

    fn reading() -> LineReading {
        LineReading {
            line_index: 3,
            bbox: BBox {
                x: 1.0,
                y: 2.5,
                w: 30.0,
                h: 4.0,
            },
            text: "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<".to_string(),
        }
    }

    fn keys(value: &serde_json::Value) -> Vec<String> {
        let mut keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    }

    fn page() -> OcrPage {
        OcrPage {
            text: "text".to_string(),
            rotation: 90,
            retry_variant_id: Some("pass-01".to_string()),
            retry_stop: Some("variant_valid".to_string()),
            retry_budget_hit: false,
            ..OcrPage::default()
        }
    }

    #[test]
    fn a_pass_object_has_exactly_the_documented_keys() {
        let objects = pass_objects(&[record(1, PassOutcome::Appended, vec![reading()])]);
        let json = serde_json::to_value(&objects[0]).unwrap();
        assert_eq!(
            keys(&json),
            [
                "chargrid",
                "id",
                "image_height",
                "image_width",
                "order",
                "outcome",
                "readings",
                "transform",
                "turn"
            ]
        );
        assert!(
            json.as_object().unwrap().contains_key("chargrid"),
            "always present"
        );
        assert!(json["chargrid"].is_null(), "null when there is no attempt");
        assert_eq!(json["id"], "pass-00");
        assert_eq!(json["transform"], "mrz_variants:0");
        assert_eq!(json["outcome"], "appended");
        assert_eq!(
            keys(&json["readings"][0]),
            ["bbox", "line_index", "text"],
            "a reading carries its index, box and text"
        );
        assert_eq!(keys(&json["readings"][0]["bbox"]), ["h", "w", "x", "y"]);
        assert_eq!(json["readings"][0]["line_index"], 3);
        assert_eq!(json["readings"][0]["bbox"]["y"], 2.5);
    }

    #[test]
    fn a_row_has_exactly_the_documented_keys() {
        let mut collector = OcrPassesCollector::new(Some("run.json".to_string()));
        collector.push(
            "specimen",
            Some("passports/specimen.png"),
            Some("ab"),
            &page(),
            &[record(0, PassOutcome::NoMrzShapedLines, Vec::new())],
        );
        let json = serde_json::to_value(&collector.rows()[0]).unwrap();
        assert_eq!(
            keys(&json),
            [
                "asset_id",
                "chargrid",
                "mrz_band_score",
                "name",
                "ocr_passes",
                "ocr_text",
                "retry_budget_hit",
                "retry_damaged_recovery",
                "retry_stop",
                "retry_variant_id",
                "rotation",
                "run_manifest",
                "source_sha256"
            ]
        );
        assert_eq!(json["run_manifest"], "run.json");
        assert!(json["chargrid"].is_null(), "null when the arm is off");
        assert_eq!(json["rotation"], 90);
        assert_eq!(json["ocr_passes"][0]["outcome"], "no_mrz_shaped_lines");
    }

    fn chargrid_record(
        repair: Result<synthpass_ocr::chargrid::Repair, synthpass_ocr::chargrid::Rejected>,
    ) -> synthpass_ocr::ChargridRecord {
        use synthpass_ocr::chargrid::{Glyph, Grid};
        synthpass_ocr::ChargridRecord {
            mode: "on",
            verdict: "repaired".to_string(),
            line: Some(ChargridLineCapture {
                name_line_index: 0,
                width: 44,
                raw_name_line: "P<UTOERIKSSONANNA".to_string(),
                recognized_line_index: 4,
                recognized_text: "PUTOERIKSSONANNA".to_string(),
                downscaled: true,
                image_width: 1600,
                image_height: 90,
                line_box: synthpass_ocr::LineBox {
                    left: 10,
                    top: 20,
                    right: 1500,
                    bottom: 60,
                },
                band_sha256: "ab".repeat(32),
                glyphs: vec![
                    Glyph {
                        ch: 'P',
                        left: 10.5,
                        right: 27.25,
                    },
                    Glyph {
                        ch: 'U',
                        left: 44.0,
                        right: 60.0,
                    },
                ],
                column_ink: vec![0, 3, 40],
                ink_floor: 0.05,
                fit: Some(ChargridFit {
                    grid: Grid {
                        origin: 9.75,
                        pitch: 33.875,
                        cells: 44,
                    },
                    cells: Some(vec![0, 1]),
                    cell_ink: vec![0.5, 0.125],
                    repair,
                }),
            }),
        }
    }

    fn repaired() -> synthpass_ocr::chargrid::Repair {
        synthpass_ocr::chargrid::Repair {
            line: "P<UTOERIKSSON<<ANNA".to_string(),
            fillers_placed: 2,
        }
    }

    #[test]
    fn a_chargrid_object_has_exactly_the_documented_keys_at_every_level() {
        let mut record = record(4, PassOutcome::Accepted, vec![reading()]);
        record.chargrid = Some(chargrid_record(Ok(repaired())));
        let json = serde_json::to_value(&pass_objects(&[record])[0]).unwrap();
        let chargrid = &json["chargrid"];
        assert_eq!(keys(chargrid), ["line", "mode", "verdict"]);
        assert_eq!(chargrid["mode"], "on");
        assert_eq!(chargrid["verdict"], "repaired");
        let line = &chargrid["line"];
        assert_eq!(
            keys(line),
            [
                "band_sha256",
                "column_ink",
                "downscaled",
                "fit",
                "glyphs",
                "image_height",
                "image_width",
                "ink_floor",
                "line_box",
                "name_line_index",
                "raw_name_line",
                "recognized_line_index",
                "recognized_text",
                "width"
            ]
        );
        assert_eq!(
            keys(&line["line_box"]),
            ["bottom", "left", "right", "top"],
            "edges, not x/y/w/h"
        );
        assert_eq!(line["line_box"]["right"], 1500);
        assert_eq!(keys(&line["glyphs"][0]), ["ch", "left", "right"]);
        assert_eq!(line["glyphs"][0]["ch"], "P");
        assert_eq!(line["glyphs"][1]["left"], 44.0);
        assert_eq!(line["column_ink"], serde_json::json!([0, 3, 40]));
        assert_eq!(line["downscaled"], true);
        assert_eq!(line["recognized_line_index"], 4);
        let fit = &line["fit"];
        assert_eq!(
            keys(fit),
            ["cell_ink", "cells", "origin", "pitch", "repair"]
        );
        assert_eq!(fit["origin"], 9.75);
        assert_eq!(fit["pitch"], 33.875);
        assert_eq!(fit["cells"], serde_json::json!([0, 1]));
        assert_eq!(fit["cell_ink"], serde_json::json!([0.5, 0.125]));
    }

    #[test]
    fn a_chargrid_line_and_fit_are_null_when_they_were_never_made() {
        let no_line = synthpass_ocr::ChargridRecord {
            mode: "control",
            verdict: "skipped:no_line_match".to_string(),
            line: None,
        };
        let json = serde_json::to_value(chargrid_object(&no_line)).unwrap();
        assert_eq!(keys(&json), ["line", "mode", "verdict"]);
        assert!(json["line"].is_null());

        let mut no_fit = chargrid_record(Ok(repaired()));
        no_fit.line.as_mut().unwrap().fit = None;
        let json = serde_json::to_value(chargrid_object(&no_fit)).unwrap();
        assert!(json["line"]["fit"].is_null());
        assert!(json["line"]["glyphs"].is_array());

        let mut no_cells = chargrid_record(Ok(repaired()));
        no_cells.line.as_mut().unwrap().fit.as_mut().unwrap().cells = None;
        let json = serde_json::to_value(chargrid_object(&no_cells)).unwrap();
        assert!(json["line"]["fit"]["cells"].is_null());
    }

    #[test]
    fn the_repair_object_has_a_fixed_key_set_and_nulls_what_does_not_apply() {
        use synthpass_ocr::chargrid::Rejected;
        let repair_of = |repair| {
            let record = chargrid_record(repair);
            serde_json::to_value(chargrid_object(&record)).unwrap()["line"]["fit"]["repair"].clone()
        };
        let keys_expected = ["expected", "fillers_placed", "found", "line", "outcome"];

        let ok = repair_of(Ok(repaired()));
        assert_eq!(keys(&ok), keys_expected);
        assert_eq!(ok["outcome"], "ok");
        assert_eq!(ok["line"], "P<UTOERIKSSON<<ANNA");
        assert_eq!(ok["fillers_placed"], 2);
        assert!(ok["expected"].is_null() && ok["found"].is_null());

        let mismatch = repair_of(Err(Rejected::InkMismatch {
            expected: 2,
            found: 1,
        }));
        assert_eq!(keys(&mismatch), keys_expected);
        assert_eq!(mismatch["outcome"], "ink_mismatch");
        assert_eq!(mismatch["expected"], 2);
        assert_eq!(mismatch["found"], 1);
        assert!(mismatch["line"].is_null() && mismatch["fillers_placed"].is_null());

        for (rejected, label) in [
            (Rejected::NoDeficit, "no_deficit"),
            (Rejected::TooManyGlyphs, "too_many_glyphs"),
            (Rejected::GridFit, "grid_fit"),
            (Rejected::PrefixChanged, "prefix_changed"),
        ] {
            let other = repair_of(Err(rejected));
            assert_eq!(keys(&other), keys_expected, "{label}");
            assert_eq!(other["outcome"], label);
            for key in ["line", "fillers_placed", "expected", "found"] {
                assert!(other[key].is_null(), "{label}: {key}");
            }
        }
    }

    /// The `f32`s the file carries must read back as the identical `f32`: a
    /// replay checked against a rounded value would disagree with the run.
    #[test]
    fn every_float_parses_back_to_the_identical_f32() {
        use synthpass_ocr::chargrid::{Glyph, Grid};
        #[derive(serde::Deserialize)]
        struct Back {
            line: BackLine,
        }
        #[derive(serde::Deserialize)]
        struct BackLine {
            ink_floor: f32,
            glyphs: Vec<BackGlyph>,
            fit: BackFit,
        }
        #[derive(serde::Deserialize)]
        struct BackGlyph {
            left: f32,
            right: f32,
        }
        #[derive(serde::Deserialize)]
        struct BackFit {
            origin: f32,
            pitch: f32,
            cell_ink: Vec<f32>,
        }
        let awkward: [f32; 8] = [
            0.1,
            1.0 / 3.0,
            f32::from_bits(24.0f32.to_bits() - 1),
            f32::from_bits(33.875f32.to_bits() + 1),
            -30.25,
            1e-7,
            0.05,
            1_234_567.9,
        ];
        let mut record = chargrid_record(Ok(repaired()));
        let line = record.line.as_mut().unwrap();
        line.ink_floor = awkward[6];
        line.glyphs = awkward
            .windows(2)
            .map(|pair| Glyph {
                ch: 'A',
                left: pair[0],
                right: pair[1],
            })
            .collect();
        let fit = line.fit.as_mut().unwrap();
        fit.grid = Grid {
            origin: awkward[2],
            pitch: awkward[3],
            cells: 44,
        };
        fit.cell_ink = awkward.to_vec();

        let text = serde_json::to_string(&chargrid_object(&record)).unwrap();
        let back: Back = serde_json::from_str(&text).unwrap();
        let bits = |values: &[f32]| values.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
        assert_eq!(back.line.ink_floor.to_bits(), awkward[6].to_bits());
        assert_eq!(back.line.fit.origin.to_bits(), awkward[2].to_bits());
        assert_eq!(back.line.fit.pitch.to_bits(), awkward[3].to_bits());
        assert_eq!(bits(&back.line.fit.cell_ink), bits(&awkward));
        for (glyph, pair) in back.line.glyphs.iter().zip(awkward.windows(2)) {
            assert_eq!(glyph.left.to_bits(), pair[0].to_bits());
            assert_eq!(glyph.right.to_bits(), pair[1].to_bits());
        }
    }

    #[test]
    fn a_row_carries_the_page_verdict_verbatim() {
        let mut collector = OcrPassesCollector::new(None);
        for verdict in [None, Some("skipped:no_source_image"), Some("repaired")] {
            let mut page = page();
            page.chargrid = verdict.map(str::to_string);
            collector.push("doc", None, None, &page, &[]);
        }
        let json: Vec<serde_json::Value> = collector
            .rows()
            .iter()
            .map(|row| serde_json::to_value(row).unwrap())
            .collect();
        assert!(json[0]["chargrid"].is_null());
        assert_eq!(json[1]["chargrid"], "skipped:no_source_image");
        assert_eq!(json[2]["chargrid"], "repaired");
    }

    /// One row per OCR'd document. The rows come from the corpus prep, which
    /// runs once however many providers read the documents afterwards.
    #[test]
    fn the_file_has_one_line_per_document() {
        let dir =
            std::env::temp_dir().join(format!("synthpass-passes-rows-{}", std::process::id()));
        let mut collector = OcrPassesCollector::new(None);
        for name in ["a", "b", "c"] {
            collector.push(name, None, None, &page(), &[]);
        }
        let (path, rows) = collector.write(&dir).expect("writes");
        assert_eq!(rows, 3);
        assert_eq!(path.file_name().unwrap(), OCR_PASSES_FILENAME);
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.ends_with('\n'));
        let names: Vec<String> = body
            .lines()
            .map(|l| {
                let v: serde_json::Value = serde_json::from_str(l).unwrap();
                v["name"].as_str().unwrap().to_string()
            })
            .collect();
        assert_eq!(names, ["a", "b", "c"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The page the pinned-bytes and round-trip tests share: synthetic text,
    /// every optional value set, so a row that dropped one would differ.
    fn full_page() -> OcrPage {
        OcrPage {
            text: "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<".to_string(),
            rotation: 90,
            retry_variant_id: Some("pass-01".to_string()),
            retry_damaged_recovery: Some(true),
            retry_budget_hit: false,
            retry_stop: Some("variant_valid".to_string()),
            mrz_band_score: Some(0.875),
            ..OcrPage::default()
        }
    }

    fn full_row() -> OcrPassesRow {
        let mut collector = OcrPassesCollector::new(Some("run.json".to_string()));
        collector.push(
            "specimen",
            Some("passports/specimen.png"),
            Some("ab"),
            &full_page(),
            &[
                record(0, PassOutcome::NoMrzShapedLines, Vec::new()),
                record(1, PassOutcome::Appended, vec![reading()]),
            ],
        );
        collector.rows()[0].clone()
    }

    /// The bytes one row is written as. Two halves: everything up to and
    /// including `ocr_passes` is what the file held before replay (amendment 3)
    /// and must not move; the two new keys follow it, at the end. A change to
    /// either half is a change to a file other tools read.
    const ROW_BEFORE_THE_NEW_KEYS: &str = concat!(
        r#"{"name":"specimen","asset_id":"passports/specimen.png","source_sha256":"ab","#,
        r#""run_manifest":"run.json","rotation":90,"retry_variant_id":"pass-01","#,
        r#""retry_stop":"variant_valid","retry_budget_hit":false,"chargrid":null,"#,
        r#""ocr_text":"P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<","#,
        r#""ocr_passes":[{"order":0,"id":"general","transform":"general","turn":0,"#,
        r#""image_width":640,"image_height":480,"outcome":"no_mrz_shaped_lines","#,
        r#""readings":[],"chargrid":null},"#,
        r#"{"order":1,"id":"pass-00","transform":"mrz_variants:0","turn":0,"#,
        r#""image_width":640,"image_height":480,"outcome":"appended","#,
        r#""readings":[{"line_index":3,"bbox":{"x":1.0,"y":2.5,"w":30.0,"h":4.0},"#,
        r#""text":"P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<"}],"chargrid":null}]"#,
    );
    const ROW_NEW_KEYS: &str = r#","retry_damaged_recovery":true,"mrz_band_score":0.875}"#;

    #[test]
    fn a_rows_bytes_are_pinned_before_and_after_the_new_keys() {
        let written = serde_json::to_string(&full_row()).unwrap();
        assert_eq!(written, format!("{ROW_BEFORE_THE_NEW_KEYS}{ROW_NEW_KEYS}"));
        assert!(written.starts_with(ROW_BEFORE_THE_NEW_KEYS));
        assert!(written.ends_with(ROW_NEW_KEYS));
    }

    #[test]
    fn a_row_with_no_damaged_recovery_and_no_band_writes_them_as_null() {
        let mut collector = OcrPassesCollector::new(None);
        collector.push("doc", None, None, &OcrPage::default(), &[]);
        let written = serde_json::to_string(&collector.rows()[0]).unwrap();
        assert!(
            written.ends_with(r#","retry_damaged_recovery":null,"mrz_band_score":null}"#),
            "the keys are always present: {written}"
        );
    }

    /// Every field of a written row reads back equal, chargrid capture included.
    #[test]
    fn a_row_round_trips_write_then_read_rows() {
        let mut with_chargrid = record(2, PassOutcome::Accepted, vec![reading()]);
        with_chargrid.chargrid = Some(chargrid_record(Ok(repaired())));
        let mut collector = OcrPassesCollector::new(Some("run.json".to_string()));
        collector.push(
            "specimen",
            Some("passports/specimen.png"),
            Some("ab"),
            &OcrPage {
                chargrid: Some("repaired".to_string()),
                ..full_page()
            },
            &[
                record(0, PassOutcome::NoMrzShapedLines, Vec::new()),
                with_chargrid,
            ],
        );
        collector.push("empty", None, None, &OcrPage::default(), &[]);
        let dir = scratch("round-trip");
        collector.write(&dir).expect("writes");
        let file = read_rows(&dir).expect("reads back");
        assert_eq!(file.rows, collector.rows());
        assert!(
            file.missing_keys.is_empty(),
            "a file this code wrote lacks no key: {:?}",
            file.missing_keys
        );
        assert_eq!(file.rows[0].retry_damaged_recovery, Some(true));
        assert_eq!(file.rows[0].mrz_band_score, Some(0.875));
        assert_eq!(
            file.rows[0].ocr_passes[1].chargrid.as_ref().unwrap().mode,
            "on"
        );
        // And what was read writes the same bytes: the reader loses nothing.
        let again: Vec<String> = file
            .rows
            .iter()
            .map(|row| serde_json::to_string(row).unwrap())
            .collect();
        let original: Vec<String> = collector
            .rows()
            .iter()
            .map(|row| serde_json::to_string(row).unwrap())
            .collect();
        assert_eq!(again, original);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `mrz_band_score` is an `f64` the dump row writes back out, so the reader
    /// must return the identical value (`serde_json`'s `float_roundtrip`).
    #[test]
    fn a_band_score_reads_back_as_the_identical_f64() {
        for score in [0.1_f64, 1.0 / 3.0, 0.7_777_777_777_777_777, 1e-9] {
            let mut collector = OcrPassesCollector::new(None);
            collector.push(
                "doc",
                None,
                None,
                &OcrPage {
                    mrz_band_score: Some(score),
                    ..OcrPage::default()
                },
                &[],
            );
            let body = serde_json::to_string(&collector.rows()[0]).unwrap() + "\n";
            let file = parse_rows(&body).expect("parses");
            assert_eq!(
                file.rows[0].mrz_band_score.map(f64::to_bits),
                Some(score.to_bits()),
                "{score}"
            );
        }
    }

    /// A row written before replay lacks the two new keys. It still reads, the
    /// reader says which keys were absent, and their values are defaults.
    #[test]
    fn a_file_written_before_the_new_keys_reads_and_names_what_it_lacks() {
        let old = format!("{ROW_BEFORE_THE_NEW_KEYS}}}\n");
        let file = parse_rows(&old).expect("an old row still reads");
        assert_eq!(file.rows.len(), 1);
        assert_eq!(
            file.missing_keys,
            ["retry_damaged_recovery", "mrz_band_score"]
        );
        assert_eq!(file.rows[0].retry_damaged_recovery, None);
        assert_eq!(file.rows[0].mrz_band_score, None);
        assert_eq!(
            file.rows[0].asset_id.as_deref(),
            Some("passports/specimen.png")
        );
        // A key absent from any one row is named, however many rows have it.
        let mixed = format!("{old}{ROW_BEFORE_THE_NEW_KEYS}{ROW_NEW_KEYS}\n");
        assert_eq!(
            parse_rows(&mixed).unwrap().missing_keys,
            ["retry_damaged_recovery", "mrz_band_score"]
        );
    }

    #[test]
    fn a_malformed_row_is_an_error_that_quotes_no_value() {
        let err = parse_rows("{\"name\":\"a\"}\nnot json SECRET-TEXT\n").expect_err("refused");
        assert!(err.starts_with("row 2 is not JSON"), "{err}");
        assert!(!err.contains("SECRET-TEXT"), "{err}");
        let err = parse_rows("[1]\n").expect_err("refused");
        assert_eq!(err, "row 1 is not a JSON object");
        let err = parse_rows("{\"rotation\":\"SECRET-TEXT\"}\n").expect_err("refused");
        assert!(err.starts_with("row 1 does not match"), "{err}");
        assert!(!err.contains("SECRET-TEXT"), "{err}");
        assert_eq!(parse_rows("\n\n").unwrap().rows, Vec::new());
    }

    #[test]
    fn a_missing_file_is_an_error_naming_it() {
        let dir = scratch("no-file");
        let err = read_rows(&dir).expect_err("no file");
        assert!(err.contains(OCR_PASSES_FILENAME), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("synthpass-passes-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::canonicalize(&dir).unwrap()
    }

    #[test]
    fn a_destination_outside_the_working_tree_is_allowed() {
        let root = scratch("root-out");
        let elsewhere = scratch("elsewhere");
        let never_asked = |_: &Path| -> Result<bool, String> { panic!("git must not be asked") };
        assert!(check_passes_destination(&root, &root, &elsewhere, never_asked).is_ok());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&elsewhere);
    }

    #[test]
    fn an_in_tree_destination_git_does_not_ignore_is_refused() {
        let root = scratch("root-in");
        let err = check_passes_destination(&root, &root, Path::new("out/here"), |relative| {
            assert_eq!(relative, Path::new("out/here").join(OCR_PASSES_FILENAME));
            Ok(false)
        })
        .expect_err("refused");
        assert!(err.contains("not ignored"), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_in_tree_destination_git_ignores_is_allowed() {
        let root = scratch("root-ignored");
        assert!(
            check_passes_destination(&root, &root, Path::new("artifacts"), |_| Ok(true)).is_ok()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_git_failure_is_a_refusal_not_a_pass() {
        let root = scratch("root-git-fails");
        let err = check_passes_destination(&root, &root, Path::new("artifacts"), |_| {
            Err("git check-ignore could not answer".to_string())
        })
        .expect_err("refused");
        assert!(err.contains("could not answer"), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn parent_traversal_past_the_existing_directories_is_refused() {
        let root = scratch("root-dotdot");
        let err =
            check_passes_destination(&root, &root, Path::new("new/../../escape"), |_| Ok(true))
                .expect_err("refused");
        assert!(err.contains("parent"), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
