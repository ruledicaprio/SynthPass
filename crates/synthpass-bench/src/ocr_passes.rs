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
//! Both are off by default, and the readings never enter the `--out` trend
//! report, the outcome ledger, stdout, stderr or a log: they are diagnostic
//! zone text, and for the real-specimen track they are a document's OCR.

use serde::Serialize;
use std::path::{Path, PathBuf};
use synthpass_ocr::{OcrPage, PassRecord};

/// The file `provider-bench --dump-ocr-passes` writes next to `--out`.
pub const OCR_PASSES_FILENAME: &str = "provider-bench-ocr-passes.jsonl";

/// A bounding box in the reading pass's own image space.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BoxObject {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// One MRZ-shaped line one pass read.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReadingObject {
    pub line_index: usize,
    pub bbox: BoxObject,
    pub text: String,
}

/// One executed OCR pass, as written to a report. Key set pinned by tests.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PassObject {
    pub order: usize,
    pub id: String,
    /// The `synthpass_ocr::PassTransform` wire label, e.g. `mrz_variants:2`.
    pub transform: String,
    pub turn: u16,
    pub image_width: u32,
    pub image_height: u32,
    /// `failed`, `no_mrz_shaped_lines`, `appended` or `accepted`.
    pub outcome: &'static str,
    pub readings: Vec<ReadingObject>,
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
            outcome: record.outcome.as_str(),
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
        })
        .collect()
}

/// One row of `provider-bench-ocr-passes.jsonl`: one OCR'd real specimen,
/// whatever its outcome. Key set pinned by tests.
///
/// A separate file rather than a widened `MissOcrDump`: ADR-0024 rejected
/// widening that row because `tools/classify_mrz_mechanisms.py` depends on its
/// shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
    /// The full `OcrPage::text` the providers were handed.
    pub ocr_text: String,
    pub ocr_passes: Vec<PassObject>,
}

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
            ocr_text: page.text.clone(),
            ocr_passes: pass_objects(records),
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
                "name",
                "ocr_passes",
                "ocr_text",
                "retry_budget_hit",
                "retry_stop",
                "retry_variant_id",
                "rotation",
                "run_manifest",
                "source_sha256"
            ]
        );
        assert_eq!(json["run_manifest"], "run.json");
        assert_eq!(json["rotation"], 90);
        assert_eq!(json["ocr_passes"][0]["outcome"], "no_mrz_shaped_lines");
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
