//! Glyph atlas (#433, PR 1) — what the OCR models do with the two document-code
//! cells of an MRZ line (`P<` of a TD3 line by default), measured under one
//! controlled degradation at a time.
//!
//! **Evidence label for anything derived from this tool: "Observed on
//! synthetic renders, vendored font, oracle crop".** It says nothing about the
//! shipped pipeline (the atlas bypasses imageprep, the retry loop and the
//! grid fit), nothing about real documents (synthetic results have not
//! transferred before — the chargrid A/B netted +1/-1 on 257 real documents),
//! and it adopts nothing: ranks measured here are inputs to a decision, not a
//! decision (ADR-0021 is still Proposed).
//!
//! # What one run does
//!
//! For each seed, [`synthpass_gen`] renders a clean page of the chosen
//! `--format` (TD3 by default; fictional identity, the vendored OFL OCR-B
//! font), with the chosen `--code` as its document code (the format's letter
//! and the filler by default, `P<` for TD3). For each requested axis and each
//! step of it, the clean page is degraded **on its own** (axes never
//! compose), and for the chosen cells of the first MRZ line (default cells 0
//! and 1, i.e. the two code characters) the tool records two things.
//!
//! - **Detection.** `OcrEngine::detect_text_pixels` over the cell's box in
//!   the degraded image: the mean and the max of the detection map, the
//!   fraction of pixels at or above `detection_threshold()`, and whether any
//!   `detect_words` box contains the cell centre.
//! - **Recognition.** The raw CTC matrix of the *oracle* line crop — the true
//!   line rectangle, mapped through the degradation's geometry, not a
//!   detected one. Per cell: the probability of the true glyph (peak over the
//!   cell's timesteps, and mean), the winner, the runner-up (over any label,
//!   and over `MRZ_CHARSET` only), the margin, and the blank mass. The beam
//!   decoder's read of the same crop is recorded as two booleans, as a
//!   control.
//!
//! # Axes (one at a time, from the clean render)
//!
//! | axis | unit | steps | what is applied |
//! | --- | --- | --- | --- |
//! | `resolution` | px per cell (native 22) | 22 18 15 13 11 10 9 8 7 6 5 4 | the page downscaled with `FilterType::Triangle` |
//! | `jpeg` | quality | 100 90 75 60 50 40 30 20 15 10 5 | a real `image` JPEG encode and decode |
//! | `rotation` | degrees | 0 0.25 0.5 1 1.5 2 3 4 5 | bilinear rotation about the line centre, paper-coloured fill; positive is clockwise as displayed |
//! | `blur` | Gaussian sigma, native px | 0 0.5 1 1.5 2 2.5 3 4 5 6 | `synthpass_gen::degrade` `GaussianBlur` |
//! | `noise` | sigma, grey levels | 0 2 4 8 12 16 24 32 | additive Gaussian noise, the same draw on all three channels |
//! | `contrast` | ink scale `c` | 1 0.8 0.6 0.5 0.4 0.3 0.2 0.15 0.1 | `p' = paper - c * (paper - p)` per channel |
//!
//! ISO 1831's skew tolerance is 3 degrees, so the rotation steps straddle it.
//! `blur` reuses the generator's degradation because it is deterministic and
//! takes no RNG; `noise` and `contrast` are implemented here because the
//! generator's versions are uniform noise and a contrast pivot at 128, not
//! Gaussian noise and an ink scale about the paper colour. Every step that
//! equals the clean render (resolution 22, rotation 0, blur 0, noise 0,
//! contrast 1) is evaluated once per seed and its result reused, flagged
//! `reused_identity` on the render line.
//!
//! # Conventions (recorded in the header as well)
//!
//! - **Label collapse.** Labels 44 (`ocrs`'s mangled EUR class) and 49 both
//!   decode to `E`; probability is summed over every label that decodes to
//!   the same character, and the group is keyed by its lowest label.
//! - **Winner, runner-up, margin, blank mass** are computed on the
//!   distribution averaged over the cell's timestep range (the convention
//!   `probe_matrix`'s `line` mode uses), blank excluded from winner and
//!   runner-up. `p_true_peak` is the largest per-timestep probability of the
//!   true glyph inside the range; `p_true_mean` is its average.
//! - **Ties** go to the lower label index. A negative margin means the
//!   runner-up beat the true glyph.
//! - **Timesteps** come from `ctc_matrix::timestep_range` on the cell's
//!   extent along the line axis. Under rotation that extent is the
//!   transformed cell's centre plus or minus half the cell width times
//!   `cos(theta)`; the oracle crop is the rotated line's axis-aligned
//!   bounding box, so glyph slant inside the crop is ignored. This is an
//!   approximation, and the detection box under rotation is the bounding box
//!   of the transformed cell.
//! - **The crop is padded exactly as `ocrs` pads a batch** (right, with
//!   `ocrs`'s black value, to a multiple of 50), so the matrix is the one
//!   `ocrs` itself decodes.
//!
//! # Guards
//!
//! - **Model pin.** Both `.rten` files must match `synthpass_ocr::verify`'s
//!   pinned hashes (or its `SYNTHPASS_OCR_*_SHA256` override); the run
//!   refuses otherwise. `SYNTHPASS_OCR_MODEL_SKIP_VERIFY` is deliberately not
//!   honoured: an unpinned measurement is not a measurement.
//! - **Alphabet self-check.** Before the sweep, a greedy decode of the raw
//!   matrix of seed 0's clean line, through `ctc_matrix`'s copy of the
//!   alphabet, must equal `ocrs`'s own greedy read of the same crop. An
//!   upstream alphabet or preprocessing change fails it and aborts the run.
//! - **Determinism.** Every degradation is a pure function of the clean
//!   render and `(seed, axis, step)`. The only RNG is the noise axis's
//!   SplitMix64, seeded from `(seed, axis, step)`. Inference is
//!   floating-point: compare runs on one machine with pinned models and a
//!   pinned `RTEN_NUM_THREADS`, never across machines.
//!
//! # Output
//!
//! Under `--out` (default `artifacts/glyph-atlas/<date>-<shortsha>/`; an
//! in-tree directory git does not ignore is refused, and an existing
//! `records.jsonl` is never overwritten):
//!
//! - `records.jsonl` — a header line, then per render a `render` line and one
//!   `cell` line per cell. Local only: it is per-seed.
//! - `summary.json` — aggregates only (percentiles, Wilson 95% intervals,
//!   counts). No per-seed rows. This is the only publishable shape.
//!
//! Nothing here reads a private crop or font; the font is the vendored
//! `crates/synthpass-gen/fonts/ocr-b.ttf` (OFL-1.1), the identities are
//! generator fictions, and no decoded text is written, only glyph classes and
//! booleans.
//!
//! # Usage
//!
//! ```text
//! cargo run -p synthpass-ocr --release --example glyph_atlas -- --seeds 2
//! cargo run -p synthpass-ocr --release --example glyph_atlas -- \
//!   --axes blur,noise --seeds 100 --seed-start 0 --cells 0,1
//! cargo run -p synthpass-ocr --release --example glyph_atlas -- \
//!   --format td1 --code ID --axes resolution --seeds 30 --cells 0,1
//! ```
//!
//! `--format` is `td1`, `td2`, `td3`, `mrva` or `mrvb`, and sets the layout and
//! the line-one length (the `--cells` limit). `--code` is two characters: the
//! first fits the format (`P` for td3; `A`, `C` or `I` for td1 and td2; `V` for
//! mrva and mrvb, Doc 9303 Parts 4-7), the second is `A`-`Z` or the filler
//! `<`. A non-default code needs `--release`: the generator's debug assertion
//! refuses a document code other than the format's own. Quote a code holding
//! `<` in PowerShell. A default run (TD3, `P<`) is what the tool has always
//! measured, to the byte.
//!
//! Set `RTEN_NUM_THREADS` and never run it beside an A/B arm or a gate. After
//! 20 computed renders it prints the rate and a projected total.
//!
//! **CPU (derived, unmeasured):** one computed render runs the detector
//! twice (map and words), the recognition model twice (raw matrix and beam
//! control) and the generator once per seed, roughly 0.6-1.2 s on the
//! desktop. 100 seeds over all six axes is about 5,000 computed renders, so
//! about 50-100 minutes; per axis, roughly `steps x seeds x 1 s`.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use image::{DynamicImage, Rgb, RgbImage};
use ocrs::{ImageSource, OcrEngine, OcrInput};
use rten::Model;
use rten_imageproc::{bounding_rect, PointF, RotatedRect};
use rten_tensor::prelude::*;
use rten_tensor::NdTensor;
use serde_json::{json, Value};

use synthpass_gen::degrade::{self, Degradation};
use synthpass_gen::layout::{self, Rect};
use synthpass_gen::{DocumentType, GeneratorConfig};
use synthpass_ocr::{verify, MRZ_CHARSET};

// The label table, batch runner, pixel-to-timestep arithmetic and engine
// loaders, shared with `probe_matrix.rs`. This example uses a subset, hence
// the `allow`.
#[path = "common/ctc_matrix.rs"]
#[allow(dead_code)]
mod ctc_matrix;
use ctc_matrix::{
    axis_aligned_rect, ctc_char, load_general_engine, load_mrz_engine, load_raw_recognition_model,
    model_dir, run_recognition_batch, timestep_range, CTC_ALPHABET, CTC_BLANK_LABEL,
    MRZ_BEAM_WIDTH,
};

// ---------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------

/// 2 adds `code` to the header (and so to the summary's `run` block) and lets
/// `format` take the five `--format` values; schema 1 was TD3 `P<` only, and
/// its `format` was always `"TD3"`. Every other field keeps its meaning.
const SCHEMA_VERSION: u32 = 2;
const TOOL_NAME: &str = "glyph_atlas";
const EVIDENCE_LABEL: &str = "Observed on synthetic renders, vendored font, oracle crop";

/// The `ocrs` and `rten` versions this tool was written against. A test
/// checks them against this crate's `Cargo.toml`, so a bump has to touch
/// this file too.
const OCRS_VERSION: &str = "0.13.1";
const RTEN_VERSION: &str = "0.26.0";

/// `ocrs::preprocess::BLACK_VALUE`, which `ocrs` does not export: the value
/// it pads a batch with. The run-start self-check would notice if it changed
/// enough to matter.
const OCRS_BLACK_VALUE: f32 = -0.5;

/// `ocrs` pads each recognition batch to the next multiple of this width
/// (`group_width` in its `recognize_text_lines`).
const OCRS_GROUP_WIDTH_MULTIPLE: usize = 50;

/// The vendored OFL OCR-B font, embedded a second time here only to hash it
/// for the header; `synthpass-gen` renders with the same file.
static FONT_BYTES: &[u8] = include_bytes!("../../synthpass-gen/fonts/ocr-b.ttf");
const FONT_SHA256: &str = "367d876cca948ecd4900851f6e85687cbb6e71de9d0d2f36348edec5655526af";
const FONT_NAME: &str = "OCR-B (Raisty), vendored";
const FONT_PATH: &str = "crates/synthpass-gen/fonts/ocr-b.ttf";
const FONT_LICENSE: &str = "OFL-1.1";

/// The MRZ line this atlas reads: the first, whose first two cells are the
/// document code.
const LINE_INDEX: usize = 0;

// ---------------------------------------------------------------------
// Format and document code
// ---------------------------------------------------------------------

/// The five MRZ formats the generator renders, as `--format` spells them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Td1,
    Td2,
    Td3,
    MrvA,
    MrvB,
}

/// The two characters that open MRZ line one: a document code.
type Code = [char; 2];

impl Format {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "td1" => Some(Self::Td1),
            "td2" => Some(Self::Td2),
            "td3" => Some(Self::Td3),
            "mrva" => Some(Self::MrvA),
            "mrvb" => Some(Self::MrvB),
            _ => None,
        }
    }

    /// How the header records the format.
    fn name(self) -> &'static str {
        match self {
            Self::Td1 => "TD1",
            Self::Td2 => "TD2",
            Self::Td3 => "TD3",
            Self::MrvA => "MRV-A",
            Self::MrvB => "MRV-B",
        }
    }

    /// How a path spells the format: the flag's own spelling.
    fn flag(self) -> &'static str {
        match self {
            Self::Td1 => "td1",
            Self::Td2 => "td2",
            Self::Td3 => "td3",
            Self::MrvA => "mrva",
            Self::MrvB => "mrvb",
        }
    }

    fn document_type(self) -> DocumentType {
        match self {
            Self::Td1 => DocumentType::TD1,
            Self::Td2 => DocumentType::TD2,
            Self::Td3 => DocumentType::TD3,
            Self::MrvA => DocumentType::MrvA,
            Self::MrvB => DocumentType::MrvB,
        }
    }

    /// The length of line one in cells, the `--cells` limit.
    fn cells(self) -> usize {
        (match self {
            Self::Td1 => layout::TD1_MRZ_CHARS,
            Self::Td2 => layout::TD2_MRZ_CHARS,
            Self::Td3 => layout::TD3_MRZ_CHARS,
            Self::MrvA => layout::MRVA_MRZ_CHARS,
            Self::MrvB => layout::MRVB_MRZ_CHARS,
        }) as usize
    }

    /// The first characters a document code of this format may start with:
    /// Doc 9303 Part 4 (passports: `P`), Parts 5 and 6 (`A`, `C` or `I`, per
    /// Note k) and Part 7 (visas: `V`).
    fn code_letters(self) -> &'static [char] {
        match self {
            Self::Td1 | Self::Td2 => &['A', 'C', 'I'],
            Self::Td3 => &['P'],
            Self::MrvA | Self::MrvB => &['V'],
        }
    }

    /// The code the generator writes for this format: its letter, then the
    /// filler.
    fn default_code(self) -> Code {
        [
            self.document_type()
                .document_code()
                .chars()
                .next()
                .unwrap_or('<'),
            '<',
        ]
    }
}

/// `--code XY`: exactly two characters, the first one that fits `format`, the
/// second `A`-`Z` or the filler `<`.
fn parse_code(text: &str, format: Format) -> Result<Code, String> {
    let chars: Vec<char> = text.chars().collect();
    let [first, second] = chars[..] else {
        return Err(format!(
            "--code: {text:?} is not two characters (a letter, then a letter or <)"
        ));
    };
    if !format.code_letters().contains(&first) {
        let letters: Vec<String> = format.code_letters().iter().map(char::to_string).collect();
        return Err(format!(
            "--code: {text:?} does not start with {} as a {} code must",
            letters.join(", "),
            format.name()
        ));
    }
    if !(second.is_ascii_uppercase() || second == '<') {
        return Err(format!(
            "--code: {text:?}: the second character must be A-Z or <"
        ));
    }
    Ok([first, second])
}

fn code_text(code: Code) -> String {
    code.iter().collect()
}

/// A code as a path spells it: the filler `<` is written `0`, as `samples/`
/// does, because a Windows path cannot hold `<`.
fn code_path_part(code: Code) -> String {
    code.iter()
        .map(|&c| if c == '<' { '0' } else { c })
        .collect()
}

/// The default `--out` directory name. A run that is not the default format
/// and code adds both, so two runs of one commit on one day do not collide;
/// the default run keeps the name it always had.
fn default_out_name(date: &str, short: &str, format: Format, code: Code) -> String {
    if format == Format::Td3 && code == Format::Td3.default_code() {
        format!("{date}-{short}")
    } else {
        format!("{date}-{short}-{}-{}", format.flag(), code_path_part(code))
    }
}

/// What `Passport::document_type` holds for `code`: the emitter pads the field
/// to two characters with the filler, so a trailing `<` is left off.
fn passport_code_field(code: Code) -> String {
    if code[1] == '<' {
        code[0].to_string()
    } else {
        code_text(code)
    }
}

/// One seed's clean page and ground-truth labels for `format`, with `code` as
/// the document code. With the default code this is what
/// `generate_from_seed` returns.
fn render_clean(seed: u64, format: Format, code: Code) -> (DynamicImage, synthpass_gen::Labels) {
    let config = GeneratorConfig::with_document_type(seed, format.document_type());
    let mut passport = synthpass_gen::data::generate_passport(&config);
    passport.document_type = passport_code_field(code);
    synthpass_gen::generate(&passport, &config)
}

/// Line one must start with the code, or the generator did not write what the
/// run says it measures.
fn check_line_prefix(line: &[char], code: Code) -> Result<(), String> {
    if line.iter().take(2).copied().eq(code.iter().copied()) {
        Ok(())
    } else {
        Err(format!(
            "the generator's line one does not start with {}",
            code_text(code)
        ))
    }
}

/// The generator's `debug_assert` refuses a document code that is not the
/// format's own, so a non-default code needs a release build.
fn check_build_for_code(format: Format, code: Code, debug_build: bool) -> Result<(), String> {
    if debug_build && code != format.default_code() {
        return Err(format!(
            "--code {}: the generator's debug assertion refuses a document code other than the \
             format's own; run with cargo's --release",
            code_text(code)
        ));
    }
    Ok(())
}

const NATIVE_PX_PER_CELL: f64 = layout::MRZ_CELL_WIDTH as f64;
const DEFAULT_SEEDS: u64 = 100;
const DEFAULT_CELLS: [usize; 2] = [0, 1];
const PROGRESS_AFTER: usize = 20;

const RESIZE_FILTER: image::imageops::FilterType = image::imageops::FilterType::Triangle;
const RESIZE_FILTER_NAME: &str = "image::imageops::FilterType::Triangle";

const RNG_DESCRIPTION: &str = "splitmix64 (state += 0x9E3779B97F4A7C15, then the two xor-shift \
     multiply rounds with 0xBF58476D1CE4E5B9 and 0x94D049BB133111EB); uniforms in (0,1) from \
     the top 53 bits; Box-Muller pairs; one draw per pixel in row-major order, added to all \
     three channels; noise axis only";
const SEED_DERIVATION: &str = "noise stream seed = mix(mix(mix(seed) ^ axis_index) ^ step_index), \
     where mix(x) is the first splitmix64 output from state x, axis_index is 0 resolution, \
     1 jpeg, 2 rotation, 3 blur, 4 noise, 5 contrast, and step_index is the 0-based position \
     in that axis's step list";

// ---------------------------------------------------------------------
// Axes
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Axis {
    Resolution,
    Jpeg,
    Rotation,
    Blur,
    Noise,
    Contrast,
}

static RESOLUTION_STEPS: [f64; 12] = [
    22.0, 18.0, 15.0, 13.0, 11.0, 10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0,
];
static JPEG_STEPS: [f64; 11] = [
    100.0, 90.0, 75.0, 60.0, 50.0, 40.0, 30.0, 20.0, 15.0, 10.0, 5.0,
];
static ROTATION_STEPS: [f64; 9] = [0.0, 0.25, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0];
static BLUR_STEPS: [f64; 10] = [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0];
static NOISE_STEPS: [f64; 8] = [0.0, 2.0, 4.0, 8.0, 12.0, 16.0, 24.0, 32.0];
static CONTRAST_STEPS: [f64; 9] = [1.0, 0.8, 0.6, 0.5, 0.4, 0.3, 0.2, 0.15, 0.1];

impl Axis {
    const ALL: [Axis; 6] = [
        Axis::Resolution,
        Axis::Jpeg,
        Axis::Rotation,
        Axis::Blur,
        Axis::Noise,
        Axis::Contrast,
    ];

    /// Position in [`Axis::ALL`], used in the noise seed derivation.
    fn index(self) -> usize {
        match self {
            Axis::Resolution => 0,
            Axis::Jpeg => 1,
            Axis::Rotation => 2,
            Axis::Blur => 3,
            Axis::Noise => 4,
            Axis::Contrast => 5,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Axis::Resolution => "resolution",
            Axis::Jpeg => "jpeg",
            Axis::Rotation => "rotation",
            Axis::Blur => "blur",
            Axis::Noise => "noise",
            Axis::Contrast => "contrast",
        }
    }

    fn unit(self) -> &'static str {
        match self {
            Axis::Resolution => "px_per_cell",
            Axis::Jpeg => "jpeg_quality",
            Axis::Rotation => "degrees_clockwise",
            Axis::Blur => "gaussian_sigma_native_px",
            Axis::Noise => "gaussian_sigma_grey_levels",
            Axis::Contrast => "ink_scale",
        }
    }

    fn method(self) -> &'static str {
        match self {
            Axis::Resolution => {
                "page downscaled to the target px per cell (native 22) with a fixed filter; \
                 the step equal to native is the clean render"
            }
            Axis::Jpeg => {
                "image crate JpegEncoder::new_with_quality encode, then decode with the image \
                 crate; there is no identity step"
            }
            Axis::Rotation => {
                "bilinear rotation about the centre of the MRZ line, pixel-centre convention, \
                 samples outside the page take the paper colour; every cell box is transformed \
                 exactly"
            }
            Axis::Blur => {
                "synthpass_gen::degrade GaussianBlur (image::imageops::blur), sigma in native \
                 pixels, no RNG"
            }
            Axis::Noise => {
                "additive Gaussian noise, sigma in grey levels, rounded and clamped to 0..=255"
            }
            Axis::Contrast => {
                "p' = paper - c * (paper - p) per channel; paper is the render background \
                 sampled left of the MRZ line"
            }
        }
    }

    fn steps(self) -> &'static [f64] {
        match self {
            Axis::Resolution => &RESOLUTION_STEPS,
            Axis::Jpeg => &JPEG_STEPS,
            Axis::Rotation => &ROTATION_STEPS,
            Axis::Blur => &BLUR_STEPS,
            Axis::Noise => &NOISE_STEPS,
            Axis::Contrast => &CONTRAST_STEPS,
        }
    }

    /// The step that must reproduce the clean render byte for byte, if the
    /// axis has one.
    fn identity_step(self) -> Option<f64> {
        match self {
            Axis::Resolution => Some(NATIVE_PX_PER_CELL),
            Axis::Jpeg => None,
            Axis::Rotation => Some(0.0),
            Axis::Blur => Some(0.0),
            Axis::Noise => Some(0.0),
            Axis::Contrast => Some(1.0),
        }
    }

    fn parse(name: &str) -> Option<Axis> {
        Axis::ALL.iter().copied().find(|axis| axis.name() == name)
    }
}

// ---------------------------------------------------------------------
// Page geometry: where a clean-page point lands in the degraded image.
// ---------------------------------------------------------------------

/// How a degradation moves points of the clean page. Only `resolution` and
/// `rotation` move anything.
#[derive(Debug, Clone, Copy, PartialEq)]
enum PageMap {
    Identity,
    Scale {
        sx: f64,
        sy: f64,
    },
    /// Clockwise as displayed (y grows downward), about `(cx, cy)`.
    Rotate {
        degrees: f64,
        cx: f64,
        cy: f64,
    },
}

impl PageMap {
    fn point(&self, (x, y): (f64, f64)) -> (f64, f64) {
        match *self {
            PageMap::Identity => (x, y),
            PageMap::Scale { sx, sy } => (x * sx, y * sy),
            PageMap::Rotate { degrees, cx, cy } => {
                let (sin, cos) = degrees.to_radians().sin_cos();
                let (dx, dy) = (x - cx, y - cy);
                (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
            }
        }
    }

    fn quad(&self, quad: &Quad) -> Quad {
        Quad(quad.0.map(|p| self.point(p)))
    }

    /// The x-extent a clean cell occupies along the line axis, in degraded
    /// image coordinates: the timestep mapping's input. For a rotation it is
    /// the transformed centre plus or minus half the cell width times
    /// `cos(theta)`; the shear a rotation puts into glyph columns is ignored
    /// (see the module doc).
    fn axis_extent(&self, cell: Rect) -> (f64, f64) {
        let (x0, x1) = (f64::from(cell.x), f64::from(cell.x + cell.width));
        match *self {
            PageMap::Identity => (x0, x1),
            PageMap::Scale { sx, .. } => (x0 * sx, x1 * sx),
            PageMap::Rotate { degrees, .. } => {
                let centre = self.point((
                    f64::from(cell.x) + f64::from(cell.width) / 2.0,
                    f64::from(cell.y) + f64::from(cell.height) / 2.0,
                ));
                let half = f64::from(cell.width) / 2.0 * degrees.to_radians().cos();
                (centre.0 - half, centre.0 + half)
            }
        }
    }

    fn px_per_cell(&self) -> f64 {
        match *self {
            PageMap::Scale { sx, .. } => NATIVE_PX_PER_CELL * sx,
            _ => NATIVE_PX_PER_CELL,
        }
    }
}

/// A convex quadrilateral, corners in order (top-left, top-right,
/// bottom-right, bottom-left of the untransformed rectangle).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Quad([(f64, f64); 4]);

impl Quad {
    fn of_rect(rect: Rect) -> Quad {
        let (x0, y0) = (f64::from(rect.x), f64::from(rect.y));
        let (x1, y1) = (
            f64::from(rect.x + rect.width),
            f64::from(rect.y + rect.height),
        );
        Quad([(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
    }

    /// `(left, top, right, bottom)`.
    fn bounds(&self) -> (f64, f64, f64, f64) {
        let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for &(x, y) in &self.0 {
            b.0 = b.0.min(x);
            b.1 = b.1.min(y);
            b.2 = b.2.max(x);
            b.3 = b.3.max(y);
        }
        b
    }

    fn centre(&self) -> (f64, f64) {
        let (sx, sy) = self
            .0
            .iter()
            .fold((0.0, 0.0), |(ax, ay), &(x, y)| (ax + x, ay + y));
        (sx / 4.0, sy / 4.0)
    }

    /// Whether `p` lies inside or on the quad (all edge cross products share
    /// a sign). Used by the geometry tests only.
    #[cfg(test)]
    fn contains(&self, p: (f64, f64)) -> bool {
        let (mut positive, mut negative) = (false, false);
        for i in 0..4 {
            let a = self.0[i];
            let b = self.0[(i + 1) % 4];
            let cross = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
            if cross > 1e-9 {
                positive = true;
            } else if cross < -1e-9 {
                negative = true;
            }
        }
        !(positive && negative)
    }
}

/// An integer pixel window `[x0, x1) x [y0, y1)` inside the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PxRect {
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
}

/// The smallest pixel window covering `(left, top, right, bottom)`, at least
/// one pixel each way and inside a `w x h` image. `None` for an empty image.
fn pixel_bounds(b: (f64, f64, f64, f64), w: usize, h: usize) -> Option<PxRect> {
    if w == 0 || h == 0 {
        return None;
    }
    let x0 = (b.0.floor().max(0.0) as usize).min(w - 1);
    let y0 = (b.1.floor().max(0.0) as usize).min(h - 1);
    let x1 = (b.2.ceil().max(0.0) as usize).clamp(x0 + 1, w);
    let y1 = (b.3.ceil().max(0.0) as usize).clamp(y0 + 1, h);
    Some(PxRect { x0, y0, x1, y1 })
}

/// Everything the recognition and detection reads need about one cell in one
/// degraded image.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CellGeometry {
    /// The transformed cell's centre.
    centre: (f64, f64),
    /// Bounding box of the transformed cell: `(left, top, right, bottom)`.
    bounds: (f64, f64, f64, f64),
    /// Extent along the line axis, for the timestep mapping.
    x_lo: f64,
    x_hi: f64,
}

fn cell_geometry(map: &PageMap, line: Rect, mrz_chars: u32, cell: usize) -> CellGeometry {
    let clean_cell = layout::mrz_char_rect_for_line(line, mrz_chars, cell as u32);
    let quad = map.quad(&Quad::of_rect(clean_cell));
    let (x_lo, x_hi) = map.axis_extent(clean_cell);
    CellGeometry {
        centre: quad.centre(),
        bounds: quad.bounds(),
        x_lo,
        x_hi,
    }
}

/// `(left, top, right, bottom)` of the oracle line crop: the transformed
/// line's axis-aligned bounding box, clamped to the image.
fn oracle_line_bounds(map: &PageMap, line: Rect, w: u32, h: u32) -> (f64, f64, f64, f64) {
    let b = map.quad(&Quad::of_rect(line)).bounds();
    (
        b.0.max(0.0),
        b.1.max(0.0),
        b.2.min(f64::from(w)),
        b.3.min(f64::from(h)),
    )
}

// ---------------------------------------------------------------------
// Degradations
// ---------------------------------------------------------------------

struct Degraded {
    image: RgbImage,
    map: PageMap,
}

struct DegradeCtx<'a> {
    clean: &'a RgbImage,
    /// The render's background colour, sampled left of the MRZ line.
    paper: [u8; 3],
    line: Rect,
}

/// The paper colour: the pixel left of the MRZ line, inside the page frame.
fn paper_colour(clean: &RgbImage, line: Rect) -> [u8; 3] {
    let x = (line.x / 2).min(clean.width().saturating_sub(1));
    let y = (line.y + line.height / 2).min(clean.height().saturating_sub(1));
    clean.get_pixel(x, y).0
}

fn degrade_step(
    axis: Axis,
    step_index: usize,
    param: f64,
    seed: u64,
    ctx: &DegradeCtx<'_>,
) -> Result<Degraded, String> {
    match axis {
        Axis::Resolution => resolution_step(ctx.clean, param),
        Axis::Jpeg => {
            let quality = param.round();
            if !(1.0..=100.0).contains(&quality) {
                return Err(format!("jpeg quality {param} is outside 1..=100"));
            }
            Ok(Degraded {
                image: jpeg_roundtrip(ctx.clean, quality as u8)?,
                map: PageMap::Identity,
            })
        }
        Axis::Rotation => Ok(rotation_step(ctx, param)),
        Axis::Blur => Ok(Degraded {
            image: blur_step(ctx.clean, param),
            map: PageMap::Identity,
        }),
        Axis::Noise => Ok(Degraded {
            image: add_gaussian_noise(
                ctx.clean,
                param,
                derive_seed(seed, axis.index() as u64, step_index as u64),
            ),
            map: PageMap::Identity,
        }),
        Axis::Contrast => Ok(Degraded {
            image: scale_ink(ctx.clean, param, ctx.paper),
            map: PageMap::Identity,
        }),
    }
}

fn resolution_step(clean: &RgbImage, px_per_cell: f64) -> Result<Degraded, String> {
    if !(px_per_cell > 0.0 && px_per_cell <= NATIVE_PX_PER_CELL) {
        return Err(format!(
            "resolution {px_per_cell} px per cell is outside (0, {NATIVE_PX_PER_CELL}]"
        ));
    }
    if px_per_cell == NATIVE_PX_PER_CELL {
        return Ok(Degraded {
            image: clean.clone(),
            map: PageMap::Identity,
        });
    }
    let scale = px_per_cell / NATIVE_PX_PER_CELL;
    let (w, h) = clean.dimensions();
    let new_w = ((f64::from(w) * scale).round() as u32).max(1);
    let new_h = ((f64::from(h) * scale).round() as u32).max(1);
    let image = image::imageops::resize(clean, new_w, new_h, RESIZE_FILTER);
    Ok(Degraded {
        image,
        map: PageMap::Scale {
            sx: f64::from(new_w) / f64::from(w),
            sy: f64::from(new_h) / f64::from(h),
        },
    })
}

fn jpeg_roundtrip(clean: &RgbImage, quality: u8) -> Result<RgbImage, String> {
    let mut bytes: Vec<u8> = Vec::new();
    {
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, quality);
        encoder
            .encode(
                clean.as_raw(),
                clean.width(),
                clean.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| format!("jpeg encode failed: {e}"))?;
    }
    let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg)
        .map_err(|e| format!("jpeg decode failed: {e}"))?;
    Ok(decoded.into_rgb8())
}

fn rotation_step(ctx: &DegradeCtx<'_>, degrees: f64) -> Degraded {
    if degrees == 0.0 {
        return Degraded {
            image: ctx.clean.clone(),
            map: PageMap::Identity,
        };
    }
    let cx = f64::from(ctx.line.x) + f64::from(ctx.line.width) / 2.0;
    let cy = f64::from(ctx.line.y) + f64::from(ctx.line.height) / 2.0;
    Degraded {
        image: rotate_about(ctx.clean, degrees, cx, cy, ctx.paper),
        map: PageMap::Rotate { degrees, cx, cy },
    }
}

/// Rotates `clean` clockwise (as displayed) by `degrees` about `(cx, cy)` in
/// continuous page coordinates, where pixel `i` covers `[i, i + 1)`. Each
/// output pixel centre is mapped back into the source and sampled bilinearly;
/// a source pixel outside the page is the paper colour.
fn rotate_about(clean: &RgbImage, degrees: f64, cx: f64, cy: f64, paper: [u8; 3]) -> RgbImage {
    let (w, h) = clean.dimensions();
    let (sin, cos) = degrees.to_radians().sin_cos();
    let fetch = |x: i64, y: i64, channel: usize| -> f64 {
        if x < 0 || y < 0 || x >= i64::from(w) || y >= i64::from(h) {
            f64::from(paper[channel])
        } else {
            f64::from(clean.get_pixel(x as u32, y as u32).0[channel])
        }
    };
    let mut out = RgbImage::new(w, h);
    for oy in 0..h {
        for ox in 0..w {
            // Inverse map: rotate the output pixel centre by -degrees.
            let dx = f64::from(ox) + 0.5 - cx;
            let dy = f64::from(oy) + 0.5 - cy;
            let sx = cx + dx * cos + dy * sin;
            let sy = cy - dx * sin + dy * cos;
            // Back to pixel-index space, where pixel centres are integers.
            let u = sx - 0.5;
            let v = sy - 0.5;
            let x0 = u.floor();
            let y0 = v.floor();
            let (fx, fy) = (u - x0, v - y0);
            let (x0, y0) = (x0 as i64, y0 as i64);
            let mut pixel = [0u8; 3];
            for (channel, value) in pixel.iter_mut().enumerate() {
                let top = fetch(x0, y0, channel) * (1.0 - fx) + fetch(x0 + 1, y0, channel) * fx;
                let bottom =
                    fetch(x0, y0 + 1, channel) * (1.0 - fx) + fetch(x0 + 1, y0 + 1, channel) * fx;
                *value = (top * (1.0 - fy) + bottom * fy).round().clamp(0.0, 255.0) as u8;
            }
            out.put_pixel(ox, oy, Rgb(pixel));
        }
    }
    out
}

/// Gaussian blur through the generator's own degradation (which wraps
/// `image::imageops::blur`). `GaussianBlur` consumes no RNG, so the seed
/// argument is irrelevant and fixed at 0; sigma 0 returns the input.
fn blur_step(clean: &RgbImage, sigma: f64) -> RgbImage {
    let image = DynamicImage::ImageRgb8(clean.clone());
    degrade::apply(
        &image,
        &[Degradation::GaussianBlur {
            sigma: sigma as f32,
        }],
        0,
    )
    .into_rgb8()
}

/// SplitMix64: a 64-bit generator small enough to state in the header and
/// stable across platforms and dependency bumps, which is the point.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A uniform in the open interval (0, 1), from the top 53 bits.
    fn next_unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / 9_007_199_254_740_992.0
    }

    /// Two independent standard normals (Box-Muller).
    fn next_gaussian_pair(&mut self) -> (f64, f64) {
        let u1 = self.next_unit();
        let u2 = self.next_unit();
        let radius = (-2.0 * u1.ln()).sqrt();
        let angle = 2.0 * std::f64::consts::PI * u2;
        (radius * angle.cos(), radius * angle.sin())
    }
}

fn mix(x: u64) -> u64 {
    SplitMix64(x).next_u64()
}

/// The noise stream's seed for `(seed, axis, step)`; see [`SEED_DERIVATION`].
fn derive_seed(seed: u64, axis_index: u64, step_index: u64) -> u64 {
    mix(mix(mix(seed) ^ axis_index) ^ step_index)
}

/// Adds one Gaussian draw of standard deviation `sigma` (grey levels) to all
/// three channels of every pixel, row-major, rounding and clamping to
/// `0..=255`. `sigma <= 0` returns the input and draws nothing.
fn add_gaussian_noise(clean: &RgbImage, sigma: f64, stream_seed: u64) -> RgbImage {
    let mut out = clean.clone();
    if sigma <= 0.0 {
        return out;
    }
    let mut rng = SplitMix64(stream_seed);
    let mut spare: Option<f64> = None;
    for pixel in out.pixels_mut() {
        let z = match spare.take() {
            Some(z) => z,
            None => {
                let (a, b) = rng.next_gaussian_pair();
                spare = Some(b);
                a
            }
        };
        let delta = z * sigma;
        for channel in pixel.0.iter_mut() {
            *channel = (f64::from(*channel) + delta).round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

/// Ink density: `p' = paper - c * (paper - p)` per channel. `c = 1` is the
/// identity; smaller `c` fades ink toward the paper colour.
fn scale_ink(clean: &RgbImage, c: f64, paper: [u8; 3]) -> RgbImage {
    let mut out = clean.clone();
    for pixel in out.pixels_mut() {
        for (channel, value) in pixel.0.iter_mut().enumerate() {
            let paper_value = f64::from(paper[channel]);
            let scaled = paper_value - c * (paper_value - f64::from(*value));
            *value = scaled.round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

// ---------------------------------------------------------------------
// Per-cell statistics (pure; unit-tested on hand-built matrices)
// ---------------------------------------------------------------------

/// The recognition classes grouped by the character they decode to, in
/// ascending order of each group's lowest label. Labels 44 and 49 both
/// decode to `E`, so they form one group keyed by 44.
struct Alphabet {
    groups: Vec<(char, Vec<usize>)>,
}

impl Alphabet {
    fn new() -> Alphabet {
        let mut groups: Vec<(char, Vec<usize>)> = Vec::new();
        for label in 1..=CTC_ALPHABET.chars().count() {
            let Some(ch) = ctc_char(label) else { continue };
            match groups.iter().position(|(c, _)| *c == ch) {
                Some(index) => groups[index].1.push(label),
                None => groups.push((ch, vec![label])),
            }
        }
        Alphabet { groups }
    }

    fn group_of(&self, ch: char) -> Option<usize> {
        self.groups.iter().position(|(c, _)| *c == ch)
    }

    /// Sums `probs` over each group's labels.
    fn fold(&self, probs: &[f64]) -> Vec<f64> {
        self.groups
            .iter()
            .map(|(_, labels)| {
                labels
                    .iter()
                    .map(|&label| probs.get(label).copied().unwrap_or(0.0))
                    .sum::<f64>()
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct CellStats {
    /// The clamped timestep range `[t0, t1)` the statistics cover.
    timesteps: (usize, usize),
    p_true_peak: f64,
    p_true_mean: f64,
    winner: Option<char>,
    winner_p: f64,
    runner_up: Option<char>,
    runner_up_p: f64,
    runner_up_mrz: Option<char>,
    runner_up_mrz_p: f64,
    /// `p_true_mean - runner_up_p`: negative when the runner-up beat the
    /// true glyph.
    margin: f64,
    blank_mean: f64,
}

/// The highest-probability group satisfying `keep`; a tie goes to the group
/// with the lower label (groups are in ascending label order and only a
/// strictly greater value replaces the current best).
fn best_group(
    alphabet: &Alphabet,
    mean: &[f64],
    keep: impl Fn(usize, char) -> bool,
) -> Option<(char, f64)> {
    let mut best: Option<(char, f64)> = None;
    for (g, (ch, _)) in alphabet.groups.iter().enumerate() {
        if !keep(g, *ch) {
            continue;
        }
        let p = mean[g];
        match best {
            Some((_, best_p)) if p <= best_p => {}
            _ => best = Some((*ch, p)),
        }
    }
    best
}

/// Statistics for one cell from its per-timestep probability rows (each row
/// indexed by label, blank first). `None` when there are no timesteps.
fn cell_stats(
    alphabet: &Alphabet,
    steps: &[Vec<f64>],
    truth: char,
    timesteps: (usize, usize),
) -> Option<CellStats> {
    if steps.is_empty() {
        return None;
    }
    let n = steps.len() as f64;
    let folded: Vec<Vec<f64>> = steps.iter().map(|row| alphabet.fold(row)).collect();
    let group_count = alphabet.groups.len();
    let mean: Vec<f64> = (0..group_count)
        .map(|g| folded.iter().map(|row| row[g]).sum::<f64>() / n)
        .collect();
    let truth_group = alphabet.group_of(truth);
    let p_true_peak =
        truth_group.map_or(0.0, |g| folded.iter().map(|row| row[g]).fold(0.0, f64::max));
    let p_true_mean = truth_group.map_or(0.0, |g| mean[g]);

    let winner = best_group(alphabet, &mean, |_, _| true);
    let runner_up = best_group(alphabet, &mean, |g, _| Some(g) != truth_group);
    let runner_up_mrz = best_group(alphabet, &mean, |g, ch| {
        Some(g) != truth_group && MRZ_CHARSET.contains(ch)
    });
    let blank_mean = steps
        .iter()
        .map(|row| row.get(CTC_BLANK_LABEL).copied().unwrap_or(0.0))
        .sum::<f64>()
        / n;

    let runner_up_p = runner_up.map_or(0.0, |(_, p)| p);
    Some(CellStats {
        timesteps,
        p_true_peak,
        p_true_mean,
        winner: winner.map(|(ch, _)| ch),
        winner_p: winner.map_or(0.0, |(_, p)| p),
        runner_up: runner_up.map(|(ch, _)| ch),
        runner_up_p,
        runner_up_mrz: runner_up_mrz.map(|(ch, _)| ch),
        runner_up_mrz_p: runner_up_mrz.map_or(0.0, |(_, p)| p),
        margin: p_true_mean - runner_up_p,
        blank_mean,
    })
}

/// The clamped timestep range and the probability rows (`.exp()` of the
/// log-probabilities — never `softmax`) of batch item `item` over it.
fn step_probs(
    batch: &NdTensor<f32, 3>,
    item: usize,
    t0: usize,
    t1: usize,
) -> ((usize, usize), Vec<Vec<f64>>) {
    let seq = batch.shape()[1];
    let classes = batch.shape()[2];
    let t0 = t0.min(seq);
    let t1 = t1.min(seq).max(t0);
    let rows = (t0..t1)
        .map(|t| {
            (0..classes)
                .map(|c| f64::from(batch[[item, t, c]].exp()))
                .collect()
        })
        .collect();
    ((t0, t1), rows)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DetStats {
    p_mean: f64,
    p_max: f64,
    frac_above: f64,
}

/// Mean, max and fraction at or above `threshold` of the detection map over
/// the window. `None` for an empty window.
fn detection_stats(map: &NdTensor<f32, 2>, window: PxRect, threshold: f32) -> Option<DetStats> {
    let h = map.shape()[0];
    let w = map.shape()[1];
    let x1 = window.x1.min(w);
    let y1 = window.y1.min(h);
    if window.x0 >= x1 || window.y0 >= y1 {
        return None;
    }
    let (mut sum, mut max, mut above, mut count) = (0.0f64, f32::MIN, 0usize, 0usize);
    for y in window.y0..y1 {
        for x in window.x0..x1 {
            let v = map[[y, x]];
            sum += f64::from(v);
            if v > max {
                max = v;
            }
            if v >= threshold {
                above += 1;
            }
            count += 1;
        }
    }
    Some(DetStats {
        p_mean: sum / count as f64,
        p_max: f64::from(max),
        frac_above: above as f64 / count as f64,
    })
}

// ---------------------------------------------------------------------
// Reading the oracle line crop
// ---------------------------------------------------------------------

struct OracleMatrix {
    /// `[1, seq, class]` log-probabilities.
    batch: NdTensor<f32, 3>,
    /// Width of the crop after `ocrs`'s resize, before padding.
    resized_width: usize,
    /// Width after padding to `ocrs`'s group width.
    padded_width: usize,
    /// The crop's integral bounding box in image coordinates.
    crop_left: f32,
    crop_width: f32,
}

/// Pads a crop on the right with `ocrs`'s black value to the next multiple of
/// [`OCRS_GROUP_WIDTH_MULTIPLE`], as `ocrs` does when it batches lines.
fn pad_like_ocrs(crop: &NdTensor<f32, 2>) -> NdTensor<f32, 2> {
    let h = crop.shape()[0];
    let w = crop.shape()[1];
    let padded = w.next_multiple_of(OCRS_GROUP_WIDTH_MULTIPLE);
    let mut data = vec![OCRS_BLACK_VALUE; h * padded];
    for y in 0..h {
        for x in 0..w {
            data[y * padded + x] = crop[[y, x]];
        }
    }
    NdTensor::from_data([h, padded], data)
}

fn oracle_matrix(
    engine: &OcrEngine,
    input: &OcrInput,
    raw_model: &Model,
    rect: &RotatedRect,
) -> Result<OracleMatrix, String> {
    let crop = engine
        .prepare_recognition_input(input, std::slice::from_ref(rect))
        .map_err(|e| format!("prepare_recognition_input failed: {e}"))?;
    let resized_width = crop.shape()[1];
    let int_rect = bounding_rect(std::slice::from_ref(rect).iter())
        .ok_or_else(|| "empty oracle rect".to_string())?
        .integral_bounding_rect();
    let padded = pad_like_ocrs(&crop);
    let padded_width = padded.shape()[1];
    let batch = run_recognition_batch(raw_model, std::slice::from_ref(&padded))?;
    Ok(OracleMatrix {
        batch,
        resized_width,
        padded_width,
        crop_left: int_rect.left() as f32,
        crop_width: int_rect.width() as f32,
    })
}

/// Greedy CTC decode of item 0, the way `ocrs` does it: per-timestep argmax
/// (the first maximum), repeats collapsed, blanks dropped, and characters
/// whose start lies in the padding region ignored.
fn greedy_text(m: &OracleMatrix) -> String {
    let seq = m.batch.shape()[1];
    let classes = m.batch.shape()[2];
    if seq == 0 || classes == 0 {
        return String::new();
    }
    let downsample = (m.padded_width as f64 / seq as f64).round() as usize;
    let mut text = String::new();
    let mut last = 0usize;
    for t in 0..seq {
        let mut best = 0usize;
        let mut best_value = m.batch[[0, t, 0]];
        for c in 1..classes {
            let v = m.batch[[0, t, c]];
            if v > best_value {
                best_value = v;
                best = c;
            }
        }
        if best == last {
            continue;
        }
        last = best;
        if best > 0 && t * downsample < m.resized_width {
            text.push(ctc_char(best).unwrap_or('?'));
        }
    }
    text
}

/// Whether the beam read starts with the truth: strictly (the first `n`
/// characters), and ignoring the filler `<`, which `ocrs` essentially never
/// emits (the truth's non-filler characters among its first `n`).
fn beam_prefix_flags(text: &str, truth: &[char], n: usize) -> (bool, bool) {
    let want: Vec<char> = truth.iter().copied().take(n).collect();
    let strict = text.chars().take(n).eq(want.iter().copied());
    let want_no_filler: Vec<char> = want.iter().copied().filter(|&c| c != '<').collect();
    let lenient = !want_no_filler.is_empty()
        && text
            .chars()
            .take(want_no_filler.len())
            .eq(want_no_filler.iter().copied());
    (strict, lenient)
}

// ---------------------------------------------------------------------
// One render
// ---------------------------------------------------------------------

struct EvalCtx<'a> {
    engine: &'a OcrEngine,
    raw_model: &'a Model,
    alphabet: &'a Alphabet,
    line: Rect,
    mrz_chars: u32,
    cells: &'a [usize],
}

#[derive(Debug, Clone)]
struct CellOutcome {
    cell: usize,
    truth: char,
    det: DetStats,
    word_covers_center: bool,
    rec: Option<CellStats>,
}

#[derive(Debug, Clone)]
struct RenderOutcome {
    image_w: u32,
    image_h: u32,
    beam_prefix_ok: bool,
    beam_prefix_ok_ignoring_filler: bool,
    cells: Vec<CellOutcome>,
}

fn evaluate_render(
    ctx: &EvalCtx<'_>,
    degraded: &Degraded,
    truth_line: &[char],
) -> Result<RenderOutcome, String> {
    let (w, h) = degraded.image.dimensions();
    let source = ImageSource::from_bytes(degraded.image.as_raw(), (w, h))
        .map_err(|e| format!("failed to prepare image source: {e}"))?;
    let input = ctx
        .engine
        .prepare_input(source)
        .map_err(|e| format!("failed to prepare ocr input: {e}"))?;
    let det_map = ctx
        .engine
        .detect_text_pixels(&input)
        .map_err(|e| format!("detect_text_pixels failed: {e}"))?;
    let words = ctx
        .engine
        .detect_words(&input)
        .map_err(|e| format!("detect_words failed: {e}"))?;
    let threshold = ctx.engine.detection_threshold();

    let (left, top, right, bottom) = oracle_line_bounds(&degraded.map, ctx.line, w, h);
    let rect = axis_aligned_rect(top as f32, left as f32, bottom as f32, right as f32);
    let matrix = oracle_matrix(ctx.engine, &input, ctx.raw_model, &rect)?;

    let mut cells = Vec::with_capacity(ctx.cells.len());
    for &cell in ctx.cells {
        let truth = *truth_line
            .get(cell)
            .ok_or_else(|| format!("cell {cell} is past the end of the truth line"))?;
        let geometry = cell_geometry(&degraded.map, ctx.line, ctx.mrz_chars, cell);
        let window = pixel_bounds(geometry.bounds, w as usize, h as usize)
            .ok_or_else(|| "empty image".to_string())?;
        let det = detection_stats(&det_map, window, threshold)
            .ok_or_else(|| format!("cell {cell} has an empty detection window"))?;
        let centre = PointF::from_yx(geometry.centre.1 as f32, geometry.centre.0 as f32);
        let word_covers_center = words.iter().any(|word| word.contains(centre));

        let (t0, t1) = timestep_range(
            geometry.x_lo as f32,
            geometry.x_hi as f32,
            matrix.crop_left,
            matrix.crop_width,
            matrix.resized_width as f32,
        );
        let (range, rows) = step_probs(&matrix.batch, 0, t0, t1);
        let rec = cell_stats(ctx.alphabet, &rows, truth, range);
        cells.push(CellOutcome {
            cell,
            truth,
            det,
            word_covers_center,
            rec,
        });
    }

    // The control: the beam decoder's read of the same crop.
    let prefix_len = ctx.cells.iter().copied().max().map_or(0, |c| c + 1);
    let lines = ctx
        .engine
        .recognize_text(&input, &[vec![rect.clone()]])
        .map_err(|e| format!("beam recognition failed: {e}"))?;
    let beam_text = lines
        .into_iter()
        .next()
        .flatten()
        .map(|line| line.to_string())
        .unwrap_or_default();
    let (strict, lenient) = beam_prefix_flags(&beam_text, truth_line, prefix_len);

    Ok(RenderOutcome {
        image_w: w,
        image_h: h,
        beam_prefix_ok: strict,
        beam_prefix_ok_ignoring_filler: lenient,
        cells,
    })
}

/// The run-start self-check: on seed 0's clean line, the greedy decode of the
/// raw matrix through `ctc_matrix`'s alphabet must equal `ocrs`'s own greedy
/// read of the same crop. The upstream alphabet pin.
fn self_check(
    greedy_engine: &OcrEngine,
    raw_model: &Model,
    clean: &RgbImage,
    line: Rect,
) -> Result<(), String> {
    let (w, h) = clean.dimensions();
    let source = ImageSource::from_bytes(clean.as_raw(), (w, h))
        .map_err(|e| format!("self-check: failed to prepare image source: {e}"))?;
    let input = greedy_engine
        .prepare_input(source)
        .map_err(|e| format!("self-check: failed to prepare ocr input: {e}"))?;
    let (left, top, right, bottom) = oracle_line_bounds(&PageMap::Identity, line, w, h);
    let rect = axis_aligned_rect(top as f32, left as f32, bottom as f32, right as f32);
    let matrix = oracle_matrix(greedy_engine, &input, raw_model, &rect)?;
    let mine = greedy_text(&matrix);
    let lines = greedy_engine
        .recognize_text(&input, &[vec![rect.clone()]])
        .map_err(|e| format!("self-check: ocrs greedy read failed: {e}"))?;
    let theirs = lines
        .into_iter()
        .next()
        .flatten()
        .map(|l| l.to_string())
        .unwrap_or_default();
    if mine == theirs {
        return Ok(());
    }
    let first_difference = mine
        .chars()
        .zip(theirs.chars())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| mine.chars().count().min(theirs.chars().count()));
    Err(format!(
        "alphabet self-check failed: the raw matrix decoded through ctc_matrix's alphabet reads \
         {} characters, ocrs's own greedy read {}, first difference at index {first_difference}; \
         an ocrs alphabet or preprocessing change makes every figure this tool would record \
         unreliable",
        mine.chars().count(),
        theirs.chars().count()
    ))
}

// ---------------------------------------------------------------------
// Aggregation and the publishable summary
// ---------------------------------------------------------------------

/// `(cell, axis, step index)`.
type AggKey = (usize, Axis, usize);

#[derive(Default)]
struct CellAgg {
    /// Renders aggregated.
    n: u64,
    /// Renders where the cell had no timesteps.
    rec_missing: u64,
    /// Renders where the winner was the true glyph.
    correct: u64,
    p_true_peak: Vec<f64>,
    margins: Vec<f64>,
    wrong_winners: BTreeMap<String, u64>,
    det_frac_above: Vec<f64>,
    word_hits: u64,
}

impl CellAgg {
    fn add(&mut self, outcome: &CellOutcome) {
        self.n += 1;
        self.det_frac_above.push(outcome.det.frac_above);
        if outcome.word_covers_center {
            self.word_hits += 1;
        }
        match &outcome.rec {
            None => self.rec_missing += 1,
            Some(rec) => {
                self.p_true_peak.push(rec.p_true_peak);
                self.margins.push(rec.margin);
                if rec.winner == Some(outcome.truth) {
                    self.correct += 1;
                } else {
                    let key = rec
                        .winner
                        .map_or_else(|| "none".to_string(), |ch| ch.to_string());
                    *self.wrong_winners.entry(key).or_insert(0) += 1;
                }
            }
        }
    }
}

/// Nearest-rank percentile of an unsorted sample: index `round(p (n - 1))` of
/// the sorted values. `None` for an empty sample.
fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted.get(index).copied()
}

/// The Wilson score interval for `k` successes in `n` trials at 95%. `None`
/// for `n = 0`.
fn wilson_95(k: u64, n: u64) -> Option<(f64, f64)> {
    if n == 0 {
        return None;
    }
    let z = 1.959_963_984_540_054_f64;
    let n = n as f64;
    let p = k as f64 / n;
    let denominator = 1.0 + z * z / n;
    let centre = (p + z * z / (2.0 * n)) / denominator;
    let half = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt() / denominator;
    Some(((centre - half).max(0.0), (centre + half).min(1.0)))
}

fn build_summary(
    run: &Value,
    cell_truths: &[(usize, char)],
    axes: &[Axis],
    aggs: &BTreeMap<AggKey, CellAgg>,
) -> Value {
    let mut results: Vec<Value> = Vec::new();
    for &(cell, truth) in cell_truths {
        for &axis in axes {
            for (step_index, &param) in axis.steps().iter().enumerate() {
                let Some(agg) = aggs.get(&(cell, axis, step_index)) else {
                    continue;
                };
                let n_rec = agg.n - agg.rec_missing;
                let wilson = wilson_95(agg.correct, n_rec);
                let mut wrong: Vec<(&String, &u64)> = agg.wrong_winners.iter().collect();
                wrong.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
                let winner_when_wrong: Vec<Value> = wrong
                    .into_iter()
                    .map(|(ch, count)| json!({"char": ch, "count": count}))
                    .collect();
                results.push(json!({
                    "cell": cell,
                    "truth": truth.to_string(),
                    "axis": axis.name(),
                    "unit": axis.unit(),
                    "step": step_index,
                    "param": param,
                    "n": agg.n,
                    "rec_missing": agg.rec_missing,
                    "p_true_peak": {
                        "p10": percentile(&agg.p_true_peak, 0.10),
                        "median": percentile(&agg.p_true_peak, 0.50),
                        "p90": percentile(&agg.p_true_peak, 0.90),
                    },
                    "accuracy": {
                        "correct": agg.correct,
                        "n": n_rec,
                        "rate": if n_rec > 0 { Some(agg.correct as f64 / n_rec as f64) } else { None },
                        "wilson95_lo": wilson.map(|w| w.0),
                        "wilson95_hi": wilson.map(|w| w.1),
                    },
                    "median_margin": percentile(&agg.margins, 0.50),
                    "winner_when_wrong": winner_when_wrong,
                    "median_det_frac_above_threshold": percentile(&agg.det_frac_above, 0.50),
                    "word_covers_center_rate": if agg.n > 0 {
                        Some(agg.word_hits as f64 / agg.n as f64)
                    } else {
                        None
                    },
                }));
            }
        }
    }
    json!({
        "schema": SCHEMA_VERSION,
        "tool": TOOL_NAME,
        "evidence": EVIDENCE_LABEL,
        "run": run,
        "results": results,
    })
}

// ---------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------

/// Every key this tool ever serializes, in `records.jsonl` and
/// `summary.json`. A test checks the emitted keys against it, so a new field
/// has to be added here on purpose, and nothing that looks like text, a
/// path, or a per-seed row in the summary can slip in unnoticed.
const ALLOWED_KEYS: &[&str] = &[
    // header
    "type",
    "schema",
    "tool",
    "evidence",
    "git",
    "commit",
    "dirty",
    "machine",
    "os",
    "arch",
    "logical_cpus",
    "cpu_model",
    "rten_num_threads",
    "ocrs",
    "rten",
    "models",
    "detection_sha256",
    "recognition_sha256",
    "font",
    "name",
    "path",
    "sha256",
    "license",
    "format",
    "code",
    "line",
    "cells",
    "cell",
    "truth",
    "crop",
    "axes",
    "axis",
    "unit",
    "steps",
    "identity_step",
    "method",
    "seeds",
    "start",
    "count",
    "resize_filter",
    "rng",
    "seed_derivation",
    "detection_threshold",
    "beam_width",
    "conventions",
    "label_collapse",
    "winner_basis",
    "tie_rule",
    "args",
    // render lines
    "seed",
    "step",
    "param",
    "px_per_cell",
    "image_w",
    "image_h",
    "beam_prefix_ok",
    "beam_prefix_ok_ignoring_filler",
    "ms",
    "reused_identity",
    // cell lines
    "det",
    "p_mean",
    "p_max",
    "frac_above_threshold",
    "word_covers_center",
    "rec",
    "timesteps",
    "p_true_peak",
    "p_true_mean",
    "winner",
    "winner_p",
    "runner_up",
    "runner_up_p",
    "runner_up_mrz",
    "runner_up_mrz_p",
    "margin",
    "blank_mean",
    // summary
    "run",
    "results",
    "n",
    "rec_missing",
    "p10",
    "median",
    "p90",
    "accuracy",
    "correct",
    "rate",
    "wilson95_lo",
    "wilson95_hi",
    "median_margin",
    "winner_when_wrong",
    "char",
    "count",
    "median_det_frac_above_threshold",
    "word_covers_center_rate",
];

struct HeaderInputs {
    git_commit: String,
    git_dirty: bool,
    os: String,
    arch: String,
    logical_cpus: usize,
    cpu_model: Option<String>,
    rten_num_threads: String,
    detection_sha256: String,
    recognition_sha256: String,
    font_sha256: String,
    detection_threshold: f32,
    format: Format,
    code: Code,
    cell_truths: Vec<(usize, char)>,
    axes: Vec<Axis>,
    seed_start: u64,
    seeds: u64,
    args: Vec<String>,
}

fn build_header(inputs: &HeaderInputs) -> Value {
    let axes: Vec<Value> = inputs
        .axes
        .iter()
        .map(|axis| {
            json!({
                "axis": axis.name(),
                "unit": axis.unit(),
                "steps": axis.steps(),
                "identity_step": axis.identity_step(),
                "method": axis.method(),
            })
        })
        .collect();
    let cells: Vec<Value> = inputs
        .cell_truths
        .iter()
        .map(|&(cell, truth)| json!({"cell": cell, "truth": truth.to_string()}))
        .collect();
    json!({
        "type": "header",
        "schema": SCHEMA_VERSION,
        "tool": TOOL_NAME,
        "evidence": EVIDENCE_LABEL,
        "git": {"commit": inputs.git_commit, "dirty": inputs.git_dirty},
        "machine": {
            "os": inputs.os,
            "arch": inputs.arch,
            "logical_cpus": inputs.logical_cpus,
            "cpu_model": inputs.cpu_model,
            "rten_num_threads": inputs.rten_num_threads,
        },
        "ocrs": OCRS_VERSION,
        "rten": RTEN_VERSION,
        "models": {
            "detection_sha256": inputs.detection_sha256,
            "recognition_sha256": inputs.recognition_sha256,
        },
        "font": {
            "name": FONT_NAME,
            "path": FONT_PATH,
            "sha256": inputs.font_sha256,
            "license": FONT_LICENSE,
        },
        "format": inputs.format.name(),
        "code": code_text(inputs.code),
        "line": LINE_INDEX,
        "cells": cells,
        "crop": "oracle_line",
        "axes": axes,
        "seeds": {"start": inputs.seed_start, "count": inputs.seeds},
        "resize_filter": RESIZE_FILTER_NAME,
        "rng": RNG_DESCRIPTION,
        "seed_derivation": SEED_DERIVATION,
        "detection_threshold": (f64::from(inputs.detection_threshold) * 1e6).round() / 1e6,
        "beam_width": MRZ_BEAM_WIDTH,
        "conventions": {
            "label_collapse": "probability is summed over every label decoding to the same \
                character (labels 44 and 49 are both E); a group is keyed by its lowest label",
            "winner_basis": "winner, runner-up, margin and blank_mean use the distribution \
                averaged over the cell's timestep range, blank excluded from winner and \
                runner-up; p_true_peak is the largest per-timestep probability of the true glyph",
            "tie_rule": "a tie goes to the lower label index",
        },
        "args": inputs.args,
    })
}

#[allow(clippy::too_many_arguments)]
fn render_line(
    seed: u64,
    axis: Axis,
    step_index: usize,
    param: f64,
    px_per_cell: f64,
    outcome: &RenderOutcome,
    ms: f64,
    reused_identity: bool,
) -> Value {
    json!({
        "type": "render",
        "seed": seed,
        "axis": axis.name(),
        "step": step_index,
        "param": param,
        "px_per_cell": px_per_cell,
        "image_w": outcome.image_w,
        "image_h": outcome.image_h,
        "beam_prefix_ok": outcome.beam_prefix_ok,
        "beam_prefix_ok_ignoring_filler": outcome.beam_prefix_ok_ignoring_filler,
        "ms": ms,
        "reused_identity": reused_identity,
    })
}

fn cell_line(seed: u64, axis: Axis, step_index: usize, cell: &CellOutcome) -> Value {
    let opt_char = |c: Option<char>| c.map(|ch| ch.to_string());
    let rec = cell.rec.as_ref().map(|r| {
        json!({
            "timesteps": [r.timesteps.0, r.timesteps.1],
            "p_true_peak": r.p_true_peak,
            "p_true_mean": r.p_true_mean,
            "winner": opt_char(r.winner),
            "winner_p": r.winner_p,
            "runner_up": opt_char(r.runner_up),
            "runner_up_p": r.runner_up_p,
            "runner_up_mrz": opt_char(r.runner_up_mrz),
            "runner_up_mrz_p": r.runner_up_mrz_p,
            "margin": r.margin,
            "blank_mean": r.blank_mean,
        })
    });
    json!({
        "type": "cell",
        "seed": seed,
        "axis": axis.name(),
        "step": step_index,
        "cell": cell.cell,
        "truth": cell.truth.to_string(),
        "det": {
            "p_mean": cell.det.p_mean,
            "p_max": cell.det.p_max,
            "frac_above_threshold": cell.det.frac_above,
            "word_covers_center": cell.word_covers_center,
        },
        "rec": rec,
    })
}

// ---------------------------------------------------------------------
// Arguments, output directory, small utilities
// ---------------------------------------------------------------------

#[derive(Debug, PartialEq)]
struct Args {
    format: Format,
    code: Code,
    axes: Vec<Axis>,
    seeds: u64,
    seed_start: u64,
    cells: Vec<usize>,
    out: Option<PathBuf>,
}

const USAGE: &str = "glyph_atlas: per-cell detection and recognition measurements under one \
degradation at a time (MRZ line one, synthetic renders, vendored font, oracle crop)

  --format <f>        td1, td2, td3, mrva or mrvb (default td3); sets the layout and the
                      line-one length, the --cells limit
  --code <XY>         the two-character document code line one starts with (default: the
                      format's letter and the filler, e.g. P<). The first character must fit
                      the format: P for td3; A, C or I for td1 and td2; V for mrva and mrvb.
                      The second is A-Z or <. Quote a code holding < in PowerShell. Needs a
                      --release build when it is not the default.
  --axes <list>       comma list of resolution,jpeg,rotation,blur,noise,contrast (default: all)
  --seeds <n>         renders per step (default 100)
  --seed-start <n>    first seed (default 0)
  --cells <list>      cell indices of the first MRZ line (default 0,1)
  --out <dir>         output directory (default artifacts/glyph-atlas/<date>-<shortsha>/, with
                      -<format>-<code> added for a non-default format or code, < written 0;
                      an in-tree directory that git does not ignore is refused)
  --help              this text

Models come from SYNTHPASS_OCR_MODEL_DIR, else the repo root, and must match their pinned hashes.";

fn parse_axes(list: &str) -> Result<Vec<Axis>, String> {
    let mut axes: Vec<Axis> = Vec::new();
    for name in list.split(',').map(str::trim) {
        let axis = Axis::parse(name).ok_or_else(|| {
            format!("unknown axis {name:?}; expected resolution, jpeg, rotation, blur, noise or contrast")
        })?;
        if axes.contains(&axis) {
            return Err(format!("axis {name:?} listed twice"));
        }
        axes.push(axis);
    }
    Ok(axes)
}

/// The cell numbers in `list`, each listed once. The limit is the format's,
/// and is checked by [`check_cells`] once every flag has been read.
fn parse_cells(list: &str) -> Result<Vec<usize>, String> {
    let mut cells: Vec<usize> = Vec::new();
    for item in list.split(',').map(str::trim) {
        let cell: usize = item
            .parse()
            .map_err(|e| format!("--cells: {item:?}: {e}"))?;
        if cells.contains(&cell) {
            return Err(format!("--cells: cell {cell} listed twice"));
        }
        cells.push(cell);
    }
    Ok(cells)
}

/// Every cell must lie inside line one of `format`.
fn check_cells(cells: &[usize], format: Format) -> Result<(), String> {
    let limit = format.cells();
    match cells.iter().find(|&&cell| cell >= limit) {
        Some(cell) => Err(format!(
            "--cells: cell {cell} is outside the {limit}-cell {} line",
            format.name()
        )),
        None => Ok(()),
    }
}

fn parse_args(raw: &[String]) -> Result<Args, String> {
    let mut args = Args {
        format: Format::Td3,
        code: Format::Td3.default_code(),
        axes: Axis::ALL.to_vec(),
        seeds: DEFAULT_SEEDS,
        seed_start: 0,
        cells: DEFAULT_CELLS.to_vec(),
        out: None,
    };
    // The code is checked against the format, so it waits until every flag
    // has been read and the two may come in either order.
    let mut code_arg: Option<&str> = None;
    let mut i = 0;
    while i < raw.len() {
        let flag = raw[i].as_str();
        let value = raw.get(i + 1).map(String::as_str);
        let need = || value.ok_or_else(|| format!("{flag} needs a value"));
        match flag {
            "--format" => {
                let text = need()?;
                args.format = Format::parse(text).ok_or_else(|| {
                    format!("--format: {text:?} is not one of td1, td2, td3, mrva, mrvb")
                })?;
            }
            "--code" => code_arg = Some(need()?),
            "--axes" => args.axes = parse_axes(need()?)?,
            "--seeds" => {
                args.seeds = need()?.parse().map_err(|e| format!("--seeds: {e}"))?;
            }
            "--seed-start" => {
                args.seed_start = need()?.parse().map_err(|e| format!("--seed-start: {e}"))?;
            }
            "--cells" => args.cells = parse_cells(need()?)?,
            "--out" => args.out = Some(PathBuf::from(need()?)),
            other => return Err(format!("unknown argument {other:?}; try --help")),
        }
        i += 2;
    }
    args.code = match code_arg {
        Some(text) => parse_code(text, args.format)?,
        None => args.format.default_code(),
    };
    check_cells(&args.cells, args.format)?;
    if args.seeds == 0 {
        return Err("--seeds must be at least 1".to_string());
    }
    if args.seed_start.checked_add(args.seeds).is_none() {
        return Err("--seed-start plus --seeds overflows".to_string());
    }
    Ok(args)
}

/// An absolute path with `.` and `..` resolved lexically (symlinks are not
/// followed; `--out` need not exist yet).
fn absolutize(path: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    let mut out = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Refuses an `out` inside `root` unless `is_ignored` says git ignores a
/// file in it: records are per-seed and must never reach a commit by
/// accident. A directory outside the repository is always fine.
fn check_out_dir(
    out: &Path,
    root: &Path,
    is_ignored: impl Fn(&Path) -> bool,
) -> Result<(), String> {
    if !out.starts_with(root) {
        return Ok(());
    }
    if is_ignored(&out.join("records.jsonl")) {
        return Ok(());
    }
    Err(format!(
        "refusing --out {}: it is inside the repository and git does not ignore it, so per-seed \
         records could be committed by accident; use a directory under artifacts/ or outside \
         the tree",
        out.display()
    ))
}

fn git_ignores(root: &Path, path: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["check-ignore", "-q", "--"])
        .arg(path)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// `(year, month, day)` in UTC for a Unix time (Hinnant's civil-from-days).
fn civil_date_from_unix(secs: i64) -> (i64, u32, u32) {
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month as u32, day as u32)
}

fn today_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, m, d) = civil_date_from_unix(secs);
    format!("{y:04}-{m:02}-{d:02}")
}

fn cpu_model() -> Option<String> {
    let text = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    text.lines().find_map(|line| {
        line.strip_prefix("model name")
            .and_then(|rest| rest.split_once(':'))
            .map(|(_, value)| value.trim().to_string())
    })
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/synthpass-ocr -> repo root is two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn model_sha256(path: &Path) -> Result<String, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    Ok(synthpass_core::audit::sha256_hex(&bytes))
}

fn write_line(out: &mut impl std::io::Write, value: &Value) -> Result<(), String> {
    let text = serde_json::to_string(value).map_err(|e| format!("serialize failed: {e}"))?;
    writeln!(out, "{text}").map_err(|e| format!("write failed: {e}"))
}

// ---------------------------------------------------------------------
// main
// ---------------------------------------------------------------------

fn main() {
    if let Err(e) = run() {
        eprintln!("glyph_atlas: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let args = parse_args(&raw)?;
    check_build_for_code(args.format, args.code, cfg!(debug_assertions))?;
    let root = repo_root();
    let models = model_dir(&root);

    // Refuse to measure with unpinned models. `SYNTHPASS_OCR_MODEL_SKIP_VERIFY`
    // is deliberately not consulted.
    let detection_path = models.join("text-detection.rten");
    let recognition_path = models.join("text-recognition.rten");
    verify::verify_detection_model(&detection_path)
        .map_err(|e| format!("detection model failed its pin: {e}"))?;
    verify::verify_recognition_model(&recognition_path)
        .map_err(|e| format!("recognition model failed its pin: {e}"))?;
    let detection_sha256 = model_sha256(&detection_path)?;
    let recognition_sha256 = model_sha256(&recognition_path)?;

    if synthpass_gen::fonts::load_fonts().is_err() {
        return Err(
            "the generator has no embedded fonts; the atlas needs real OCR-B glyphs".to_string(),
        );
    }
    // The font is part of the measurement: refuse a different file.
    let font_sha256 = synthpass_core::audit::sha256_hex(FONT_BYTES);
    if font_sha256 != FONT_SHA256 {
        return Err(format!(
            "the vendored OCR-B font has sha256 {font_sha256}, not the pinned {FONT_SHA256}"
        ));
    }

    // Output location, before any expensive work.
    let date = today_utc();
    let git_commit = git_output(&root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into());
    // Unknown counts as dirty: a run that cannot say which tree it measured
    // must not claim a clean one.
    let git_dirty = match git_output(&root, &["status", "--porcelain"]) {
        Some(status) => !status.is_empty(),
        None => true,
    };
    let short: String = git_commit.chars().take(7).collect();
    let out_dir = absolutize(&args.out.clone().unwrap_or_else(|| {
        root.join("artifacts")
            .join("glyph-atlas")
            .join(default_out_name(&date, &short, args.format, args.code))
    }));
    check_out_dir(&out_dir, &absolutize(&root), |p| git_ignores(&root, p))?;
    let records_path = out_dir.join("records.jsonl");
    if records_path.exists() {
        return Err(format!(
            "{} already exists; refusing to overwrite a run",
            records_path.display()
        ));
    }

    // Engines. One engine (MRZ charset, beam) serves detection and the beam
    // control; the raw model gives the matrix; a general greedy engine exists
    // only for the self-check.
    let engine = load_mrz_engine(&models)?;
    let raw_model = load_raw_recognition_model(&models)?;
    let greedy_engine = load_general_engine(&models)?;
    let alphabet = Alphabet::new();

    let page = layout::for_format(args.format.document_type());
    let line = *page
        .mrz_lines
        .get(LINE_INDEX)
        .ok_or_else(|| format!("the {} layout has no first MRZ line", args.format.name()))?;
    let mrz_chars = page.mrz_chars;

    // Seed 0's clean render: the expected truth for each cell and the input
    // of the alphabet self-check.
    let (image0, labels0) = render_clean(0, args.format, args.code);
    let truth0: Vec<char> = labels0
        .mrz_lines
        .get(LINE_INDEX)
        .ok_or_else(|| "generator produced no first MRZ line".to_string())?
        .chars()
        .collect();
    check_line_prefix(&truth0, args.code)?;
    let cell_truths: Vec<(usize, char)> = args
        .cells
        .iter()
        .map(|&cell| {
            truth0
                .get(cell)
                .copied()
                .map(|ch| (cell, ch))
                .ok_or_else(|| format!("cell {cell} is past the end of the truth line"))
        })
        .collect::<Result<_, _>>()?;
    self_check(&greedy_engine, &raw_model, &image0.into_rgb8(), line)?;
    drop(greedy_engine);
    eprintln!("glyph_atlas: alphabet self-check passed");

    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("failed to create {}: {e}", out_dir.display()))?;
    let file = std::fs::File::create(&records_path)
        .map_err(|e| format!("failed to create {}: {e}", records_path.display()))?;
    let mut records = std::io::BufWriter::new(file);

    let header_inputs = HeaderInputs {
        git_commit,
        git_dirty,
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        logical_cpus: std::thread::available_parallelism().map_or(0, |n| n.get()),
        cpu_model: cpu_model(),
        rten_num_threads: std::env::var("RTEN_NUM_THREADS").unwrap_or_else(|_| "unset".into()),
        detection_sha256,
        recognition_sha256,
        font_sha256,
        detection_threshold: engine.detection_threshold(),
        format: args.format,
        code: args.code,
        cell_truths: cell_truths.clone(),
        axes: args.axes.clone(),
        seed_start: args.seed_start,
        seeds: args.seeds,
        args: raw.clone(),
    };
    let header = build_header(&header_inputs);
    write_line(&mut records, &header)?;

    let eval = EvalCtx {
        engine: &engine,
        raw_model: &raw_model,
        alphabet: &alphabet,
        line,
        mrz_chars,
        cells: &args.cells,
    };

    let steps_per_seed: usize = args.axes.iter().map(|a| a.steps().len()).sum();
    let planned_renders = steps_per_seed * args.seeds as usize;
    let mut aggs: BTreeMap<AggKey, CellAgg> = BTreeMap::new();
    let mut computed = 0usize;
    let mut done = 0usize;
    let started = Instant::now();

    for seed in args.seed_start..args.seed_start + args.seeds {
        let (image, labels) = render_clean(seed, args.format, args.code);
        let clean = image.into_rgb8();
        let truth_line: Vec<char> = labels
            .mrz_lines
            .get(LINE_INDEX)
            .ok_or_else(|| format!("seed {seed}: no first MRZ line"))?
            .chars()
            .collect();
        for &(cell, expected) in &cell_truths {
            if truth_line.get(cell) != Some(&expected) {
                return Err(format!(
                    "seed {seed}: the truth glyph at cell {cell} differs from seed 0's; the \
                     atlas keys its results by cell and needs a fixed truth per cell"
                ));
            }
        }
        let dctx = DegradeCtx {
            clean: &clean,
            paper: paper_colour(&clean, line),
            line,
        };
        let mut identity_cache: Option<RenderOutcome> = None;

        for &axis in &args.axes {
            for (step_index, &param) in axis.steps().iter().enumerate() {
                let degraded = degrade_step(axis, step_index, param, seed, &dctx)
                    .map_err(|e| format!("seed {seed}, {} step {step_index}: {e}", axis.name()))?;
                let is_identity = degraded.map == PageMap::Identity
                    && degraded.image.dimensions() == clean.dimensions()
                    && degraded.image.as_raw() == clean.as_raw();
                let cached = if is_identity {
                    identity_cache.clone()
                } else {
                    None
                };
                let step_started = Instant::now();
                let (outcome, reused) = match cached {
                    Some(outcome) => (outcome, true),
                    None => {
                        let outcome =
                            evaluate_render(&eval, &degraded, &truth_line).map_err(|e| {
                                format!("seed {seed}, {} step {step_index}: {e}", axis.name())
                            })?;
                        computed += 1;
                        if is_identity {
                            identity_cache = Some(outcome.clone());
                        }
                        (outcome, false)
                    }
                };
                let ms = step_started.elapsed().as_secs_f64() * 1000.0;
                done += 1;

                write_line(
                    &mut records,
                    &render_line(
                        seed,
                        axis,
                        step_index,
                        param,
                        degraded.map.px_per_cell(),
                        &outcome,
                        ms,
                        reused,
                    ),
                )?;
                for cell in &outcome.cells {
                    write_line(&mut records, &cell_line(seed, axis, step_index, cell))?;
                    aggs.entry((cell.cell, axis, step_index))
                        .or_default()
                        .add(cell);
                }

                if !reused && computed == PROGRESS_AFTER {
                    let elapsed = started.elapsed().as_secs_f64();
                    let rate = computed as f64 / elapsed;
                    let remaining = planned_renders.saturating_sub(done) as f64;
                    eprintln!(
                        "glyph_atlas: {computed} computed renders in {elapsed:.1}s = {rate:.2}/s; \
                         {planned_renders} planned (identity steps reused, so an upper bound), \
                         about {:.0} min to go",
                        remaining / rate / 60.0
                    );
                }
            }
        }
        records
            .flush()
            .map_err(|e| format!("failed to flush records: {e}"))?;
    }

    let mut run_block = header.clone();
    if let Some(object) = run_block.as_object_mut() {
        // The summary is the publishable shape: no type tag, and no argv
        // (which can carry a local path).
        object.remove("type");
        object.remove("args");
    }
    let summary = build_summary(&run_block, &cell_truths, &args.axes, &aggs);
    let summary_text = serde_json::to_string_pretty(&summary)
        .map_err(|e| format!("failed to serialize summary: {e}"))?;
    std::fs::write(out_dir.join("summary.json"), summary_text)
        .map_err(|e| format!("failed to write summary.json: {e}"))?;

    print_table(&cell_truths, &args.axes, &aggs);
    eprintln!(
        "glyph_atlas: {computed} computed renders ({done} rendered steps) in {:.1}s; wrote {}",
        started.elapsed().as_secs_f64(),
        out_dir.display()
    );
    Ok(())
}

fn print_table(cell_truths: &[(usize, char)], axes: &[Axis], aggs: &BTreeMap<AggKey, CellAgg>) {
    println!(
        "cell truth axis        step param    n   correct   p_true_peak(median)  margin(median)"
    );
    for &(cell, truth) in cell_truths {
        for &axis in axes {
            for (step_index, &param) in axis.steps().iter().enumerate() {
                let Some(agg) = aggs.get(&(cell, axis, step_index)) else {
                    continue;
                };
                let n_rec = agg.n - agg.rec_missing;
                let show =
                    |v: Option<f64>| v.map_or_else(|| "-".to_string(), |x| format!("{x:.3}"));
                println!(
                    "{cell:>4} {truth:>5} {:<11} {step_index:>4} {param:>5} {:>4} {:>4}/{:<4} {:>10} {:>18}",
                    axis.name(),
                    agg.n,
                    agg.correct,
                    n_rec,
                    show(percentile(&agg.p_true_peak, 0.5)),
                    show(percentile(&agg.margins, 0.5)),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------
// Tests (model-free)
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use synthpass_gen::generate_from_seed;

    // -- helpers --------------------------------------------------------

    fn label_of(ch: char) -> usize {
        (1..=CTC_ALPHABET.chars().count())
            .find(|&l| ctc_char(l) == Some(ch))
            .expect("char is in the alphabet")
    }

    /// The label of the second occurrence of a char, for `E` (label 49).
    fn last_label_of(ch: char) -> usize {
        (1..=CTC_ALPHABET.chars().count())
            .rev()
            .find(|&l| ctc_char(l) == Some(ch))
            .expect("char is in the alphabet")
    }

    /// A probability row over all 97 classes, zero except the given entries.
    fn row(entries: &[(usize, f64)]) -> Vec<f64> {
        let mut row = vec![0.0; CTC_ALPHABET.chars().count() + 1];
        for &(label, p) in entries {
            row[label] = p;
        }
        row
    }

    fn stats(steps: &[Vec<f64>], truth: char) -> CellStats {
        cell_stats(&Alphabet::new(), steps, truth, (0, steps.len())).expect("has timesteps")
    }

    /// A small deterministic test page: light paper with dark "ink" blocks.
    fn test_page() -> RgbImage {
        RgbImage::from_fn(96, 64, |x, y| {
            if (x / 6 + y / 6) % 3 == 0 && y > 20 && y < 44 {
                Rgb([30, 30, 40])
            } else {
                Rgb([244, 243, 236])
            }
        })
    }

    fn test_ctx(page: &RgbImage) -> DegradeCtx<'_> {
        DegradeCtx {
            clean: page,
            paper: [244, 243, 236],
            line: Rect::new(12, 22, 66, 20),
        }
    }

    fn collect_keys(value: &Value, keys: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (k, v) in map {
                    keys.push(k.clone());
                    collect_keys(v, keys);
                }
            }
            Value::Array(items) => {
                for item in items {
                    collect_keys(item, keys);
                }
            }
            _ => {}
        }
    }

    fn sample_cell(rec: Option<CellStats>) -> CellOutcome {
        CellOutcome {
            cell: 0,
            truth: 'P',
            det: DetStats {
                p_mean: 0.5,
                p_max: 0.9,
                frac_above: 0.75,
            },
            word_covers_center: true,
            rec,
        }
    }

    fn sample_inputs() -> HeaderInputs {
        HeaderInputs {
            git_commit: "0".repeat(40),
            git_dirty: false,
            os: "linux".into(),
            arch: "x86_64".into(),
            logical_cpus: 8,
            cpu_model: None,
            rten_num_threads: "unset".into(),
            detection_sha256: "a".repeat(64),
            recognition_sha256: "b".repeat(64),
            font_sha256: FONT_SHA256.into(),
            detection_threshold: 0.3,
            format: Format::Td3,
            code: ['P', '<'],
            cell_truths: vec![(0, 'P'), (1, '<')],
            axes: Axis::ALL.to_vec(),
            seed_start: 0,
            seeds: 2,
            args: vec!["--seeds".into(), "2".into()],
        }
    }

    // -- per-cell statistics -------------------------------------------

    #[test]
    fn blank_is_excluded_from_the_winner_and_runner_up() {
        let p = label_of('P');
        let q = label_of('Q');
        let s = stats(&[row(&[(CTC_BLANK_LABEL, 0.9), (p, 0.06), (q, 0.03)])], 'P');
        assert_eq!(s.winner, Some('P'));
        assert_eq!(s.runner_up, Some('Q'));
        assert!((s.blank_mean - 0.9).abs() < 1e-12);
        assert!((s.margin - (0.06 - 0.03)).abs() < 1e-12);
    }

    #[test]
    fn a_tie_goes_to_the_lower_label() {
        let a = label_of('A');
        let b = label_of('B');
        assert!(a < b);
        let s = stats(&[row(&[(a, 0.4), (b, 0.4)])], 'Z');
        assert_eq!(s.winner, Some('A'));
        assert_eq!(s.runner_up, Some('A'));
    }

    #[test]
    fn e_collapses_across_labels_44_and_49() {
        let eur_e = label_of('E');
        let letter_e = last_label_of('E');
        assert_eq!((eur_e, letter_e), (44, 49));
        let s = stats(
            &[row(&[(eur_e, 0.3), (letter_e, 0.4), (label_of('F'), 0.2)])],
            'E',
        );
        assert!((s.p_true_mean - 0.7).abs() < 1e-12);
        assert!((s.p_true_peak - 0.7).abs() < 1e-12);
        assert_eq!(s.winner, Some('E'));
        assert_eq!(s.runner_up, Some('F'));
    }

    #[test]
    fn a_tie_between_e_and_d_goes_to_the_e_group_keyed_by_label_44() {
        let s = stats(
            &[row(&[(label_of('D'), 0.4), (last_label_of('E'), 0.4)])],
            'Z',
        );
        assert_eq!(s.winner, Some('E'));
    }

    #[test]
    fn the_mrz_runner_up_skips_characters_outside_the_charset() {
        let s = stats(
            &[row(&[
                (label_of('P'), 0.5),
                (label_of('p'), 0.3),
                (label_of('Q'), 0.1),
            ])],
            'P',
        );
        assert_eq!(s.runner_up, Some('p'));
        assert!((s.runner_up_p - 0.3).abs() < 1e-12);
        assert_eq!(s.runner_up_mrz, Some('Q'));
        assert!((s.runner_up_mrz_p - 0.1).abs() < 1e-12);
    }

    #[test]
    fn the_margin_is_negative_when_the_runner_up_wins() {
        let lost = stats(&[row(&[(label_of('P'), 0.2), (label_of('B'), 0.6)])], 'P');
        assert_eq!(lost.winner, Some('B'));
        assert_eq!(lost.runner_up, Some('B'));
        assert!((lost.margin - (-0.4)).abs() < 1e-12);
        let won = stats(&[row(&[(label_of('P'), 0.6), (label_of('B'), 0.2)])], 'P');
        assert_eq!(won.winner, Some('P'));
        assert!((won.margin - 0.4).abs() < 1e-12);
    }

    #[test]
    fn peak_and_mean_differ_over_several_timesteps() {
        let p = label_of('P');
        let s = stats(&[row(&[(p, 0.9)]), row(&[(p, 0.1)])], 'P');
        assert!((s.p_true_peak - 0.9).abs() < 1e-12);
        assert!((s.p_true_mean - 0.5).abs() < 1e-12);
    }

    #[test]
    fn no_timesteps_means_no_statistics() {
        assert!(cell_stats(&Alphabet::new(), &[], 'P', (3, 3)).is_none());
    }

    #[test]
    fn alphabet_groups_are_in_ascending_label_order_and_cover_e_once() {
        let alphabet = Alphabet::new();
        let e_groups = alphabet.groups.iter().filter(|(c, _)| *c == 'E').count();
        assert_eq!(e_groups, 1);
        let (_, labels) = &alphabet.groups[alphabet.group_of('E').expect("E is a group")];
        assert_eq!(labels, &vec![44, 49]);
        let firsts: Vec<usize> = alphabet.groups.iter().map(|(_, l)| l[0]).collect();
        assert!(firsts.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn detection_statistics_on_a_hand_built_map() {
        let map = NdTensor::from_data(
            [3, 4],
            vec![
                0.0f32, 0.1, 0.2, 0.3, //
                0.4, 0.5, 0.6, 0.7, //
                0.8, 0.9, 1.0, 0.0,
            ],
        );
        let window = PxRect {
            x0: 1,
            y0: 1,
            x1: 3,
            y1: 3,
        };
        let s = detection_stats(&map, window, 0.7).expect("non-empty window");
        assert!((s.p_mean - (0.5 + 0.6 + 0.9 + 1.0) / 4.0).abs() < 1e-6);
        assert!((s.p_max - 1.0).abs() < 1e-6);
        assert!((s.frac_above - 0.5).abs() < 1e-12);
        let empty = PxRect {
            x0: 9,
            y0: 9,
            x1: 10,
            y1: 10,
        };
        assert!(detection_stats(&map, empty, 0.5).is_none());
    }

    #[test]
    fn greedy_decode_collapses_repeats_and_ignores_the_padding_region() {
        // Log-probabilities: 0 for the chosen class, -5 elsewhere.
        let classes = CTC_ALPHABET.chars().count() + 1;
        let path = [
            label_of('P'),
            label_of('P'),
            CTC_BLANK_LABEL,
            label_of('P'),
            label_of('A'),
            label_of('B'),
        ];
        let mut data = vec![-5.0f32; path.len() * classes];
        for (t, &label) in path.iter().enumerate() {
            data[t * classes + label] = 0.0;
        }
        let batch = NdTensor::from_data([1, path.len(), classes], data);
        // downsample 4, resized width 16: timesteps 0..4 are real, 4.. padding.
        let m = OracleMatrix {
            batch,
            resized_width: 16,
            padded_width: 24,
            crop_left: 0.0,
            crop_width: 16.0,
        };
        assert_eq!(greedy_text(&m), "PP");
    }

    #[test]
    fn the_beam_control_flags_strict_and_filler_free_prefixes() {
        let truth: Vec<char> = "P<UTOX".chars().collect();
        assert_eq!(beam_prefix_flags("PUTO", &truth, 2), (false, true));
        assert_eq!(beam_prefix_flags("P<UT", &truth, 2), (true, true));
        assert_eq!(beam_prefix_flags("QUTO", &truth, 2), (false, false));
        assert_eq!(beam_prefix_flags("", &truth, 2), (false, false));
        // Only filler wanted: the filler-free flag cannot be vacuously true.
        assert_eq!(beam_prefix_flags("P", &truth[1..], 1), (false, false));
    }

    // -- degradations ---------------------------------------------------

    #[test]
    fn identity_steps_are_byte_identical_to_the_clean_render() {
        let page = test_page();
        let ctx = test_ctx(&page);
        for axis in Axis::ALL {
            let Some(identity) = axis.identity_step() else {
                continue;
            };
            let index = axis
                .steps()
                .iter()
                .position(|&s| s == identity)
                .expect("the identity step is one of the steps");
            let degraded = degrade_step(axis, index, identity, 7, &ctx).expect("degrades");
            assert_eq!(
                degraded.image.as_raw(),
                page.as_raw(),
                "{} step {index} must equal the clean render",
                axis.name()
            );
            assert_eq!(degraded.map, PageMap::Identity, "{}", axis.name());
        }
    }

    #[test]
    fn every_step_is_deterministic_for_a_given_seed_axis_and_step() {
        let page = test_page();
        let ctx = test_ctx(&page);
        for axis in Axis::ALL {
            for (index, &param) in axis.steps().iter().enumerate() {
                let a = degrade_step(axis, index, param, 3, &ctx).expect("degrades");
                let b = degrade_step(axis, index, param, 3, &ctx).expect("degrades");
                assert_eq!(
                    a.image.as_raw(),
                    b.image.as_raw(),
                    "{} {param}",
                    axis.name()
                );
                assert_eq!(a.map, b.map);
            }
        }
    }

    #[test]
    fn non_identity_steps_change_the_page() {
        let page = test_page();
        let ctx = test_ctx(&page);
        for (axis, index) in [
            (Axis::Resolution, 5),
            (Axis::Jpeg, 9),
            (Axis::Rotation, 4),
            (Axis::Blur, 4),
            (Axis::Noise, 4),
            (Axis::Contrast, 4),
        ] {
            let param = axis.steps()[index];
            let degraded = degrade_step(axis, index, param, 3, &ctx).expect("degrades");
            assert_ne!(
                degraded.image.as_raw(),
                page.as_raw(),
                "{} {param} should differ from the clean render",
                axis.name()
            );
        }
    }

    #[test]
    fn noise_is_seed_stable_and_depends_on_seed_axis_and_step() {
        let page = test_page();
        let a = add_gaussian_noise(&page, 8.0, derive_seed(1, 4, 3));
        let b = add_gaussian_noise(&page, 8.0, derive_seed(1, 4, 3));
        assert_eq!(a.as_raw(), b.as_raw());
        let other_seed = add_gaussian_noise(&page, 8.0, derive_seed(2, 4, 3));
        let other_step = add_gaussian_noise(&page, 8.0, derive_seed(1, 4, 4));
        assert_ne!(a.as_raw(), other_seed.as_raw());
        assert_ne!(a.as_raw(), other_step.as_raw());
        assert_ne!(derive_seed(1, 3, 3), derive_seed(1, 4, 3));
    }

    #[test]
    fn splitmix64_matches_its_published_first_output() {
        // The reference SplitMix64 from state 0 begins 0xE220A8397B1DCDAF.
        assert_eq!(SplitMix64(0).next_u64(), 0xE220_A839_7B1D_CDAF);
    }

    #[test]
    fn noise_has_roughly_the_requested_standard_deviation() {
        let flat = RgbImage::from_pixel(120, 120, Rgb([128, 128, 128]));
        let noisy = add_gaussian_noise(&flat, 8.0, derive_seed(0, 4, 0));
        let values: Vec<f64> = noisy.pixels().map(|p| f64::from(p.0[0])).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        assert!(
            (variance.sqrt() - 8.0).abs() < 0.5,
            "sigma {}",
            variance.sqrt()
        );
        // Grey noise: the three channels move together.
        assert!(noisy.pixels().all(|p| p.0[0] == p.0[1] && p.0[1] == p.0[2]));
    }

    #[test]
    fn contrast_moves_ink_toward_paper_and_leaves_paper_alone() {
        let page = test_page();
        let faded = scale_ink(&page, 0.5, [244, 243, 236]);
        assert_eq!(faded.get_pixel(0, 0).0, [244, 243, 236]);
        // (12, 24) is an ink block: 244 - 0.5 * (244 - 30) = 137, 243 - 0.5 *
        // (243 - 30) = 136.5 rounds to 137, 236 - 0.5 * (236 - 40) = 138.
        assert_eq!(page.get_pixel(12, 24).0, [30, 30, 40]);
        assert_eq!(faded.get_pixel(12, 24).0, [137, 137, 138]);
    }

    #[test]
    fn jpeg_round_trip_keeps_the_dimensions() {
        let page = test_page();
        let out = jpeg_roundtrip(&page, 50).expect("round trips");
        assert_eq!(out.dimensions(), page.dimensions());
    }

    #[test]
    fn resolution_scales_the_page_and_reports_the_actual_factor() {
        let page = test_page();
        let degraded = resolution_step(&page, 11.0).expect("downscales");
        assert_eq!(degraded.image.dimensions(), (48, 32));
        assert_eq!(degraded.map, PageMap::Scale { sx: 0.5, sy: 0.5 });
        assert!((degraded.map.px_per_cell() - 11.0).abs() < 1e-9);
        assert!(resolution_step(&page, 23.0).is_err());
        assert!(resolution_step(&page, 0.0).is_err());
    }

    // -- geometry -------------------------------------------------------

    #[test]
    fn rect_scaling_under_downscale_scales_the_cell_and_its_extent() {
        let line = Rect::new(60, 720, 968, 50);
        let map = PageMap::Scale { sx: 0.5, sy: 0.5 };
        let g = cell_geometry(&map, line, 44, 1);
        // Cell 1 spans x 82..104 on the clean page.
        assert!((g.x_lo - 41.0).abs() < 1e-9);
        assert!((g.x_hi - 52.0).abs() < 1e-9);
        assert_eq!(g.bounds, (41.0, 360.0, 52.0, 385.0));
        let window = pixel_bounds(g.bounds, 600, 420).expect("inside the image");
        assert_eq!(
            window,
            PxRect {
                x0: 41,
                y0: 360,
                x1: 52,
                y1: 385
            }
        );
    }

    #[test]
    fn pixel_bounds_is_at_least_one_pixel_and_inside_the_image() {
        let w = pixel_bounds((3.2, 4.7, 3.2, 4.7), 10, 10).expect("non-empty image");
        assert_eq!((w.x1 - w.x0, w.y1 - w.y0), (1, 1));
        let edge = pixel_bounds((9.5, 9.5, 20.0, 20.0), 10, 10).expect("non-empty image");
        assert_eq!(
            edge,
            PxRect {
                x0: 9,
                y0: 9,
                x1: 10,
                y1: 10
            }
        );
        assert!(pixel_bounds((0.0, 0.0, 1.0, 1.0), 0, 10).is_none());
    }

    #[test]
    fn a_zero_degree_rotation_is_the_identity_map() {
        let map = PageMap::Rotate {
            degrees: 0.0,
            cx: 544.0,
            cy: 745.0,
        };
        assert_eq!(map.point((100.0, 700.0)), (100.0, 700.0));
        let line = Rect::new(60, 720, 968, 50);
        let g = cell_geometry(&map, line, 44, 5);
        let identity = cell_geometry(&PageMap::Identity, line, 44, 5);
        assert_eq!(g, identity);
    }

    #[test]
    fn a_rotated_cell_box_contains_the_transformed_centre() {
        let line = Rect::new(60, 720, 968, 50);
        for degrees in [0.25, 1.0, 3.0, 5.0] {
            let map = PageMap::Rotate {
                degrees,
                cx: 544.0,
                cy: 745.0,
            };
            for cell in [0usize, 1, 21, 43] {
                let clean_cell = layout::mrz_char_rect_for_line(line, 44, cell as u32);
                let quad = map.quad(&Quad::of_rect(clean_cell));
                let centre = map.point((
                    f64::from(clean_cell.x) + f64::from(clean_cell.width) / 2.0,
                    f64::from(clean_cell.y) + f64::from(clean_cell.height) / 2.0,
                ));
                assert!(quad.contains(centre), "{degrees} degrees, cell {cell}");
                let b = quad.bounds();
                assert!(b.0 <= centre.0 && centre.0 <= b.2 && b.1 <= centre.1 && centre.1 <= b.3);
                assert!(!quad.contains((centre.0 + 200.0, centre.1)));
            }
        }
    }

    #[test]
    fn rotation_is_clockwise_as_displayed() {
        let map = PageMap::Rotate {
            degrees: 90.0,
            cx: 0.0,
            cy: 0.0,
        };
        let (x, y) = map.point((10.0, 0.0));
        assert!(x.abs() < 1e-9 && (y - 10.0).abs() < 1e-9, "({x}, {y})");
    }

    #[test]
    fn rotation_moves_pixels_the_way_the_map_says() {
        // A single dark pixel on paper, rotated 90 degrees about the page
        // centre: the map says where it must land.
        let mut page = RgbImage::from_pixel(21, 21, Rgb([244, 243, 236]));
        page.put_pixel(15, 10, Rgb([0, 0, 0]));
        let out = rotate_about(&page, 90.0, 10.5, 10.5, [244, 243, 236]);
        // The pixel centre (15.5, 10.5) maps to (10.5, 15.5): pixel (10, 15).
        assert_eq!(out.get_pixel(10, 15).0, [0, 0, 0]);
        assert_eq!(out.get_pixel(15, 10).0, [244, 243, 236]);
    }

    #[test]
    fn timestep_ranges_are_monotone_and_never_empty_under_every_geometry() {
        let line = Rect::new(60, 720, 968, 50);
        let maps = [
            PageMap::Identity,
            PageMap::Scale {
                sx: 4.0 / 22.0,
                sy: 4.0 / 22.0,
            },
            PageMap::Scale { sx: 0.5, sy: 0.5 },
            PageMap::Rotate {
                degrees: 3.0,
                cx: 544.0,
                cy: 745.0,
            },
            PageMap::Rotate {
                degrees: 5.0,
                cx: 544.0,
                cy: 745.0,
            },
        ];
        for map in maps {
            let (left, top, right, bottom) = oracle_line_bounds(&map, line, 1200, 840);
            let crop_left = left.floor() as f32;
            let crop_width = (right.ceil() - left.floor()) as f32;
            let crop_height = (bottom.ceil() - top.floor()) as f32;
            let resized_width = 64.0 * crop_width / crop_height;
            let mut previous_start = 0usize;
            for cell in 0..44 {
                let g = cell_geometry(&map, line, 44, cell);
                assert!(g.x_hi > g.x_lo, "{map:?} cell {cell}");
                let (t0, t1) = timestep_range(
                    g.x_lo as f32,
                    g.x_hi as f32,
                    crop_left,
                    crop_width,
                    resized_width,
                );
                assert!(t1 > t0, "{map:?} cell {cell}: {t0}..{t1}");
                assert!(t0 >= previous_start, "{map:?} cell {cell}");
                previous_start = t0;
            }
        }
        // The downsample constant this arithmetic rests on.
        assert_eq!(ctc_matrix::CTC_DOWNSAMPLE, 4.0);
    }

    #[test]
    fn the_oracle_crop_of_a_rotated_line_is_its_bounding_box() {
        let line = Rect::new(60, 720, 968, 50);
        let straight = oracle_line_bounds(&PageMap::Identity, line, 1200, 840);
        assert_eq!(straight, (60.0, 720.0, 1028.0, 770.0));
        let rotated = oracle_line_bounds(
            &PageMap::Rotate {
                degrees: 5.0,
                cx: 544.0,
                cy: 745.0,
            },
            line,
            1200,
            840,
        );
        assert!(rotated.1 < straight.1 && rotated.3 > straight.3);
    }

    // -- the generator's line ------------------------------------------

    #[test]
    fn the_generators_first_td3_line_starts_with_p_filler_and_paper_is_light() {
        let (image, labels, _) =
            generate_from_seed(&GeneratorConfig::with_document_type(0, DocumentType::TD3));
        let first: Vec<char> = labels.mrz_lines[LINE_INDEX].chars().take(2).collect();
        assert_eq!(first, Format::Td3.default_code());
        let page = layout::for_format(DocumentType::TD3);
        let paper = paper_colour(&image.into_rgb8(), page.mrz_lines[LINE_INDEX]);
        assert!(paper.iter().all(|&c| c > 200), "{paper:?}");
    }

    #[test]
    fn the_embedded_font_is_the_pinned_vendored_file() {
        assert_eq!(synthpass_core::audit::sha256_hex(FONT_BYTES), FONT_SHA256);
    }

    #[test]
    fn the_pinned_ocrs_and_rten_versions_match_the_manifest() {
        let manifest = include_str!("../Cargo.toml");
        assert!(manifest.contains(&format!("ocrs = \"{OCRS_VERSION}\"")));
        assert!(manifest.contains(&format!("rten = \"{RTEN_VERSION}\"")));
    }

    // -- output shape ---------------------------------------------------

    #[test]
    fn every_serialized_key_is_on_the_allowlist() {
        let mut values = vec![build_header(&sample_inputs())];
        let rec = stats(&[row(&[(label_of('P'), 0.5), (label_of('B'), 0.2)])], 'P');
        for cell in [sample_cell(Some(rec.clone())), sample_cell(None)] {
            values.push(cell_line(0, Axis::Blur, 1, &cell));
        }
        let outcome = RenderOutcome {
            image_w: 10,
            image_h: 10,
            beam_prefix_ok: true,
            beam_prefix_ok_ignoring_filler: true,
            cells: vec![sample_cell(Some(rec))],
        };
        values.push(render_line(
            0,
            Axis::Blur,
            1,
            0.5,
            22.0,
            &outcome,
            1.0,
            false,
        ));
        let mut aggs = BTreeMap::new();
        aggs.entry((0usize, Axis::Blur, 1usize))
            .or_insert_with(CellAgg::default)
            .add(&outcome.cells[0]);
        values.push(build_summary(
            &build_header(&sample_inputs()),
            &[(0, 'P')],
            &[Axis::Blur],
            &aggs,
        ));
        let mut keys = Vec::new();
        for value in &values {
            collect_keys(value, &mut keys);
        }
        assert!(!keys.is_empty());
        for key in keys {
            assert!(
                ALLOWED_KEYS.contains(&key.as_str()),
                "key {key:?} is not on the allowlist"
            );
        }
    }

    #[test]
    fn the_summary_has_no_per_seed_rows() {
        let rec = stats(&[row(&[(label_of('P'), 0.5)])], 'P');
        let cell = sample_cell(Some(rec));
        let mut aggs: BTreeMap<AggKey, CellAgg> = BTreeMap::new();
        for _ in 0..3 {
            aggs.entry((0, Axis::Blur, 0)).or_default().add(&cell);
        }
        let mut run = build_header(&sample_inputs());
        if let Some(object) = run.as_object_mut() {
            object.remove("type");
            object.remove("args");
        }
        let summary = build_summary(&run, &[(0, 'P')], &[Axis::Blur], &aggs);
        let results = summary["results"].as_array().expect("results is an array");
        // One row per (cell, axis, step) with data, however many seeds fed it.
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["n"], 3);
        let mut keys = Vec::new();
        collect_keys(&summary, &mut keys);
        assert!(!keys.iter().any(|k| k == "seed" || k == "ms" || k == "args"));
        assert!(!keys.iter().any(|k| k == "type"));
        assert_eq!(summary["evidence"], EVIDENCE_LABEL);
    }

    #[test]
    fn the_header_records_the_run_it_describes() {
        let header = build_header(&sample_inputs());
        assert_eq!(header["type"], "header");
        assert_eq!(header["schema"], SCHEMA_VERSION);
        assert_eq!(header["crop"], "oracle_line");
        assert_eq!(header["format"], "TD3");
        assert_eq!(header["code"], "P<");
        assert_eq!(header["schema"], 2);
        assert_eq!(header["ocrs"], "0.13.1");
        assert_eq!(header["axes"].as_array().expect("axes").len(), 6);
        assert_eq!(header["cells"][1]["truth"], "<");
        assert_eq!(header["font"]["license"], "OFL-1.1");
        assert_eq!(header["font"]["sha256"], FONT_SHA256);
    }

    #[test]
    fn wilson_intervals_and_percentiles_match_hand_values() {
        assert!(wilson_95(0, 0).is_none());
        let (lo, hi) = wilson_95(10, 10).expect("n > 0");
        assert!(hi > 0.999 && lo > 0.72 && lo < 0.73, "({lo}, {hi})");
        let (lo, hi) = wilson_95(5, 10).expect("n > 0");
        assert!((lo - 0.2366).abs() < 1e-3 && (hi - 0.7634).abs() < 1e-3);
        let values: Vec<f64> = (0..=10).map(f64::from).collect();
        assert_eq!(percentile(&values, 0.1), Some(1.0));
        assert_eq!(percentile(&values, 0.5), Some(5.0));
        assert_eq!(percentile(&values, 0.9), Some(9.0));
        assert_eq!(percentile(&[], 0.5), None);
    }

    // -- arguments and the output directory ----------------------------

    #[test]
    fn arguments_default_to_all_axes_two_cells_and_a_hundred_seeds() {
        let args = parse_args(&[]).expect("defaults parse");
        assert_eq!(args.axes, Axis::ALL.to_vec());
        assert_eq!(args.seeds, 100);
        assert_eq!(args.seed_start, 0);
        assert_eq!(args.cells, vec![0, 1]);
        assert_eq!(args.out, None);
    }

    #[test]
    fn arguments_parse_and_reject_what_they_should() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<String>>();
        let args = parse_args(&s(&[
            "--axes",
            "blur,noise",
            "--seeds",
            "5",
            "--seed-start",
            "9",
            "--cells",
            "0,1,2",
            "--out",
            "/tmp/x",
        ]))
        .expect("parses");
        assert_eq!(args.axes, vec![Axis::Blur, Axis::Noise]);
        assert_eq!((args.seeds, args.seed_start), (5, 9));
        assert_eq!(args.cells, vec![0, 1, 2]);
        assert_eq!(args.out, Some(PathBuf::from("/tmp/x")));
        assert!(parse_args(&s(&["--axes", "sharpen"])).is_err());
        assert!(parse_args(&s(&["--axes", "blur,blur"])).is_err());
        assert!(parse_args(&s(&["--cells", "44"])).is_err());
        assert!(parse_args(&s(&["--seeds", "0"])).is_err());
        assert!(parse_args(&s(&["--seeds"])).is_err());
        assert!(parse_args(&s(&["--bogus", "1"])).is_err());
    }

    // -- --format and --code --------------------------------------------

    fn parsed(list: &[&str]) -> Result<Args, String> {
        parse_args(&list.iter().map(|x| x.to_string()).collect::<Vec<String>>())
    }

    #[test]
    fn the_default_format_and_code_are_td3_and_p_filler() {
        let args = parsed(&[]).expect("defaults parse");
        assert_eq!((args.format, args.code), (Format::Td3, ['P', '<']));
    }

    #[test]
    fn each_format_defaults_to_its_letter_and_the_filler() {
        for (flag, letter) in [
            ("td1", 'I'),
            ("td2", 'I'),
            ("td3", 'P'),
            ("mrva", 'V'),
            ("mrvb", 'V'),
        ] {
            let args = parsed(&["--format", flag]).expect(flag);
            assert_eq!(args.code, [letter, '<'], "{flag}");
        }
    }

    #[test]
    fn a_code_that_fits_its_format_is_accepted() {
        for (format, code) in [
            ("td3", "PS"),
            ("td3", "P<"),
            ("td1", "ID"),
            ("td1", "AR"),
            ("td1", "C<"),
            ("td2", "IO"),
            ("td2", "A<"),
            ("mrva", "V<"),
            ("mrvb", "VC"),
        ] {
            let args = parsed(&["--format", format, "--code", code]).expect(code);
            assert_eq!(args.code.iter().collect::<String>(), code);
        }
        // The flags may come in either order.
        let args = parsed(&["--code", "ID", "--format", "td1"]).expect("code before format");
        assert_eq!((args.format, args.code), (Format::Td1, ['I', 'D']));
    }

    #[test]
    fn a_code_that_does_not_fit_its_format_is_refused_naming_the_flag() {
        for (format, code) in [
            ("td3", "IS"),
            ("td3", "ID"),
            ("td1", "P<"),
            ("td1", "VC"),
            ("td3", "VC"),
            ("mrva", "P<"),
            ("td3", "P0"),
            ("td3", "P"),
            ("td3", "PSS"),
            ("td3", "ps"),
            ("td3", "P-"),
            ("td3", ""),
        ] {
            let error = parsed(&["--format", format, "--code", code])
                .expect_err(&format!("{format} {code:?} must be refused"));
            assert!(error.contains("--code"), "{error}");
        }
        // With no --format the format is td3.
        assert!(parsed(&["--code", "ID"]).is_err());
        assert!(parsed(&["--code"]).is_err());
    }

    #[test]
    fn an_unknown_format_is_refused() {
        for format in ["td4", "TD3", "mrv", "mrv-a", ""] {
            let error = parsed(&["--format", format]).expect_err(format);
            assert!(error.contains("--format"), "{error}");
        }
        assert!(parsed(&["--format"]).is_err());
    }

    #[test]
    fn the_cell_limit_follows_the_format_whatever_the_flag_order() {
        assert!(parsed(&["--cells", "43"]).is_ok());
        assert!(parsed(&["--cells", "44"]).is_err());
        assert!(parsed(&["--format", "td1", "--cells", "29"]).is_ok());
        assert!(parsed(&["--format", "td1", "--cells", "30"]).is_err());
        assert!(parsed(&["--cells", "30", "--format", "td1"]).is_err());
        assert!(parsed(&["--cells", "36", "--format", "td2"]).is_err());
        assert!(parsed(&["--cells", "35", "--format", "mrvb"]).is_ok());
        assert!(parsed(&["--cells", "43", "--format", "mrva"]).is_ok());
    }

    #[test]
    fn a_code_becomes_a_path_part_with_the_filler_written_as_zero() {
        assert_eq!(code_path_part(['P', '<']), "P0");
        assert_eq!(code_path_part(['P', 'S']), "PS");
        assert_eq!(code_path_part(['C', '<']), "C0");
    }

    #[test]
    fn the_default_output_name_moves_only_for_a_non_default_run() {
        assert_eq!(
            default_out_name("2026-10-01", "abc1234", Format::Td3, ['P', '<']),
            "2026-10-01-abc1234"
        );
        assert_eq!(
            default_out_name("2026-10-01", "abc1234", Format::Td3, ['P', 'S']),
            "2026-10-01-abc1234-td3-PS"
        );
        assert_eq!(
            default_out_name("2026-10-01", "abc1234", Format::Td1, ['I', '<']),
            "2026-10-01-abc1234-td1-I0"
        );
    }

    #[test]
    fn line_one_must_start_with_the_code() {
        let line: Vec<char> = "PSUTO".chars().collect();
        assert!(check_line_prefix(&line, ['P', 'S']).is_ok());
        let error = check_line_prefix(&line, ['P', '<']).expect_err("PS is not P<");
        assert!(error.contains("P<"), "{error}");
        assert!(check_line_prefix(&['P'], ['P', '<']).is_err());
    }

    // The generator's `debug_assert` refuses a document code that is not the
    // format's own, so these two render tests run in a release build only.
    // The tool itself refuses a non-default code in a debug build.
    #[test]
    #[cfg_attr(debug_assertions, ignore = "needs a release build")]
    fn seed_zero_with_code_ps_starts_with_ps() {
        let (_, labels) = render_clean(0, Format::Td3, ['P', 'S']);
        let first: String = labels.mrz_lines[LINE_INDEX].chars().take(2).collect();
        assert_eq!(first, "PS");
        assert_eq!(labels.mrz_lines[LINE_INDEX].chars().count(), 44);
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = "needs a release build")]
    fn seed_zero_with_td1_and_code_id_starts_with_id_and_is_thirty_cells() {
        let (_, labels) = render_clean(0, Format::Td1, ['I', 'D']);
        let line = &labels.mrz_lines[LINE_INDEX];
        assert!(line.starts_with("ID"), "{line}");
        assert_eq!(line.chars().count(), 30);
        assert_eq!(Format::Td1.cells(), 30);
    }

    #[test]
    fn the_default_code_renders_exactly_what_the_generator_renders() {
        // `--code` left at its default must leave today's runs untouched.
        for format in [Format::Td3, Format::Td1, Format::MrvA] {
            let (image, labels) = render_clean(7, format, format.default_code());
            let (plain, plain_labels, _) = generate_from_seed(
                &GeneratorConfig::with_document_type(7, format.document_type()),
            );
            assert_eq!(labels.mrz_lines, plain_labels.mrz_lines, "{format:?}");
            assert_eq!(
                image.into_rgb8().as_raw(),
                plain.into_rgb8().as_raw(),
                "{format:?}"
            );
        }
    }

    #[test]
    fn a_non_default_code_is_refused_in_a_debug_build_and_a_default_one_is_not() {
        assert!(check_build_for_code(Format::Td3, ['P', '<'], true).is_ok());
        assert!(check_build_for_code(Format::Td3, ['P', 'S'], false).is_ok());
        let error = check_build_for_code(Format::Td3, ['P', 'S'], true).expect_err("debug");
        assert!(error.contains("--release"), "{error}");
    }

    #[test]
    fn an_in_tree_out_dir_is_refused_unless_git_ignores_it() {
        let root = Path::new("/repo");
        let inside = Path::new("/repo/results/run1");
        assert!(check_out_dir(inside, root, |_| false).is_err());
        assert!(check_out_dir(inside, root, |_| true).is_ok());
        // The ignore check is made on a file inside the directory.
        let asked = std::cell::RefCell::new(None);
        let _ = check_out_dir(inside, root, |p| {
            *asked.borrow_mut() = Some(p.to_path_buf());
            true
        });
        assert_eq!(
            asked.into_inner(),
            Some(PathBuf::from("/repo/results/run1/records.jsonl"))
        );
        // Outside the tree, git is never consulted.
        let outside = Path::new("/tmp/atlas");
        assert!(check_out_dir(outside, root, |_| panic!("must not be asked")).is_ok());
        // A sibling that merely shares a name prefix is outside.
        assert!(check_out_dir(Path::new("/repo-other/x"), root, |_| false).is_ok());
    }

    #[test]
    fn dot_dot_cannot_smuggle_an_in_tree_path_out_of_the_check() {
        // The root goes through `absolutize` like the path under test, so the
        // assertion reads the same on every OS: on Windows a rootless `/repo`
        // is not absolute, and both come back prefixed with the current drive.
        let root = absolutize(Path::new("/repo"));
        let cleaned = absolutize(Path::new("/tmp/../repo/results/./run"));
        assert_eq!(cleaned, root.join("results").join("run"));
        assert!(check_out_dir(&cleaned, &root, |_| false).is_err());
    }

    #[test]
    fn civil_dates_match_known_instants() {
        assert_eq!(civil_date_from_unix(0), (1970, 1, 1));
        assert_eq!(civil_date_from_unix(951_782_400), (2000, 2, 29));
        assert_eq!(civil_date_from_unix(1_000_000_000), (2001, 9, 9));
        assert_eq!(civil_date_from_unix(1_790_000_000), (2026, 9, 21));
    }
}
