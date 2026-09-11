//! In-process pure-Rust OCR for Tier 1 via `ocrs`/`rten` — the default engine
//! since v0.7.0 (it replaced the `docling-serve` Docker OCR service that
//! version), mirroring `synthpass-llm`'s `NativeLlm` naming/lifecycle pattern but
//! for text detection+recognition instead of generation.
//!
//! [`NativeOcr`] loads both `.rten` weight files once and is kept warm for
//! the process lifetime; `recognize` is blocking — callers on an async
//! runtime (see `synthpass-pipeline`) must run it via `spawn_blocking`, mirroring
//! how the native LLM inferer is wrapped.
//!
//! # Structured geometry (M5)
//!
//! [`NativeOcr::recognize_detailed`] is the richer sibling of `recognize`:
//! it returns an [`geometry::OcrPage`] with per-line text/bounding boxes, an
//! auto-detected page rotation, and two layout heuristics (`mrz_band`,
//! `portrait` — see [`geometry`]'s module docs). `recognize` is now defined
//! in terms of it (`recognize_detailed(..)?.text`); the retry-pass loop,
//! pass budget and time budget described below are unchanged and live
//! inside `recognize_detailed`, so `recognize`'s signature and returned text
//! are unaffected by this split. See `recognize_detailed`'s own doc comment
//! for exactly what's new versus what's verbatim.
//!
//! # MRZ retry passes
//!
//! A general full-page pass runs first. If its output does not contain a
//! checksum-valid MRZ (the `mrz` crate's ICAO 9303 check digits are a perfect
//! oracle for a faithful read — see knowledge/ARCHITECTURE.md §8), a second engine
//! constrained to the MRZ charset (`A–Z 0–9 <`, beam-search decoding) re-reads
//! preprocessed variants of the image ([`preprocess::mrz_variants`]: a
//! row-density-isolated MRZ-band crop, contrast-stretched/binarized/locally-
//! thresholded and deskewed, then the upscaled full page), appending any
//! MRZ-shaped lines it finds to the output. When the geometry pass (below)
//! found a confident `mrz_band`, [`preprocess::geometry_band_variants`]
//! contributes up to two more — a crop to that content-scored box rather
//! than a blind or row-density-searched one — chained on strictly as
//! trailing extras after `mrz_variants`'s own. The loop stops at the first
//! variant that validates. Retries are additive-only — the general pass's
//! text is never replaced — so Tier-2 input can only gain candidate lines,
//! and a checksum gate upstream decides what is trusted.
//!
//! # Pass budget
//!
//! `SYNTHPASS_OCR_MAX_PASSES` (default [`DEFAULT_MAX_PASSES`]) and
//! `SYNTHPASS_OCR_MAX_SECONDS` (default [`DEFAULT_MAX_SECONDS`]) bound the retry
//! loop so a hopeless document (dense/photographic scan with no recoverable
//! MRZ) aborts remaining variants instead of burning minutes — the general
//! pass's text is always returned regardless of where the budget cuts the
//! loop off.
//!
//! # Diagnostics
//!
//! Set `SYNTHPASS_OCR_VERBOSE=1` to log each pass's elapsed time and detected-
//! region count to stderr, and — on a Tier-1 miss — the MRZ-band candidate
//! lines each pass found. Off by default: it re-runs detection once per pass
//! purely for the region count, which `get_text` doesn't otherwise expose.
//!
//! Image-only: `ocrs` has no PDF parsing, and as of v0.7.5 there is no other
//! engine to route PDF input to — PDF is rejected outright at the
//! `synthpass-pipeline` layer (see `crates/synthpass-pipeline/src/ocr.rs`).

pub mod download;
#[cfg(feature = "embedded-models")]
pub mod embedded;
pub mod verify;

// Preprocessing and layout geometry now live in `synthpass-imageprep`, a
// pure-Rust crate that also compiles to wasm32 so the browser demo can run
// this exact code instead of a hand-written JavaScript port of it. They are
// re-exported at their original paths, so `synthpass_ocr::preprocess::…`,
// `synthpass_ocr::geometry::…` and `synthpass_ocr::BBox` are unchanged for
// every caller — the move is an implementation detail of this crate.
pub use synthpass_imageprep::{geometry, preprocess, sniff, BBox, OcrLine, OcrPage, UpscaleFilter};

/// The resampling filter this engine's retry variants upscale with.
///
/// `ocrs` measurably prefers the sharper filter: over the `mrz_corpus`
/// specimen set it reads 18/20 with [`UpscaleFilter::Lanczos3`] against 17/20
/// with `Triangle`. The browser demo, running tesseract's OCR-B model,
/// measurably prefers the opposite — see [`UpscaleFilter`] for the full table
/// and the mechanism. Stated explicitly here rather than defaulted, because a
/// silent default is exactly how the two engines' needs would get conflated
/// again.
const NATIVE_UPSCALE_FILTER: UpscaleFilter = UpscaleFilter::Lanczos3;

/// The full ICAO 9303 MRZ character set. Constraining recognition to it makes
/// the classic filler misreads (`<` read as `«`, `?`, or lowercase noise)
/// unrepresentable at the decoder level instead of repaired after the fact.
///
/// Re-exported rather than redefined: the browser demo needs the identical
/// string for its tesseract character whitelist, so it is single-sourced in
/// [`synthpass_imageprep::MRZ_CHARSET`].
pub use synthpass_imageprep::MRZ_CHARSET;

use image::metadata::Orientation;
use image::{ImageDecoder, RgbImage};
use ocrs::{DecodeMethod, ImageSource, OcrEngine as OcrsEngine, OcrEngineParams, TextItem};
use rten::Model;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Beam width for the constrained pass. Greedy decoding commits to one
/// reading; a modest beam lets the decoder recover when the top character is
/// wrong but the MRZ-charset-consistent alternative was a close second.
const MRZ_BEAM_WIDTH: u32 = 24;

/// Default `SYNTHPASS_OCR_MAX_PASSES` ceiling on total OCR passes per document
/// (the general pass plus retry variants) when the env var is unset or
/// invalid. `preprocess::mrz_variants` yields at most 7 variants (two
/// blind-crop, one full-page, four trailing isolated-band — the fourth being
/// the second deskew angle `preprocess::SkewMode::Default` appends when the two
/// estimators disagree about the band),
/// `preprocess::geometry_band_variants` appends at most 2 more (trailing
/// again, and only when the geometry-detected band differs from the blind
/// crop), two texture-suppression passes trail all of those when
/// `SYNTHPASS_OCR_TEXTURE` enables them, and two quarter-turn band crops trail
/// even those under [`RotateMode::Default`], so the worst case is
/// 7 + 2 + 2 + 2 = [`MAX_RETRY_VARIANTS`] retry variants plus the general
/// pass: **14**.
/// Note the retry loop seeds its counter at 1
/// for the *first* variant and breaks on `passes_run >= max_passes`, so the
/// number of variants that can actually run is `max_passes - 1` — an
/// off-by-one here silently truncates the last variant on exactly the
/// dense/bilingual scans the geometry-band variants exist for, which is the
/// failure this budget is sized to avoid. `max_passes_admits_every_variant`
/// pins the constant arithmetic and `max_passes_reaches_the_last_worst_case_variant`
/// proves it against a real worst-case fixture and the loop's actual break
/// condition, so a future variant — or another off-by-one — can't quietly
/// outgrow this budget again. The `SYNTHPASS_OCR_MAX_SECONDS` wall-clock
/// budget is what actually bounds a pathological document.
const DEFAULT_MAX_PASSES: usize = MAX_RETRY_VARIANTS + 1;

/// Worst-case number of retry variants: `preprocess::mrz_variants`'s 7, plus
/// `preprocess::geometry_band_variants`'s 2, plus the 2 trailing
/// texture-suppression passes ([`TextureMode::On`] emits `plain_band` *and*
/// `preprocess::texture_variants`, measured to recover different miss classes;
/// the [`TextureMode::Control`] placebo emits only the first, so `On` is the
/// worst case — see [`trailing_texture_mode`]), plus the 2 quarter-turn band
/// crops that trail everything under [`RotateMode::Default`] (see
/// [`ROTATION_RETRY_TURNS`]).
///
/// This is the ceiling across every env arm, not the count any single arm
/// produces: `RotateMode::Legacy` and `Off` emit 10, `preprocess::SkewMode::Legacy`
/// emits one fewer than `Default`, and the spare passes in those budgets simply go
/// unused — the loop runs out of variants before it runs out of budget. A ceiling
/// that covers the worst arm is the only value that cannot silently truncate one
/// of them.
///
/// Single-sourced with [`DEFAULT_MAX_PASSES`] above so the budget and the
/// variant count cannot drift apart silently — see that constant's doc comment.
const MAX_RETRY_VARIANTS: usize = 13;

/// Default `SYNTHPASS_OCR_MAX_SECONDS` wall-clock ceiling on the whole
/// `recognize` call when the env var is unset or invalid. Measured
/// (`examples/mrz_corpus.rs`, post-Phase-1 variants, reference hardware): a
/// full negative-control sweep (all retry variants run, none validate) costs
/// ~15-27s, and `Slovenian_ID_Specimen_2022_back_mrz.jpg` — whose checksum-valid
/// MRZ only assembles once enough variants have each contributed a line —
/// needs close to 30s of its own. 45s keeps clearance above that measured
/// worst case while still bounding the multi-minute-per-document blowup
/// that motivated this budget in the first place (see the
/// `Israel_Biometric_Passport.jpg` corpus entry, whose multi-minute cost was
/// dominated by Tier 2's LLM generation on garbage OCR text, not the OCR
/// passes themselves — this budget caps the OCR side of that problem).
///
/// **Raised 45 → 52 by ADR-0008 chunk 2** for the two quarter-turn variants
/// ([`ROTATION_RETRY_TURNS`]) now trailing the chain, at the ~2–3s per pass the
/// measured worst case costs. The increase is sized to the new passes and
/// nothing else: leaving it at 45 would have let the wall clock truncate
/// exactly the sideways documents the new passes exist to recover, which is the
/// silent-truncation failure [`DEFAULT_MAX_PASSES`]'s own doc comment describes
/// one budget over.
///
/// Only documents that fail every upright variant ever reach the new ceiling —
/// a document that validates earlier still finishes in the time it always did.
const DEFAULT_MAX_SECONDS: u64 = 52;

pub struct NativeOcr {
    /// General-purpose engine: full alphabet, greedy decode — produces the
    /// Markdown-ish page text Tier 2's prompt needs.
    engine: OcrsEngine,
    /// MRZ-targeted engine: recognition restricted to [`MRZ_CHARSET`] with
    /// beam-search decoding. Only consulted when the general pass fails the
    /// checksum oracle.
    mrz_engine: OcrsEngine,
}

fn build_general(detection: Model, recognition: Model) -> Result<OcrsEngine, String> {
    OcrsEngine::new(OcrEngineParams {
        detection_model: Some(detection),
        recognition_model: Some(recognition),
        ..Default::default()
    })
    .map_err(|e| format!("failed to build ocrs engine: {e}"))
}

fn build_mrz(detection: Model, recognition: Model) -> Result<OcrsEngine, String> {
    OcrsEngine::new(OcrEngineParams {
        detection_model: Some(detection),
        recognition_model: Some(recognition),
        allowed_chars: Some(MRZ_CHARSET.into()),
        decode_method: DecodeMethod::BeamSearch {
            width: MRZ_BEAM_WIDTH,
        },
        ..Default::default()
    })
    .map_err(|e| format!("failed to build mrz-constrained ocrs engine: {e}"))
}

impl NativeOcr {
    /// Load both `.rten` model files and build the warm, reusable engines.
    /// (`rten::Model` is not `Clone`, so each file is loaded twice — once per
    /// engine; ~12 MB each, a one-off cost at process start.)
    pub fn load(detection_path: &Path, recognition_path: &Path) -> Result<Self, String> {
        let load = |path: &Path, what: &str| {
            Model::load_file(path)
                .map_err(|e| format!("failed to load {what} model at {}: {e}", path.display()))
        };
        let engine = build_general(
            load(detection_path, "detection")?,
            load(recognition_path, "recognition")?,
        )?;
        let mrz_engine = build_mrz(
            load(detection_path, "detection")?,
            load(recognition_path, "recognition")?,
        )?;
        Ok(Self { engine, mrz_engine })
    }

    /// Build warm engines from the models baked into the binary at compile
    /// time (`embedded-models` feature) — no filesystem or network access,
    /// for a true single-file air-gapped deployment (see
    /// knowledge/ARCHITECTURE.md §10).
    #[cfg(feature = "embedded-models")]
    pub fn load_embedded() -> Result<Self, String> {
        let load = |bytes: &'static [u8], what: &str| {
            Model::load_static_slice(bytes)
                .map_err(|e| format!("failed to load embedded {what} model: {e}"))
        };
        let engine = build_general(
            load(embedded::DETECTION_BYTES, "detection")?,
            load(embedded::RECOGNITION_BYTES, "recognition")?,
        )?;
        let mrz_engine = build_mrz(
            load(embedded::DETECTION_BYTES, "detection")?,
            load(embedded::RECOGNITION_BYTES, "recognition")?,
        )?;
        Ok(Self { engine, mrz_engine })
    }

    /// Run OCR on the image at `image_path`, returning all recognized text as
    /// a single string — exactly what Tier 1's MRZ pattern search and the
    /// Tier-2 LLM prompt both need; no requirement for structured layout.
    ///
    /// If the general pass's text lacks a checksum-valid MRZ, the constrained
    /// retry passes run (see the module docs) and their MRZ-shaped lines are
    /// appended to the returned text.
    ///
    /// A thin wrapper over [`Self::recognize_detailed`] — this signature and
    /// its returned text are unchanged from before this crate gained a
    /// geometry API. See that method's doc comment for the "unchanged
    /// verbatim vs. new" boundary that makes this true.
    pub fn recognize(&self, image_path: &Path) -> Result<String, String> {
        Ok(self.recognize_detailed(image_path)?.text)
    }

    /// [`Self::recognize`]'s richer sibling: same recognized text, plus
    /// per-line detail, an auto-detected page rotation, and the `mrz_band`/
    /// `portrait` layout heuristics (see [`geometry`]'s module docs) —
    /// together, [`OcrPage`].
    ///
    /// **What's unchanged vs. what's new**, for anyone auditing that
    /// `recognize`'s behaviour didn't shift under this split: the pass
    /// budget / time budget / MRZ retry logic below is from before this
    /// method existed (still calling the original `run_pass`/`region_count`/
    /// `mrz_shaped_lines` helpers, and `preprocess::mrz_variants` itself is
    /// untouched), plus exactly one additive change to the retry loop's
    /// variant list (below). What's new is layered strictly around and
    /// after that core: orientation detection ([`choose_rotation`], A3) runs
    /// first and — being detection-only and conservatively biased toward "no
    /// rotation" (see its doc comment) — only ever changes the `image` this
    /// pipeline sees when it is confident the page is rotated. The
    /// structured line/word geometry ([`geometry_pass`]) is best-effort: its
    /// own detect+recognize pass never feeds back into `text`, and a failure
    /// in it degrades `OcrPage`'s structured fields to empty rather than
    /// failing this call. Its scored `mrz_band` output then feeds two more
    /// additive steps: the 0°/180° tie-break ([`should_flip_180`]) that
    /// `choose_rotation` can't resolve on its own, and — down in the retry
    /// loop — `preprocess::geometry_band_variants`, chained strictly as
    /// *trailing* extras after every existing `mrz_variants` entry. The
    /// variant chaining is gated on a `mrz_band` being found at all; absent
    /// one, the variant list is exactly `mrz_variants`'s own. The tie-break
    /// only ever *replaces* `image` when the flipped page scores strictly
    /// better by a margin, so an already-passing upright specimen's
    /// `image`/`text` stay byte-identical to their pre-M5 behaviour — and an
    /// upright band scoring above `geometry::MRZ_BAND_CONFIDENT_SCORE` skips
    /// the probe altogether. Everything through `choose_rotation` and the
    /// geometry passes runs *before* `overall_started` is set, so none of it
    /// eats into the retry loop's own wall-clock budget
    /// (`DEFAULT_MAX_SECONDS`) — it adds to this call's total latency, not
    /// to the retry loop's.
    pub fn recognize_detailed(&self, image_path: &Path) -> Result<OcrPage, String> {
        let verbose = verbose_enabled();
        // `None` on every normal run; `Some` only under the
        // `SYNTHPASS_OCR_DUMP_VARIANTS` cell-(c) diagnostic (see its doc).
        let dump_dir = dump_variants_dir();
        let image = decode_image(image_path)?.into_rgb8();

        // A3: auto-rotate before the main pass (detection-only, cheap; see
        // `choose_rotation`'s doc comment, including its known 0°-vs-180°
        // limitation). `None` means "keep the image as-is", which also
        // covers detection failure — orientation is a best-effort
        // enhancement, never a reason to fail the whole call.
        let (rotation, image) = match choose_rotation(&self.engine, &image) {
            Some((angle, rotated)) => {
                if verbose {
                    eprintln!("[synthpass-ocr] orientation: auto-rotated {angle}°");
                }
                (angle, rotated)
            }
            None => (0, image),
        };

        // A1/A2/A4: structured line/word geometry for this pass, best-effort
        // (see this method's doc comment on why this is a separate pass
        // rather than threaded through `run_pass`/`region_count` below).
        let (lines, word_boxes) = geometry_pass(&self.engine, &image).unwrap_or_default();
        let scored = geometry::detect_mrz_band_scored(&lines, MRZ_CHARSET);

        // A3 continued — the 0°/180° tie-break `choose_rotation` cannot
        // resolve on its own (see its doc comment: an upside-down horizontal
        // line measures identically wide-and-short as a right-side-up one).
        // Only *content* can settle direction, so the page is scored in both
        // orientations and the better-scoring one wins.
        //
        // The obvious cheaper rule — "the MRZ sits at the bottom on every
        // ICAO layout, so a confident band near the top means upside-down" —
        // was tried first and does not work, for a reason worth recording:
        // on a genuinely upside-down page the real MRZ is garbled, so it
        // scores *low*, and some unrelated mid-page noise routinely wins
        // `detect_mrz_band` instead. The band's location then describes the
        // noise, not the MRZ. On the 180°-rotated Croatian specimen the
        // winning band sat mid-page, the "upper third" test was false, and
        // the flip never fired. Asking which orientation scores better sides
        // steps that entirely: it never has to locate the MRZ on the page it
        // cannot read.
        //
        // Two guards, both measured over the 42-image `samples/` corpus (see
        // `geometry::MRZ_BAND_CONFIDENT_SCORE`):
        // - An already-confident upright band skips the probe, so the common
        //   case pays nothing. No upside-down page in the corpus scored that
        //   well, so a flip could not have won anyway.
        // - The flip must beat upright by [`BAND_FLIP_MARGIN`] rather than
        //   merely tie, which suppresses the corpus's one wrong-direction
        //   vote and every exact tie (where the two orientations produce the
        //   same score and there is genuinely nothing to choose between).
        let upright_score = scored.map_or(0.0, |(_, score)| score);
        let confident = upright_score >= geometry::MRZ_BAND_CONFIDENT_SCORE;
        // The winning `(bbox, score)` pair is carried intact and split apart
        // below. Previously the score was dropped here via `.map(|(b, _)| b)`
        // — it is the one number that says how MRZ-shaped the band actually
        // looked, which a routing decision needs and a bounding box cannot
        // answer.
        // `mut` because the retry chain's outermost tier can turn the page a
        // further quarter (see `ROTATION_RETRY_TURNS`), and a reported rotation
        // that does not describe the buffer the winning MRZ came off is a lie a
        // caller cannot detect.
        let (mut rotation, image, lines, word_boxes, band_scored) = if confident {
            (rotation, image, lines, word_boxes, scored)
        } else {
            let flipped = rotate_image(&image, 180);
            let (f_lines, f_word_boxes) = geometry_pass(&self.engine, &flipped).unwrap_or_default();
            let f_scored = geometry::detect_mrz_band_scored(&f_lines, MRZ_CHARSET);
            let flipped_score = f_scored.map_or(0.0, |(_, score)| score);

            if should_flip_180(upright_score, flipped_score) {
                if verbose {
                    eprintln!(
                        "[synthpass-ocr] orientation: MRZ band scores {flipped_score:.4} flipped \
                         vs {upright_score:.4} upright; flipping 180°"
                    );
                }
                (
                    (rotation + 180) % 360,
                    flipped,
                    f_lines,
                    f_word_boxes,
                    f_scored,
                )
            } else {
                (rotation, image, lines, word_boxes, scored)
            }
        };
        let (mrz_band, mrz_band_score) = match band_scored {
            Some((bbox, score)) => (Some(bbox), Some(score)),
            None => (None, None),
        };
        let portrait = geometry::detect_portrait(&word_boxes, image.width(), image.height());

        let overall_started = Instant::now();
        let general_started = Instant::now();
        if let Some(dir) = &dump_dir {
            dump_pass_image(dir, image_path, "general", &image, verbose);
        }
        let mut text = run_pass(&self.engine, &image)?;
        if verbose {
            let regions = region_count(&self.engine, &image).unwrap_or(0);
            eprintln!(
                "[synthpass-ocr] general pass: {:?} elapsed, {regions} region(s) detected",
                general_started.elapsed()
            );
        }
        if has_valid_mrz(&text) {
            let text_sanity = page_sanity(&text);
            return Ok(OcrPage {
                text,
                lines,
                mrz_band,
                mrz_band_score,
                portrait,
                rotation,
                text_sanity,
            });
        }
        if verbose {
            eprintln!("[synthpass-ocr] Tier-1 miss on general pass; MRZ-band candidate lines:");
            for line in mrz_shaped_lines(&text).lines() {
                eprintln!("[synthpass-ocr]   {line}");
            }
        }

        let max_passes = max_passes();
        let max_duration = max_duration();

        // `passes_run` counts total passes including the general one above
        // (seeded at 1) — a `zip` counter rather than a manually incremented
        // one so clippy's `explicit_counter_loop` stays clean; its value at
        // the top of each iteration is exactly "passes run so far".
        //
        // The geometry-detected band (A2's `mrz_band`, computed above) is
        // chained on strictly as *trailing* extras after every existing
        // `mrz_variants` entry — never modifying `mrz_variants` itself, so no
        // specimen that already validates on one of those can regress. The
        // loop below breaks on the first checksum-valid MRZ, so these are
        // only ever reached once every blind/isolated variant has already
        // failed. `geometry_band_variants` itself is the cheap part: it
        // returns nothing when there's no confident band, or when the band
        // it found is close enough to the blind crop to be a duplicate (see
        // its doc comment), so this costs nothing on the common case.
        let geometry_variants = mrz_band
            .map(|band| preprocess::geometry_band_variants(&image, band, NATIVE_UPSCALE_FILTER))
            .unwrap_or_default();
        // Texture suppression chains on *after* the geometry variants, making
        // it the last pass of all — the same trailing contract, one step
        // further out. It is built lazily so that a document which validates
        // on any earlier variant never pays even its construction cost, and it
        // is env-gated so the three measurement arms come out of one binary
        // (see `trailing_texture_mode`).
        let texture_mode = trailing_texture_mode();
        // `SYNTHPASS_OCR_ORDER` moves (never duplicates) the untreated
        // `plain_band` pass to the front of the chain under `band-first` —
        // see `OcrOrder`. `band_first_pass` and the texture stage below both
        // check `order`, so the pass appears exactly once regardless of
        // which arm is selected.
        let order = ocr_order();
        let band_first_pass = if order == OcrOrder::BandFirst && texture_mode != TextureMode::Off {
            vec![preprocess::plain_band(&image, NATIVE_UPSCALE_FILTER)]
        } else {
            Vec::new()
        };
        let texture_variants = std::iter::once_with(|| match texture_mode {
            TextureMode::Off => Vec::new(),
            // Two variants, not one, and the order is deliberate. The
            // 2026-09-04 real-specimen A/B found that these fix *different*
            // miss classes — `plain_band` recovers `no_mrz_found` (a detection
            // failure) while the median recovers `checksum_failed` (a
            // recognition failure) — and that running only one of them made
            // them compete for a single trailing slot, so each arm lost the
            // documents the other would have caught. `plain_band` goes first
            // because it is the cheaper of the two (a crop and an upscale, no
            // per-pixel window sort), and on a document that validates from it
            // the median is never built at all. Under `OcrOrder::BandFirst`
            // that first entry has already run, at the front of the whole
            // chain, so it is left out here rather than duplicated.
            TextureMode::On => {
                let mut variants = if order == OcrOrder::BandFirst {
                    Vec::new()
                } else {
                    vec![preprocess::plain_band(&image, NATIVE_UPSCALE_FILTER)]
                };
                variants.extend(preprocess::texture_variants(&image, NATIVE_UPSCALE_FILTER));
                variants
            }
            // Placebo. `plain_band` is the untreated band crop, and its own
            // doc comment records that it is deliberately kept out of
            // `mrz_variants` because `ocrs` normalizes internally and an
            // untreated variant "adds nothing there" — which is exactly the
            // property a control needs. It costs a real pass and appends real
            // text, while being the one band treatment already measured as
            // inert on this engine. Same `BandFirst` de-duplication as `On`.
            TextureMode::Control => {
                if order == OcrOrder::BandFirst {
                    Vec::new()
                } else {
                    vec![preprocess::plain_band(&image, NATIVE_UPSCALE_FILTER)]
                }
            }
        })
        .flatten();
        let mut mrz_variants_list =
            preprocess::mrz_variants_with(&image, NATIVE_UPSCALE_FILTER, skew_mode());
        // `OcrOrder::Control`'s reorder: swap the two blind-crop variants
        // (contrast-stretched, binarized) that neither `BandFirst` nor any
        // other hypothesis here expects to matter individually — see
        // `OcrOrder::Control`'s doc comment for why this arm exists.
        if order == OcrOrder::Control && mrz_variants_list.len() >= 2 {
            mrz_variants_list.swap(0, 1);
        }
        // The outermost trailing tier (ADR-0008 chunk 2, layer 1): the page
        // turned a quarter in each direction, band-cropped. This is the other
        // half of gating `choose_rotation` — the gate stops a low-signal page
        // from being turned on a guess, and these two passes recover the page
        // that genuinely *was* sideways, by trying both turns instead of voting
        // on which one to trust.
        //
        // Deliberately last, after `texture_variants`, mirroring `web/scan.js`
        // ("only after EVERY upright attempt failed") and the same additive
        // trailing contract every tier before it follows: the loop breaks on the
        // first checksum-valid MRZ, so a document that already validates
        // anywhere earlier never pays for these, and no specimen that passes
        // today can regress onto them. Built lazily for the same reason — a
        // document that never reaches them never allocates them.
        //
        // Absent under `Legacy`/`Off`, which is what makes those arms a clean
        // baseline rather than "the gate, minus the gate".
        let quarter_turn_variants = std::iter::once_with(|| {
            if rotate_mode() == RotateMode::Default {
                ROTATION_RETRY_TURNS
                    .iter()
                    .map(|&turn| {
                        (
                            turn,
                            preprocess::plain_band(
                                &rotate_image(&image, turn),
                                NATIVE_UPSCALE_FILTER,
                            ),
                        )
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        })
        .flatten();
        // Every variant carries the cardinal turn its pixels came off, so the
        // winning one can correct `rotation` below. Zero for all the upright
        // tiers, which is the honest answer rather than a default.
        let variants = band_first_pass
            .into_iter()
            .chain(mrz_variants_list)
            .chain(geometry_variants)
            .chain(texture_variants)
            .map(|variant| (0u16, variant))
            .chain(quarter_turn_variants)
            .enumerate();
        for (passes_run, (i, (turn, variant))) in (1usize..).zip(variants) {
            if passes_run >= max_passes {
                if verbose {
                    eprintln!(
                        "[synthpass-ocr] pass budget ({max_passes}) reached before variant {i}; stopping retries"
                    );
                }
                break;
            }
            if overall_started.elapsed() >= max_duration {
                if verbose {
                    eprintln!(
                        "[synthpass-ocr] time budget ({max_duration:?}) reached before variant {i}; stopping retries"
                    );
                }
                break;
            }

            let variant_started = Instant::now();
            if let Some(dir) = &dump_dir {
                dump_pass_image(
                    dir,
                    image_path,
                    // Matches the `variant {i}` index the verbose pass log
                    // prints, so a dumped crop and its stderr timing line
                    // line up.
                    &format!("variant{i:02}"),
                    &variant,
                    verbose,
                );
            }
            // A failed retry pass must never fail the whole OCR — the general
            // pass's text is already in hand and Tier 2 can still run on it.
            let Ok(pass_text) = run_pass(&self.mrz_engine, &variant) else {
                if verbose {
                    eprintln!("[synthpass-ocr] variant {i}: pass failed, skipping");
                }
                continue;
            };
            if verbose {
                let regions = region_count(&self.mrz_engine, &variant).unwrap_or(0);
                eprintln!(
                    "[synthpass-ocr] variant {i}: {:?} elapsed, {regions} region(s) detected",
                    variant_started.elapsed()
                );
            }
            let candidates = mrz_shaped_lines(&pass_text);
            if candidates.is_empty() {
                if verbose {
                    eprintln!("[synthpass-ocr] variant {i}: no MRZ-shaped lines");
                }
                continue;
            }
            text.push('\n');
            text.push_str(&candidates);
            // Check just this pass's lines: a valid MRZ appended means Tier 1
            // will find it — later (costlier) variants have nothing to add.
            if has_valid_mrz(&candidates) {
                // The reported rotation has to describe the buffer this MRZ was
                // actually read off, or `OcrPage.rotation` stops being usable
                // evidence for anything downstream.
                if turn != 0 {
                    rotation = (rotation + turn) % 360;
                }
                if verbose {
                    if turn == 0 {
                        eprintln!("[synthpass-ocr] variant {i}: valid MRZ found, stopping retries");
                    } else {
                        eprintln!(
                            "[synthpass-ocr] variant {i}: valid MRZ found on the {turn}°-turned \
                             page (reported rotation now {rotation}°), stopping retries"
                        );
                    }
                }
                break;
            } else if verbose {
                eprintln!("[synthpass-ocr] variant {i}: MRZ-shaped but checksum-invalid lines:");
                for line in candidates.lines() {
                    eprintln!("[synthpass-ocr]   {line}");
                }
            }
        }
        let text_sanity = page_sanity(&text);
        Ok(OcrPage {
            text,
            lines,
            mrz_band,
            mrz_band_score,
            portrait,
            rotation,
            text_sanity,
        })
    }
}

/// Opens an image by **what its bytes are**, not by what its name claims.
///
/// `image::open` picks a decoder from the file extension. A file whose
/// extension disagrees with its content therefore fails to decode even though
/// the format is fully supported — and the error names the format the *name*
/// implied, so it reads as a corrupt file rather than a mislabelled one.
///
/// This is not hypothetical. Three specimens in this repository's own corpus
/// were JPEGs carrying `.png` and `.webp` names; all three failed with
/// `Invalid PNG signature` and were silently skipped by every OCR walk in the
/// tree — the corpus manifest generator, both surveys, `mrz_corpus` and
/// `synthpass-bench` — until a magic-byte sweep found them. One of them,
/// `Spain_Passport_Specimen_P0_2022_mrz`, is the specimen
/// `crates/mrz/tests/line1_nonconformance.rs` pins as having no issuing state,
/// and it had never once been read by the engine that pins it.
///
/// Cameras, scanners, phone exports and download managers all mislabel files,
/// so a user handing this a `.png` that is really a JPEG is the ordinary case,
/// not the pathological one. `with_guessed_format` reads the magic bytes and
/// overrides the extension's guess, which is what the browser demo's
/// `createImageBitmap` has always done.
///
/// When decoding genuinely fails, the leading bytes are consulted so the
/// message names the real problem: a PDF or HEIC file renamed to `.jpg` gets
/// the same actionable explanation `synthpass-pipeline` gives when the
/// *extension* says so, instead of a decoder error about a format the file
/// never was.
///
/// **Public because the mistake was repo-wide, not engine-local.** Four call
/// sites reached for `image::open` independently — this engine,
/// `synthpass_bench::load_specimen` (which drops an undecodable specimen from
/// the benchmark *silently*, so a mislabelled file quietly shrinks the corpus),
/// and two survey examples. One decoder they can all share is the only way that
/// stays fixed.
///
/// **EXIF orientation is applied here** (ADR-0008 chunk 2, layer 1), which is
/// why this calls `into_decoder` + `from_decoder` rather than the one-line
/// `ImageReader::decode`: orientation lives on the *decoder*, and `decode()`
/// drops it on the floor. A phone photo of a passport is commonly stored
/// landscape with an orientation tag saying "rotate 90° to display" — every
/// viewer honours that tag, so the user sees an upright page, and before this
/// the pipeline saw the sideways buffer and had to guess its way back with
/// [`choose_rotation`]. Applying the tag is the one orientation correction that
/// is *stated by the file* rather than inferred from pixels, so it runs first
/// and unconditionally.
///
/// The transform is lossless for all eight EXIF states — whole-pixel transposes
/// and flips, no resampling — which is what keeps it compatible with the
/// "layers never compound each other's interpolation loss" contract.
///
/// Corpus yield is low by design: the `_rotated` specimens in `samples/` are
/// rotated *pixels* with no tag to read. This is correctness for real camera
/// input, not a benchmark move.
pub fn decode_image(path: &Path) -> Result<image::DynamicImage, String> {
    let reader = image::ImageReader::open(path)
        .map_err(|e| format!("failed to open image {}: {e}", path.display()))?
        .with_guessed_format()
        .map_err(|e| format!("failed to read image {}: {e}", path.display()))?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| decode_failure(path, &e))?;
    // Best-effort, like every other orientation step in this module: an
    // unreadable or malformed EXIF block means "no transform", never a failed
    // read. A file whose pixels decode is never rejected over its metadata.
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image =
        image::DynamicImage::from_decoder(decoder).map_err(|e| decode_failure(path, &e))?;
    image.apply_orientation(orientation);
    Ok(image)
}

/// The decode-failure message, with the magic-byte hint appended when the
/// leading bytes name a container this pipeline cannot read.
///
/// Factored out because [`decode_image`] now has two fallible decode steps
/// (build the decoder, then read the pixels) and a file that fails either one
/// deserves the same explanation — a renamed PDF should not be described as a
/// broken JPEG just because it failed at a different line.
fn decode_failure(path: &Path, e: &image::ImageError) -> String {
    let hint = match sniff_unsupported(path) {
        Some(f) => format!(
            " — the file is {f}, whatever its extension says. \
             Convert it to JPEG or PNG first."
        ),
        None => String::new(),
    };
    format!("failed to decode image {}: {e}{hint}", path.display())
}

/// Reads the leading bytes of `path` and names the container if
/// [`sniff::unsupported_format`] recognises it.
///
/// The table itself lives in `synthpass-imageprep` so the browser demo — which
/// has no filesystem, and reads the same bytes from a `Blob` — gives the same
/// answer from the same source. This wrapper is only the file read.
fn sniff_unsupported(path: &Path) -> Option<&'static str> {
    let mut head = [0u8; sniff::SNIFF_BYTES];
    let read = {
        use std::io::Read as _;
        let mut f = std::fs::File::open(path).ok()?;
        f.read(&mut head).ok()?
    };
    sniff::unsupported_format(&head[..read])
}

/// Whole-page [`geometry::text_sanity`], or `None` when nothing was
/// recognized.
///
/// Distinguishing "no text" from "text that scored 0.0" matters: the first is
/// an OCR failure and the second is a page full of unrecognizable glyphs, and
/// a caller deciding whether to escalate should not have to guess which it is
/// looking at.
fn page_sanity(text: &str) -> Option<f32> {
    (!text.trim().is_empty()).then(|| geometry::text_sanity(text))
}

/// Right-angle rotation candidates probed by [`choose_rotation`] (A3),
/// clockwise from the page as photographed/scanned. `0°` (no rotation) is
/// the baseline scored inline in `choose_rotation` rather than listed here.
const ROTATION_CANDIDATES: [u16; 3] = [90, 180, 270];

/// How much higher a non-zero rotation's line-geometry score must be than
/// 0°'s before [`choose_rotation`] commits to it. Documents are far more
/// often already upright than not, and guessing wrong feeds the whole
/// downstream pipeline a garbled page — so ties and close calls default to
/// "no rotation", which is also the choice that keeps `recognize()`'s
/// output byte-identical to its behaviour before this module gained
/// orientation detection (see `recognize_detailed`'s doc comment).
const ROTATION_MARGIN: f64 = 1.2;

/// How much better the 180°-flipped page's MRZ-band score must be than the
/// upright one's before `recognize_detailed` commits to the flip. Same
/// value and the same bias as [`ROTATION_MARGIN`] — ties and close calls
/// default to "leave it alone" — but a separate constant because it applies
/// to a different quantity (`geometry::detect_mrz_band_scored`'s band score,
/// not the line-aspect score) and would move independently.
///
/// **Measured.** Over the 42-image `samples/` corpus scored in both
/// orientations, this margin gets the direction right on 41 of 42 with zero
/// false flips. Every genuine correction clears it comfortably (the smallest
/// winning ratio is 1.27, most are 2–5×). It also suppresses the corpus's
/// only wrong-direction vote — `Serbia_Passport_Specimen_2012_mrz.jpg`
/// scores 0.4474 upside-down vs 0.3804 upright, a 1.18× lead that falls just
/// short of this bar — and the four exact ties, where both orientations
/// score identically and there is nothing to choose between them. Serbia
/// 2009 is therefore the one page in the corpus this cannot re-orient: an
/// honest miss, not a silent wrong answer.
const BAND_FLIP_MARGIN: f64 = 1.2;

/// The quarter-turns appended to the retry chain as its outermost trailing
/// tier under [`RotateMode::Default`] — see the chain construction in
/// [`NativeOcr::recognize_detailed`].
///
/// 180° is deliberately absent: `should_flip_180` already settles that axis
/// from page *content*, one layer up, and it does so with a measured margin on
/// 42 documents. Re-probing it here would spend two more passes re-deciding a
/// question that is already answered better.
const ROTATION_RETRY_TURNS: [u16; 2] = [90, 270];

/// A3 — cheap, detection-only page-orientation heuristic. Scores how
/// "line-shaped" the detected text is at each right-angle rotation: Latin
/// text reads in wide, short horizontal lines once `find_text_lines` groups
/// detected words together; turned 90°/270° sideways, the same words end up
/// as tall, narrow single-word "lines" instead of being grouped, so the mean
/// per-line width:height ratio drops sharply. No recognition model is
/// invoked — four `detect_words`+`find_text_lines` calls (0°, 90°, 180°,
/// 270°) cost roughly what one recognition pass does, honoring this step's
/// "keep it cheap" constraint.
///
/// Known limitation: the aspect-ratio signal is symmetric under a further
/// 180° turn (an upside-down horizontal line still measures wide-and-short),
/// so this reliably catches a sideways photo but cannot on its own tell a
/// right-side-up page from a fully upside-down one — disambiguating that
/// would need a recognition-confidence signal, which this deliberately does
/// not run (see [`ROTATION_MARGIN`]'s doc comment). This function does not
/// close that gap; `recognize_detailed` does, one layer up, by scoring the
/// content-and-geometry `mrz_band` in both orientations and keeping the
/// better (see [`should_flip_180`]) — a content-based tie-break, not a
/// generic recognition-confidence one, and only ever a correction *after*
/// this function has already run, never a change to what this function
/// itself returns.
///
/// Returns `None` (meaning "keep the image as-is") when no candidate beats
/// 0° by [`ROTATION_MARGIN`], including whenever `ocrs` detection itself
/// fails on any candidate — orientation is a best-effort enhancement, never
/// a reason to fail the whole recognition call.
///
/// **Retired from the default path by ADR-0008 chunk 2** — it runs only under
/// [`RotateMode::Legacy`] now, as the arm every new number is measured against.
///
/// Chunk 1 established that this probe is what turns eleven readable documents
/// into vertical crops that neither `ocrs` nor tesseract's OCR-B can read.
/// Chunk 2 then asked the obvious follow-up — can it be *gated* rather than
/// removed? — and measured sixteen documents to answer it. It cannot, and the
/// numbers are worth keeping because they refute two plausible fixes at once.
///
/// Ratio of the winning candidate's score to 0°'s, wrongly-rotated pages
/// against pages that genuinely are sideways:
///
/// | wrongly rotated | ratio | genuinely sideways | ratio |
/// |---|---|---|---|
/// | India 2024 | 1.23 | Pakistan 2024 *(not rotated today)* | 1.13 |
/// | Russia 2014 | 1.24 | Angola 2026 *(not rotated today)* | 1.18 |
/// | Finland 2007 | 1.28 | Ghana 2025 | 1.26 |
/// | Oman 2004 | 1.29 | Indonesia 2024 | 1.48 |
/// | Canada 2023 | 1.32 | | |
/// | Vietnam 2023 | 1.34 | | |
/// | Portugal 2017 | 1.42 | | |
/// | Kuwait 2023 | 1.48 | | |
/// | Argentina 2026 blur | 1.60 | | |
/// | Finland 2023 | 1.67 | | |
/// | Monaco back | 2.98 | | |
///
/// The ranges **overlap completely**, so no [`ROTATION_MARGIN`] separates them.
/// Angola and Pakistan are the sharpest illustration: both genuinely need a
/// 90° turn, and both miss today's 1.2 bar by a hair — while Monaco, which
/// needs no turn at all, clears it three times over.
///
/// The second refuted fix was a floor on *how much* was detected, on the theory
/// that these pages give the probe too little to work with. They do not:
/// detection finds 27–88 words on the wrongly-rotated pages and 192 on Angola.
/// What collapses on them is **recognition**, not detection — cell (b)'s 12–61
/// recognised characters per page measured the wrong stage. Worse, at the
/// *correct* orientation Angola detects 97 words against 192 wrong and Pakistan
/// 66 against 113: a horizontal-text detector **fragments** sideways text into
/// more boxes, so "more words" indicates *wrong*, not right, and a floor in
/// either direction is unsound.
///
/// So the vote is not gated, tuned or inverted — it is removed from the path
/// that matters, and the question it was guessing at is handed to evidence
/// instead: [`ROTATION_RETRY_TURNS`] tries both quarter-turns late in the retry
/// chain and an ICAO check digit decides which one was right. That is
/// `project_principles.md` P1 applied to orientation — deterministic proof
/// beating a heuristic that cannot be made reliable — and it is the same
/// conclusion `web/scan.js` reached: *"wrong often enough to cost 9 upright
/// documents."*
///
/// Removing it from [`RotateMode::Default`] also takes four `detect_words`
/// passes off every document, which is a latency saving on the whole corpus,
/// not only on the pages it was misreading.
fn choose_rotation(engine: &OcrsEngine, image: &RgbImage) -> Option<(u16, RgbImage)> {
    // `Default` and `Off` both decline to vote; only the baseline arm probes.
    if rotate_mode() != RotateMode::Legacy {
        return None;
    }
    let verbose = verbose_enabled();
    let zero = orientation_signal(engine, image).unwrap_or_default();
    if verbose {
        eprintln!(
            "[synthpass-ocr] orientation: 0° score {:.3}, {} word(s), area fraction {:.4}",
            zero.score, zero.words, zero.area_fraction
        );
    }
    let mut best: Option<(u16, RgbImage, f64)> = None;
    for &angle in &ROTATION_CANDIDATES {
        let candidate = rotate_image(image, angle);
        let Ok(signal) = orientation_signal(engine, &candidate) else {
            continue;
        };
        if verbose {
            eprintln!(
                "[synthpass-ocr] orientation: {angle}° score {:.3}, {} word(s), area fraction {:.4}",
                signal.score, signal.words, signal.area_fraction
            );
        }
        if signal.score <= zero.score * ROTATION_MARGIN {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|&(_, _, best_score)| signal.score > best_score)
        {
            best = Some((angle, candidate, signal.score));
        }
    }
    best.map(|(angle, rotated, _)| (angle, rotated))
}

/// The 0°/180° decision, factored out of `recognize_detailed` so it can be
/// pinned by a test against the measured corpus rather than only exercised
/// through a real-model run. `true` iff the flipped page's MRZ-band score
/// beats the upright one's by [`BAND_FLIP_MARGIN`].
///
/// Multiplying by the margin (rather than comparing a difference) also makes
/// a `0.0` upright score behave correctly on its own: any real flipped band
/// beats it, and two absent bands tie at zero and stay put.
fn should_flip_180(upright_score: f64, flipped_score: f64) -> bool {
    flipped_score > upright_score * BAND_FLIP_MARGIN && flipped_score > 0.0
}

/// What one detection pass says about a candidate rotation: how *line-shaped*
/// the text looks, and — added by ADR-0008 chunk 2 — how much text there was to
/// look at in the first place.
///
/// The strength half exists because of what the ADR-0008 chunk-2 sweep found,
/// and it is kept as the record of it: `score` is a **ratio**, so it is exactly
/// as confident about four boxes of scanner noise as about a full page of MRZ,
/// and the obvious fix — refuse to vote when detection found too little — was
/// measured and **does not work**. See [`choose_rotation`] for the table.
///
/// These two numbers are what that sweep was taken in, so they stay behind the
/// verbose log that produced them rather than being deleted along with the
/// hypothesis. Re-deriving them cost a release build and sixteen documents.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct OrientationSignal {
    /// Mean per-line width÷height over the grouped text lines. Wide, short
    /// lines (upright Latin text) score high; tall, narrow single-word "lines"
    /// (the same page turned sideways) score low.
    score: f64,
    /// How many words `detect_words` found.
    words: usize,
    /// Total detected word-box area as a fraction of the whole image area.
    /// Scale-free, so it means the same thing on a 300px scan and a 4000px
    /// photo — which a raw word count does not.
    area_fraction: f64,
}

/// Detection-only orientation signal for one candidate rotation — see
/// [`choose_rotation`]'s doc comment for what this measures and why, and
/// [`OrientationSignal`] for what the three numbers are.
fn orientation_signal(engine: &OcrsEngine, image: &RgbImage) -> Result<OrientationSignal, String> {
    let source = ImageSource::from_bytes(image.as_raw(), image.dimensions())
        .map_err(|e| format!("failed to prepare image source: {e}"))?;
    let input = engine
        .prepare_input(source)
        .map_err(|e| format!("failed to prepare ocr input: {e}"))?;
    let words = engine
        .detect_words(&input)
        .map_err(|e| format!("ocr word detection failed: {e}"))?;
    if words.is_empty() {
        return Ok(OrientationSignal::default());
    }
    // Measured off the detected boxes rather than the grouped lines: grouping is
    // itself orientation-sensitive (that is what `score` exploits), so a
    // strength taken after grouping would collapse on exactly the sideways pages
    // this is meant to distinguish from empty ones.
    let word_area: f64 = words
        .iter()
        .map(|w| f64::from(w.width()) * f64::from(w.height()))
        .sum();
    let image_area = f64::from(image.width()) * f64::from(image.height());
    let area_fraction = if image_area > 0.0 {
        word_area / image_area
    } else {
        0.0
    };
    let words_found = words.len();

    let lines = engine.find_text_lines(&input, &words);
    if lines.is_empty() {
        return Ok(OrientationSignal {
            score: 0.0,
            words: words_found,
            area_fraction,
        });
    }
    let mut total = 0.0;
    for line_words in &lines {
        if line_words.is_empty() {
            continue;
        }
        let width_sum: f64 = line_words.iter().map(|w| f64::from(w.width())).sum();
        let height_mean: f64 = line_words
            .iter()
            .map(|w| f64::from(w.height()))
            .sum::<f64>()
            / line_words.len() as f64;
        if height_mean > 0.0 {
            total += width_sum / height_mean;
        }
    }
    Ok(OrientationSignal {
        score: total / lines.len() as f64,
        words: words_found,
        area_fraction,
    })
}

/// Rotate `image` clockwise by `angle` degrees (must be 0/90/180/270 — any
/// other value is treated as a no-op copy). Lossless pixel rotation via the
/// `image` crate (no new dependency): exact for right angles, unlike
/// `preprocess::deskew`'s bilinear arbitrary-angle rotation for small-tilt
/// correction.
fn rotate_image(image: &RgbImage, angle: u16) -> RgbImage {
    match angle {
        90 => image::imageops::rotate90(image),
        180 => image::imageops::rotate180(image),
        270 => image::imageops::rotate270(image),
        _ => image.clone(),
    }
}

/// One detect+group+recognize pass over `image`, used only to surface
/// structured line/word geometry for [`OcrPage`] (A1/A2/A4) — run
/// separately from (and in addition to) [`run_pass`]/[`region_count`] below
/// rather than threaded through them, so the general pass's `text` output
/// and error messages stay byte-for-byte what `recognize` has always
/// produced (see `recognize_detailed`'s doc comment). The cost is one extra
/// detect+recognize pass on the general image; correctness of `recognize`'s
/// output was judged worth more than avoiding it.
///
/// Best-effort: a failure here (mapped to `Err` and discarded by the caller
/// via `.unwrap_or_default()`) degrades `OcrPage`'s structured fields to
/// empty, never the returned `text`.
fn geometry_pass(
    engine: &OcrsEngine,
    image: &RgbImage,
) -> Result<(Vec<OcrLine>, Vec<BBox>), String> {
    let source = ImageSource::from_bytes(image.as_raw(), image.dimensions())
        .map_err(|e| format!("failed to prepare image source: {e}"))?;
    let input = engine
        .prepare_input(source)
        .map_err(|e| format!("failed to prepare ocr input: {e}"))?;
    let words = engine
        .detect_words(&input)
        .map_err(|e| format!("ocr word detection failed: {e}"))?;
    let word_boxes: Vec<BBox> = words
        .iter()
        .map(|w| geometry::bbox_from_points(w.corners().map(|c| (c.x, c.y))))
        .collect();
    let line_groups = engine.find_text_lines(&input, &words);
    let recognized = engine
        .recognize_text(&input, &line_groups)
        .map_err(|e| format!("ocr text extraction failed: {e}"))?;
    let lines = recognized
        .into_iter()
        .flatten()
        .map(|line| {
            let r = line.bounding_rect();
            let bbox = BBox::from_tlbr(
                r.top() as f32,
                r.left() as f32,
                r.bottom() as f32,
                r.right() as f32,
            );
            let text = line.to_string();
            let confidence = geometry::text_sanity(&text);
            OcrLine {
                text,
                bbox,
                confidence,
            }
        })
        .collect();
    Ok((lines, word_boxes))
}

/// Run one detection+recognition pass over an in-memory image.
fn run_pass(engine: &OcrsEngine, image: &RgbImage) -> Result<String, String> {
    let source = ImageSource::from_bytes(image.as_raw(), image.dimensions())
        .map_err(|e| format!("failed to prepare image source: {e}"))?;
    let input = engine
        .prepare_input(source)
        .map_err(|e| format!("failed to prepare ocr input: {e}"))?;
    engine
        .get_text(&input)
        .map_err(|e| format!("ocr text extraction failed: {e}"))
}

/// Does this text already contain a checksum-valid MRZ? The oracle that
/// decides whether the retry passes are worth their CPU.
///
/// Deliberately checksum-only, despite line 1 (`document_type`/
/// `issuing_country`/`surname`/`given_names`) carrying no check digit at all
/// — a stricter oracle here (require a line-1 plausibility heuristic too,
/// not just checksum validity, before stopping retries) was measured and
/// reverted: it makes the retry loop run more MRZ-constrained passes per
/// document, appending more candidate lines for `find_and_parse` to search,
/// which measurably hurt accuracy rather than helping. See
/// `synthpass_core::fusion`'s module doc comment for the full writeup and
/// numbers.
fn has_valid_mrz(text: &str) -> bool {
    mrz::find_and_parse(text).is_ok_and(|d| d.valid())
}

/// Is `SYNTHPASS_OCR_VERBOSE=1` set? Gates the per-pass diagnostic logging
/// described in the module docs.
fn verbose_enabled() -> bool {
    std::env::var("SYNTHPASS_OCR_VERBOSE").as_deref() == Ok("1")
}

/// `SYNTHPASS_OCR_MAX_PASSES`, or [`DEFAULT_MAX_PASSES`] if unset/invalid/zero.
fn max_passes() -> usize {
    std::env::var("SYNTHPASS_OCR_MAX_PASSES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_MAX_PASSES)
}

/// Which trailing texture-suppression pass, if any, to append after every
/// other retry variant. See [`trailing_texture_mode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextureMode {
    /// No trailing pass at all — behaviour identical to before the stage
    /// existed. The measurement baseline.
    Off,
    /// The real treatment: `preprocess::plain_band` followed by
    /// [`preprocess::texture_variants`]. Two passes, because the 2026-09-04
    /// real-specimen A/B measured them recovering disjoint miss classes.
    On,
    /// A **placebo**: one extra trailing pass carrying pass-budget and
    /// text-accumulation pressure with no new pixel maths —
    /// `preprocess::plain_band`, the one band treatment already measured as
    /// inert on `ocrs`.
    ///
    /// Note this is no longer a *pass-count* match for [`On`](Self::On), which
    /// now emits two. That is deliberate: `On`'s first variant is exactly this
    /// one, so `control` is the arm that isolates what the median filter adds
    /// on top of `plain_band` — the question the next measurement is asking.
    Control,
}

/// `SYNTHPASS_OCR_TEXTURE` — `on`, `off` or `control`, defaulting to
/// [`TextureMode::On`] since the 2026-09-04 two-variant measurement (114/229
/// vs 111/229 with zero regressions; see
/// `knowledge/benchmarks/texture-suppression-ab-2026-09-03.md`). It shipped
/// default-`off` while it was unmeasured, and the default flipped once, after
/// the final shape of the stage was decided — not twice.
///
/// This exists so the stage can be A/B-measured from **one binary**. A
/// rebuild-based before/after has already hidden a real hit-rate regression on
/// this project once; toggling in-process removes every difference between the
/// arms except the one under test.
///
/// The `control` arm is the non-obvious half and is not optional. Appending
/// *any* ninth pass changes two things besides the pixels: it consumes pass and
/// wall-clock budget that a slow document might have been relying on, and it
/// appends more text to the accumulated page that downstream parsing sees. The
/// placebo arm exposes both, so a delta between `on` and `off` can be
/// attributed to the median filter rather than to the mere existence of another
/// pass. The decision rule is `on > control` **and** `control >= off`; if
/// `control` sits below `off`, the budget was mis-sized and that must be fixed
/// before the treatment can be judged at all.
///
/// Unrecognised values fall back to the default rather than erroring: this is a
/// measurement knob, and a typo should not take down a production read.
fn trailing_texture_mode() -> TextureMode {
    match std::env::var("SYNTHPASS_OCR_TEXTURE")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "off" => TextureMode::Off,
        "control" => TextureMode::Control,
        _ => TextureMode::On,
    }
}

/// Where the untreated `preprocess::plain_band` pass sits in the retry
/// chain. See [`ocr_order`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OcrOrder {
    /// Today's order, unchanged: `mrz_variants` first, then
    /// `geometry_band_variants`, then the texture stage — `plain_band` sits
    /// wherever [`TextureMode`] already puts it (last, or absent under
    /// `TextureMode::Off`). The measurement baseline.
    Default,
    /// `plain_band` is *moved*, not duplicated, to the very first retry
    /// pass — ahead of `mrz_variants` — mirroring `web/scan.js`'s own first
    /// attempt (untreated band crop, wins ~88% of the browser's reads on
    /// its own). It is removed from its default trailing position so the
    /// total pass count is unchanged; `DEFAULT_MAX_PASSES` is proven (see
    /// its doc comment) to reach every variant regardless of order, so this
    /// is a pure reordering with no budget confound. Under
    /// `TextureMode::Off` there is no `plain_band` pass to move, so this
    /// arm is equivalent to [`Default`](Self::Default) in that
    /// configuration.
    BandFirst,
    /// A **control** for [`BandFirst`](Self::BandFirst): swaps the first
    /// two `mrz_variants` entries (contrast-stretched and binarized blind
    /// band), neither of which any hypothesis here expects to matter more
    /// than the other. A `BandFirst` delta is only trusted if `Control`
    /// tracks [`Default`](Self::Default) document-for-document — otherwise
    /// the delta is explained by "the harness is sensitive to reordering
    /// passes at all," not by the untreated band crop specifically.
    Control,
}

/// `SYNTHPASS_OCR_ORDER` — `default`, `band-first` or `control`, defaulting
/// to [`OcrOrder::Default`] since this axis is unmeasured as of its
/// introduction. See [`OcrOrder`] for what each arm does and
/// `knowledge/benchmarks/ocr-stack-gap-2026-09-09.md` for why it exists:
/// native already runs `plain_band` (see `TextureMode::On`), just as the
/// second-to-last pass rather than the browser's first.
///
/// Unrecognised values fall back to the default, matching
/// [`trailing_texture_mode`]'s reasoning: this is a measurement knob, and a
/// typo should not take down a production read.
fn ocr_order() -> OcrOrder {
    match std::env::var("SYNTHPASS_OCR_ORDER")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "band-first" => OcrOrder::BandFirst,
        "control" => OcrOrder::Control,
        _ => OcrOrder::Default,
    }
}

/// How page orientation is handled. See [`rotate_mode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RotateMode {
    /// ADR-0008 chunk 2, layer 1: **no upfront orientation vote at all**, and
    /// two quarter-turn band crops trailing the retry chain so an ICAO check
    /// digit settles the question the vote was guessing at. See
    /// [`choose_rotation`] for the measurement that retired the vote.
    ///
    /// **This moves orientation recovery into the retry chain, which is a real
    /// cost and not merely a refactor.** The retired probe ran *before* the
    /// general pass, so it re-oriented a sideways page even with retries
    /// switched off entirely. Recovery now lives in the chain's outermost tier,
    /// so a tightened [`DEFAULT_MAX_PASSES`] override or an exhausted
    /// [`DEFAULT_MAX_SECONDS`] budget loses sideways pages. They degrade to
    /// "no MRZ found" rather than to a confident wrong answer, which is the
    /// right direction to fail — but a caller trimming either budget for
    /// latency is making that trade, and should know it.
    ///
    /// Found the honest way, by CI rather than by reasoning: `ci.yml` runs the
    /// OCR smoke tests with `SYNTHPASS_OCR_MAX_PASSES=1` to assert the stage
    /// executes at all, and that configuration cannot reach these variants.
    Default,
    /// The behaviour before chunk 2: [`choose_rotation`] probes all four
    /// right angles and commits to any [`ROTATION_MARGIN`] win, with no
    /// quarter-turn retry variants behind it. **The A/B baseline** — the arm
    /// every `default` number is compared against, which is why it stays in the
    /// binary rather than in git history.
    Legacy,
    /// No upfront probe and no quarter-turn retries. Isolates how much of the
    /// measured delta is "rotating helps" from "rotating *upfront* helps" — if
    /// `off` matches `default`, the gate could be stricter still and the probe
    /// is carrying nothing.
    ///
    /// **Not "no rotation anywhere."** `recognize_detailed`'s 0°/180° content
    /// tie-break ([`should_flip_180`]) still runs under every arm: it is a
    /// separate mechanism, measured separately on 42 documents against its own
    /// margin, and predates this chunk. An A/B that reads `off` as "zero
    /// rotation in the pipeline" would mis-attribute every 180° recovery to
    /// this enum.
    Off,
}

/// `SYNTHPASS_OCR_ROTATE` — `default`, `legacy` or `off`, defaulting to
/// [`RotateMode::Default`]. See [`RotateMode`] for what each arm does and
/// `knowledge/decisions/ADR-0008-mrz-detection-track.md` for the measurement
/// that motivated the gate.
///
/// Unrecognised values fall back to the default, matching
/// [`trailing_texture_mode`] and [`ocr_order`]: this is a measurement knob, and
/// a typo should not take down a production read. Note the direction that
/// implies — a mistyped `legacy` measures `default`, so an A/B reading "no
/// difference at all" should suspect its spelling before its hypothesis.
fn rotate_mode() -> RotateMode {
    match std::env::var("SYNTHPASS_OCR_ROTATE")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "legacy" => RotateMode::Legacy,
        "off" => RotateMode::Off,
        _ => RotateMode::Default,
    }
}

/// `SYNTHPASS_OCR_SKEW` — `default` or `legacy`, defaulting to
/// [`preprocess::SkewMode::Default`] (ADR-0008 chunk 2, layer 2).
///
/// `default` estimates the skew angle in one pass over the probe's dark pixels
/// and resamples at most once; `legacy` is the pre-chunk-2 search that rotated
/// the probe once per candidate angle. The arm lives here, and not in
/// `synthpass-imageprep`, because that crate has no `std::env` reads at all —
/// a property that keeps it deterministic and `wasm32`-clean so the browser
/// demo runs the same code. An env toggle there would always read the default
/// under `wasm32` and let the two paths diverge without anyone noticing.
///
/// Unrecognised values fall back to the default, matching every other knob in
/// this module.
fn skew_mode() -> preprocess::SkewMode {
    match std::env::var("SYNTHPASS_OCR_SKEW")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "legacy" => preprocess::SkewMode::Legacy,
        _ => preprocess::SkewMode::Default,
    }
}

/// `SYNTHPASS_OCR_DUMP_VARIANTS` — when set to a non-empty path, every
/// preprocessed image [`NativeOcr::recognize_detailed`] hands the recognizer
/// (the general full-page pass and each retry variant that actually runs) is
/// written there as a PNG, named `<source-stem>__<label>.png`.
///
/// A diagnostic hook for ADR-0008 chunk 1C cell (c): cell (b) established that
/// on the ~11 documents driving the browser/native gap, `ocrs` returns near-
/// empty text for the whole page. This dump lets a *different* recognizer be
/// run offline over the exact bytes `ocrs` saw, to tell "native's crop is
/// unusable" apart from "native's recognizer is the weak link". See
/// `knowledge/benchmarks/ocr-crop-recognizer-2026-09-10.md`.
///
/// Unset — the default, and every normal run — it is a no-op: the OCR path is
/// byte-identical, since the only new work is behind `Option::is_some`. Writes
/// are best-effort; a failure logs under `SYNTHPASS_OCR_VERBOSE` and never
/// fails the OCR call.
fn dump_variants_dir() -> Option<PathBuf> {
    std::env::var("SYNTHPASS_OCR_DUMP_VARIANTS")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

/// Write one preprocessed pass image for the [`dump_variants_dir`] diagnostic.
/// Best-effort: any error is logged (verbose only) and swallowed, so a full
/// disk or a bad path can never turn a dump run into a failed OCR run.
fn dump_pass_image(dir: &Path, source: &Path, label: &str, image: &RgbImage, verbose: bool) {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let path = dir.join(format!("{stem}__{label}.png"));
    let result = std::fs::create_dir_all(dir)
        .map_err(|e| e.to_string())
        .and_then(|()| image.save(&path).map_err(|e| e.to_string()));
    match result {
        Ok(()) if verbose => eprintln!("[synthpass-ocr] dump-variants: wrote {}", path.display()),
        Err(e) if verbose => {
            eprintln!(
                "[synthpass-ocr] dump-variants: {} failed: {e}",
                path.display()
            )
        }
        _ => {}
    }
}

/// `SYNTHPASS_OCR_MAX_SECONDS`, or [`DEFAULT_MAX_SECONDS`] if unset/invalid/zero.
fn max_duration() -> Duration {
    std::env::var("SYNTHPASS_OCR_MAX_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&n| n > 0)
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(DEFAULT_MAX_SECONDS))
}

/// Detected text-region count for one pass, for `SYNTHPASS_OCR_VERBOSE` logging
/// only — `get_text` already runs detection internally but doesn't expose
/// the count, so this re-runs it; callers must only invoke this when
/// verbose logging is on.
fn region_count(engine: &OcrsEngine, image: &RgbImage) -> Result<usize, String> {
    let source = ImageSource::from_bytes(image.as_raw(), image.dimensions())
        .map_err(|e| format!("failed to prepare image source: {e}"))?;
    let input = engine
        .prepare_input(source)
        .map_err(|e| format!("failed to prepare ocr input: {e}"))?;
    let words = engine
        .detect_words(&input)
        .map_err(|e| format!("ocr word detection failed: {e}"))?;
    Ok(words.len())
}

/// Keep only lines long enough to be MRZ material (a TD1 line is 30 chars;
/// 20 tolerates truncation, matching `mrz::find_and_parse`'s own token
/// threshold). The constrained pass reads the whole crop, so this drops the
/// non-MRZ text it garbles (it can only emit `A–Z 0–9 <`) instead of feeding
/// that noise to Tier 2's prompt.
fn mrz_shaped_lines(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|l| l.chars().filter(|c| !c.is_whitespace()).count() >= 20)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod decode_tests {
    use super::*;
    use image::{ImageFormat, RgbImage};

    /// Writes `img` encoded as `format`, to a path whose extension deliberately
    /// says something else.
    fn write_mislabelled(dir: &Path, name: &str, format: ImageFormat) -> std::path::PathBuf {
        let path = dir.join(name);
        let img = RgbImage::from_pixel(8, 8, image::Rgb([200, 40, 90]));
        image::DynamicImage::ImageRgb8(img)
            .save_with_format(&path, format)
            .expect("encodes");
        path
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("synthpass-decode-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// The regression this function exists for: a JPEG named `.png` must decode.
    ///
    /// `image::open` fails here with `Invalid PNG signature`, which is how three
    /// corpus specimens stayed invisible to every OCR walk in the repo.
    #[test]
    fn decodes_a_jpeg_that_claims_to_be_a_png() {
        let dir = scratch("jpeg-as-png");
        let path = write_mislabelled(&dir, "actually_a_jpeg.png", ImageFormat::Jpeg);

        assert!(
            image::open(&path).is_err(),
            "precondition: extension-based opening must fail, or this test proves nothing"
        );
        let decoded = decode_image(&path).expect("content sniffing decodes it anyway");
        assert_eq!(decoded.width(), 8);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The mirror case, so the fix isn't accidentally JPEG-specific.
    #[test]
    fn decodes_a_png_that_claims_to_be_a_webp() {
        let dir = scratch("png-as-webp");
        let path = write_mislabelled(&dir, "actually_a_png.webp", ImageFormat::Png);

        assert!(image::open(&path).is_err(), "precondition");
        assert!(decode_image(&path).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A correctly-named file must behave exactly as before — sniffing is a
    /// fallback for liars, not a change of behaviour for everyone else.
    #[test]
    fn an_honestly_named_file_still_decodes() {
        let dir = scratch("honest");
        let path = write_mislabelled(&dir, "honest.png", ImageFormat::Png);

        assert!(image::open(&path).is_ok());
        assert!(decode_image(&path).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A PDF renamed to `.jpg` gets the PDF message, not a JPEG decoder error.
    /// The pipeline's extension-based gate cannot catch this one.
    #[test]
    fn a_renamed_pdf_is_named_in_the_error() {
        let dir = scratch("pdf");
        let path = dir.join("not_really.jpg");
        std::fs::write(&path, b"%PDF-1.7\n1 0 obj\n<<>>\n").expect("write");

        let err = decode_image(&path).expect_err("a PDF is not a decodable image");
        assert!(err.contains("PDF"), "message should name PDF: {err}");
        assert!(
            err.contains("Convert"),
            "message should say what to do: {err}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Likewise HEIC — the iPhone default, and the format users are most likely
    /// to hand over with a wrong extension.
    #[test]
    fn a_renamed_heic_is_named_in_the_error() {
        let dir = scratch("heic");
        let path = dir.join("photo.jpg");
        let mut bytes = vec![0u8, 0, 0, 0x18];
        bytes.extend_from_slice(b"ftypheic");
        bytes.extend_from_slice(b"\0\0\0\0mif1heic");
        std::fs::write(&path, &bytes).expect("write");

        let err = decode_image(&path).expect_err("HEIC is not decodable here");
        assert!(err.contains("HEIC"), "message should name HEIC: {err}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Garbage stays a plain decode error — `sniff_unsupported` must not invent
    /// a diagnosis it cannot support.
    #[test]
    fn unrecognized_bytes_get_no_invented_explanation() {
        let dir = scratch("garbage");
        let path = dir.join("junk.png");
        std::fs::write(&path, b"not an image at all, just some bytes").expect("write");

        let err = decode_image(&path).expect_err("garbage does not decode");
        assert!(!err.contains("PDF"));
        assert!(!err.contains("HEIC"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `None` (nothing was recognized) and `Some(0.0)` (something was
    /// recognized and it was garbage) are different facts, and a caller
    /// deciding whether to escalate acts differently on each: the first is an
    /// OCR failure, the second is a page of unreadable glyphs. Collapsing
    /// them into `0.0` would make an escalation policy unable to tell a blank
    /// scan from a ruined one.
    #[test]
    fn page_sanity_separates_no_text_from_unreadable_text() {
        assert_eq!(page_sanity(""), None);
        assert_eq!(page_sanity("   \n\t "), None, "whitespace is not text");

        let noise = page_sanity("#@%^&*()!!").expect("symbols are still recognized text");
        let clean = page_sanity("P<UTOERIKSSON<<ANNA<MARIA").expect("clean text scores");
        assert!(
            clean > noise,
            "clean MRZ text ({clean}) should outscore symbol noise ({noise})"
        );
        assert!((0.0..=1.0).contains(&noise) && (0.0..=1.0).contains(&clean));
    }

    /// Pins the 0°/180° decision against the real numbers it was derived
    /// from — the `samples/` sweep described on [`BAND_FLIP_MARGIN`] — so a
    /// future tweak to either constant has to face the corpus rather than
    /// just recompile.
    #[test]
    fn flip_decision_matches_the_measured_corpus() {
        // Genuine corrections: the page as given is upside-down, flipping it
        // recovers the real MRZ. (fixture-as-given, flipped) band scores.
        for (upright, flipped, name) in [
            (0.1765, 0.7318, "Croatia_Passport_Specimen_2009_mrz"),
            (0.0301, 0.9216, "Canada_Passport_Specimen_2023_mrz"),
            (0.7132, 0.9080, "North_Macedonia_Passport_Specimen"), // narrowest real win, 1.27x
            (0.3608, 0.6149, "Kosovo_Passport_Specimen_2"),
            (0.0000, 0.6105, "Croatia_Border_Pass_Specimen_Back"),
        ] {
            assert!(
                should_flip_180(upright, flipped),
                "{name}: {flipped} vs {upright} should flip"
            );
        }

        // Must not flip: the corpus's one wrong-direction vote (a 1.18x lead,
        // just under the margin) and an exact tie, where both orientations
        // score the same and there is nothing to choose between them.
        assert!(
            !should_flip_180(0.3804, 0.4474),
            "Serbia_Passport_Specimen_2012_mrz: a 1.18x lead is below the margin"
        );
        assert!(
            !should_flip_180(0.3781, 0.3781),
            "Kazakhstan_Passport_Specimen: an exact tie must not flip"
        );

        // A page with no band either way carries no evidence at all.
        assert!(
            !should_flip_180(0.0, 0.0),
            "no band either way must not flip"
        );
    }

    #[test]
    fn mrz_shaped_lines_keeps_mrz_and_drops_noise() {
        let text = "REPUBLIKA\nP<HRVSPECIMEN<<SPECIMEN<<<<<<<<<<<<<<<<<<<<<\nZAGREB\n\
                    0070070071HRV8212258F1407019<<<<<<<<<<<<<<06\nOK";
        let kept = mrz_shaped_lines(text);
        assert_eq!(kept.lines().count(), 2);
        assert!(kept.lines().all(|l| l.len() >= 20));
    }

    #[test]
    fn has_valid_mrz_accepts_specimen_and_rejects_prose() {
        let valid = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n\
                     L898902C36UTO7408122F1204159ZE184226B<<<<<10";
        assert!(has_valid_mrz(valid));
        assert!(!has_valid_mrz("just some regular text\nwith two lines"));
    }

    // Env-var tests mutate process-global state (unavoidable for `std::env`
    // config in Rust 2024, hence the `unsafe` blocks — mirrors the existing
    // pattern in `verify.rs`). `cargo test` runs this binary's tests in
    // parallel threads by default, so each var gets exactly one test
    // (covering every case serially within it) rather than being spread
    // across multiple tests, which would race on the shared process env.
    #[test]
    fn max_passes_reads_env_with_fallback() {
        unsafe { std::env::remove_var("SYNTHPASS_OCR_MAX_PASSES") };
        assert_eq!(max_passes(), DEFAULT_MAX_PASSES, "unset falls back");

        unsafe { std::env::set_var("SYNTHPASS_OCR_MAX_PASSES", "2") };
        assert_eq!(max_passes(), 2, "valid override honored");

        unsafe { std::env::set_var("SYNTHPASS_OCR_MAX_PASSES", "not-a-number") };
        assert_eq!(max_passes(), DEFAULT_MAX_PASSES, "invalid falls back");

        unsafe { std::env::set_var("SYNTHPASS_OCR_MAX_PASSES", "0") };
        assert_eq!(max_passes(), DEFAULT_MAX_PASSES, "zero falls back");

        unsafe { std::env::remove_var("SYNTHPASS_OCR_MAX_PASSES") };
    }

    /// The default pass budget must admit *every* retry variant, including
    /// the last one. This is arithmetic, not behaviour, and it is a test
    /// rather than a comment because a comment is exactly what failed here:
    /// the budget was first raised to 8 alongside a doc comment asserting
    /// "6 + 2 more, plus the general pass" — which is 9. The truncation is
    /// silent (the loop just stops early) and lands only on documents
    /// producing the full variant set, i.e. the dense/bilingual scans the
    /// geometry-band variants were added for, so it would have shown up as
    /// the feature looking weaker than it is rather than as a failure.
    #[test]
    fn max_passes_admits_every_variant() {
        // The loop seeds `passes_run` at 1 for the *first* variant and breaks
        // on `passes_run >= max_passes`, so the count it actually admits is
        // one less than the budget. See the retry loop in `recognize_detailed`.
        let admitted = DEFAULT_MAX_PASSES - 1;
        assert_eq!(
            admitted, MAX_RETRY_VARIANTS,
            "default budget admits {admitted} variants but {MAX_RETRY_VARIANTS} can be produced — \
             the last one(s) would be silently truncated"
        );
    }

    /// Guards the other half of the arithmetic above, and does it for real
    /// rather than against a blank image: a blank image never triggers
    /// `mrz_variants`' row-density isolation (it falls back to the blind
    /// crop, yielding only 3), so it can only ever prove "producers stay
    /// under budget," never "the worst case actually needs the whole
    /// budget." This builds the actual worst-case input — bottom stripes
    /// that trigger isolation (6 from `mrz_variants`) plus a geometry band
    /// far enough from the blind crop to be treated as distinct (2 more from
    /// `geometry_band_variants`) — for the full 8, then mirrors
    /// `recognize_detailed`'s retry-loop break condition line-for-line
    /// (`passes_run` seeded at 1 for the first variant, checked *before*
    /// that variant runs) to prove the loop, as coded, actually reaches the
    /// last of them under [`DEFAULT_MAX_PASSES`] — not just that the
    /// constants agree with each other.
    #[test]
    fn max_passes_reaches_the_last_worst_case_variant() {
        let mut image = image::RgbImage::from_pixel(400, 300, image::Rgb([250, 250, 250]));
        // Two MRZ-like dark stripes tight against the bottom, isolated from
        // blank margin above — the same shape `preprocess.rs`'s own
        // isolation test uses to trigger the trailing isolated-band
        // variants — but tilted by a **half-integer** angle.
        //
        // The tilt is what makes this the worst case rather than merely a
        // heavy one. `SkewMode::Default` appends a second deskew angle only
        // when the two estimators disagree about the band, and they cannot
        // disagree on an axis-aligned fixture: both answer 0.0, the seventh
        // variant is never built, and a budget regression would hide in the
        // gap. A half-integer tilt is the cheapest way to force disagreement,
        // because the legacy search only offers integer degrees
        // (`DESKEW_CANDIDATES_DEG`) while the estimator resolves 0.5° steps.
        //
        // It has to stay *small*, which is the non-obvious part: the fixture
        // needs row-density isolation to fire as well, and isolation is what a
        // tilt destroys first. At 1.5° the stripes drift ~10px across 400px —
        // a full stripe height — smearing the row histogram until the band
        // search finds nothing confident and falls back to the blind crop,
        // collapsing this fixture from seven variants to three. 0.5° drifts
        // ~3.5px: enough for the estimators to disagree, little enough that the
        // band is still a band.
        const TILT_DEG: f64 = 0.5;
        let slope = TILT_DEG.to_radians().tan();
        for line in 0..2u32 {
            let y0 = 300 - (2 * 10 + 4) + line * 14;
            for x in 0..400u32 {
                if x % 3 == 0 {
                    continue;
                }
                let drift = (f64::from(x) * slope).round() as i64;
                for y in y0..y0 + 10 {
                    let y = i64::from(y) - drift;
                    if (0..300).contains(&y) {
                        #[allow(clippy::cast_sign_loss)] // guarded by the range check
                        image.put_pixel(x, y as u32, image::Rgb([10, 10, 10]));
                    }
                }
            }
        }
        let blind = preprocess::mrz_variants(&image, NATIVE_UPSCALE_FILTER);
        assert_eq!(
            blind.len(),
            7,
            "fixture should trigger row-density isolation and a disagreeing second deskew angle \
             (2 blind-crop + 1 full-page + 4 isolated)"
        );

        // Far from the blind bottom-45% crop (top ~165 for this 300px-tall
        // image), so it counts as a distinct band rather than a duplicate.
        let band = BBox {
            x: 0.0,
            y: 20.0,
            w: 400.0,
            h: 20.0,
        };
        let geometry = preprocess::geometry_band_variants(&image, band, NATIVE_UPSCALE_FILTER);
        assert_eq!(
            geometry.len(),
            2,
            "a geometry band distinct from the blind crop should add both treatments"
        );

        // The two trailing texture passes, which chain after everything above.
        // This mirrors `TextureMode::On`'s construction in `recognize_detailed`
        // exactly, including the order.
        let mut texture = vec![preprocess::plain_band(&image, NATIVE_UPSCALE_FILTER)];
        texture.extend(preprocess::texture_variants(&image, NATIVE_UPSCALE_FILTER));
        assert_eq!(
            texture.len(),
            2,
            "the enabled texture stage contributes exactly two trailing variants"
        );

        // The two quarter-turn band crops (ADR-0008 chunk 2), which chain
        // outside even the texture stage under `RotateMode::Default`. Mirrors
        // `recognize_detailed`'s construction, including the turns and order.
        let quarter_turns: Vec<_> = ROTATION_RETRY_TURNS
            .iter()
            .map(|&turn| preprocess::plain_band(&rotate_image(&image, turn), NATIVE_UPSCALE_FILTER))
            .collect();
        assert_eq!(
            quarter_turns.len(),
            2,
            "the default rotate mode contributes exactly two trailing quarter-turn variants"
        );

        let all_variants: Vec<_> = blind
            .into_iter()
            .chain(geometry)
            .chain(texture.iter().cloned())
            .chain(quarter_turns.iter().cloned())
            .collect();
        assert_eq!(
            all_variants.len(),
            MAX_RETRY_VARIANTS,
            "worst-case variant count this fixture was built to hit"
        );
        // Ordering is a correctness property here, not a style choice: the
        // whole no-regression argument rests on each added tier running *after*
        // every variant that already validates specimens today. The texture
        // pass held this position until chunk 2 chained the quarter turns
        // outside it — which is exactly the kind of move that silently demotes
        // a tier, so the outermost one is pinned by name. Nothing else in the
        // codebase pins this.
        assert_eq!(
            all_variants.last(),
            quarter_turns.last(),
            "the quarter-turn pass must be the last variant of all"
        );
        assert_eq!(
            all_variants.get(all_variants.len() - quarter_turns.len() - 1),
            texture.last(),
            "the texture pass must still run immediately before the quarter turns"
        );

        let mut passes_that_ran = 0usize;
        for (passes_run, _) in (1usize..).zip(all_variants.iter()) {
            if passes_run >= DEFAULT_MAX_PASSES {
                break;
            }
            passes_that_ran += 1;
        }
        assert_eq!(
            passes_that_ran,
            all_variants.len(),
            "DEFAULT_MAX_PASSES ({DEFAULT_MAX_PASSES}) must let the retry loop reach every \
             worst-case variant, including the last geometry-band one"
        );
    }

    #[test]
    fn max_duration_reads_env_with_fallback() {
        unsafe { std::env::remove_var("SYNTHPASS_OCR_MAX_SECONDS") };
        assert_eq!(
            max_duration(),
            Duration::from_secs(DEFAULT_MAX_SECONDS),
            "unset falls back"
        );

        unsafe { std::env::set_var("SYNTHPASS_OCR_MAX_SECONDS", "5") };
        assert_eq!(
            max_duration(),
            Duration::from_secs(5),
            "valid override honored"
        );

        unsafe { std::env::set_var("SYNTHPASS_OCR_MAX_SECONDS", "0") };
        assert_eq!(
            max_duration(),
            Duration::from_secs(DEFAULT_MAX_SECONDS),
            "zero falls back"
        );

        unsafe { std::env::remove_var("SYNTHPASS_OCR_MAX_SECONDS") };
    }

    #[test]
    fn verbose_enabled_only_on_exact_flag() {
        unsafe { std::env::remove_var("SYNTHPASS_OCR_VERBOSE") };
        assert!(!verbose_enabled());
        unsafe { std::env::set_var("SYNTHPASS_OCR_VERBOSE", "true") };
        assert!(
            !verbose_enabled(),
            "only the literal \"1\" should enable it"
        );
        unsafe { std::env::set_var("SYNTHPASS_OCR_VERBOSE", "1") };
        assert!(verbose_enabled());
        unsafe { std::env::remove_var("SYNTHPASS_OCR_VERBOSE") };
    }

    #[test]
    fn texture_mode_defaults_on_and_parses_all_three_arms() {
        unsafe { std::env::remove_var("SYNTHPASS_OCR_TEXTURE") };
        assert_eq!(
            trailing_texture_mode(),
            TextureMode::On,
            "unset must be the measured-best behaviour, not the baseline"
        );

        for (raw, expected) in [
            ("on", TextureMode::On),
            ("ON", TextureMode::On),
            ("  control  ", TextureMode::Control),
            ("off", TextureMode::Off),
            ("OFF", TextureMode::Off),
            // A typo is a measurement mistake, not a production outage: it
            // falls back to the default rather than erroring. Note the
            // direction changed with the default — a mistyped arm name now
            // measures `on`, so an A/B that reads "no difference" should
            // suspect the spelling of its `off` arm first.
            ("onn", TextureMode::On),
            ("", TextureMode::On),
        ] {
            unsafe { std::env::set_var("SYNTHPASS_OCR_TEXTURE", raw) };
            assert_eq!(trailing_texture_mode(), expected, "parsing {raw:?}");
        }

        unsafe { std::env::remove_var("SYNTHPASS_OCR_TEXTURE") };
    }

    #[test]
    fn ocr_order_defaults_to_default_and_parses_all_three_arms() {
        unsafe { std::env::remove_var("SYNTHPASS_OCR_ORDER") };
        assert_eq!(
            ocr_order(),
            OcrOrder::Default,
            "unset must be today's chain, unchanged, not an unmeasured reorder"
        );

        for (raw, expected) in [
            ("default", OcrOrder::Default),
            ("DEFAULT", OcrOrder::Default),
            ("band-first", OcrOrder::BandFirst),
            ("BAND-FIRST", OcrOrder::BandFirst),
            ("  control  ", OcrOrder::Control),
            // A typo is a measurement mistake, not a production outage: it
            // falls back to the default rather than erroring, same as
            // `trailing_texture_mode`.
            ("bandfirst", OcrOrder::Default),
            ("", OcrOrder::Default),
        ] {
            unsafe { std::env::set_var("SYNTHPASS_OCR_ORDER", raw) };
            assert_eq!(ocr_order(), expected, "parsing {raw:?}");
        }

        unsafe { std::env::remove_var("SYNTHPASS_OCR_ORDER") };
    }

    #[test]
    fn rotate_mode_defaults_to_default_and_parses_all_three_arms() {
        unsafe { std::env::remove_var("SYNTHPASS_OCR_ROTATE") };
        assert_eq!(
            rotate_mode(),
            RotateMode::Default,
            "unset must be the gated behaviour — the fix, not the baseline it is measured against"
        );

        for (raw, expected) in [
            ("default", RotateMode::Default),
            ("DEFAULT", RotateMode::Default),
            ("legacy", RotateMode::Legacy),
            ("LEGACY", RotateMode::Legacy),
            ("  off  ", RotateMode::Off),
            // A typo is a measurement mistake, not a production outage: it
            // falls back to the default, same as `trailing_texture_mode` and
            // `ocr_order`. Note which way that cuts — a mistyped `legacy`
            // measures `default`, so an A/B whose two arms agree exactly should
            // check this spelling before believing the result.
            ("legacyy", RotateMode::Default),
            ("", RotateMode::Default),
        ] {
            unsafe { std::env::set_var("SYNTHPASS_OCR_ROTATE", raw) };
            assert_eq!(rotate_mode(), expected, "parsing {raw:?}");
        }

        unsafe { std::env::remove_var("SYNTHPASS_OCR_ROTATE") };
    }

    #[test]
    fn skew_mode_defaults_to_default_and_parses_both_arms() {
        unsafe { std::env::remove_var("SYNTHPASS_OCR_SKEW") };
        assert_eq!(
            skew_mode(),
            preprocess::SkewMode::Default,
            "unset must be the estimator, not the search it replaced"
        );

        for (raw, expected) in [
            ("default", preprocess::SkewMode::Default),
            ("  LEGACY  ", preprocess::SkewMode::Legacy),
            ("legacy", preprocess::SkewMode::Legacy),
            ("legacyy", preprocess::SkewMode::Default),
            ("", preprocess::SkewMode::Default),
        ] {
            unsafe { std::env::set_var("SYNTHPASS_OCR_SKEW", raw) };
            assert_eq!(skew_mode(), expected, "parsing {raw:?}");
        }

        unsafe { std::env::remove_var("SYNTHPASS_OCR_SKEW") };
    }

    #[test]
    fn dump_variants_dir_is_none_unless_set_to_a_non_empty_path() {
        unsafe { std::env::remove_var("SYNTHPASS_OCR_DUMP_VARIANTS") };
        assert_eq!(
            dump_variants_dir(),
            None,
            "unset must be a no-op — the diagnostic never fires on a normal run"
        );

        for blank in ["", "   ", "\t"] {
            unsafe { std::env::set_var("SYNTHPASS_OCR_DUMP_VARIANTS", blank) };
            assert_eq!(
                dump_variants_dir(),
                None,
                "a blank value is treated as unset, not as a dump to the cwd: {blank:?}"
            );
        }

        unsafe { std::env::set_var("SYNTHPASS_OCR_DUMP_VARIANTS", "  artifacts/cell-c/crops  ") };
        assert_eq!(
            dump_variants_dir(),
            Some(PathBuf::from("artifacts/cell-c/crops")),
            "surrounding whitespace is trimmed"
        );

        unsafe { std::env::remove_var("SYNTHPASS_OCR_DUMP_VARIANTS") };
    }
}
