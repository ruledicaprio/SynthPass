//! Measurement harness for GitHub issue #103: how noisy is the *non-MRZ*
//! visual zone (photo, header text, data fields) across `samples/`, using
//! only signal `synthpass-ocr` already computes — no ground truth needed.
//!
//! This exists to answer one question before proposing any code change: is
//! visual-zone OCR noise a *systematic* property of the corpus (in which
//! case a `synthpass-ocr::preprocess`-style treatment for the visual zone,
//! mirroring the MRZ band's upscale/threshold/deskew helpers, is worth
//! scoping), or is a specific specimen (the private, untracked Türkiye
//! candidate that motivated #103) an outlier the rest of the corpus doesn't
//! share? See `knowledge/ROADMAP.md`'s Execution notes / `knowledge/CORPUS_COVERAGE.md`
//! for where this measurement's finding was recorded. Nothing here changes
//! production behavior.
//!
//! Per specimen, this reports:
//! - **total detected lines** — `OcrPage::lines` from the geometry pass
//!   (best-effort; empty when that pass fails on a specimen, in which case
//!   the specimen is skipped from the noise-fraction stats below).
//! - **MRZ lines** — lines whose [`geometry::mrz_line_score`] clears
//!   [`MRZ_LINE_SCORE_THRESHOLD`] (a local mirror of `geometry`'s private
//!   `MRZ_BAND_MIN_AVG_SCORE`), the same per-line MRZ-likelihood signal and
//!   threshold `detect_mrz_band_scored` already uses to decide whether a
//!   group of lines looks like an MRZ at all.
//! - **noise-line fraction** — of the *non-MRZ* lines, the fraction that are
//!   either <= 2 non-whitespace characters or >= 50% non-alphanumeric
//!   (whitespace excluded from both counts). This targets exactly the kind
//!   of OCR noise a dense/photographic visual zone produces: stray symbol
//!   fragments and short garbled tokens, not genuine short-but-clean field
//!   labels.
//!
//! Run from the repo root (models auto-resolve there):
//! ```powershell
//! cargo run -p synthpass-ocr --release --example visual_zone_survey -- --batch 5
//! ```

use std::path::{Path, PathBuf};
use synthpass_ocr::geometry::{self, OcrLine};
use synthpass_ocr::NativeOcr;

/// Mirrors `crate::MRZ_CHARSET` (not `pub` from `synthpass_ocr`'s root), so
/// this survey's per-line MRZ scoring uses the exact same charset the
/// production MRZ-band detector does.
const MRZ_CHARSET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789<";

/// A non-MRZ line this short (non-whitespace character count) is treated as
/// noise regardless of content — too little text to be a genuine field
/// label or caption.
const NOISE_MAX_LEN: usize = 2;

/// A non-MRZ line at or above this fraction of non-alphanumeric (and
/// non-whitespace) characters is treated as noise — symbol-fragment garbage
/// a garbled visual-zone read tends to produce.
const NOISE_NON_ALNUM_FRACTION: f64 = 0.5;

/// Mirrors `geometry::MRZ_BAND_MIN_AVG_SCORE` (not `pub` — it's an internal
/// tuning constant for `detect_mrz_band_scored`'s group-average threshold),
/// duplicated here as the per-line cutoff for "does this one line look
/// MRZ-shaped at all". Kept in lockstep deliberately: drifting from the
/// production value would make this survey's MRZ/non-MRZ split disagree
/// with what `recognize_detailed` itself considers a plausible MRZ line.
const MRZ_LINE_SCORE_THRESHOLD: f64 = 0.15;

#[derive(Debug, Default, Clone)]
struct SpecimenResult {
    filename: String,
    total_lines: usize,
    mrz_lines: usize,
    non_mrz_lines: usize,
    noise_lines: usize,
    /// `None` when there were zero non-MRZ lines to divide by (e.g. the
    /// geometry pass found nothing, or every detected line scored as MRZ).
    noise_line_fraction: Option<f64>,
}

fn is_mrz_line(line: &OcrLine) -> bool {
    geometry::mrz_line_score(line, MRZ_CHARSET) >= MRZ_LINE_SCORE_THRESHOLD
}

/// A non-MRZ line counts as noise if it's very short or mostly symbol/junk
/// characters — see the module docs for why these two signals were chosen.
fn is_noise_line(text: &str) -> bool {
    let non_ws: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if non_ws.is_empty() {
        return true;
    }
    if non_ws.len() <= NOISE_MAX_LEN {
        return true;
    }
    let non_alnum = non_ws.iter().filter(|c| !c.is_alphanumeric()).count();
    (non_alnum as f64 / non_ws.len() as f64) >= NOISE_NON_ALNUM_FRACTION
}

fn evaluate(lines: &[OcrLine]) -> SpecimenResult {
    let mut mrz_lines = 0usize;
    let mut non_mrz_lines = 0usize;
    let mut noise_lines = 0usize;

    for line in lines {
        if is_mrz_line(line) {
            mrz_lines += 1;
        } else {
            non_mrz_lines += 1;
            if is_noise_line(&line.text) {
                noise_lines += 1;
            }
        }
    }

    let noise_line_fraction = if non_mrz_lines == 0 {
        None
    } else {
        Some(noise_lines as f64 / non_mrz_lines as f64)
    };

    SpecimenResult {
        filename: String::new(),
        total_lines: lines.len(),
        mrz_lines,
        non_mrz_lines,
        noise_lines,
        noise_line_fraction,
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

/// Image extensions this survey will decode.
///
/// The walk below used to take every file it found, which meant the 19 loose
/// `samples/*.json` bench reports and `samples/README.md` were handed to the
/// OCR engine -- several seconds each, to produce nothing.
const SURVEY_IMAGE_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "gif"];

fn walk_samples(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_samples(&path, out);
        } else if path.is_file() {
            let is_image = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .is_some_and(|e| SURVEY_IMAGE_EXTENSIONS.contains(&e.as_str()));
            if is_image {
                out.push(path);
            }
        }
    }
}

fn median(sorted: &[f64]) -> f64 {
    percentile(sorted, 0.5)
}

fn percentile(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * pct).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn main() {
    let batch_size: usize = std::env::args()
        .position(|a| a == "--batch")
        .and_then(|i| std::env::args().nth(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);

    let only: Option<String> = std::env::args()
        .position(|a| a == "--only")
        .and_then(|i| std::env::args().nth(i + 1));

    let root = repo_root();
    let ocr = NativeOcr::load(
        &root.join("text-detection.rten"),
        &root.join("text-recognition.rten"),
    )
    .expect("failed to load OCR models — run from the repo root");

    // `--file <path>` evaluates one arbitrary image (e.g. an untracked
    // candidate specimen outside `samples/`) and exits without touching the
    // corpus JSONL below — a one-off comparison point, not a corpus member.
    if let Some(file_arg) = std::env::args()
        .position(|a| a == "--file")
        .and_then(|i| std::env::args().nth(i + 1))
    {
        let path = PathBuf::from(&file_arg);
        let page = ocr
            .recognize_detailed(&path)
            .unwrap_or_else(|e| panic!("OCR failed on {file_arg}: {e}"));
        let result = evaluate(&page.lines);
        match result.noise_line_fraction {
            Some(frac) => println!(
                "{file_arg}: total_lines={} mrz_lines={} non_mrz_lines={} noise_lines={} \
                 noise_line_fraction={frac:.3}",
                result.total_lines, result.mrz_lines, result.non_mrz_lines, result.noise_lines,
            ),
            None => println!(
                "{file_arg}: total_lines={} mrz_lines={} non_mrz_lines=0 noise_line_fraction=n/a",
                result.total_lines, result.mrz_lines,
            ),
        }
        return;
    }

    let mut files = Vec::new();
    walk_samples(&root.join("samples"), &mut files);
    files.sort();
    if let Some(substr) = &only {
        files.retain(|p| {
            p.file_name()
                .and_then(|f| f.to_str())
                .is_some_and(|f| f.contains(substr.as_str()))
        });
    }

    let mut results: Vec<SpecimenResult> = Vec::new();
    let mut jsonl_lines = Vec::new();

    let total = files.len();
    for (batch_idx, batch) in files.chunks(batch_size).enumerate() {
        println!(
            "\n=== batch {} ({}-{} of {total}) ===",
            batch_idx + 1,
            batch_idx * batch_size + 1,
            (batch_idx * batch_size + batch.len()).min(total)
        );

        for path in batch {
            let filename = path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("?")
                .to_string();

            let page = match ocr.recognize_detailed(path) {
                Ok(p) => p,
                Err(e) => {
                    println!("SKIP  {filename}: OCR error: {e}");
                    continue;
                }
            };

            let mut result = evaluate(&page.lines);
            result.filename = filename.clone();

            match result.noise_line_fraction {
                Some(frac) => println!(
                    "{filename}: total_lines={} mrz_lines={} non_mrz_lines={} noise_lines={} \
                     noise_line_fraction={frac:.3}",
                    result.total_lines, result.mrz_lines, result.non_mrz_lines, result.noise_lines,
                ),
                None => println!(
                    "{filename}: total_lines={} mrz_lines={} non_mrz_lines=0 noise_line_fraction=n/a",
                    result.total_lines, result.mrz_lines,
                ),
            }

            jsonl_lines.push(
                serde_json::json!({
                    "filename": result.filename,
                    "total_lines": result.total_lines,
                    "mrz_lines": result.mrz_lines,
                    "non_mrz_lines": result.non_mrz_lines,
                    "noise_lines": result.noise_lines,
                    "noise_line_fraction": result.noise_line_fraction,
                })
                .to_string(),
            );
            results.push(result);
        }
    }

    let fractions: Vec<f64> = results
        .iter()
        .filter_map(|r| r.noise_line_fraction)
        .collect();
    let mut sorted = fractions.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));

    println!("\n=== corpus-wide noise-line fraction ===");
    println!(
        "n={} (of {} specimens; the rest had zero non-MRZ lines) median={:.3} p25={:.3} p75={:.3}",
        sorted.len(),
        results.len(),
        median(&sorted),
        percentile(&sorted, 0.25),
        percentile(&sorted, 0.75),
    );

    // Per-specimen table sorted worst-to-best, so an outlier is easy to spot
    // by eye without grepping the JSONL.
    // Collected into a Vec, not a map keyed on `filename`: `walk_samples`
    // recurses, and `filename` is a bare basename, so two specimens sharing a
    // basename across subdirectories would collide and one would vanish from
    // this table without a word. The median/IQR above read `results` directly
    // and were never affected — only this ranking was.
    let mut ranked: Vec<(&str, f64)> = results
        .iter()
        .filter_map(|r| r.noise_line_fraction.map(|f| (r.filename.as_str(), f)))
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    println!("\n=== ranked by noise-line fraction (highest first) ===");
    for (name, frac) in &ranked {
        println!("{frac:.3}  {name}");
    }

    // `knowledge/visual-zone-survey.jsonl` is a checked-in, full-corpus
    // artifact that prose cites. `--only` deliberately narrows the run to a
    // handful of specimens, so writing it here would silently replace the
    // committed dataset with a partial one — a corruption that survives into
    // `git diff` looking like a legitimate re-measurement. Same reasoning the
    // `--file` path above already applies to itself ("a one-off comparison
    // point, not a corpus member"), just applied to the other filter too.
    if let Some(substr) = &only {
        println!(
            "\nskipping {} — this was a --only={substr} run over {} of {} specimens, \
             not the full corpus the checked-in file records",
            root.join("knowledge")
                .join("visual-zone-survey.jsonl")
                .display(),
            results.len(),
            total,
        );
        return;
    }

    let out_path = root.join("knowledge").join("visual-zone-survey.jsonl");
    if let Err(e) = std::fs::write(&out_path, jsonl_lines.join("\n") + "\n") {
        eprintln!("failed to write {}: {e}", out_path.display());
    } else {
        println!("\nwrote {}", out_path.display());
    }
}
