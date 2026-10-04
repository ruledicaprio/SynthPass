//! `synthpass-gen` — a deterministic, pure-Rust synthetic identity-document
//! data-page generator for the five ICAO 9303 machine-readable formats (TD1,
//! TD2, TD3, MRV-A and MRV-B).
//!
//! Given a seed and a small set of parameters, [`generate`] produces a
//! rendered document-style image plus perfectly accurate ground-truth labels
//! for every field, including a checksum-valid MRZ. This is **synthetic-data
//! infrastructure for testing and benchmarking identity-document extraction
//! pipelines** — it is not a tool for imitating genuine documents, and that
//! posture is enforced at the artifact level, not just in this doc comment:
//!
//! - **Unconditional watermark.** Every render carries a "SYNTHETIC /
//!   SPECIMEN" watermark, drawn from a hand-authored bitmap font baked into
//!   the binary (see [`render`]). It cannot be disabled through
//!   [`GeneratorConfig`] and does not depend on any font file being present.
//! - **Generic, non-country template.** The background is a plain frame with
//!   no national emblem, coat of arms, or issuing-country branding of any
//!   kind — regardless of which issuing-country *code* a given identity
//!   happens to carry in its MRZ/VIZ text.
//! - **No real PII, ever.** Identities are drawn deterministically from a
//!   seed out of small, hand-authored pools of clearly fictional names (see
//!   [`data`]) — never real people, never sourced from real documents.
//!   Cyrillic-script issuing states (`RUS`/`SRB`/`BGR`/`MKD`/`UKR`/`BLR`) draw
//!   a native-script name and store its ICAO 9303 Part 3 §6 B transliteration
//!   (`mrz::transliterate_cyrillic`) as the Latin `surname`/`given_names` the
//!   MRZ carries; the native strings are on [`Labels`] too.
//!
//! ## MRZ correctness
//!
//! The rendered MRZ is assembled locally in [`mrz_line`] by reusing
//! `mrz::check_digit` (the standalone `mrz` crate's checksum oracle) rather
//! than duplicating ICAO 9303 checksum math. The keystone test in
//! `tests/mrz_roundtrip.rs` parses generated output back through
//! `mrz::parse_td3` and asserts every check digit is valid.
//!
//! ## Determinism
//!
//! [`generate`] is a pure function of `(passport, config)`, and
//! [`data::generate_passport`] is a pure function of `config.seed`: the same
//! seed always produces byte-identical identity data and pixels. See
//! `tests/determinism.rs`.
//!
//! ## Layouts
//!
//! The renderer and the labels take a [`ValidatedLayout`]: the portrait and
//! visual-zone rectangles of one format, checked at construction
//! ([`ValidatedLayout::try_from_spec`]) so that nothing can sit on the
//! watermark or the MRZ, no two fields overlap, and every value the generator
//! can draw stays inside its labelled box. The functions without a layout
//! argument ([`generate`], [`generate_with`], [`generate_from_seed`]) use the
//! built-in layout of `config.document_type`; the `_with_layout` variants take
//! another one.

pub mod data;
pub mod degrade;
pub mod fonts;
pub mod labels;
pub mod layout;
pub mod model;
mod mrz_line;
pub mod render;

pub use labels::{FieldLabel, Labels, OccludedKind, OccludedSpan};
pub use layout::{LayoutError, LayoutSpec, ValidatedLayout};
pub use model::{DocumentType, GeneratorConfig, Passport, Sex, VizFontChoice};
pub use render::{RedactSpan, RedactStyle, RenderOptions};

/// Generate a synthetic document data page: a fictional identity drawn
/// from `config.seed`, rendered into an image, alongside its ground-truth
/// [`Labels`].
///
/// The two ethics guardrails described in the module docs (the synthetic
/// watermark and the generic template) render unconditionally as part of
/// this call — there is no configuration path that skips them.
///
/// Supports all five formats — TD1, TD2, TD3, MRV-A and MRV-B — chosen by
/// `config.document_type`, on that format's built-in layout.
pub fn generate(passport: &Passport, config: &GeneratorConfig) -> (image::DynamicImage, Labels) {
    generate_with(passport, config, &RenderOptions::default())
        .expect("default render options are valid")
}

/// [`generate`] with render options, on `config.document_type`'s built-in
/// layout.
pub fn generate_with(
    passport: &Passport,
    config: &GeneratorConfig,
    options: &RenderOptions,
) -> Result<(image::DynamicImage, Labels), String> {
    generate_with_layout(
        passport,
        config,
        ValidatedLayout::builtin(config.document_type),
        options,
    )
}

/// [`generate_with`] on an explicit [`ValidatedLayout`]. Fails when the
/// layout's format is not `config.document_type`, so a caller cannot label one
/// format's MRZ with another's rectangles.
pub fn generate_with_layout(
    passport: &Passport,
    config: &GeneratorConfig,
    layout: &ValidatedLayout,
    options: &RenderOptions,
) -> Result<(image::DynamicImage, Labels), String> {
    if layout.format() != config.document_type {
        return Err(format!(
            "layout is for {} but the config asks for {}",
            layout.format().as_str(),
            config.document_type.as_str()
        ));
    }
    let labels = labels::build_labels_with_layout(passport, layout);
    let mut labels = labels;
    labels.viz_font = config
        .viz_font
        .map(|choice| choice.resolve(config.seed).name());
    if let Some(span) = options.redact {
        labels.occluded.push(OccludedSpan {
            line: span.line(),
            first: span.first(),
            last: span.last(),
            kind: match span.style() {
                RedactStyle::FillBlack | RedactStyle::FillWhite | RedactStyle::FillGrey => {
                    labels::OccludedKind::Fill
                }
                _ => labels::OccludedKind::Blur,
            },
        });
    }
    let image = render::render_with_layout(passport, &labels, layout, options)?;
    Ok((image, labels))
}

/// Convenience: generate a fictional document from `config.seed` and render
/// it in one call.
///
/// The document type is determined by `config.document_type` (defaults to TD3).
pub fn generate_from_seed(config: &GeneratorConfig) -> (image::DynamicImage, Labels, Passport) {
    let passport = data::generate_passport(config);
    let (image, labels) = generate(&passport, config);
    (image, labels, passport)
}

/// [`generate_from_seed`] on an explicit [`ValidatedLayout`]; the identity
/// drawn for a seed does not depend on the layout. Fails when the layout's
/// format is not `config.document_type`.
pub fn generate_from_seed_with_layout(
    config: &GeneratorConfig,
    layout: &ValidatedLayout,
) -> Result<(image::DynamicImage, Labels, Passport), String> {
    let passport = data::generate_passport(config);
    let (image, labels) =
        generate_with_layout(&passport, config, layout, &RenderOptions::default())?;
    Ok((image, labels, passport))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORMATS: [DocumentType; 5] = [
        DocumentType::TD1,
        DocumentType::TD2,
        DocumentType::TD3,
        DocumentType::MrvA,
        DocumentType::MrvB,
    ];

    #[test]
    fn layout_entry_points_on_the_builtin_match_the_plain_ones() {
        for format in FORMATS {
            let config = GeneratorConfig::with_document_type(7, format);
            let layout = ValidatedLayout::builtin(format);
            let (plain_image, plain_labels, plain_passport) = generate_from_seed(&config);
            let (image, labels, passport) =
                generate_from_seed_with_layout(&config, layout).expect("matching format");
            assert_eq!(image.to_rgb8().into_raw(), plain_image.to_rgb8().into_raw());
            assert_eq!(labels, plain_labels);
            assert_eq!(passport, plain_passport);
        }
    }

    #[test]
    fn a_layout_for_another_format_is_refused() {
        let config = GeneratorConfig::with_document_type(7, DocumentType::TD3);
        let layout = ValidatedLayout::builtin(DocumentType::TD1);
        let err = generate_from_seed_with_layout(&config, layout).unwrap_err();
        assert!(err.contains("TD1") && err.contains("TD3"), "{err}");
        let passport = data::generate_passport(&config);
        assert!(
            generate_with_layout(&passport, &config, layout, &RenderOptions::default()).is_err()
        );
    }
}
