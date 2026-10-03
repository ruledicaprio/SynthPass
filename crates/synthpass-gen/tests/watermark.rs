//! The synthetic watermark must render regardless of the `embedded-fonts`
//! feature (on by default since the fonts are vendored; off falls back to
//! placeholder bars) — it is a mandatory ethics guardrail, not a
//! font-dependent nicety.
//!
//! For every built-in format the band (`layout::for_format(fmt).watermark`) must
//! hold only the exact watermark colour or the exact background, at least one
//! watermark pixel, and the same watermark mask for every seed — so nothing
//! seed-dependent draws into the band.

use synthpass_gen::layout::{for_format, Rect, WATERMARK};
use synthpass_gen::{
    data::generate_passport, generate, generate_from_seed, DocumentType, GeneratorConfig,
};

/// Background fill and watermark ink; both are private consts in
/// `src/render.rs` (`BACKGROUND`, `WATERMARK_COLOR`), duplicated here on purpose.
const BACKGROUND: [u8; 3] = [244, 243, 236];
const WATERMARK_COLOR: [u8; 3] = [176, 48, 48];

const FORMATS: [DocumentType; 5] = [
    DocumentType::TD1,
    DocumentType::TD2,
    DocumentType::TD3,
    DocumentType::MrvA,
    DocumentType::MrvB,
];
const SEEDS: [u64; 4] = [0, 1, 42, 1000];

#[test]
fn watermark_pixels_present_without_embedded_fonts() {
    let cfg = GeneratorConfig::new(77);
    let passport = generate_passport(&cfg);
    let (image, _labels) = generate(&passport, &cfg);
    let rgb = image.to_rgb8();

    let mut non_background_pixels = 0usize;
    for y in WATERMARK.y..(WATERMARK.y + WATERMARK.height) {
        for x in WATERMARK.x..(WATERMARK.x + WATERMARK.width) {
            if rgb.get_pixel(x, y).0 != BACKGROUND {
                non_background_pixels += 1;
            }
        }
    }

    assert!(
        non_background_pixels > 0,
        "expected the watermark band to contain non-background pixels"
    );
}

/// Row-major mask of watermark-coloured pixels in `band`; panics on any pixel
/// that is neither the watermark colour nor the background.
fn watermark_mask(fmt: DocumentType, seed: u64, band: Rect) -> Vec<bool> {
    let cfg = GeneratorConfig::with_document_type(seed, fmt);
    let (image, _labels, _passport) = generate_from_seed(&cfg);
    let rgb = image.to_rgb8();
    let mut mask = Vec::with_capacity((band.width * band.height) as usize);
    for y in band.y..(band.y + band.height) {
        for x in band.x..(band.x + band.width) {
            let px = rgb.get_pixel(x, y).0;
            assert!(
                px == WATERMARK_COLOR || px == BACKGROUND,
                "{fmt:?} seed {seed}: pixel ({x}, {y}) = {px:?} is neither watermark nor background"
            );
            mask.push(px == WATERMARK_COLOR);
        }
    }
    mask
}

/// Core of the per-format check, with the band as a parameter.
fn check_band(fmt: DocumentType, band: Rect) {
    let reference = watermark_mask(fmt, SEEDS[0], band);
    assert!(
        reference.iter().any(|&on| on),
        "{fmt:?}: watermark band holds no watermark pixel"
    );
    for &seed in &SEEDS[1..] {
        let mask = watermark_mask(fmt, seed, band);
        assert!(
            mask == reference,
            "{fmt:?}: watermark mask differs between seed {} and seed {seed}",
            SEEDS[0]
        );
    }
}

#[test]
fn watermark_band_is_exact_and_seed_independent_for_every_format() {
    for fmt in FORMATS {
        check_band(fmt, for_format(fmt).watermark);
    }
}
