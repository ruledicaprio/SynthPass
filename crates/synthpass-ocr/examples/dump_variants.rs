//! Cell (c) diagnostic (ADR-0008 chunk 1C): dump the exact preprocessed images
//! `ocrs` is fed for a hand-picked set of documents, so a *different* recognizer
//! can be run offline over the identical bytes.
//!
//! Cell (b) (`knowledge/benchmarks/ocr-gap-is-detection-2026-09-10.md`) found
//! that on the ~11 documents driving the browser/native gap, `ocrs` returns
//! near-empty text for the whole page — general pass and every retry variant.
//! This tells "native's crop is unusable" apart from "native's recognizer is the
//! weak link": if tesseract's OCR-B model reads a valid MRZ off native's own
//! crop, the crop is fine and the recognizer is the gap; if it also fails, the
//! whole gap is native detection/preprocessing.
//!
//! Run from the repo root (models auto-resolve there):
//! ```powershell
//! $env:SYNTHPASS_OCR_DUMP_VARIANTS = "artifacts/cell-c/crops"
//! cargo run -p synthpass-ocr --release --example dump_variants -- `
//!   Canada_Passport_Specimen_2023_mrz.jpg samples/passports/Oman_Passport_Specimen_P0_OMN_2004_mrz.jpg
//! ```
//!
//! Each argument is either a path to an image or a bare filename found anywhere
//! under `samples/`. The dump itself is done by
//! `NativeOcr::recognize_detailed` when `SYNTHPASS_OCR_DUMP_VARIANTS` is set —
//! this binary only drives it over a chosen list rather than the whole corpus
//! (which would be gigabytes of PNGs). Pair the output with the
//! `SYNTHPASS_OCR_VERBOSE=1` pass log and the cell (b) `--dump-ocr` JSONL.

use std::path::{Path, PathBuf};

use synthpass_ocr::NativeOcr;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/synthpass-ocr → repo root is two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

/// Resolves an argument to an image path: used verbatim if it exists, otherwise
/// searched for by bare filename anywhere under `samples/` (the same recursive
/// lookup `mrz_corpus.rs` uses, so a specimen can be named without its subpath).
fn resolve(arg: &str, samples_root: &Path) -> Option<PathBuf> {
    let direct = PathBuf::from(arg);
    if direct.is_file() {
        return Some(direct);
    }
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
    search(samples_root, arg)
}

fn main() {
    let dump_dir = std::env::var("SYNTHPASS_OCR_DUMP_VARIANTS")
        .ok()
        .filter(|s| !s.trim().is_empty());
    if dump_dir.is_none() {
        eprintln!(
            "SYNTHPASS_OCR_DUMP_VARIANTS is not set — nothing would be written. Set it to an \
             output directory (e.g. artifacts/cell-c/crops) and re-run."
        );
        std::process::exit(2);
    }
    let dump_dir = dump_dir.unwrap();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!(
            "usage: dump_variants <image-or-samples-filename>...\n\
             (SYNTHPASS_OCR_DUMP_VARIANTS must point at an output directory)"
        );
        std::process::exit(2);
    }

    let root = repo_root();
    let samples_root = root.join("samples");
    let ocr = NativeOcr::load(
        &root.join("text-detection.rten"),
        &root.join("text-recognition.rten"),
    )
    .expect("failed to load OCR models — run from the repo root");

    let mut ok = 0usize;
    let mut failed = 0usize;
    for arg in &args {
        let Some(path) = resolve(arg, &samples_root) else {
            eprintln!("SKIP  {arg}: not a file and not found under samples/");
            failed += 1;
            continue;
        };
        match ocr.recognize_detailed(&path) {
            Ok(page) => {
                let parsed = mrz::find_and_parse(&page.text);
                let verdict = match &parsed {
                    Ok(d) if d.valid() => "valid MRZ",
                    Ok(_) => "MRZ parsed, checksums fail",
                    Err(_) => "no MRZ",
                };
                println!(
                    "OK    {}: {} OCR chars, band {:?}, {verdict} — crops in {dump_dir}/",
                    path.display(),
                    page.text.len(),
                    page.mrz_band_score.map(|s| format!("{s:.2}")),
                );
                ok += 1;
            }
            Err(e) => {
                eprintln!("MISS  {}: OCR error: {e}", path.display());
                failed += 1;
            }
        }
    }

    println!("\n{ok} document(s) processed, {failed} skipped/failed. Preprocessed crops written to {dump_dir}/.");
    if ok == 0 {
        std::process::exit(1);
    }
}
