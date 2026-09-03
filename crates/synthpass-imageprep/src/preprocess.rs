//! Image preprocessing variants for the MRZ-targeted retry passes.
//!
//! The failure modes these address (measured on `samples/` via
//! `examples/mrz_corpus.rs`, see knowledge/ARCHITECTURE.md §8): low-resolution
//! scans whose MRZ glyphs are too small for clean recognition, low-contrast
//! photos where the detector fragments an MRZ line into disconnected
//! pieces, and — since the `Israel_Biometric_Passport.jpg` specimen —
//! dense/bilingual photographic scans where a blind bottom-band crop pulls
//! in non-Latin visual-zone text sitting directly above the MRZ, plus glare
//! and a few degrees of handheld-photo skew. Each helper is deterministic
//! and pure-Rust (`image` crate only); correctness of any retry built on
//! them is proven or rejected by the ICAO check digits upstream, never
//! assumed.

use crate::BBox;
use image::imageops::FilterType;
use image::{GrayImage, RgbImage};

/// Upscale target for the shorter side of a full-page retry pass. Below
/// roughly this, `ocrs` recognition quality on MRZ glyphs degrades sharply
/// (the classic ~300-DPI-equivalent heuristic, scaled down: `ocrs`
/// normalizes internally, so past this point extra pixels stop helping).
const FULL_PAGE_MIN_DIM: u32 = 1000;

/// Upscale target for the width of the bottom-band crop.
const BAND_MIN_WIDTH: u32 = 1600;

/// Never upscale beyond this factor — past it there is no new signal, only
/// interpolation blur and slower inference.
const MAX_SCALE: f64 = 5.0;

/// The MRZ sits in the bottom portion of every ICAO layout (TD1 rear, TD2,
/// TD3 data page). 45% keeps all three MRZ lines with margin even on rear
/// sides where the zone starts near mid-card. This is the blind fallback
/// crop; `mrz_band` tries a tighter, row-density-isolated crop first.
const BAND_FRACTION: f64 = 0.45;

/// How far above the bottom to search for a row-density-isolated MRZ band.
/// Wider than [`BAND_FRACTION`] so the search still succeeds when the
/// visual zone runs unusually low (small margins, tightly cropped
/// photographic scans) — it only ever *narrows* the blind crop, never
/// widens what gets fed to the recognizer.
const SEARCH_FRACTION: f64 = 0.55;

/// A row counts as "text" (vs. an inter-line gap) once at least this
/// fraction of its pixels are ink-dark by the search region's own Otsu
/// threshold. Low enough to catch the sparse `<` filler runs at the end of
/// MRZ lines, high enough to skip near-blank gap rows.
const TEXT_ROW_DENSITY: f64 = 0.04;

/// Plausible single-text-line height window, as a fraction of the full
/// image height. Filters non-text bands (photo blocks, guilloche-pattern
/// noise) that happen to cross the density threshold but aren't one row of
/// glyphs.
const MIN_ROW_HEIGHT_FRACTION: f64 = 0.006;
const MAX_ROW_HEIGHT_FRACTION: f64 = 0.06;

/// Padding added above/below the located band, as a fraction of its height
/// — keeps descenders/ascenders and OCR margin, mirroring the generous
/// margin the blind [`BAND_FRACTION`] crop already had.
const BAND_PAD_FRACTION: f64 = 0.5;

/// A candidate MRZ line-group may span at most this many text bands — TD1's
/// 3-line MRZ is the tallest ICAO format.
const MAX_MRZ_LINES: u32 = 3;

/// Bias subtracted from a pixel's local-neighborhood mean before it counts
/// as "dark" in [`local_threshold`] — a small margin against flat noise.
const LOCAL_THRESHOLD_BIAS: f64 = 6.0;

/// Divisor turning a measured MRZ text-row height into [`median_filter`]'s
/// radius. An OCR-B stroke is roughly an eighth of the glyph row height, and
/// the radius wants to sit at about half a stroke — wide enough to swallow
/// the security-pattern strands printed under the glyphs, narrow enough that
/// a stroke still occupies well over half the window and therefore survives
/// the median untouched. Hence row height / 16.
const TEXTURE_STROKE_DIVISOR: f64 = 16.0;

/// Floor for [`median_radius_for_band`]. A 3x3 median is the smallest window
/// that can delete a one-pixel strand at all, so there is no useful radius
/// below this and a measurement that suggests one is telling us the band is
/// too small to treat, not that a subtler kernel exists.
const TEXTURE_MEDIAN_MIN_RADIUS: u32 = 1;

/// Ceiling for [`median_radius_for_band`], and the load-bearing safety
/// property of this whole stage.
///
/// The radius is derived from a *measurement* ([`text_bands`]' row heights),
/// and a measurement can be wrong — a photo block or a dense visual zone that
/// crosses the density threshold reads as one enormous "text row". Without a
/// ceiling, that mismeasurement would produce a kernel wide enough to erase
/// glyph strokes outright, turning `1` and `I` into blank space: precisely the
/// character-confusion class this stage exists to *reduce*. Clamped to 3, the
/// worst case a mismeasurement can produce is a 7x7 median on a band upscaled
/// to [`BAND_MIN_WIDTH`], which still leaves an OCR-B stroke intact.
///
/// It also bounds [`median_filter`]'s stack buffer at `(2*3+1)^2 = 49`.
const TEXTURE_MEDIAN_MAX_RADIUS: u32 = 3;

/// Skew angles (degrees) probed by [`deskew`]. A handheld-photo tilt is a
/// few degrees, not a right angle — `ocrs`'s own detector already tolerates
/// larger rotations via its `RotatedRect` output; this variant targets the
/// smaller tilt that smears row projections just enough to fragment MRZ
/// line detection.
const DESKEW_CANDIDATES_DEG: [f64; 17] = [
    -8.0, -7.0, -6.0, -5.0, -4.0, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
];

/// Longest side (px) a probe image is downscaled to before testing
/// [`DESKEW_CANDIDATES_DEG`] — the angle search only needs the row-density
/// profile's shape, not full resolution, and this keeps 17 rotate-and-score
/// passes cheap.
const DESKEW_PROBE_MAX_DIM: u32 = 400;

/// The ordered preprocessing variants for the MRZ retry passes. Callers run
/// OCR over each until the checksum oracle validates.
///
/// Band isolation is **additive and trailing**: the proven blind
/// `bottom_band` variants — exactly the sequence that validated the entire
/// pre-isolation corpus — run *first*, in their original order; the
/// row-density-isolated `mrz_band` variants are appended only as *extra*
/// attempts reached when the blind path fails. Ordering matters because the
/// retry loop is bounded by a wall-clock/pass budget: running isolation first
/// would let it starve the proven passes on a TD1 card (whose 3-line MRZ
/// `mrz_band` can mis-locate) before they ever run. Trailing means isolation
/// can only ever *add* a winning pass on a hard dense/bilingual scan, never
/// displace or starve one that already worked — the same "retries are
/// additive-only" contract the module upholds upstream.
pub fn mrz_variants(image: &RgbImage, filter: UpscaleFilter) -> Vec<RgbImage> {
    let blind = bottom_band(image);

    // Blind bottom-band crop first, in the original order: the two band
    // thresholds, then the full-page pass as the classic last resort. Every
    // specimen that hit before isolation existed still hits here, undisturbed.
    let mut variants = vec![
        contrast_stretched(&upscale_to_width(&blind, BAND_MIN_WIDTH, filter)),
        binarized(&upscale_to_width(&blind, BAND_MIN_WIDTH, filter)),
        upscale_to_min_dim(image, FULL_PAGE_MIN_DIM, filter),
    ];

    // Row-density-isolated band as trailing extras — only when it actually
    // differs from the blind crop (`mrz_band` returns the blind crop unchanged
    // when its search finds nothing confident, so this skips duplicate passes
    // on the common non-dense case). These target dense/bilingual scans where
    // the blind crop drags in non-Latin visual-zone text above the MRZ.
    let isolated = mrz_band(image);
    if isolated.dimensions() != blind.dimensions() {
        variants.push(contrast_stretched(&upscale_to_width(
            &isolated,
            BAND_MIN_WIDTH,
            filter,
        )));
        variants.push(local_threshold(&upscale_to_width(
            &isolated,
            BAND_MIN_WIDTH,
            filter,
        )));
        variants.push(contrast_stretched(&upscale_to_width(
            &deskew(&isolated),
            BAND_MIN_WIDTH,
            filter,
        )));
    }
    variants
}

/// Vertical padding added around a geometry-detected MRZ band
/// ([`crate::geometry::detect_mrz_band`]'s content-scored box), as a
/// fraction of the band's own height, before [`geometry_band_variants`]
/// crops to it. Much tighter than [`BAND_PAD_FRACTION`]'s 50%: that margin
/// compensates for `mrz_band`'s coarse row-density search, while this band
/// is already the union of individual OCR-detected line boxes — a few
/// percent is enough safety margin for descenders/ascenders sitting just
/// outside the recognized glyph boxes, without dragging in the extra
/// non-MRZ text a generous margin would risk reintroducing on exactly the
/// dense scans this variant targets.
const GEOMETRY_BAND_PAD_FRACTION: f64 = 0.08;

/// A geometry-band crop within this many pixels (top offset and height,
/// both) of `bottom_band`'s blind crop is treated as a duplicate and
/// skipped. This is the overwhelmingly common case — most documents have no
/// dense visual zone for the content-scored band to usefully avoid, so it
/// lands close to where the blind crop already was — and it should cost
/// nothing.
const GEOMETRY_BAND_DEDUP_PX: u32 = 4;

/// Geometry-driven MRZ retry variants — the consumer `detect_mrz_band` (see
/// [`crate::geometry`]) was missing until now: crop to the content-scored
/// band the geometry pass already computed — precise enough on a
/// dense/bilingual scan to exclude the non-Latin visual-zone text sitting
/// directly above the MRZ, which both `bottom_band`'s blind crop and
/// `mrz_band`'s row-density search can still drag in — then apply the two
/// treatments already proven strongest for a band crop: contrast stretch,
/// then local threshold (glare/shadow-robust where a single Otsu cutoff
/// would wash out).
///
/// Returns an empty vec, at effectively zero cost, whenever the crop
/// wouldn't add anything: a degenerate band (near-zero width or height —
/// shouldn't happen given `detect_mrz_band`'s own confidence floor, but the
/// geometry pass upstream is best-effort, so this guards against it
/// regardless) or a crop within `GEOMETRY_BAND_DEDUP_PX` pixels of what
/// `bottom_band` already produces — the common case, where the
/// content-scored band and the blind crop land in essentially the same
/// place and a second pair of variants would only burn budget for nothing.
pub fn geometry_band_variants(
    image: &RgbImage,
    band: BBox,
    filter: UpscaleFilter,
) -> Vec<RgbImage> {
    let (w, h) = image.dimensions();
    if w == 0 || h == 0 || band.w <= 0.0 || band.h <= 0.0 {
        return Vec::new();
    }

    // Dedup against the *unpadded* band — comparing after padding would let
    // an exact-match band (the common case: nothing dense to avoid, so
    // `detect_mrz_band` lands right on the blind crop) slip past the pixel
    // threshold purely because the pad shifted its top edge, even though
    // the two crops are otherwise identical.
    let band_top = band.y.round().clamp(0.0, h as f32) as u32;
    let band_h_px = band.h.round().max(1.0) as u32;
    let blind = bottom_band(image);
    let blind_top = h.saturating_sub(blind.height());
    if band_top.abs_diff(blind_top) <= GEOMETRY_BAND_DEDUP_PX
        && band_h_px.abs_diff(blind.height()) <= GEOMETRY_BAND_DEDUP_PX
    {
        return Vec::new();
    }

    let pad = f64::from(band.h) * GEOMETRY_BAND_PAD_FRACTION;
    let top = (f64::from(band.y) - pad).round().clamp(0.0, f64::from(h)) as u32;
    let bottom = (f64::from(band.y + band.h) + pad)
        .round()
        .clamp(0.0, f64::from(h)) as u32;
    if bottom <= top || bottom - top < 2 {
        return Vec::new();
    }
    let crop_h = bottom - top;

    let crop = image::imageops::crop_imm(image, 0, top, w, crop_h).to_image();
    vec![
        contrast_stretched(&upscale_to_width(&crop, BAND_MIN_WIDTH, filter)),
        local_threshold(&upscale_to_width(&crop, BAND_MIN_WIDTH, filter)),
    ]
}

/// Texture-suppressed MRZ band retry variant — the one treatment in this
/// module that targets the *spatial* structure of a document's security
/// printing rather than its tonal effect.
///
/// Identity documents print guilloche, rosette and microprint underneath the
/// MRZ glyphs. Two helpers here already acknowledge that pattern —
/// [`contrast_stretched`] calls itself "robust to the washed-out look of
/// guilloche-patterned document backgrounds", and [`text_bands`] filters out
/// "guilloche-pattern noise" — but both address only what the pattern does to
/// the *histogram*. A percentile stretch is a point operation and cannot
/// remove a spatially-varying pattern; [`local_threshold`] models the
/// background with a box mean whose radius (`min_dim/16`) is tuned for
/// illumination gradients, an order of magnitude coarser than a strand of
/// microprint. So the strands survive into the binarized image and merge with
/// the glyph strokes they sit under.
///
/// That is a candidate mechanism for the dominant real-specimen miss kind:
/// `checksum_failed` outnumbers `no_mrz_found`, and the character confusions
/// behind it were measured as diffuse — `0`/`O`, `1`/`I`/`L`, `5`/`S`, `8`/`B`
/// scattered across many specimens with no single shared shape, which is what
/// a textured background produces and what a geometric fault does not.
///
/// [`median_filter`] is the operator because it **fails safe**. It deletes any
/// structure occupying less than half its window while leaving wider
/// structures and straight edges alone, so a strand thinner than the kernel
/// disappears and a glyph stroke does not. If the texture turns out to be
/// wider than the kernel, the output is close to the input and this becomes a
/// redundant pass — never a corrupting one. A morphological closing would be
/// the sharper instrument and is the documented follow-up, but it is only
/// selective while `strand < kernel < stroke`, and mis-sized by a single pixel
/// it erases stroke terminals and thins `1` into nothing — manufacturing the
/// exact confusion class this is meant to reduce.
///
/// It composes with rather than duplicates [`local_threshold`]: the median
/// removes high-frequency structure *before* thresholding, adaptive
/// thresholding removes low-frequency illumination. Disjoint parts of the
/// spectrum.
///
/// Returns exactly one variant, or an empty vec on a degenerate image. Like
/// every other producer here this is **additive**: the caller runs it after
/// all existing variants, and the ICAO check digits decide whether it was
/// right.
pub fn texture_variants(image: &RgbImage, filter: UpscaleFilter) -> Vec<RgbImage> {
    let (w, h) = image.dimensions();
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let band = upscale_to_width(&mrz_band(image), BAND_MIN_WIDTH, filter);
    let gray = to_gray(&band);
    let cleaned = median_filter(&gray, median_radius_for_band(&gray));
    vec![local_threshold(&gray_to_rgb_map(&cleaned, |v| v))]
}

/// Median radius for [`texture_variants`], derived from the band's own
/// measured text-row height rather than hardcoded.
///
/// The band reaching this point has been upscaled to [`BAND_MIN_WIDTH`], but
/// that fixes the *width*, not the glyph height — a TD1 card's three 30-column
/// lines and a TD3 passport's two 44-column lines land at very different row
/// heights for the same band width. So measure: [`text_bands`] already returns
/// the runs of consecutive ink-dense rows, and on an MRZ band those runs *are*
/// the glyph lines. The median of their heights is the row height, and
/// [`TEXTURE_STROKE_DIVISOR`] turns that into half a stroke width.
///
/// Bounds are deliberately wide open (`1..=h`) where [`mrz_band`] uses
/// page-relative fractions: those fractions describe where an MRZ sits on a
/// *page*, and this input is already the cropped band, so reapplying them here
/// would reject the very rows being measured.
///
/// Falls back to [`TEXTURE_MEDIAN_MIN_RADIUS`] when no band is found — a blank
/// or degenerate crop yields no rows to measure, and the smallest kernel is
/// the right thing to attempt when the measurement is absent.
fn median_radius_for_band(gray: &GrayImage) -> u32 {
    let h = gray.height();
    let mut heights: Vec<u32> = text_bands(gray, 0, h, 1, h.max(1))
        .into_iter()
        .map(|(s, e)| e - s)
        .collect();
    if heights.is_empty() {
        return TEXTURE_MEDIAN_MIN_RADIUS;
    }
    heights.sort_unstable();
    let row_h = f64::from(heights[heights.len() / 2]);
    ((row_h / TEXTURE_STROKE_DIVISOR).round() as u32)
        .clamp(TEXTURE_MEDIAN_MIN_RADIUS, TEXTURE_MEDIAN_MAX_RADIUS)
}

/// Grayscale median filter over a `(2*radius+1)^2` window, edges clamped.
///
/// Deliberately the naive form rather than a sliding histogram (Huang): the
/// radius is capped at [`TEXTURE_MEDIAN_MAX_RADIUS`], so the window holds at
/// most 49 values and the whole pass costs tens of milliseconds on a band —
/// on a retry path that only runs after eight other variants have already
/// failed. A histogram version would be several times the code for a saving
/// nothing here can perceive.
///
/// The `[u8; 49]` buffer is sized by that same cap. `radius` is re-clamped on
/// entry rather than trusted: this is a private helper today, but a buffer
/// whose bound lives in a *different* function's clamp is exactly the kind of
/// coupling that breaks silently later, and the cost of not trusting it is one
/// `min` call per image.
fn median_filter(gray: &GrayImage, radius: u32) -> GrayImage {
    let radius = radius.min(TEXTURE_MEDIAN_MAX_RADIUS);
    let (w, h) = gray.dimensions();
    if w == 0 || h == 0 || radius == 0 {
        return gray.clone();
    }
    let side = (2 * radius + 1) as usize;
    let capacity = side * side;
    debug_assert!(capacity <= 49, "median window outgrew its buffer");

    GrayImage::from_fn(w, h, |x, y| {
        let mut window = [0u8; 49];
        let mut n = 0usize;
        let x0 = x.saturating_sub(radius);
        let x1 = (x + radius).min(w - 1);
        let y0 = y.saturating_sub(radius);
        let y1 = (y + radius).min(h - 1);
        for yy in y0..=y1 {
            for xx in x0..=x1 {
                if n < window.len() {
                    window[n] = gray.get_pixel(xx, yy)[0];
                    n += 1;
                }
            }
        }
        // `n` is always >= 1: the pixel's own position is inside the clamped
        // rectangle, so the loops above run at least once.
        window[..n].sort_unstable();
        image::Luma([window[n / 2]])
    })
}

/// The blind band crop, upscaled, with **no tonal treatment** — no contrast
/// stretch, no threshold.
///
/// Deliberately absent from [`mrz_variants`], which is tuned for `ocrs`: that
/// engine normalizes internally, so an untreated variant adds nothing there.
/// A recognizer already trained on OCR-B is a different case. On the browser
/// demo's tesseract build this single variant produced **112 of 122**
/// checksum-valid reads across the specimen corpus, with all five treated
/// variants recovering ten documents between them
/// (`knowledge/WEB_OCR_BASELINE.md`, 2026-09-03).
///
/// So this is not a browser-specific hack, it is a recognizer-dependent
/// ordering fact, and the caller that knows which recognizer it is running
/// decides where this goes in the retry sequence.
pub fn plain_band(image: &RgbImage, filter: UpscaleFilter) -> RgbImage {
    upscale_to_width(&bottom_band(image), BAND_MIN_WIDTH, filter)
}

/// Crop the bottom [`BAND_FRACTION`] of the image (full width). The blind
/// fallback used when `mrz_band`'s row-density search finds nothing
/// confident.
fn bottom_band(image: &RgbImage) -> RgbImage {
    let (w, h) = image.dimensions();
    let band_h = ((f64::from(h) * BAND_FRACTION).round() as u32).clamp(1, h);
    image::imageops::crop_imm(image, 0, h - band_h, w, band_h).to_image()
}

/// Row-density-isolated MRZ band: locates the monospace OCR-B block by its
/// projection profile (1-3 text rows of consistent height, tightly spaced,
/// near the bottom) instead of blindly cropping [`BAND_FRACTION`] of the
/// page, which on dense bilingual/photographic scans (e.g. Hebrew
/// visual-zone text sitting directly above the MRZ) pulls in non-Latin
/// lines the MRZ-charset-constrained engine can only garble. Falls back to
/// `bottom_band` when no confident band is found.
fn mrz_band(image: &RgbImage) -> RgbImage {
    let (w, h) = image.dimensions();
    let gray = to_gray(image);
    let search_top = h.saturating_sub((f64::from(h) * SEARCH_FRACTION).round() as u32);
    let min_row_h = ((f64::from(h) * MIN_ROW_HEIGHT_FRACTION).round() as u32).max(1);
    let max_row_h = ((f64::from(h) * MAX_ROW_HEIGHT_FRACTION).round() as u32).max(min_row_h);
    if let Some((top, bottom)) = locate_mrz_band(&gray, search_top, h, min_row_h, max_row_h) {
        let band_h = bottom - top;
        let pad = ((f64::from(band_h)) * BAND_PAD_FRACTION).round() as u32;
        let top = top.saturating_sub(pad);
        let bottom = (bottom + pad).min(h);
        return image::imageops::crop_imm(image, 0, top, w, bottom - top).to_image();
    }
    bottom_band(image)
}

/// Find the bottommost run of up to [`MAX_MRZ_LINES`] text bands with
/// plausible MRZ-line height and tight spacing — the signature of a TD1
/// (3-line) or TD2/TD3 (2-line) MRZ block. Returns the pixel range
/// `[top, bottom)` spanning the bands found, or `None` if nothing in the
/// search region looks like isolated text lines.
fn locate_mrz_band(
    gray: &GrayImage,
    from: u32,
    to: u32,
    min_row_h: u32,
    max_row_h: u32,
) -> Option<(u32, u32)> {
    let bands = text_bands(gray, from, to, min_row_h, max_row_h);
    let &(mut group_top, group_bottom) = bands.last()?;
    let mut last_height = f64::from(group_bottom - group_top);

    // `lines` counts the bands already accepted into the group (the
    // bottommost one, seeded above, plus one per completed iteration below)
    // — a `zip` counter rather than a manually incremented one so clippy's
    // `explicit_counter_loop` stays clean.
    for (lines, &(s, e)) in (1u32..).zip(bands.iter().rev().skip(1)) {
        if lines >= MAX_MRZ_LINES {
            break;
        }
        let height = f64::from(e - s);
        let gap = f64::from(group_top.saturating_sub(e));
        let ratio = height / last_height;
        if gap > last_height * 3.0 || !(0.4..=2.5).contains(&ratio) {
            break;
        }
        group_top = s;
        last_height = height;
    }
    Some((group_top, group_bottom))
}

/// Consecutive-text-row bands `(start, end)` within `[from, to)`, kept only
/// if their height falls in `[min_row_h, max_row_h]` — filters out photo
/// blocks and guilloche-pattern noise that happen to cross the density
/// threshold but aren't a single line of glyphs.
fn text_bands(
    gray: &GrayImage,
    from: u32,
    to: u32,
    min_row_h: u32,
    max_row_h: u32,
) -> Vec<(u32, u32)> {
    let density = row_density(gray, from, to);
    let mut bands = Vec::new();
    let mut run_start: Option<usize> = None;
    for (i, &d) in density.iter().enumerate() {
        let is_text = d >= TEXT_ROW_DENSITY;
        match (is_text, run_start) {
            (true, None) => run_start = Some(i),
            (false, Some(s)) => {
                bands.push((from + s as u32, from + i as u32));
                run_start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = run_start {
        bands.push((from + s as u32, to));
    }
    bands
        .into_iter()
        .filter(|&(s, e)| {
            let h = e - s;
            h >= min_row_h && h <= max_row_h
        })
        .collect()
}

/// Per-row ink density (fraction of dark pixels) for rows in `[from, to)`,
/// thresholded against that region's own Otsu cutoff so it adapts to local
/// contrast instead of a fixed page-wide value.
fn row_density(gray: &GrayImage, from: u32, to: u32) -> Vec<f64> {
    let w = gray.width().max(1);
    let t = otsu_threshold_region(gray, from, to);
    (from..to)
        .map(|y| {
            let dark = (0..w).filter(|&x| gray.get_pixel(x, y)[0] <= t).count();
            dark as f64 / f64::from(w)
        })
        .collect()
}

/// Which resampling filter the band/page upscales use.
///
/// **This is a recognizer-dependent choice, and it is deliberately not a
/// default.** Measured over the specimen corpus (see
/// `knowledge/WEB_OCR_BASELINE.md`, 2026-09-03), the two engines this project
/// runs want opposite answers, so every call site states which it is:
///
/// | | [`Lanczos3`](Self::Lanczos3) | [`Triangle`](Self::Triangle) |
/// | :-- | --: | --: |
/// | browser (tesseract, OCR-B model), 190 docs | 125 valid, 143/153 fields | 125 valid, **150/153 fields** |
/// | native (`ocrs`), 20 docs | **18/20** | 17/20 |
///
/// The mechanism behind the split: Lanczos3 sharpens, which helps the
/// *detector* find MRZ-shaped text on a blurred or low-contrast scan, but it
/// rings on high-contrast edges — and an MRZ glyph stroke is nothing but
/// high-contrast edges — which costs *character* accuracy. Triangle is the
/// reverse.
///
/// The single clearest piece of evidence that this is a real effect rather
/// than corpus noise: `Cyprus_Passport_Specimen_P0_CYP_2010` is the document
/// native *loses* under Triangle, and simultaneously the one where the browser
/// *recovers* line-1 fields under it. Same image, same change, opposite
/// directions in two engines.
///
/// Character accuracy matters disproportionately on line 1, which carries no
/// check digit in TD1, TD2 or TD3 — for `document_type`, `issuing_country`,
/// `surname` and `given_names` the recognizer is the only protection there is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UpscaleFilter {
    /// Sharper. Better MRZ *detection*; rings on glyph edges. What `ocrs`
    /// wants.
    Lanczos3,
    /// Smoother. Better *character accuracy*, especially on the unchecksummed
    /// line-1 fields. What tesseract's OCR-B model wants.
    Triangle,
}

impl UpscaleFilter {
    fn as_image_filter(self) -> FilterType {
        match self {
            Self::Lanczos3 => FilterType::Lanczos3,
            Self::Triangle => FilterType::Triangle,
        }
    }
}

fn scale_by(image: &RgbImage, scale: f64, filter: UpscaleFilter) -> RgbImage {
    if scale <= 1.0 {
        return image.clone();
    }
    let scale = scale.min(MAX_SCALE);
    let (w, h) = image.dimensions();
    image::imageops::resize(
        image,
        (f64::from(w) * scale).round() as u32,
        (f64::from(h) * scale).round() as u32,
        filter.as_image_filter(),
    )
}

fn upscale_to_width(image: &RgbImage, min_width: u32, filter: UpscaleFilter) -> RgbImage {
    let w = image.width().max(1);
    scale_by(image, f64::from(min_width) / f64::from(w), filter)
}

fn upscale_to_min_dim(image: &RgbImage, min_dim: u32, filter: UpscaleFilter) -> RgbImage {
    let shorter = image.width().min(image.height()).max(1);
    scale_by(image, f64::from(min_dim) / f64::from(shorter), filter)
}

/// Grayscale + linear contrast stretch mapping the 1st..99th intensity
/// percentiles to 0..255, returned as RGB (what `ocrs`'s input path expects).
/// Robust to the washed-out look of guilloche-patterned document backgrounds
/// without the all-or-nothing commitment of hard binarization.
fn contrast_stretched(image: &RgbImage) -> RgbImage {
    let gray = to_gray(image);
    let (lo, hi) = percentile_bounds(&gray, 0.01, 0.99);
    let range = f64::from(hi.saturating_sub(lo)).max(1.0);
    gray_to_rgb_map(&gray, |v| {
        ((f64::from(v.saturating_sub(lo)) / range * 255.0).round() as i64).clamp(0, 255) as u8
    })
}

/// Grayscale + Otsu global threshold, returned as RGB. The strongest variant
/// on clean-but-tiny scans, the weakest on uneven lighting — which is why it
/// runs as one candidate among several rather than unconditionally.
fn binarized(image: &RgbImage) -> RgbImage {
    let gray = to_gray(image);
    let t = otsu_threshold(&gray);
    gray_to_rgb_map(&gray, |v| if v > t { 255 } else { 0 })
}

/// Local-mean threshold via an integral image (box filter), returned as RGB.
/// Where [`binarized`]'s single page-wide Otsu cutoff washes out under
/// glare or a shadow gradient across a photographed page, each pixel here is
/// compared to its own neighborhood's mean — the "photographic scan"
/// counterpart to `binarized`'s "clean scan" strength.
fn local_threshold(image: &RgbImage) -> RgbImage {
    let gray = to_gray(image);
    let (w, h) = gray.dimensions();
    if w == 0 || h == 0 {
        return image.clone();
    }
    let integral = Integral::build(&gray);
    let radius = (w.min(h) / 16).clamp(4, 40);
    RgbImage::from_fn(w, h, |x, y| {
        let x0 = x.saturating_sub(radius);
        let x1 = (x + radius).min(w - 1);
        let y0 = y.saturating_sub(radius);
        let y1 = (y + radius).min(h - 1);
        let count = u64::from(x1 - x0 + 1) * u64::from(y1 - y0 + 1);
        let mean = integral.sum(x0, y0, x1, y1) as f64 / count as f64;
        let v = f64::from(gray.get_pixel(x, y)[0]);
        let out = if v + LOCAL_THRESHOLD_BIAS < mean {
            0
        } else {
            255
        };
        image::Rgb([out, out, out])
    })
}

/// Summed-area table over a [`GrayImage`] for O(1) rectangular-region sums,
/// backing [`local_threshold`]'s per-pixel local mean.
struct Integral {
    data: Vec<u64>,
    /// Row stride (`image width + 1`).
    stride: usize,
}

impl Integral {
    fn build(gray: &GrayImage) -> Self {
        let (w, h) = (gray.width() as usize, gray.height() as usize);
        let stride = w + 1;
        let mut data = vec![0u64; stride * (h + 1)];
        for y in 0..h {
            let mut row_sum = 0u64;
            for x in 0..w {
                row_sum += u64::from(gray.get_pixel(x as u32, y as u32)[0]);
                data[(y + 1) * stride + (x + 1)] = data[y * stride + (x + 1)] + row_sum;
            }
        }
        Self { data, stride }
    }

    /// Sum over the inclusive pixel rectangle `[x0,x1] x [y0,y1]`.
    fn sum(&self, x0: u32, y0: u32, x1: u32, y1: u32) -> u64 {
        let (x0, y0, x1, y1) = (x0 as usize, y0 as usize, x1 as usize, y1 as usize);
        let a = self.data[y0 * self.stride + x0];
        let b = self.data[y0 * self.stride + x1 + 1];
        let c = self.data[(y1 + 1) * self.stride + x0];
        let d = self.data[(y1 + 1) * self.stride + x1 + 1];
        d + a - b - c
    }
}

/// Deskew by the rotation angle (from [`DESKEW_CANDIDATES_DEG`]) that
/// maximizes horizontal row-density variance: text rows align into sharp
/// density peaks when level, and skew smears them into a flatter profile.
/// Pure-Rust bilinear rotation around the image center — no `imageproc`
/// dependency.
fn deskew(image: &RgbImage) -> RgbImage {
    let gray = to_gray(image);
    let probe = downscale_longest_side(&gray, DESKEW_PROBE_MAX_DIM);
    let best = DESKEW_CANDIDATES_DEG
        .iter()
        .copied()
        .max_by(|&a, &b| {
            let ca = projection_contrast(&rotate_gray(&probe, a));
            let cb = projection_contrast(&rotate_gray(&probe, b));
            ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0.0);
    if best == 0.0 {
        image.clone()
    } else {
        rotate_rgb(image, best)
    }
}

fn downscale_longest_side(gray: &GrayImage, max_dim: u32) -> GrayImage {
    let (w, h) = gray.dimensions();
    let longest = w.max(h).max(1);
    if longest <= max_dim {
        return gray.clone();
    }
    let scale = f64::from(max_dim) / f64::from(longest);
    image::imageops::resize(
        gray,
        ((f64::from(w) * scale).round() as u32).max(1),
        ((f64::from(h) * scale).round() as u32).max(1),
        FilterType::Triangle,
    )
}

/// Turn `image` clockwise by a whole number of quarter turns.
///
/// Exact pixel transposition via `image`'s right-angle rotations, never a
/// resample — a quarter turn is lossless and must stay that way, because
/// anything fed to a recognizer after this has already had its one upscale.
/// `turns` is taken modulo 4, so any multiple of 90° works.
pub fn rotate_quarter_turns(image: &RgbImage, turns: u32) -> RgbImage {
    match turns % 4 {
        1 => image::imageops::rotate90(image),
        2 => image::imageops::rotate180(image),
        3 => image::imageops::rotate270(image),
        _ => image.clone(),
    }
}

/// Variance of the row-density profile — higher means sharper line/gap
/// contrast, i.e. better horizontal alignment.
fn projection_contrast(gray: &GrayImage) -> f64 {
    let h = gray.height();
    if h == 0 {
        return 0.0;
    }
    let density = row_density(gray, 0, h);
    let mean = density.iter().sum::<f64>() / density.len() as f64;
    density.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / density.len() as f64
}

fn rotate_gray(gray: &GrayImage, degrees: f64) -> GrayImage {
    if degrees == 0.0 {
        return gray.clone();
    }
    let (w, h) = gray.dimensions();
    let (cx, cy) = (f64::from(w) / 2.0, f64::from(h) / 2.0);
    let (sin_t, cos_t) = (-degrees).to_radians().sin_cos();
    GrayImage::from_fn(w, h, |x, y| {
        let (sx, sy) = inverse_rotate(x, y, cx, cy, sin_t, cos_t);
        image::Luma([sample_gray_bilinear(gray, sx, sy)])
    })
}

fn rotate_rgb(image: &RgbImage, degrees: f64) -> RgbImage {
    if degrees == 0.0 {
        return image.clone();
    }
    let (w, h) = image.dimensions();
    let (cx, cy) = (f64::from(w) / 2.0, f64::from(h) / 2.0);
    let (sin_t, cos_t) = (-degrees).to_radians().sin_cos();
    RgbImage::from_fn(w, h, |x, y| {
        let (sx, sy) = inverse_rotate(x, y, cx, cy, sin_t, cos_t);
        sample_rgb_bilinear(image, sx, sy)
    })
}

/// Map output pixel `(x, y)` back to the source image's coordinate space
/// under a rotation of `degrees` about `(cx, cy)` (`sin_t`/`cos_t` already
/// negated by the caller so this performs the *inverse* rotation).
fn inverse_rotate(x: u32, y: u32, cx: f64, cy: f64, sin_t: f64, cos_t: f64) -> (f64, f64) {
    let dx = f64::from(x) - cx;
    let dy = f64::from(y) - cy;
    (cos_t * dx - sin_t * dy + cx, sin_t * dx + cos_t * dy + cy)
}

fn sample_gray_bilinear(gray: &GrayImage, x: f64, y: f64) -> u8 {
    let (w, h) = gray.dimensions();
    if w < 2 || h < 2 || x < 0.0 || y < 0.0 || x >= f64::from(w - 1) || y >= f64::from(h - 1) {
        return 255; // background
    }
    let (x0, y0) = (x.floor() as u32, y.floor() as u32);
    let (fx, fy) = (x - f64::from(x0), y - f64::from(y0));
    let p = |px: u32, py: u32| f64::from(gray.get_pixel(px, py)[0]);
    let top = p(x0, y0) * (1.0 - fx) + p(x0 + 1, y0) * fx;
    let bot = p(x0, y0 + 1) * (1.0 - fx) + p(x0 + 1, y0 + 1) * fx;
    (top * (1.0 - fy) + bot * fy).round().clamp(0.0, 255.0) as u8
}

fn sample_rgb_bilinear(image: &RgbImage, x: f64, y: f64) -> image::Rgb<u8> {
    let (w, h) = image.dimensions();
    if w < 2 || h < 2 || x < 0.0 || y < 0.0 || x >= f64::from(w - 1) || y >= f64::from(h - 1) {
        return image::Rgb([255, 255, 255]);
    }
    let (x0, y0) = (x.floor() as u32, y.floor() as u32);
    let (fx, fy) = (x - f64::from(x0), y - f64::from(y0));
    let mut out = [0u8; 3];
    for (c, slot) in out.iter_mut().enumerate() {
        let p = |px: u32, py: u32| f64::from(image.get_pixel(px, py)[c]);
        let top = p(x0, y0) * (1.0 - fx) + p(x0 + 1, y0) * fx;
        let bot = p(x0, y0 + 1) * (1.0 - fx) + p(x0 + 1, y0 + 1) * fx;
        *slot = (top * (1.0 - fy) + bot * fy).round().clamp(0.0, 255.0) as u8;
    }
    image::Rgb(out)
}

fn to_gray(image: &RgbImage) -> GrayImage {
    image::imageops::grayscale(image)
}

fn gray_to_rgb_map(gray: &GrayImage, f: impl Fn(u8) -> u8) -> RgbImage {
    RgbImage::from_fn(gray.width(), gray.height(), |x, y| {
        let v = f(gray.get_pixel(x, y)[0]);
        image::Rgb([v, v, v])
    })
}

/// Intensity values at the given cumulative-histogram fractions.
fn percentile_bounds(gray: &GrayImage, lo_frac: f64, hi_frac: f64) -> (u8, u8) {
    let mut hist = [0u64; 256];
    for p in gray.pixels() {
        hist[p[0] as usize] += 1;
    }
    let total: u64 = hist.iter().sum();
    if total == 0 {
        return (0, 255);
    }
    let lo_target = (total as f64 * lo_frac) as u64;
    let hi_target = (total as f64 * hi_frac) as u64;
    let (mut lo, mut hi, mut cum) = (0u8, 255u8, 0u64);
    let mut lo_set = false;
    for (v, &count) in hist.iter().enumerate() {
        cum += count;
        if !lo_set && cum > lo_target {
            lo = v as u8;
            lo_set = true;
        }
        if cum >= hi_target {
            hi = v as u8;
            break;
        }
    }
    (lo, hi.max(lo))
}

/// Otsu's global threshold over the whole image.
fn otsu_threshold(gray: &GrayImage) -> u8 {
    let mut hist = [0u32; 256];
    for p in gray.pixels() {
        hist[p[0] as usize] += 1;
    }
    otsu_from_hist(&hist)
}

/// Otsu's threshold restricted to rows `[from, to)` — lets [`row_density`]
/// adapt to a sub-region's own contrast instead of the whole page's.
fn otsu_threshold_region(gray: &GrayImage, from: u32, to: u32) -> u8 {
    let mut hist = [0u32; 256];
    for y in from..to {
        for x in 0..gray.width() {
            hist[gray.get_pixel(x, y)[0] as usize] += 1;
        }
    }
    otsu_from_hist(&hist)
}

/// Otsu's threshold from a 256-bin intensity histogram (inherited from the
/// retired Tesseract engine's `preprocess::otsu_threshold` — the technique
/// outlived the C dependency chain it came from).
fn otsu_from_hist(hist: &[u32; 256]) -> u8 {
    let total: u32 = hist.iter().sum();
    if total == 0 {
        return 127;
    }
    let sum: f64 = hist
        .iter()
        .enumerate()
        .map(|(i, &c)| i as f64 * c as f64)
        .sum();

    let mut sum_b = 0f64;
    let mut w_b = 0u32;
    let mut max_var = -1f64;
    let mut thresh = 0u8;
    for (t, &count) in hist.iter().enumerate() {
        w_b += count;
        if w_b == 0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0 {
            break;
        }
        sum_b += t as f64 * count as f64;
        let m_b = sum_b / w_b as f64;
        let m_f = (sum - sum_b) / w_f as f64;
        let var = w_b as f64 * w_f as f64 * (m_b - m_f).powi(2);
        if var > max_var {
            max_var = var;
            thresh = t as u8;
        }
    }
    thresh
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    fn solid(w: u32, h: u32, v: u8) -> RgbImage {
        RgbImage::from_pixel(w, h, Rgb([v, v, v]))
    }

    /// Build an image with `n_lines` dark horizontal text-like stripes near
    /// the bottom, each `line_h` tall, separated by `gap_h` light rows —
    /// a synthetic stand-in for an MRZ block sitting under blank margin.
    fn image_with_bottom_stripes(
        w: u32,
        h: u32,
        n_lines: u32,
        line_h: u32,
        gap_h: u32,
    ) -> RgbImage {
        let mut img = solid(w, h, 250);
        let block_h = n_lines * line_h + n_lines.saturating_sub(1) * gap_h;
        let start = h - block_h;
        for line in 0..n_lines {
            let y0 = start + line * (line_h + gap_h);
            for y in y0..y0 + line_h {
                for x in 0..w {
                    // Every other column dark, mimicking glyph+filler texture.
                    if x % 3 != 0 {
                        img.put_pixel(x, y, Rgb([10, 10, 10]));
                    }
                }
            }
        }
        img
    }

    #[test]
    fn bottom_band_keeps_full_width_and_bottom_rows() {
        let mut img = solid(100, 100, 255);
        // Mark the bottom row so we can prove the crop kept the *bottom*.
        for x in 0..100 {
            img.put_pixel(x, 99, Rgb([0, 0, 0]));
        }
        let band = bottom_band(&img);
        assert_eq!(band.width(), 100);
        assert_eq!(band.height(), 45);
        assert_eq!(band.get_pixel(0, 44)[0], 0, "bottom row survives the crop");
    }

    /// A page of horizontal text rows: sharp density peaks along y.
    fn ruled_page(w: u32, h: u32) -> RgbImage {
        RgbImage::from_fn(w, h, |_, y| {
            // 4px of "ink" every 12 rows, with margins left blank.
            let ink = (y % 12) < 4 && y > 10 && y < h - 10;
            image::Rgb(if ink { [20, 20, 20] } else { [235, 235, 235] })
        })
    }

    #[test]
    fn four_quarter_turns_are_the_identity() {
        let img = ruled_page(37, 53); // deliberately non-square and odd-sized
        assert_eq!(rotate_quarter_turns(&img, 4), img);
        assert_eq!(rotate_quarter_turns(&img, 0), img);
        // And a quarter turn swaps the dimensions rather than resampling.
        assert_eq!(rotate_quarter_turns(&img, 1).dimensions(), (53, 37));
        assert_eq!(rotate_quarter_turns(&img, 2).dimensions(), (37, 53));
    }

    #[test]
    fn quarter_turns_are_lossless() {
        let img = ruled_page(64, 48);
        let round_tripped = rotate_quarter_turns(&rotate_quarter_turns(&img, 1), 3);
        assert_eq!(round_tripped, img, "a quarter turn must not resample");
    }

    #[test]
    fn upscale_caps_at_max_scale() {
        let img = solid(100, 50, 128);
        let out = upscale_to_width(&img, 1600, UpscaleFilter::Lanczos3);
        // 16× requested, capped at MAX_SCALE (5×).
        assert_eq!(out.width(), 500);
        assert_eq!(out.height(), 250);
    }

    #[test]
    fn upscale_never_downscales() {
        let img = solid(2000, 1500, 128);
        let same = upscale_to_min_dim(&img, 1000, UpscaleFilter::Lanczos3);
        assert_eq!(same.dimensions(), (2000, 1500));
    }

    #[test]
    fn contrast_stretch_expands_a_narrow_range() {
        // Two clusters squeezed into 100..140 must spread toward 0/255.
        let img = RgbImage::from_fn(64, 64, |x, _| {
            let v = if x < 32 { 100 } else { 140 };
            Rgb([v, v, v])
        });
        let out = contrast_stretched(&img);
        let dark = out.get_pixel(0, 0)[0];
        let light = out.get_pixel(63, 0)[0];
        assert!(dark < 30, "dark cluster should map near 0, got {dark}");
        assert!(
            light > 225,
            "light cluster should map near 255, got {light}"
        );
    }

    #[test]
    fn binarized_is_pure_black_and_white() {
        let img = RgbImage::from_fn(32, 32, |x, _| {
            let v = if x < 16 { 60 } else { 200 };
            Rgb([v, v, v])
        });
        for p in binarized(&img).pixels() {
            assert!(p[0] == 0 || p[0] == 255);
        }
    }

    #[test]
    fn local_threshold_is_pure_black_and_white() {
        let img = RgbImage::from_fn(64, 64, |x, y| {
            let v = if (x + y) % 7 == 0 { 40 } else { 210 };
            Rgb([v, v, v])
        });
        for p in local_threshold(&img).pixels() {
            assert!(p[0] == 0 || p[0] == 255);
        }
    }

    #[test]
    fn local_threshold_survives_a_glare_gradient() {
        // A left-to-right brightness ramp (simulated glare) shouldn't wash
        // out the embedded dark stripe the way a single global cutoff would.
        let img = RgbImage::from_fn(80, 40, |x, y| {
            let ramp = (x as f64 / 79.0 * 200.0) as u8;
            let base = 40u8.saturating_add(ramp);
            let v = if (20..24).contains(&y) {
                base / 3
            } else {
                base
            };
            Rgb([v, v, v])
        });
        let out = local_threshold(&img);
        // The stripe should read dark on both the dim and bright ends.
        assert_eq!(out.get_pixel(5, 22)[0], 0, "stripe dark under low glare");
        assert_eq!(out.get_pixel(74, 22)[0], 0, "stripe dark under high glare");
    }

    #[test]
    fn variants_on_a_blank_image_skip_isolation_and_keep_the_blind_path() {
        // A blank image has no text bands, so `mrz_band` falls back to the
        // blind crop and the isolated block is skipped as a duplicate: just
        // the two blind-crop variants plus the full-page pass remain.
        let v = mrz_variants(&solid(600, 400, 200), UpscaleFilter::Lanczos3);
        assert_eq!(v.len(), 3);
        // The band crops come before the full-page pass, the last and most
        // expensive entry.
        let full_page = v.last().unwrap();
        assert!(v[0].height() < full_page.height() * 3 / 4);
    }

    #[test]
    fn isolation_adds_trailing_variants_without_dropping_the_blind_path() {
        // With MRZ-like stripes the row-density search isolates a tighter
        // band, so its three variants are *appended after* the blind-crop
        // path (which must still lead — that's what keeps already-passing
        // specimens passing within budget): 2 blind + 1 full-page + 3 isolated.
        let img = image_with_bottom_stripes(400, 300, 2, 10, 4);
        let v = mrz_variants(&img, UpscaleFilter::Lanczos3);
        assert_eq!(v.len(), 6);
        // The blind path leads: the full-page pass sits at index 2, ahead of
        // the trailing isolated variants, and is taller than the first band.
        let full_page = &v[2];
        assert!(v[0].height() < full_page.height() * 3 / 4);
    }

    #[test]
    fn geometry_band_variants_empty_on_degenerate_band() {
        let img = solid(200, 200, 200);
        let band = BBox {
            x: 0.0,
            y: 50.0,
            w: 100.0,
            h: 0.0,
        };
        assert!(geometry_band_variants(&img, band, UpscaleFilter::Lanczos3).is_empty());
    }

    #[test]
    fn geometry_band_variants_empty_when_matching_blind_crop() {
        let img = solid(200, 200, 200);
        let blind = bottom_band(&img);
        let blind_top = img.height() - blind.height();
        let band = BBox {
            x: 0.0,
            y: blind_top as f32,
            w: 200.0,
            h: blind.height() as f32,
        };
        assert!(
            geometry_band_variants(&img, band, UpscaleFilter::Lanczos3).is_empty(),
            "a band matching the blind crop should be skipped as a duplicate"
        );
    }

    #[test]
    fn geometry_band_variants_produces_two_treatments_for_a_distinct_band() {
        let img = solid(400, 300, 200);
        // Well above the blind bottom-45% crop (top at y=165 for a 300px
        // image), so this counts as a genuinely distinct geometry-detected
        // band rather than a near-duplicate.
        let band = BBox {
            x: 0.0,
            y: 40.0,
            w: 400.0,
            h: 20.0,
        };
        let variants = geometry_band_variants(&img, band, UpscaleFilter::Lanczos3);
        assert_eq!(variants.len(), 2, "contrast-stretched + local-threshold");
        for v in &variants {
            assert_eq!(
                v.width(),
                BAND_MIN_WIDTH,
                "upscaled to the band width floor"
            );
        }
    }

    #[test]
    fn mrz_band_isolates_bottom_stripes_from_blank_margin_above() {
        // Two MRZ-like stripes near the bottom of an otherwise-blank page;
        // a blind 45% crop would include a lot of empty margin above them.
        let img = image_with_bottom_stripes(400, 300, 2, 10, 4);
        let band = mrz_band(&img);
        // The isolated band should be much shorter than the blind crop.
        let blind = bottom_band(&img);
        assert!(
            band.height() < blind.height(),
            "row-density band ({}) should be tighter than the blind crop ({})",
            band.height(),
            blind.height()
        );
        // And it should still be tall enough to hold both stripes + padding.
        assert!(band.height() >= 20);
    }

    #[test]
    fn mrz_band_falls_back_to_blind_crop_on_blank_image() {
        let img = solid(300, 200, 255);
        let band = mrz_band(&img);
        assert_eq!(band.dimensions(), bottom_band(&img).dimensions());
    }

    #[test]
    fn text_bands_finds_expected_line_count() {
        let img = image_with_bottom_stripes(300, 200, 3, 8, 6);
        let gray = to_gray(&img);
        let bands = text_bands(&gray, 0, 200, 1, 30);
        assert_eq!(bands.len(), 3, "should find all three synthetic lines");
    }

    #[test]
    fn deskew_is_a_noop_within_tolerance_for_already_level_text() {
        let img = image_with_bottom_stripes(300, 150, 2, 10, 5);
        let out = deskew(&img);
        assert_eq!(out.dimensions(), img.dimensions());
    }

    #[test]
    fn rotate_rgb_by_zero_degrees_is_identity() {
        let img = solid(20, 20, 100);
        let out = rotate_rgb(&img, 0.0);
        assert_eq!(out, img);
    }

    #[test]
    fn integral_sum_matches_brute_force() {
        let img = RgbImage::from_fn(16, 12, |x, y| {
            let v = ((x * 7 + y * 13) % 250) as u8;
            Rgb([v, v, v])
        });
        let gray = to_gray(&img);
        let integral = Integral::build(&gray);
        let (x0, y0, x1, y1) = (2u32, 3u32, 9u32, 8u32);
        let expected: u64 = (y0..=y1)
            .flat_map(|y| (x0..=x1).map(move |x| (x, y)))
            .map(|(x, y)| u64::from(gray.get_pixel(x, y)[0]))
            .sum();
        assert_eq!(integral.sum(x0, y0, x1, y1), expected);
    }

    /// A light field ruled with one-pixel dark lines every `period` rows —
    /// the synthetic stand-in for the security-pattern strands printed under
    /// an MRZ. Deliberately thinner than any glyph stroke.
    fn hairline_texture(w: u32, h: u32, period: u32) -> GrayImage {
        GrayImage::from_fn(w, h, |_, y| {
            image::Luma(if y % period == 0 { [20] } else { [250] })
        })
    }

    /// The first half of the claim: a strand thinner than the kernel is gone.
    ///
    /// Kept separate from the preservation test below so a regression says
    /// *which* half broke — an over-aggressive kernel passes this one and
    /// fails that one, an inert kernel does the reverse.
    #[test]
    fn median_filter_deletes_hairline_texture() {
        let textured = hairline_texture(40, 40, 4);
        let before = mean_intensity(&textured);
        let after = mean_intensity(&median_filter(&textured, 1));
        assert!(
            before < 200.0,
            "fixture should start visibly textured, got mean {before}"
        );
        assert!(
            after > 245.0,
            "3x3 median should erase 1px strands, mean only reached {after}"
        );
    }

    /// The second half: a glyph-scale stroke is untouched. Same operator,
    /// same radius, opposite expectation.
    #[test]
    fn median_filter_preserves_glyph_scale_strokes() {
        // A 4px bar is roughly an OCR-B stroke at the width this stage runs at.
        let mut img = GrayImage::from_pixel(40, 40, image::Luma([250]));
        for y in 18..22 {
            for x in 0..40 {
                img.put_pixel(x, y, image::Luma([20]));
            }
        }
        let out = median_filter(&img, 1);
        for y in 19..21 {
            assert_eq!(
                out.get_pixel(5, y)[0],
                20,
                "stroke interior row {y} must survive the median"
            );
        }
    }

    #[test]
    fn median_filter_radius_zero_is_the_identity() {
        let img = hairline_texture(12, 12, 3);
        assert_eq!(median_filter(&img, 0), img);
    }

    #[test]
    fn median_filter_handles_degenerate_dimensions() {
        for (w, h) in [(0, 0), (1, 1), (1, 9), (9, 1)] {
            let img = GrayImage::from_pixel(w, h, image::Luma([128]));
            let out = median_filter(&img, 3);
            assert_eq!(out.dimensions(), (w, h));
        }
    }

    /// The load-bearing safety property: a mismeasured band — here a photo
    /// block read as one enormous "text row" — must not be able to produce a
    /// stroke-destroying kernel.
    #[test]
    fn median_radius_clamps_a_mismeasured_band_to_the_ceiling() {
        let mut img = GrayImage::from_pixel(200, 200, image::Luma([250]));
        for y in 100..200 {
            for x in 0..200 {
                img.put_pixel(x, y, image::Luma([20]));
            }
        }
        // A 100px "row" would ask for radius 6 unclamped.
        assert_eq!(median_radius_for_band(&img), TEXTURE_MEDIAN_MAX_RADIUS);
    }

    #[test]
    fn median_radius_always_within_bounds() {
        let cases = [
            hairline_texture(60, 60, 3),
            GrayImage::from_pixel(40, 40, image::Luma([250])),
            GrayImage::from_pixel(40, 40, image::Luma([10])),
            to_gray(&image_with_bottom_stripes(120, 90, 3, 6, 3)),
        ];
        for (i, img) in cases.iter().enumerate() {
            let r = median_radius_for_band(img);
            assert!(
                (TEXTURE_MEDIAN_MIN_RADIUS..=TEXTURE_MEDIAN_MAX_RADIUS).contains(&r),
                "case {i} produced out-of-range radius {r}"
            );
        }
    }

    #[test]
    fn texture_variants_is_empty_on_a_degenerate_image() {
        assert!(texture_variants(&RgbImage::new(0, 0), UpscaleFilter::Lanczos3).is_empty());
    }

    #[test]
    fn texture_variants_produces_exactly_one_treatment() {
        let img = image_with_bottom_stripes(240, 180, 2, 8, 4);
        assert_eq!(
            texture_variants(&img, UpscaleFilter::Lanczos3).len(),
            1,
            "one trailing variant, so the pass budget moves by exactly one"
        );
    }

    #[test]
    fn texture_variants_is_deterministic() {
        let img = image_with_bottom_stripes(240, 180, 2, 8, 4);
        let a = texture_variants(&img, UpscaleFilter::Lanczos3);
        let b = texture_variants(&img, UpscaleFilter::Lanczos3);
        assert_eq!(a, b);
    }

    fn mean_intensity(gray: &GrayImage) -> f64 {
        let n = u64::from(gray.width()) * u64::from(gray.height());
        if n == 0 {
            return 0.0;
        }
        let sum: u64 = gray.pixels().map(|p| u64::from(p[0])).sum();
        sum as f64 / n as f64
    }
}
