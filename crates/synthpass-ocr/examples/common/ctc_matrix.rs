//! Reading `ocrs`'s CTC recognition matrix directly — the pieces
//! `probe_matrix.rs` and `glyph_atlas.rs` share.
//!
//! Included with `#[path = "common/ctc_matrix.rs"] mod ctc_matrix;` in each
//! example that needs it, the same way `common/mod.rs` is: Cargo's example
//! auto-discovery picks up `examples/*.rs` and `examples/*/main.rs`, and
//! neither `common/mod.rs` nor `common/ctc_matrix.rs` matches. Each includer
//! uses a different subset, so the `mod` declaration carries
//! `#[allow(dead_code)]`.
//!
//! # What lives here
//!
//! - The CTC label table ([`CTC_ALPHABET`], [`ctc_char`]) and the label
//!   constants the probes' items 2 and 3 depend on.
//! - Running the raw recognition model on a batch of line crops
//!   ([`run_recognition_batch`]) and turning a timestep range into an
//!   averaged per-class distribution ([`averaged_distribution`]).
//! - The pixel-to-timestep arithmetic ([`timestep_range`]) and the
//!   axis-aligned crop rectangle ([`axis_aligned_rect`]).
//! - Loading the two `.rten` models into the engines the probes need, from
//!   [`model_dir`] (`SYNTHPASS_OCR_MODEL_DIR`, else the repo root).
//!
//! # Facts about the model output
//!
//! The recognition model's terminal op is `LogSoftmax`, so its raw output is
//! already normalized log-probabilities: converting a value to a probability
//! is `.exp()`. `softmax()` on an already-log-softmax matrix is not the
//! identity and flattens the distribution toward uniform, so nothing here
//! calls it. `class` is alphabet length + 1 = 97 (index 0 is the CTC blank;
//! label `i + 1` is the character at index `i` of [`CTC_ALPHABET`]).

use std::path::{Path, PathBuf};

use ocrs::{DecodeMethod, OcrEngine, OcrEngineParams};
use rten::Model;
use rten_imageproc::{PointF, RotatedRect, Vec2};
use rten_tensor::prelude::*;
use rten_tensor::NdTensor;

use synthpass_ocr::MRZ_CHARSET;

// ---------------------------------------------------------------------
// The CTC label table (verified facts; see the module doc).
// ---------------------------------------------------------------------

/// `ocrs` 0.13.1's private recognition alphabet (`DEFAULT_ALPHABET` in
/// `ocrs`'s `lib.rs`), duplicated here because it is never exposed —
/// `OcrEngineParams::allowed_chars` only masks decode probabilities to
/// `-Inf` for excluded characters, it does not change the model's own
/// alphabet or class count (`ocrs::OcrEngine::new_impl` computes
/// `excluded_char_labels` *from* this alphabet, which is used verbatim by
/// both the general and MRZ-constrained engines).
///
/// Two guards, of different strength. `ctc_alphabet_tests` below only pins
/// this copy's shape and the labels the probes rely on (29, 44, 49); it
/// cannot see upstream. What catches an upstream reorder is
/// `glyph_atlas`'s run-start self-check, which compares a greedy decode of
/// the raw matrix through this table against `ocrs`'s own greedy read of the
/// same crop and aborts the run if they differ.
pub const CTC_ALPHABET: &str = " 0123456789!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~EABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// CTC label 0 is reserved for the blank symbol; label `i + 1` is
/// [`CTC_ALPHABET`]'s character at index `i`.
pub const CTC_BLANK_LABEL: usize = 0;

/// `<` — the MRZ filler `ocrs`'s beam decoder essentially never emits (see
/// `synthpass_ocr::chargrid`'s module doc). Item 2.
pub const CTC_LABEL_FILLER: usize = 29;

/// The mangled EUR-symbol class `ocrs`'s alphabet stores as ASCII `'E'`
/// (`// nb. The "E" before "ABCDE" should be the EUR symbol.` — `ocrs`'s own
/// source comment), distinct from the genuine alphabetic `'E'` at
/// [`CTC_LABEL_LETTER_E`]. Both decode to the same character, so only the
/// raw matrix can tell them apart. Item 3.
pub const CTC_LABEL_EUR_E: usize = 44;

/// The genuine alphabetic `E`. Item 3.
pub const CTC_LABEL_LETTER_E: usize = 49;

/// Width downsample factor of the recognition model's CNN stage (verified;
/// see `probe_matrix`'s module doc).
pub const CTC_DOWNSAMPLE: f32 = 4.0;

/// Beam width for `ocrs`'s MRZ-constrained engine, mirroring
/// `synthpass_ocr`'s own private `MRZ_BEAM_WIDTH` so the control read matches
/// production's actual decode.
pub const MRZ_BEAM_WIDTH: u32 = 24;

pub fn ctc_char(label: usize) -> Option<char> {
    if label == CTC_BLANK_LABEL {
        None
    } else {
        CTC_ALPHABET.chars().nth(label - 1)
    }
}

// ---------------------------------------------------------------------
// Engine loading
// ---------------------------------------------------------------------

/// Where the two `.rten` model files are: `SYNTHPASS_OCR_MODEL_DIR` if set
/// (the variable every other entry point in this crate honours), else
/// `repo_root`.
pub fn model_dir(repo_root: &Path) -> PathBuf {
    std::env::var_os("SYNTHPASS_OCR_MODEL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.to_path_buf())
}

pub fn load_model(path: &Path) -> Result<Model, String> {
    Model::load_file(path).map_err(|e| format!("failed to load model {}: {e}", path.display()))
}

pub fn load_general_engine(root: &Path) -> Result<OcrEngine, String> {
    let detection = load_model(&root.join("text-detection.rten"))?;
    let recognition = load_model(&root.join("text-recognition.rten"))?;
    OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection),
        recognition_model: Some(recognition),
        ..Default::default()
    })
    .map_err(|e| format!("failed to build general engine: {e}"))
}

pub fn load_mrz_engine(root: &Path) -> Result<OcrEngine, String> {
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

pub fn load_raw_recognition_model(root: &Path) -> Result<Model, String> {
    load_model(&root.join("text-recognition.rten"))
}

// ---------------------------------------------------------------------
// Matrix reading.
// ---------------------------------------------------------------------

pub fn axis_aligned_rect(top: f32, left: f32, bottom: f32, right: f32) -> RotatedRect {
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
pub fn run_recognition_batch(
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
pub fn averaged_distribution(
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

/// Maps an x-range in a crop's ORIGINAL pixel coordinates to a timestep
/// range in that crop's own CTC output, given its own resized width (from
/// `prepare_recognition_input`) and the fixed downsample factor. `crop_left`/
/// `crop_width` describe the crop's extent in the same original-pixel space
/// as `x_lo`/`x_hi`.
pub fn timestep_range(
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

#[cfg(test)]
mod ctc_alphabet_tests {
    use super::*;

    /// Pins the three labels the probes' items 2/3 depend on against an
    /// accidental edit of this copy — if this ever fails, every downstream
    /// number reads the wrong class index. It compares the copy with itself,
    /// so it does not detect an upstream `ocrs` alphabet change; see
    /// [`CTC_ALPHABET`]'s doc comment for what does.
    #[test]
    fn labels_match_the_verified_ocrs_alphabet() {
        assert_eq!(CTC_ALPHABET.chars().count(), 96, "97 classes = 96 + blank");
        assert_eq!(ctc_char(CTC_LABEL_FILLER), Some('<'));
        assert_eq!(ctc_char(CTC_LABEL_EUR_E), Some('E'));
        assert_eq!(ctc_char(CTC_LABEL_LETTER_E), Some('E'));
        assert_eq!(ctc_char(CTC_BLANK_LABEL), None);
    }
}
