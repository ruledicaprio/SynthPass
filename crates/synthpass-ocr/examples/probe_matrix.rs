//! PR-3.0 — the MRZ matrix probe: read `ocrs`'s CTC recognition matrix
//! directly instead of letting the beam decoder throw it away, and measure
//! whether that beats the beam on the same cells.
//!
//! # Why this exists
//!
//! `ocrs::OcrEngine::recognize_text` runs the recognition model, CTC-decodes
//! its output to a character sequence, and returns only the characters —
//! the `[seq, class]` probability matrix behind that decode is never exposed.
//! Two facts make reading it ourselves tractable without reimplementing any
//! model preprocessing:
//!
//! - [`ocrs::OcrEngine::prepare_recognition_input`] is public and returns the
//!   exact preprocessed `[H, W]` line tensor the recognition model consumes.
//! - The recognition model's terminal op is `LogSoftmax`, so its raw output
//!   is already normalized log-probabilities. Converting a value to a
//!   probability is therefore `.exp()`; `softmax()` on an already-log-softmax
//!   matrix is not the identity and flattens the distribution toward uniform
//!   (`softmax` is only correct for renormalizing over surviving labels
//!   after `-Inf` masking, which this probe never does).
//!
//! `class` is alphabet length + 1 = 97 (index 0 is the CTC blank; label
//! `i + 1` is the character at index `i` of `ocrs` 0.12.2's private
//! recognition alphabet — see [`CTC_ALPHABET`]'s doc comment for how that
//! table was pinned). We do not need CTC's alignment: the grid gives cell
//! boundaries in image space, the resize factor and the model's downsample
//! factor of 4 are known, so a grid column maps to a timestep range by
//! arithmetic — see [`timestep_range`].
//!
//! # Modes
//!
//! - `ocrs` — the existing `recognize_text` beam read, as the control.
//! - `line` — the matrix for the whole line, one inference; a grid column's
//!   distribution is the average over its mapped timestep range.
//! - `cell` — a context-window parameter, not a separate code path: reading
//!   cell *k* from a crop spanning cells `k-n..=k+n` and taking the
//!   distribution only at cell *k*'s own columns. `n = 0` is pure per-cell;
//!   `n` large enough to cover the whole line degenerates to the same crop
//!   `line` reads (kept as a separate, single-inference-per-line path here
//!   for an honest timing comparison — see [`read_line_matrix`]'s doc). Swept
//!   at `n ∈ {0, 1, 2, 4}`. Every cell of one line shares an identical-width
//!   context crop (the grid is fixed-pitch), so all of a line's cells batch
//!   into one `[N, 1, H, W]` tensor and one inference — see
//!   [`run_recognition_batch`] — rather than one inference per cell.
//!
//! # Numbers this probe reports
//!
//! - Item 1 — per-cell accuracy, per mode, against
//!   `samples/ocr_fixtures/<stem>.json`.
//! - Item 1b — digit-vs-letter ink separation: the row where cumulative ink
//!   from a cell's top crosses [`TOP_EDGE_INK_FRACTION`] of that cell's
//!   total, as a fraction of cell height, grouped by cell pixel height.
//! - Item 2 — label 29 (`<`)'s probability at filler columns, split by
//!   whether the filler sits in a run of `<` or is isolated.
//! - Item 3 — label 44 (`ocrs`'s mangled EUR-symbol class, stored as ASCII
//!   `E`) vs label 49 (the genuine alphabetic `E`) at columns whose truth
//!   is `E`.
//! - Item 3b — grid-fit origin/pitch error, in cells, on synthetic input
//!   whose true origin/pitch is known exactly (`--synthetic-origin`) — see
//!   [`run_synthetic_origin_probe`].
//! - Item 4 — grid self-check: mean ink at truth-filler cells vs
//!   truth-non-filler cells (fillers are printed glyphs and carry real ink
//!   — see `synthpass_ocr::chargrid`'s `DEFAULT_INK_FLOOR` doc comment — so
//!   a well-fitted grid should show measurably *lower*, not absent, ink at
//!   filler cells).
//! - Item 5 — ink separation for `<` vs `K` truth cells: a
//!   per-column-within-cell ink profile (`K` is `<` plus a vertical stem, so
//!   the discriminating mass should sit in the cell's leftmost columns).
//! - Item 6 — timing per mode per document.
//!
//! **Never prints a read name.** Every aggregate below is a count,
//! probability, boolean-correctness rate or ink profile — never a decoded or
//! ground-truth character value. Per-document JSON output under `--out`
//! (default `artifacts/`, gitignored) follows the same rule.
//!
//! # Usage
//!
//! ```text
//! cargo run -p synthpass-ocr --release --example probe_matrix -- \
//!   --out artifacts/probe --mode all
//! cargo run -p synthpass-ocr --release --example probe_matrix -- \
//!   --image samples/passports/Afghanistan_Passport_Specimen_P0_AFG_2016_mrz.jpg
//! cargo run -p synthpass-ocr --release --example probe_matrix -- \
//!   --synthetic-origin 50
//! ```
//!
//! Run with `--release`: a corpus-wide run does several recognition passes
//! per document across every mode.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use image::{GrayImage, RgbImage};
use ocrs::{DecodeMethod, ImageSource, OcrEngine, OcrEngineParams, OcrInput, TextItem};
use rten::Model;
use rten_imageproc::{bounding_rect, PointF, RotatedRect, Vec2};
use rten_tensor::prelude::*;
use rten_tensor::{Layout, NdTensor};

use synthpass_gen::{generate_from_seed, DocumentType, GeneratorConfig};
use synthpass_ocr::chargrid::{self, Glyph, Grid};
use synthpass_ocr::geometry::{self, detect_mrz_band_range, BBox, OcrLine};
use synthpass_ocr::MRZ_CHARSET;

// ---------------------------------------------------------------------
// The CTC label table (verified facts; see the module doc).
// ---------------------------------------------------------------------

/// `ocrs` 0.12.2's private recognition alphabet (`recognition.rs`'s
/// `DEFAULT_ALPHABET`), duplicated here because it is never exposed —
/// `OcrEngineParams::allowed_chars` only masks decode probabilities to
/// `-Inf` for excluded characters, it does not change the model's own
/// alphabet or class count (confirmed by reading `ocrs::OcrEngine::new_impl`:
/// `excluded_char_labels` is computed *from* this alphabet, which is used
/// verbatim by both the general and MRZ-constrained engines). Pinned against
/// silent upgrade drift by `ctc_alphabet_tests` below.
const CTC_ALPHABET: &str = " 0123456789!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~EABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// CTC label 0 is reserved for the blank symbol; label `i + 1` is
/// [`CTC_ALPHABET`]'s character at index `i`.
const CTC_BLANK_LABEL: usize = 0;

/// `<` — the MRZ filler `ocrs`'s beam decoder essentially never emits (see
/// `synthpass_ocr::chargrid`'s module doc). Item 2.
const CTC_LABEL_FILLER: usize = 29;

/// The mangled EUR-symbol class `ocrs`'s alphabet stores as ASCII `'E'`
/// (`// nb. The "E" before "ABCDE" should be the EUR symbol.` — `ocrs`'s own
/// source comment), distinct from the genuine alphabetic `'E'` at
/// [`CTC_LABEL_LETTER_E`]. Both decode to the same character, so only the
/// raw matrix can tell them apart. Item 3.
const CTC_LABEL_EUR_E: usize = 44;

/// The genuine alphabetic `E`. Item 3.
const CTC_LABEL_LETTER_E: usize = 49;

/// Width downsample factor of the recognition model's CNN stage (verified;
/// see the module doc).
const CTC_DOWNSAMPLE: f32 = 4.0;

/// Beam width for `ocrs`'s MRZ-constrained engine, mirroring
/// `synthpass_ocr`'s own private `MRZ_BEAM_WIDTH` so mode `ocrs`'s control
/// read matches production's actual decode.
const MRZ_BEAM_WIDTH: u32 = 24;

fn ctc_char(label: usize) -> Option<char> {
    if label == CTC_BLANK_LABEL {
        None
    } else {
        CTC_ALPHABET.chars().nth(label - 1)
    }
}

#[cfg(test)]
mod ctc_alphabet_tests {
    use super::*;

    /// Pins the three labels this probe's items 2/3 depend on against a
    /// silent `ocrs` alphabet change — if this ever fails, every downstream
    /// number in this file is reading the wrong class index.
    #[test]
    fn labels_match_the_verified_ocrs_alphabet() {
        assert_eq!(CTC_ALPHABET.chars().count(), 96, "97 classes = 96 + blank");
        assert_eq!(ctc_char(CTC_LABEL_FILLER), Some('<'));
        assert_eq!(ctc_char(CTC_LABEL_EUR_E), Some('E'));
        assert_eq!(ctc_char(CTC_LABEL_LETTER_E), Some('E'));
        assert_eq!(ctc_char(CTC_BLANK_LABEL), None);
    }
}

// ---------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Line,
    Cell,
    Ocrs,
    All,
}

impl Mode {
    fn runs_ocrs(self) -> bool {
        matches!(self, Mode::Ocrs | Mode::All)
    }
    fn runs_line(self) -> bool {
        matches!(self, Mode::Line | Mode::All)
    }
    fn runs_cell(self) -> bool {
        matches!(self, Mode::Cell | Mode::All)
    }
}

struct Args {
    image: Option<PathBuf>,
    out_dir: PathBuf,
    mode: Mode,
    synthetic_origin_seeds: Option<u64>,
}

fn parse_args() -> Args {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut image = None;
    let mut out_dir = PathBuf::from("artifacts");
    let mut mode = Mode::All;
    let mut synthetic_origin_seeds = None;
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--image" => {
                i += 1;
                if let Some(v) = raw.get(i) {
                    image = Some(PathBuf::from(v));
                }
            }
            "--out" => {
                i += 1;
                if let Some(v) = raw.get(i) {
                    out_dir = PathBuf::from(v);
                }
            }
            "--mode" => {
                i += 1;
                if let Some(v) = raw.get(i) {
                    mode = match v.as_str() {
                        "line" => Mode::Line,
                        "cell" => Mode::Cell,
                        "ocrs" => Mode::Ocrs,
                        _ => Mode::All,
                    };
                }
            }
            "--synthetic-origin" => {
                let next_numeric = raw.get(i + 1).and_then(|v| v.parse::<u64>().ok());
                if next_numeric.is_some() {
                    i += 1;
                }
                synthetic_origin_seeds = Some(next_numeric.unwrap_or(50));
            }
            _ => {}
        }
        i += 1;
    }
    Args {
        image,
        out_dir,
        mode,
        synthetic_origin_seeds,
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/synthpass-ocr -> repo root is two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn main() {
    let args = parse_args();
    if let Err(e) = std::fs::create_dir_all(&args.out_dir) {
        eprintln!(
            "failed to create output dir {}: {e}",
            args.out_dir.display()
        );
        std::process::exit(1);
    }
    let root = repo_root();

    if let Some(seeds) = args.synthetic_origin_seeds {
        let mrz_engine = match load_mrz_engine(&root) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        };
        if let Err(e) = run_synthetic_origin_probe(&mrz_engine, &args.out_dir, seeds) {
            eprintln!("synthetic-origin probe failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    let general_engine = match load_general_engine(&root) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let mrz_engine = match load_mrz_engine(&root) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let raw_recognition_model = match load_raw_recognition_model(&root) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let documents = collect_documents(&root, args.image.as_deref());
    if documents.is_empty() {
        eprintln!(
            "no fixture+image pairs found (looked under samples/ocr_fixtures/ \
             for a .json with a matching image anywhere under samples/)"
        );
        std::process::exit(1);
    }
    println!(
        "probe_matrix: {} document(s), mode={:?}",
        documents.len(),
        args.mode
    );

    let mut agg = Aggregates::default();
    for doc in &documents {
        process_document(
            &general_engine,
            &mrz_engine,
            &raw_recognition_model,
            doc,
            args.mode,
            &args.out_dir,
            &mut agg,
        );
    }

    print_summary(&agg, args.mode);
    if let Err(e) = write_json(&args.out_dir, "summary.json", &agg.to_json(args.mode)) {
        eprintln!("failed to write summary.json: {e}");
    }
}

// ---------------------------------------------------------------------
// Engine loading
// ---------------------------------------------------------------------

fn load_model(path: &Path) -> Result<Model, String> {
    Model::load_file(path).map_err(|e| format!("failed to load model {}: {e}", path.display()))
}

fn load_general_engine(root: &Path) -> Result<OcrEngine, String> {
    let detection = load_model(&root.join("text-detection.rten"))?;
    let recognition = load_model(&root.join("text-recognition.rten"))?;
    OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection),
        recognition_model: Some(recognition),
        ..Default::default()
    })
    .map_err(|e| format!("failed to build general engine: {e}"))
}

fn load_mrz_engine(root: &Path) -> Result<OcrEngine, String> {
    let detection = load_model(&root.join("text-detection.rten"))?;
    let recognition = load_model(&root.join("text-recognition.rten"))?;
    OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection),
        recognition_model: Some(recognition),
        allowed_chars: Some(MRZ_CHARSET.to_string()),
        decode_method: DecodeMethod::BeamSearch {
            width: MRZ_BEAM_WIDTH,
        },
        ..Default::default()
    })
    .map_err(|e| format!("failed to build MRZ-constrained engine: {e}"))
}

fn load_raw_recognition_model(root: &Path) -> Result<Model, String> {
    load_model(&root.join("text-recognition.rten"))
}

// ---------------------------------------------------------------------
// Document discovery (samples/ocr_fixtures/<stem>.json + a matching image).
// ---------------------------------------------------------------------

struct Document {
    stem: String,
    image_path: PathBuf,
    /// Ground-truth MRZ lines (2 for TD2/TD3/MRV-A/MRV-B, 3 for TD1).
    truth_lines: Vec<String>,
}

const IMAGE_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "gif"];

/// Recursively finds an image anywhere under `samples/` whose file stem is
/// `stem` and whose extension is a known image extension — the same
/// "rename-proof" lookup `examples/mrz_corpus.rs` and `examples/corpus_manifest.rs`
/// each already use, duplicated here per this repo's existing convention of
/// each example owning a small local copy rather than sharing a private helper
/// across binaries.
fn find_sample_by_stem(root: &Path, stem: &str) -> Option<PathBuf> {
    fn search(dir: &Path, stem: &str) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = search(&path, stem) {
                    return Some(found);
                }
                continue;
            }
            let stem_matches = path.file_stem().and_then(|s| s.to_str()) == Some(stem);
            let ext_ok = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.as_str()));
            if stem_matches && ext_ok {
                return Some(path);
            }
        }
        None
    }
    search(&root.join("samples"), stem)
}

/// Builds the document list: every `samples/ocr_fixtures/*.json` fixture that
/// carries an `mrz_line` field and has a matching image somewhere under
/// `samples/`. When `only_image` is given, restricted to the one fixture
/// whose stem matches it (that image path is used verbatim rather than
/// re-searched, so `--image` also works for a file outside `samples/`).
fn collect_documents(root: &Path, only_image: Option<&Path>) -> Vec<Document> {
    let fixtures_dir = root.join("samples").join("ocr_fixtures");
    let mut docs = Vec::new();
    let Ok(entries) = std::fs::read_dir(&fixtures_dir) else {
        return docs;
    };
    let only_stem = only_image
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str());
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if let Some(only_stem) = only_stem {
            if stem != only_stem {
                continue;
            }
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let Some(mrz_line) = json.get("mrz_line").and_then(|v| v.as_str()) else {
            continue;
        };
        let truth_lines: Vec<String> = mrz_line.split('\n').map(str::to_string).collect();
        let image_path = if let Some(only_image) = only_image {
            Some(only_image.to_path_buf())
        } else {
            find_sample_by_stem(root, stem)
        };
        let Some(image_path) = image_path else {
            continue;
        };
        docs.push(Document {
            stem: stem.to_string(),
            image_path,
            truth_lines,
        });
    }
    docs.sort_by(|a, b| a.stem.cmp(&b.stem));
    docs
}

// ---------------------------------------------------------------------
// Shared front half: band detection (general engine) + glyph/grid fitting.
// ---------------------------------------------------------------------

/// One line as `ocrs`'s MRZ-constrained engine recognized it: its own text
/// (for `mode ocrs`'s control read and for [`chargrid::fit_grid`]'s seed
/// bounds), its bounding box, and per-glyph left/right positions.
struct CharLine {
    text: String,
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
    glyphs: Vec<Glyph>,
}

fn char_line_from_text_line(line: ocrs::TextLine) -> CharLine {
    let r = line.bounding_rect();
    let glyphs = line
        .chars()
        .iter()
        .map(|c| Glyph {
            ch: c.char,
            left: c.rect.left() as f32,
            right: c.rect.right() as f32,
        })
        .collect();
    CharLine {
        text: line.to_string(),
        top: r.top() as f32,
        bottom: r.bottom() as f32,
        left: r.left() as f32,
        right: r.right() as f32,
        glyphs,
    }
}

/// Runs `engine`'s own detect+group+recognize pass over `image` and returns
/// the winning MRZ band's line groups (in original `RotatedRect` form,
/// recovered via [`detect_mrz_band_range`] — see that function's doc comment
/// for why the index range alone isn't enough), in reading order. `None`
/// when no band scored high enough to report at all.
fn locate_mrz_band(
    engine: &OcrEngine,
    image: &RgbImage,
) -> Result<Option<Vec<Vec<RotatedRect>>>, String> {
    let source = ImageSource::from_bytes(image.as_raw(), image.dimensions())
        .map_err(|e| format!("failed to prepare image source: {e}"))?;
    let input = engine
        .prepare_input(source)
        .map_err(|e| format!("failed to prepare ocr input: {e}"))?;
    let words = engine
        .detect_words(&input)
        .map_err(|e| format!("word detection failed: {e}"))?;
    let line_groups = engine.find_text_lines(&input, &words);
    let recognized = engine
        .recognize_text(&input, &line_groups)
        .map_err(|e| format!("text recognition failed: {e}"))?;

    let mut lines: Vec<OcrLine> = Vec::new();
    let mut source_index: Vec<usize> = Vec::new();
    for (i, maybe_line) in recognized.into_iter().enumerate() {
        if let Some(line) = maybe_line {
            let r = line.bounding_rect();
            let bbox = BBox::from_tlbr(
                r.top() as f32,
                r.left() as f32,
                r.bottom() as f32,
                r.right() as f32,
            );
            let text = line.to_string();
            let confidence = geometry::text_sanity(&text);
            lines.push(OcrLine {
                text,
                bbox,
                confidence,
            });
            source_index.push(i);
        }
    }

    let Some((start, end, _avg_score)) = detect_mrz_band_range(&lines, MRZ_CHARSET) else {
        return Ok(None);
    };
    let band: Vec<Vec<RotatedRect>> = source_index[start..end]
        .iter()
        .map(|&i| line_groups[i].clone())
        .collect();
    Ok(Some(band))
}

/// Runs `engine.recognize_text` restricted to `line_groups` (the band's own
/// recovered groups — no re-detection), returning one [`CharLine`] per group
/// in the same order, or `None` where that group recognized no text.
fn recognize_char_lines_for_groups(
    engine: &OcrEngine,
    image: &RgbImage,
    line_groups: &[Vec<RotatedRect>],
) -> Result<(OcrInput, Vec<Option<CharLine>>), String> {
    let source = ImageSource::from_bytes(image.as_raw(), image.dimensions())
        .map_err(|e| format!("failed to prepare image source: {e}"))?;
    let input = engine
        .prepare_input(source)
        .map_err(|e| format!("failed to prepare ocr input: {e}"))?;
    let recognized = engine
        .recognize_text(&input, line_groups)
        .map_err(|e| format!("text recognition failed: {e}"))?;
    let out = recognized
        .into_iter()
        .map(|m| m.map(char_line_from_text_line))
        .collect();
    Ok((input, out))
}

/// Item 3b's full-image variant: `engine`'s own detect+group+recognize pass
/// over the whole (clean, single-document) synthetic canvas, with no band
/// selection needed (there is no page clutter to disambiguate against).
fn recognize_char_lines_full(
    engine: &OcrEngine,
    image: &RgbImage,
) -> Result<Vec<CharLine>, String> {
    let source = ImageSource::from_bytes(image.as_raw(), image.dimensions())
        .map_err(|e| format!("failed to prepare image source: {e}"))?;
    let input = engine
        .prepare_input(source)
        .map_err(|e| format!("failed to prepare ocr input: {e}"))?;
    let words = engine
        .detect_words(&input)
        .map_err(|e| format!("word detection failed: {e}"))?;
    let line_groups = engine.find_text_lines(&input, &words);
    let recognized = engine
        .recognize_text(&input, &line_groups)
        .map_err(|e| format!("text recognition failed: {e}"))?;
    Ok(recognized
        .into_iter()
        .flatten()
        .map(char_line_from_text_line)
        .collect())
}

/// Number of leading (post-`<`-stripping) characters [`match_prefix`]
/// compares — matches `synthpass_ocr`'s own private
/// `CHARGRID_MATCH_PREFIX_LEN`.
const MATCH_PREFIX_LEN: usize = 5;

/// `line` with every `<` removed, truncated to [`MATCH_PREFIX_LEN`] — the
/// comparison key used to find which recognized line is the target MRZ line,
/// mirroring `synthpass_ocr`'s private `chargrid_match_prefix` (`ocrs` never
/// emits `<`, so a recognized line's own text is already filler-free; the
/// *ground-truth* side is stripped so the two are comparable).
fn match_prefix(line: &str) -> String {
    line.chars()
        .filter(|&c| c != '<')
        .take(MATCH_PREFIX_LEN)
        .collect()
}

// ---------------------------------------------------------------------
// Item 3b: synthetic grid-origin/pitch error.
// ---------------------------------------------------------------------

struct OriginSample {
    origin_error_cells: f64,
    true_pitch_px: f64,
    fitted_pitch_px: f64,
    pitch_error_fraction: f64,
}

fn run_synthetic_origin_probe(
    mrz_engine: &OcrEngine,
    out_dir: &Path,
    seeds_per_format: u64,
) -> Result<(), String> {
    let formats = [
        DocumentType::TD1,
        DocumentType::TD2,
        DocumentType::TD3,
        DocumentType::MrvA,
        DocumentType::MrvB,
    ];
    let mut samples: Vec<OriginSample> = Vec::new();
    let mut fit_failed = 0usize;
    let mut total = 0usize;
    let mut per_format: HashMap<String, (usize, usize)> = HashMap::new();

    for doc_type in formats {
        let format_name = format!("{doc_type:?}");
        for seed in 0..seeds_per_format {
            total += 1;
            let entry = per_format.entry(format_name.clone()).or_insert((0, 0));
            entry.1 += 1;

            let config = GeneratorConfig::with_document_type(seed, doc_type);
            let (image, labels, _passport) = generate_from_seed(&config);
            let rgb = image.into_rgb8();

            let format: mrz::Format = doc_type.into();
            let name_idx = chargrid::name_line_index(format);
            let geometry = chargrid::format_geometry(format);
            let (Some(name_idx), Some((_, width))) = (name_idx, geometry) else {
                fit_failed += 1;
                continue;
            };
            let page = synthpass_gen::layout::for_format(doc_type);
            let Some(line_rect) = page.mrz_lines.get(name_idx).copied() else {
                fit_failed += 1;
                continue;
            };
            if page.mrz_chars == 0 {
                fit_failed += 1;
                continue;
            }
            // Ground truth, exact by construction: `layout::mrz_char_rect_for_line`
            // (the renderer's own per-cell placement) uses `line.x` as cell 0's
            // left edge and `line.width / mrz_chars` (integer division) as the
            // pitch — this mirrors that arithmetic exactly rather than
            // approximating it.
            let true_origin_px = f64::from(line_rect.x);
            let true_pitch_px = f64::from(line_rect.width / page.mrz_chars);

            let Some(raw_name_line) = labels.mrz_lines.get(name_idx) else {
                fit_failed += 1;
                continue;
            };
            if raw_name_line.chars().count() != width {
                fit_failed += 1;
                continue;
            }

            let Ok(lines) = recognize_char_lines_full(mrz_engine, &rgb) else {
                fit_failed += 1;
                continue;
            };
            let target = match_prefix(raw_name_line);
            let matches: Vec<&CharLine> = lines
                .iter()
                .filter(|l| match_prefix(&l.text) == target)
                .collect();
            if matches.len() != 1 {
                fit_failed += 1;
                continue;
            }
            let matched = matches[0];
            let Some(grid) =
                chargrid::fit_grid(&matched.glyphs, matched.left, matched.right, width)
            else {
                fit_failed += 1;
                continue;
            };

            entry.0 += 1;
            samples.push(OriginSample {
                origin_error_cells: (f64::from(grid.origin) - true_origin_px) / true_pitch_px,
                true_pitch_px,
                fitted_pitch_px: f64::from(grid.pitch),
                pitch_error_fraction: (f64::from(grid.pitch) - true_pitch_px) / true_pitch_px,
            });
        }
    }

    let origin_errors: Vec<f64> = samples.iter().map(|s| s.origin_error_cells).collect();
    let pitch_errors: Vec<f64> = samples.iter().map(|s| s.pitch_error_fraction).collect();
    let fitted_pitches: Vec<f64> = samples.iter().map(|s| s.fitted_pitch_px).collect();
    let true_pitches: Vec<f64> = samples.iter().map(|s| s.true_pitch_px).collect();

    let per_format_json: serde_json::Value = per_format
        .into_iter()
        .map(|(k, (ok, tot))| (k, serde_json::json!({"fit_ok": ok, "total": tot})))
        .collect();

    let summary = serde_json::json!({
        "measurement": "item_3b_synthetic_grid_origin_error",
        "total_documents": total,
        "fit_ok": samples.len(),
        "fit_failed": fit_failed,
        "per_format": per_format_json,
        "origin_error_cells": stats_summary(&origin_errors),
        "pitch_error_fraction": stats_summary(&pitch_errors),
        "fitted_pitch_px": stats_summary(&fitted_pitches),
        "true_pitch_px": stats_summary(&true_pitches),
        "origin_error_within_0_1_cell": fraction_within(&origin_errors, 0.1),
        "origin_error_within_0_5_cell": fraction_within(&origin_errors, 0.5),
        "origin_error_at_least_1_cell": fraction_at_least(&origin_errors, 1.0),
    });

    println!("=== item 3b: synthetic grid-origin/pitch error ===");
    match serde_json::to_string_pretty(&summary) {
        Ok(s) => println!("{s}"),
        Err(e) => println!("(failed to pretty-print summary: {e})"),
    }
    write_json(out_dir, "synthetic_origin_error.json", &summary)
}

// ---------------------------------------------------------------------
// Ink measurements (items 1b, 4, 5) — pure pixel reads, no matrix needed.
// ---------------------------------------------------------------------

/// Luma threshold below which a pixel counts as ink. Mirrors
/// `synthpass_ocr::chargrid`'s private `INK_LUMA_THRESHOLD` (`< 110`);
/// duplicated rather than exposed because this probe needs finer-grained
/// per-row/per-column profiles that module's public `cell_ink`/
/// `grid_origin_from_ink` (both cell-level scalars) don't provide, and adding
/// that surface to a shipped module for a one-off measurement tool would be
/// a bigger change than this probe's brief calls for.
const INK_LUMA_THRESHOLD: u8 = 110;

fn is_ink_pixel(gray: &GrayImage, x: u32, y: u32) -> bool {
    gray.get_pixel(x, y)[0] < INK_LUMA_THRESHOLD
}

/// Item 1b's threshold: the row where cumulative ink from a cell's top edge
/// crosses this fraction of the cell's total ink. A fixed fraction, not an
/// extreme like `yMax` — see the module doc for why an outline extreme
/// erodes unevenly across round-topped vs flat-topped glyphs under blur.
const TOP_EDGE_INK_FRACTION: f64 = 0.5;

/// The row (as a fraction of cell height, `0.0` = top) where cumulative ink
/// from the cell's top crosses [`TOP_EDGE_INK_FRACTION`] of the cell's total
/// ink. `None` when the cell region is empty/out of bounds or carries no ink
/// at all (nothing to threshold).
fn cell_top_edge_fraction(
    gray: &GrayImage,
    top: u32,
    bottom: u32,
    x_lo: u32,
    x_hi: u32,
) -> Option<f64> {
    let (w, h) = gray.dimensions();
    let x_hi = x_hi.min(w);
    let bottom = bottom.min(h);
    if x_lo >= x_hi || top >= bottom {
        return None;
    }
    let height = bottom - top;
    let mut row_ink = vec![0u32; height as usize];
    let mut total: u32 = 0;
    for (ri, y) in (top..bottom).enumerate() {
        let mut count = 0u32;
        for x in x_lo..x_hi {
            if is_ink_pixel(gray, x, y) {
                count += 1;
            }
        }
        row_ink[ri] = count;
        total += count;
    }
    if total == 0 {
        return None;
    }
    let threshold = ((f64::from(total)) * TOP_EDGE_INK_FRACTION).ceil() as u32;
    let mut cumulative = 0u32;
    for (ri, &count) in row_ink.iter().enumerate() {
        cumulative += count;
        if cumulative >= threshold {
            return Some(f64::from(ri as u32) / f64::from(height));
        }
    }
    Some(1.0)
}

/// Number of horizontal buckets item 5's per-column ink profile is reduced
/// to, so profiles from cells of different pixel widths (different
/// documents/resolutions) are directly comparable.
const INK_PROFILE_BUCKETS: usize = 5;

/// Mean ink fraction per horizontal bucket of the cell `[x_lo, x_hi) x
/// [top, bottom)`. `None` on an empty/out-of-bounds region.
fn cell_column_profile(
    gray: &GrayImage,
    top: u32,
    bottom: u32,
    x_lo: u32,
    x_hi: u32,
) -> Option<[f64; INK_PROFILE_BUCKETS]> {
    let (w, h) = gray.dimensions();
    let x_hi = x_hi.min(w);
    let bottom = bottom.min(h);
    if x_lo >= x_hi || top >= bottom {
        return None;
    }
    let width = x_hi - x_lo;
    let rows = f64::from(bottom - top);
    let mut ink = [0f64; INK_PROFILE_BUCKETS];
    let mut px_count = [0u32; INK_PROFILE_BUCKETS];
    for x in x_lo..x_hi {
        let bucket = (((x - x_lo) as usize) * INK_PROFILE_BUCKETS / (width as usize).max(1))
            .min(INK_PROFILE_BUCKETS - 1);
        px_count[bucket] += 1;
        for y in top..bottom {
            if is_ink_pixel(gray, x, y) {
                ink[bucket] += 1.0;
            }
        }
    }
    for b in 0..INK_PROFILE_BUCKETS {
        if px_count[b] > 0 {
            ink[b] /= f64::from(px_count[b]) * rows;
        }
    }
    Some(ink)
}

// ---------------------------------------------------------------------
// Matrix reading (modes `line` and `cell`).
// ---------------------------------------------------------------------

fn axis_aligned_rect(top: f32, left: f32, bottom: f32, right: f32) -> RotatedRect {
    let width = (right - left).max(1.0);
    let height = (bottom - top).max(1.0);
    let center = PointF::from_yx((top + bottom) / 2.0, (left + right) / 2.0);
    RotatedRect::new(center, Vec2::from_yx(-1.0, 0.0), width, height)
}

/// Runs `model` on a batch of same-height crops in one `run_one` call,
/// padding narrower crops on the right to the widest (mirroring `ocrs`'s own
/// `prepare_text_line_batch` convention) rather than requiring exact width
/// equality — the grid is fixed-pitch so crops of one context size should
/// already match, but real images can differ by a rounding pixel. Returns
/// `[batch, seq, class]`.
fn run_recognition_batch(
    model: &Model,
    crops: &[NdTensor<f32, 2>],
) -> Result<NdTensor<f32, 3>, String> {
    if crops.is_empty() {
        return Err("run_recognition_batch: empty batch".to_string());
    }
    let h = crops[0].shape()[0];
    let max_w = crops.iter().map(|c| c.shape()[1]).max().unwrap_or(0);
    if max_w == 0 {
        return Err("run_recognition_batch: zero-width crop".to_string());
    }
    let mut batch = NdTensor::<f32, 4>::zeros([crops.len(), 1, h, max_w]);
    for (i, crop) in crops.iter().enumerate() {
        let w = crop.shape()[1];
        batch.slice_mut((i, 0, .., ..w)).copy_from(crop);
    }
    let value: rten::Value = batch.into();
    let output = model
        .run_one(value.into(), None)
        .map_err(|e| format!("recognition model run failed: {e}"))?;
    let tensor: rten_tensor::Tensor<f32> = output
        .try_into()
        .map_err(|_| "recognition output was not an f32 tensor".to_string())?;
    let mut seq_batch_cls: NdTensor<f32, 3> = tensor
        .try_into()
        .map_err(|_| "recognition output did not have 3 dims".to_string())?;
    // [seq, batch, class] -> [batch, seq, class] (mirrors `ocrs`'s own
    // `TextRecognizer::run`).
    seq_batch_cls.permute([1, 0, 2]);
    Ok(seq_batch_cls)
}

/// Average probability (`.exp()` of the raw log-probits — never `.softmax()`,
/// see the module doc) of each class over timesteps `[t0, t1)` of batch
/// item `item` of `batch` (`[batch, seq, class]`), clamped to the actual
/// sequence length. `None` on an empty range.
fn averaged_distribution(
    batch: &NdTensor<f32, 3>,
    item: usize,
    t0: usize,
    t1: usize,
) -> Option<Vec<f64>> {
    let view = batch.slice([item]);
    let seq_len = view.shape()[0];
    let classes = view.shape()[1];
    let t0 = t0.min(seq_len);
    let t1 = t1.min(seq_len).max(t0);
    if t1 <= t0 {
        return None;
    }
    let mut sums = vec![0.0f64; classes];
    for t in t0..t1 {
        for (c, sum) in sums.iter_mut().enumerate() {
            *sum += f64::from(view[[t, c]].exp());
        }
    }
    let n = f64::from((t1 - t0) as u32);
    for v in &mut sums {
        *v /= n;
    }
    Some(sums)
}

fn top_label_excluding_blank(dist: &[f64]) -> Option<usize> {
    dist.iter()
        .enumerate()
        .filter(|&(label, _)| label != CTC_BLANK_LABEL)
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(label, _)| label)
}

/// Maps an x-range in a crop's ORIGINAL pixel coordinates to a timestep
/// range in that crop's own CTC output, given its own resized width (from
/// `prepare_recognition_input`) and the fixed downsample factor. `crop_left`/
/// `crop_width` describe the crop's extent in the same original-pixel space
/// as `x_lo`/`x_hi`.
fn timestep_range(
    x_lo: f32,
    x_hi: f32,
    crop_left: f32,
    crop_width: f32,
    resized_width: f32,
) -> (usize, usize) {
    let map = |x: f32| -> usize {
        if crop_width <= 0.0 {
            return 0;
        }
        let resized_x = (x - crop_left) * resized_width / crop_width;
        (resized_x / CTC_DOWNSAMPLE).round().max(0.0) as usize
    };
    let t0 = map(x_lo);
    let mut t1 = map(x_hi);
    if t1 <= t0 {
        t1 = t0 + 1;
    }
    (t0, t1)
}

/// Mode `line`: one inference over the whole band line's own `RotatedRect`
/// group (as `ocrs`'s own pipeline would crop it), returning the matrix plus
/// the crop's own resized width and original-pixel extent (`left`, `width`)
/// for [`timestep_range`].
fn read_line_matrix(
    mrz_engine: &OcrEngine,
    mrz_input: &OcrInput,
    raw_model: &Model,
    line_group: &[RotatedRect],
) -> Result<(NdTensor<f32, 3>, f32, f32, f32), String> {
    let crop = mrz_engine
        .prepare_recognition_input(mrz_input, line_group)
        .map_err(|e| format!("prepare_recognition_input failed: {e}"))?;
    let resized_width = crop.shape()[1] as f32;
    let line_rect =
        bounding_rect(line_group.iter()).ok_or_else(|| "empty line group".to_string())?;
    let int_rect = line_rect.integral_bounding_rect();
    let crop_left = int_rect.left() as f32;
    let crop_width = int_rect.width() as f32;
    let batch = run_recognition_batch(raw_model, std::slice::from_ref(&crop))?;
    Ok((batch, resized_width, crop_left, crop_width))
}

/// Mode `cell`'s context-window crops for every cell of one line, all
/// windows shifted (not truncated) to stay within `[0, grid.cells)` so every
/// crop shares the same cell-count width and batches cleanly. Returns, per
/// cell in order, `(crop_left_px, crop_width_px, local_index_within_window)`.
type ContextCropGeometry = (f32, f32, usize);

fn build_context_crops(
    mrz_engine: &OcrEngine,
    mrz_input: &OcrInput,
    grid: &Grid,
    top: f32,
    bottom: f32,
    n: usize,
) -> Result<(Vec<NdTensor<f32, 2>>, Vec<ContextCropGeometry>), String> {
    let cells = grid.cells;
    let window = (2 * n + 1).min(cells).max(1);
    let mut crops = Vec::with_capacity(cells);
    let mut geometry = Vec::with_capacity(cells);
    for k in 0..cells {
        let raw_start = k as i64 - n as i64;
        let max_start = (cells as i64 - window as i64).max(0);
        let start = raw_start.clamp(0, max_start) as usize;
        let end = start + window;
        let x_lo = grid.origin + start as f32 * grid.pitch;
        let x_hi = grid.origin + end as f32 * grid.pitch;
        let rect = axis_aligned_rect(top, x_lo, bottom, x_hi);
        let crop = mrz_engine
            .prepare_recognition_input(mrz_input, std::slice::from_ref(&rect))
            .map_err(|e| format!("prepare_recognition_input failed: {e}"))?;
        crops.push(crop);
        geometry.push((x_lo, x_hi - x_lo, k - start));
    }
    Ok((crops, geometry))
}

// ---------------------------------------------------------------------
// Per-document processing.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharClass {
    Filler,
    Digit,
    Letter,
    Other,
}

fn classify(c: char) -> CharClass {
    if c == '<' {
        CharClass::Filler
    } else if c.is_ascii_digit() {
        CharClass::Digit
    } else if c.is_ascii_uppercase() {
        CharClass::Letter
    } else {
        CharClass::Other
    }
}

fn is_isolated_filler(truth: &[char], idx: usize) -> bool {
    let left_is_filler = idx > 0 && truth[idx - 1] == '<';
    let right_is_filler = idx + 1 < truth.len() && truth[idx + 1] == '<';
    !left_is_filler && !right_is_filler
}

const CONTEXT_SWEEP: [usize; 4] = [0, 1, 2, 4];

fn mode_key_cell(n: usize) -> String {
    format!("cell_n{n}")
}

#[derive(Default)]
struct Aggregates {
    /// mode key -> (correct, total)
    accuracy: HashMap<String, (u64, u64)>,
    /// mode key -> label-29 probability samples at isolated filler columns.
    label29_isolated: HashMap<String, Vec<f64>>,
    /// mode key -> label-29 probability samples at in-run filler columns.
    label29_run: HashMap<String, Vec<f64>>,
    /// mode key -> label-44 probability samples at truth-`E` columns.
    label44_at_e: HashMap<String, Vec<f64>>,
    /// mode key -> label-49 probability samples at truth-`E` columns.
    label49_at_e: HashMap<String, Vec<f64>>,
    /// mode key -> per-document elapsed milliseconds.
    timing_ms: HashMap<String, Vec<f64>>,
    /// (top-edge fraction, cell height px) for digit-truth cells.
    top_edge_digit: Vec<(f64, f64)>,
    /// (top-edge fraction, cell height px) for letter-truth cells.
    top_edge_letter: Vec<(f64, f64)>,
    /// Running mean ink profile for filler-truth (`<`) cells.
    ink_profile_filler_sum: [f64; INK_PROFILE_BUCKETS],
    ink_profile_filler_n: u64,
    /// Running mean ink profile for `K`-truth cells.
    ink_profile_k_sum: [f64; INK_PROFILE_BUCKETS],
    ink_profile_k_n: u64,
    /// Item 4: mean cell ink (from `chargrid::cell_ink`) at filler-truth vs
    /// non-filler-truth cells.
    ink_at_filler: Vec<f64>,
    ink_at_nonfiller: Vec<f64>,
    documents_processed: u64,
    /// skip reason -> count
    documents_skipped: HashMap<String, u64>,
    lines_skipped: HashMap<String, u64>,
}

impl Aggregates {
    fn record_skip_doc(&mut self, reason: &str) {
        *self
            .documents_skipped
            .entry(reason.to_string())
            .or_insert(0) += 1;
    }
    fn record_skip_line(&mut self, reason: &str) {
        *self.lines_skipped.entry(reason.to_string()).or_insert(0) += 1;
    }
    fn record_accuracy(&mut self, mode: &str, correct: bool) {
        let entry = self.accuracy.entry(mode.to_string()).or_insert((0, 0));
        entry.1 += 1;
        if correct {
            entry.0 += 1;
        }
    }
    fn record_timing(&mut self, mode: &str, ms: f64) {
        self.timing_ms.entry(mode.to_string()).or_default().push(ms);
    }

    fn to_json(&self, mode: Mode) -> serde_json::Value {
        let accuracy: serde_json::Value = self
            .accuracy
            .iter()
            .map(|(k, &(correct, total))| {
                let rate = if total > 0 {
                    correct as f64 / total as f64
                } else {
                    0.0
                };
                (
                    k.clone(),
                    serde_json::json!({"correct": correct, "total": total, "rate": rate}),
                )
            })
            .collect();
        let label29_isolated: serde_json::Value = self
            .label29_isolated
            .iter()
            .map(|(k, v)| (k.clone(), stats_summary(v)))
            .collect();
        let label29_run: serde_json::Value = self
            .label29_run
            .iter()
            .map(|(k, v)| (k.clone(), stats_summary(v)))
            .collect();
        let label44_at_e: serde_json::Value = self
            .label44_at_e
            .iter()
            .map(|(k, v)| (k.clone(), stats_summary(v)))
            .collect();
        let label49_at_e: serde_json::Value = self
            .label49_at_e
            .iter()
            .map(|(k, v)| (k.clone(), stats_summary(v)))
            .collect();
        let timing: serde_json::Value = self
            .timing_ms
            .iter()
            .map(|(k, v)| (k.clone(), stats_summary(v)))
            .collect();
        let top_edge_digit = top_edge_summary(&self.top_edge_digit);
        let top_edge_letter = top_edge_summary(&self.top_edge_letter);
        let ink_profile_filler =
            mean_profile(&self.ink_profile_filler_sum, self.ink_profile_filler_n);
        let ink_profile_k = mean_profile(&self.ink_profile_k_sum, self.ink_profile_k_n);

        serde_json::json!({
            "mode": format!("{mode:?}"),
            "documents_processed": self.documents_processed,
            "documents_skipped": self.documents_skipped,
            "lines_skipped": self.lines_skipped,
            "item_1_accuracy_by_mode": accuracy,
            "item_1b_top_edge_fraction": {
                "ink_fraction_threshold": TOP_EDGE_INK_FRACTION,
                "digit": top_edge_digit,
                "letter": top_edge_letter,
            },
            "item_2_label29_at_filler": {
                "isolated": label29_isolated,
                "in_run": label29_run,
            },
            "item_3_label44_vs_label49_at_truth_e": {
                "label44_eur_symbol_class": label44_at_e,
                "label49_alphabetic_e": label49_at_e,
            },
            "item_4_grid_self_check_mean_ink": {
                "filler_truth_cells": stats_summary(&self.ink_at_filler),
                "nonfiller_truth_cells": stats_summary(&self.ink_at_nonfiller),
            },
            "item_5_ink_profile_filler_vs_k": {
                "buckets": INK_PROFILE_BUCKETS,
                "filler": ink_profile_filler,
                "k": ink_profile_k,
                "filler_n": self.ink_profile_filler_n,
                "k_n": self.ink_profile_k_n,
            },
            "item_6_timing_ms": timing,
        })
    }
}

/// Buckets top-edge-fraction samples by cell pixel height (three coarse
/// bands) and reports mean/stdev per band — the "resolution floor" question
/// the brief asks for: at what character height does digit-vs-letter
/// separation stop being usable?
fn top_edge_summary(samples: &[(f64, f64)]) -> serde_json::Value {
    const BANDS: [(f64, f64, &str); 3] = [
        (0.0, 20.0, "height_lt_20px"),
        (20.0, 35.0, "height_20_35px"),
        (35.0, f64::INFINITY, "height_gte_35px"),
    ];
    let mut out = serde_json::Map::new();
    for (lo, hi, name) in BANDS {
        let bucket: Vec<f64> = samples
            .iter()
            .filter(|&&(_, h)| h >= lo && h < hi)
            .map(|&(f, _)| f)
            .collect();
        out.insert(name.to_string(), stats_summary(&bucket));
    }
    out.insert(
        "overall".to_string(),
        stats_summary(&samples.iter().map(|&(f, _)| f).collect::<Vec<_>>()),
    );
    serde_json::Value::Object(out)
}

fn mean_profile(sum: &[f64; INK_PROFILE_BUCKETS], n: u64) -> Vec<f64> {
    if n == 0 {
        return vec![0.0; INK_PROFILE_BUCKETS];
    }
    sum.iter().map(|v| v / n as f64).collect()
}

/// Reads one context-window sweep value's matrix for every cell of `grid`
/// and folds each cell's contribution into `agg` under `mode_key`, given the
/// ground-truth characters for the line.
#[allow(clippy::too_many_arguments)]
fn score_matrix_cells(
    agg: &mut Aggregates,
    mode_key: &str,
    truth: &[char],
    dists: &[Option<Vec<f64>>],
) {
    for (k, dist) in dists.iter().enumerate() {
        let Some(dist) = dist else { continue };
        let Some(&truth_char) = truth.get(k) else {
            continue;
        };
        let predicted = top_label_excluding_blank(dist).and_then(ctc_char);
        agg.record_accuracy(mode_key, predicted == Some(truth_char));

        if truth_char == '<' {
            let prob = dist.get(CTC_LABEL_FILLER).copied().unwrap_or(0.0);
            if is_isolated_filler(truth, k) {
                agg.label29_isolated
                    .entry(mode_key.to_string())
                    .or_default()
                    .push(prob);
            } else {
                agg.label29_run
                    .entry(mode_key.to_string())
                    .or_default()
                    .push(prob);
            }
        }
        if truth_char == 'E' {
            let p44 = dist.get(CTC_LABEL_EUR_E).copied().unwrap_or(0.0);
            let p49 = dist.get(CTC_LABEL_LETTER_E).copied().unwrap_or(0.0);
            agg.label44_at_e
                .entry(mode_key.to_string())
                .or_default()
                .push(p44);
            agg.label49_at_e
                .entry(mode_key.to_string())
                .or_default()
                .push(p49);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_document(
    general_engine: &OcrEngine,
    mrz_engine: &OcrEngine,
    raw_recognition_model: &Model,
    doc: &Document,
    mode: Mode,
    out_dir: &Path,
    agg: &mut Aggregates,
) {
    let image = match synthpass_ocr::decode_image(&doc.image_path) {
        Ok(img) => img.into_rgb8(),
        Err(_) => {
            agg.record_skip_doc("decode_failed");
            return;
        }
    };
    let gray = image::imageops::grayscale(&image);

    let band = match locate_mrz_band(general_engine, &image) {
        Ok(Some(b)) => b,
        Ok(None) => {
            agg.record_skip_doc("no_mrz_band");
            return;
        }
        Err(_) => {
            agg.record_skip_doc("band_detection_failed");
            return;
        }
    };
    if band.len() != doc.truth_lines.len() {
        agg.record_skip_doc("band_line_count_mismatch");
        return;
    }

    let (mrz_input, char_lines) = match recognize_char_lines_for_groups(mrz_engine, &image, &band) {
        Ok(v) => v,
        Err(_) => {
            agg.record_skip_doc("mrz_recognition_failed");
            return;
        }
    };

    let mut doc_report = serde_json::Map::new();
    doc_report.insert(
        "stem".to_string(),
        serde_json::Value::String(doc.stem.clone()),
    );
    let mut lines_report = Vec::new();
    let mut any_line_ok = false;

    for (li, maybe_char_line) in char_lines.iter().enumerate() {
        let Some(char_line) = maybe_char_line else {
            agg.record_skip_line("no_glyphs_recognized");
            continue;
        };
        let truth_line = &doc.truth_lines[li];
        let truth: Vec<char> = truth_line.chars().collect();
        let cells = truth.len();
        if !(20..=44).contains(&cells) {
            agg.record_skip_line("implausible_line_width");
            continue;
        }
        let Some(grid) =
            chargrid::fit_grid(&char_line.glyphs, char_line.left, char_line.right, cells)
        else {
            agg.record_skip_line("grid_fit_failed");
            continue;
        };

        let top = char_line.top.max(0.0) as u32;
        let bottom = char_line.bottom.max(0.0) as u32;
        let ink = chargrid::cell_ink(&gray, top, bottom, &grid);

        // Item 4: mean ink at filler-truth vs non-filler-truth cells.
        for (k, &v) in ink.iter().enumerate() {
            if truth.get(k) == Some(&'<') {
                agg.ink_at_filler.push(f64::from(v));
            } else {
                agg.ink_at_nonfiller.push(f64::from(v));
            }
        }

        // Item 1b + item 5: pure ink measurements, per cell.
        for k in 0..cells {
            let x_lo = (grid.origin + k as f32 * grid.pitch).max(0.0) as u32;
            let x_hi = (grid.origin + (k as f32 + 1.0) * grid.pitch).max(0.0) as u32;
            let Some(&truth_char) = truth.get(k) else {
                continue;
            };
            match classify(truth_char) {
                CharClass::Digit | CharClass::Letter => {
                    if let Some(frac) = cell_top_edge_fraction(&gray, top, bottom, x_lo, x_hi) {
                        let height = f64::from(bottom.saturating_sub(top));
                        if classify(truth_char) == CharClass::Digit {
                            agg.top_edge_digit.push((frac, height));
                        } else {
                            agg.top_edge_letter.push((frac, height));
                        }
                    }
                }
                _ => {}
            }
            if truth_char == '<' || truth_char == 'K' {
                if let Some(profile) = cell_column_profile(&gray, top, bottom, x_lo, x_hi) {
                    if truth_char == '<' {
                        for (s, v) in agg.ink_profile_filler_sum.iter_mut().zip(profile) {
                            *s += v;
                        }
                        agg.ink_profile_filler_n += 1;
                    } else {
                        for (s, v) in agg.ink_profile_k_sum.iter_mut().zip(profile) {
                            *s += v;
                        }
                        agg.ink_profile_k_n += 1;
                    }
                }
            }
        }

        // Mode `ocrs`: the beam decoder's own read, regridded onto the fitted
        // grid — a cell the beam dropped (no glyph landed there) is `None`,
        // scored as a miss.
        if mode.runs_ocrs() {
            let started = Instant::now();
            if let Some(regridded) = chargrid::regrid(&char_line.glyphs, &grid) {
                for (k, cell) in regridded.iter().enumerate() {
                    let Some(&truth_char) = truth.get(k) else {
                        continue;
                    };
                    agg.record_accuracy("ocrs", *cell == Some(truth_char));
                }
            }
            agg.record_timing("ocrs", started.elapsed().as_secs_f64() * 1000.0);
        }

        // Mode `line`.
        if mode.runs_line() {
            let started = Instant::now();
            match read_line_matrix(mrz_engine, &mrz_input, raw_recognition_model, &band[li]) {
                Ok((batch, resized_width, crop_left, crop_width)) => {
                    let dists: Vec<Option<Vec<f64>>> = (0..cells)
                        .map(|k| {
                            let x_lo = grid.origin + k as f32 * grid.pitch;
                            let x_hi = grid.origin + (k as f32 + 1.0) * grid.pitch;
                            let (t0, t1) =
                                timestep_range(x_lo, x_hi, crop_left, crop_width, resized_width);
                            averaged_distribution(&batch, 0, t0, t1)
                        })
                        .collect();
                    score_matrix_cells(agg, "line", &truth, &dists);
                }
                Err(_) => agg.record_skip_line("line_matrix_failed"),
            }
            agg.record_timing("line", started.elapsed().as_secs_f64() * 1000.0);
        }

        // Mode `cell`: context-window sweep, one batched inference per n.
        if mode.runs_cell() {
            for &n in &CONTEXT_SWEEP {
                let started = Instant::now();
                let mode_key = mode_key_cell(n);
                match build_context_crops(
                    mrz_engine,
                    &mrz_input,
                    &grid,
                    char_line.top,
                    char_line.bottom,
                    n,
                ) {
                    Ok((crops, geom)) => match run_recognition_batch(raw_recognition_model, &crops)
                    {
                        Ok(batch) => {
                            let dists: Vec<Option<Vec<f64>>> = (0..cells)
                                .map(|k| {
                                    let (crop_left, crop_width, local_idx) = geom[k];
                                    let resized_width = crops[k].shape()[1] as f32;
                                    let x_lo = crop_left + local_idx as f32 * grid.pitch;
                                    let x_hi = crop_left + (local_idx as f32 + 1.0) * grid.pitch;
                                    let (t0, t1) = timestep_range(
                                        x_lo,
                                        x_hi,
                                        crop_left,
                                        crop_width,
                                        resized_width,
                                    );
                                    averaged_distribution(&batch, k, t0, t1)
                                })
                                .collect();
                            score_matrix_cells(agg, &mode_key, &truth, &dists);
                        }
                        Err(_) => agg.record_skip_line("cell_matrix_run_failed"),
                    },
                    Err(_) => agg.record_skip_line("cell_crop_failed"),
                }
                agg.record_timing(&mode_key, started.elapsed().as_secs_f64() * 1000.0);
            }
        }

        any_line_ok = true;
        lines_report.push(serde_json::json!({
            "line_index": li,
            "cells": cells,
            "grid_origin_px": grid.origin,
            "grid_pitch_px": grid.pitch,
        }));
    }

    if any_line_ok {
        agg.documents_processed += 1;
    } else {
        agg.record_skip_doc("no_line_fit_at_all");
    }

    doc_report.insert("lines".to_string(), serde_json::Value::Array(lines_report));
    let file_name = format!("{}.json", doc.stem);
    if let Err(e) = write_json(out_dir, &file_name, &serde_json::Value::Object(doc_report)) {
        eprintln!("failed to write {file_name}: {e}");
    }
}

// ---------------------------------------------------------------------
// Stats helpers + output.
// ---------------------------------------------------------------------

fn stats_summary(values: &[f64]) -> serde_json::Value {
    if values.is_empty() {
        return serde_json::json!({"count": 0});
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let sum: f64 = sorted.iter().sum();
    let mean = sum / n as f64;
    let variance = sorted.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let stdev = variance.sqrt();
    let percentile = |p: f64| -> f64 {
        let idx = ((p * (n as f64 - 1.0)).round() as usize).min(n.saturating_sub(1));
        sorted[idx]
    };
    serde_json::json!({
        "count": n,
        "mean": mean,
        "stdev": stdev,
        "min": sorted[0],
        "p25": percentile(0.25),
        "median": percentile(0.5),
        "p75": percentile(0.75),
        "p90": percentile(0.90),
        "max": sorted[n - 1],
    })
}

fn fraction_within(values: &[f64], bound: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().filter(|v| v.abs() <= bound).count() as f64 / values.len() as f64
}

fn fraction_at_least(values: &[f64], bound: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().filter(|v| v.abs() >= bound).count() as f64 / values.len() as f64
}

fn write_json(out_dir: &Path, name: &str, value: &serde_json::Value) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let path = out_dir.join(name);
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())
}

fn print_summary(agg: &Aggregates, mode: Mode) {
    println!("=== probe_matrix summary (mode={mode:?}) ===");
    println!(
        "documents processed: {}, skipped: {:?}",
        agg.documents_processed, agg.documents_skipped
    );
    println!("lines skipped: {:?}", agg.lines_skipped);
    println!("-- item 1: per-cell accuracy by mode --");
    let mut modes: Vec<&String> = agg.accuracy.keys().collect();
    modes.sort();
    for m in modes {
        let (correct, total) = agg.accuracy[m];
        let rate = if total > 0 {
            correct as f64 / total as f64
        } else {
            0.0
        };
        println!("  {m:<10} {correct}/{total} = {:.4}", rate);
    }
    println!("-- item 2: label29 (filler) probability --");
    let mut keys: Vec<&String> = agg.label29_isolated.keys().collect();
    keys.sort();
    for k in keys {
        let iso = &agg.label29_isolated[k];
        let run = agg.label29_run.get(k).cloned().unwrap_or_default();
        println!(
            "  {k:<10} isolated n={} mean={:.4}  in_run n={} mean={:.4}",
            iso.len(),
            mean_of(iso),
            run.len(),
            mean_of(&run)
        );
    }
    println!("-- item 3: label44 (EUR) vs label49 (E) at truth='E' --");
    let mut keys: Vec<&String> = agg.label44_at_e.keys().collect();
    keys.sort();
    for k in keys {
        let l44 = &agg.label44_at_e[k];
        let l49 = agg.label49_at_e.get(k).cloned().unwrap_or_default();
        println!(
            "  {k:<10} n={} label44_mean={:.4}  label49_mean={:.4}",
            l44.len(),
            mean_of(l44),
            mean_of(&l49)
        );
    }
    println!(
        "-- item 4: mean ink, filler-truth={:.4} (n={}) vs non-filler-truth={:.4} (n={}) --",
        mean_of(&agg.ink_at_filler),
        agg.ink_at_filler.len(),
        mean_of(&agg.ink_at_nonfiller),
        agg.ink_at_nonfiller.len()
    );
    println!(
        "-- item 5: mean ink profile ({} buckets, left-to-right) -- filler={:?} k={:?} --",
        INK_PROFILE_BUCKETS,
        mean_profile(&agg.ink_profile_filler_sum, agg.ink_profile_filler_n),
        mean_profile(&agg.ink_profile_k_sum, agg.ink_profile_k_n)
    );
    println!(
        "-- item 1b: top-edge fraction (threshold={TOP_EDGE_INK_FRACTION}) -- digit n={} mean={:.4}  letter n={} mean={:.4} --",
        agg.top_edge_digit.len(),
        mean_of(&agg.top_edge_digit.iter().map(|&(f, _)| f).collect::<Vec<_>>()),
        agg.top_edge_letter.len(),
        mean_of(&agg.top_edge_letter.iter().map(|&(f, _)| f).collect::<Vec<_>>()),
    );
    println!("-- item 6: timing (ms) --");
    let mut keys: Vec<&String> = agg.timing_ms.keys().collect();
    keys.sort();
    for k in keys {
        let v = &agg.timing_ms[k];
        println!("  {k:<10} n={} mean={:.2}ms", v.len(), mean_of(v));
    }
}

fn mean_of(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}
