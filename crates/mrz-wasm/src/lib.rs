//! WebAssembly bindings powering the GitHub Pages demo: the ICAO 9303 parser
//! (`mrz`) and the deterministic image preprocessing (`synthpass-imageprep`)
//! the native pipeline uses.
//!
//! Everything runs inside the visitor's browser. The image is decoded by the
//! browser and OCR'd by tesseract.js locally; this module prepares the pixels
//! and validates the MRZ. No document data ever leaves the page.
//!
//! # Why preprocessing is here
//!
//! `web/scan.js` used to be a hand-written JavaScript *port* of
//! `synthpass_imageprep::preprocess` — the same band crop, percentile contrast
//! stretch and Otsu threshold, written twice and drifting constant by
//! constant. The port is gone: the browser now runs the identical Rust the
//! native pipeline runs, and there is one implementation to fix when
//! preprocessing changes.
//!
//! Pixels cross the boundary as raw RGBA, the layout a canvas `ImageData`
//! already has, so neither side pays for an encode/decode round trip.

use synthpass_imageprep::{preprocess, UpscaleFilter};

/// The resampling filter the browser upscales bands with.
///
/// tesseract's OCR-B model measurably prefers the smoother filter: same
/// checksum-valid rate as `Lanczos3` over the 190-specimen corpus (125), but
/// 150/153 correct line-1 fields against 143/153, and ~9% faster. Line 1
/// carries no check digit in any ICAO format, so on those fields the
/// recognizer is the only protection there is. `synthpass-ocr` runs `ocrs`
/// and picks the opposite — see [`UpscaleFilter`].
const BROWSER_UPSCALE_FILTER: UpscaleFilter = UpscaleFilter::Triangle;
use wasm_bindgen::prelude::*;

/// Scan free-form OCR text for an ICAO 9303 MRZ (TD3 passport or TD1 ID card),
/// parse it and verify every check digit.
///
/// Returns a JSON string: `{"ok": true, "valid": bool, "sequence_completeness": {...},
/// ...MrzData}` on success or `{"ok": false, "error": "..."}` when no valid MRZ is found.
#[wasm_bindgen]
pub fn parse_mrz_text(text: &str) -> String {
    match mrz::find_and_parse(text) {
        Ok(data) => {
            let mut v = serde_json::to_value(&data).expect("MrzData serializes");
            v["ok"] = serde_json::Value::Bool(true);
            v["valid"] = serde_json::Value::Bool(data.valid());
            v["sequence_completeness"] = serde_json::to_value(data.sequence_completeness())
                .expect("SequenceCompleteness serializes");
            v.to_string()
        }
        Err(e) => serde_json::json!({ "ok": false, "error": e.to_string() }).to_string(),
    }
}

/// Compute a single ICAO 9303 check digit for a field (utility/debug helper).
#[wasm_bindgen]
pub fn icao_check_digit(field: &str) -> i32 {
    mrz::check_digit(field).map(|d| d as i32).unwrap_or(-1)
}

/// The ICAO 9303 MRZ character set, for the recognizer's character whitelist.
///
/// Exported so the demo stops keeping its own copy of the string: this is the
/// same constant the native engine constrains recognition to.
#[wasm_bindgen]
pub fn mrz_charset() -> String {
    synthpass_imageprep::MRZ_CHARSET.to_string()
}

/// One preprocessed image, in the RGBA layout a canvas `ImageData` wants.
#[wasm_bindgen]
pub struct ImageVariant {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[wasm_bindgen]
impl ImageVariant {
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Copies the pixels out to a `Uint8ClampedArray`, ready for
    /// `new ImageData(rgba, width, height)`.
    #[wasm_bindgen(getter)]
    pub fn rgba(&self) -> Vec<u8> {
        self.rgba.clone()
    }
}

/// An ordered list of [`ImageVariant`]s.
///
/// A plain struct with `len`/`get` rather than a serialized array: the pixel
/// buffers are megabytes, and this way JavaScript pulls each one across only
/// when it is about to OCR it, instead of materializing every variant up front.
#[wasm_bindgen]
pub struct VariantSet {
    items: Vec<ImageVariant>,
}

#[wasm_bindgen]
impl VariantSet {
    #[wasm_bindgen(getter)]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[wasm_bindgen(getter)]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The variant at `index`, or `undefined` if out of range.
    pub fn get(&self, index: usize) -> Option<ImageVariant> {
        self.items.get(index).map(|v| ImageVariant {
            width: v.width,
            height: v.height,
            rgba: v.rgba.clone(),
        })
    }
}

/// The band crop with no tonal treatment, upscaled — see
/// [`preprocess::plain_band`] for why this is separate from
/// [`mrz_variants`] and why it belongs first on a recognizer trained for
/// OCR-B.
///
/// `rgba` is a canvas `ImageData`'s buffer: 4 bytes per pixel, `width * height`
/// pixels. Returns `None` if the buffer length disagrees with the dimensions.
#[wasm_bindgen]
pub fn plain_band(rgba: &[u8], width: u32, height: u32) -> Option<ImageVariant> {
    let image = rgba_to_rgb(rgba, width, height)?;
    Some(to_variant(preprocess::plain_band(
        &image,
        BROWSER_UPSCALE_FILTER,
    )))
}

/// The ordered MRZ retry variants — the identical sequence
/// `synthpass_ocr::NativeOcr` retries with: contrast-stretched band,
/// binarized band, upscaled full page, then, only when the row-density search
/// actually finds a different band than the blind crop, three more targeting
/// dense/bilingual scans (contrast, local threshold, and a deskewed pass).
///
/// Retries are additive by contract: a variant may add a checksum-valid read,
/// never break one that already worked. The caller stops at the first read
/// whose ICAO check digits verify.
///
/// `rgba` is a canvas `ImageData`'s buffer. Returns an empty set if the buffer
/// length disagrees with the dimensions.
#[wasm_bindgen]
pub fn mrz_variants(rgba: &[u8], width: u32, height: u32) -> VariantSet {
    let Some(image) = rgba_to_rgb(rgba, width, height) else {
        return VariantSet { items: Vec::new() };
    };
    VariantSet {
        items: preprocess::mrz_variants(&image, BROWSER_UPSCALE_FILTER)
            .into_iter()
            .map(to_variant)
            .collect(),
    }
}

/// Canvas RGBA to the `RgbImage` the preprocessing works on, dropping alpha.
///
/// `None` rather than a panic on a length mismatch: this is called with
/// whatever a browser handed us, and a wasm panic would take the page's
/// module instance down with it.
fn rgba_to_rgb(rgba: &[u8], width: u32, height: u32) -> Option<image::RgbImage> {
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if width == 0 || height == 0 || rgba.len() != expected {
        return None;
    }
    let rgb: Vec<u8> = rgba
        .chunks_exact(4)
        .flat_map(|px| [px[0], px[1], px[2]])
        .collect();
    image::RgbImage::from_raw(width, height, rgb)
}

/// Back to RGBA, opaque, for `new ImageData(...)`.
fn to_variant(image: image::RgbImage) -> ImageVariant {
    let (width, height) = image.dimensions();
    let rgba = image
        .pixels()
        .flat_map(|p| [p.0[0], p.0[1], p.0[2], 255])
        .collect();
    ImageVariant {
        width,
        height,
        rgba,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32) -> Vec<u8> {
        (0..width * height)
            .flat_map(|i| {
                let v = (i % 256) as u8;
                [v, v, v, 255]
            })
            .collect()
    }

    #[test]
    fn rgba_round_trips_through_rgb() {
        let rgba = solid(4, 3);
        let rgb = rgba_to_rgb(&rgba, 4, 3).expect("well-formed buffer converts");
        let back = to_variant(rgb);
        assert_eq!(back.width, 4);
        assert_eq!(back.height, 3);
        // Alpha is dropped on the way in and restored opaque on the way out,
        // so an already-opaque buffer survives unchanged.
        assert_eq!(back.rgba, rgba);
    }

    #[test]
    fn a_length_mismatch_returns_none_rather_than_panicking() {
        assert!(rgba_to_rgb(&solid(4, 3), 5, 3).is_none());
        assert!(rgba_to_rgb(&[], 0, 0).is_none());
        assert!(rgba_to_rgb(&solid(2, 2), 2, 0).is_none());
    }

    #[test]
    fn variants_are_produced_and_none_are_degenerate() {
        let set = mrz_variants(&solid(80, 60), 80, 60);
        assert!(!set.is_empty(), "the blind-crop variants always exist");
        for i in 0..set.len() {
            let v = set.get(i).expect("index in range");
            assert!(v.width > 0 && v.height > 0);
            assert_eq!(v.rgba.len(), (v.width * v.height * 4) as usize);
        }
        assert!(set.get(set.len()).is_none(), "out of range yields None");
    }

    #[test]
    fn plain_band_is_the_untreated_crop() {
        let v = plain_band(&solid(80, 60), 80, 60).expect("well-formed buffer");
        assert!(v.width > 0 && v.height > 0);
        assert_eq!(v.rgba.len(), (v.width * v.height * 4) as usize);
    }

    #[test]
    fn the_charset_is_the_engines_own() {
        assert_eq!(mrz_charset(), synthpass_imageprep::MRZ_CHARSET);
        assert_eq!(mrz_charset().len(), 37); // A-Z, 0-9, '<'
    }
}
