//! Glyph loading, gated behind the `embedded-fonts` Cargo feature — **on by
//! default**, because all OFL fonts are vendored under `fonts/` (see
//! `fonts/README.md`). With the feature on, the fonts are baked into the
//! binary via `include_bytes!`. Under `--no-default-features` it is off:
//! [`load_fonts`] returns [`FontError::NotEmbedded`] and `render` degrades
//! gracefully to placeholder bars — see `render.rs`.

use ab_glyph::FontArc;
pub(crate) mod scaling;
pub use scaling::{FieldScale, FieldScales};

/// The closed VIZ font set admitted by ADR-0030. PT Sans remains the default.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum VizFont {
    #[default]
    PtSans,
    LiberationSans,
    SourceSans3,
    LiberationSerif,
    LiberationMono,
}

impl VizFont {
    /// Every member of the closed set, in stable order.
    pub const ALL: [Self; 5] = [
        Self::PtSans,
        Self::LiberationSans,
        Self::SourceSans3,
        Self::LiberationSerif,
        Self::LiberationMono,
    ];

    /// Stable name for the opt-in interface and labels.
    pub const fn name(self) -> &'static str {
        match self {
            Self::PtSans => "pt-sans",
            Self::LiberationSans => "liberation-sans",
            Self::SourceSans3 => "source-sans-3",
            Self::LiberationSerif => "liberation-serif",
            Self::LiberationMono => "liberation-mono",
        }
    }
}

#[cfg(feature = "embedded-fonts")]
static OCR_B_BYTES: &[u8] = include_bytes!("../fonts/ocr-b.ttf");
#[cfg(feature = "embedded-fonts")]
static SANS_BYTES: &[u8] = include_bytes!("../fonts/sans.ttf");

/// Embedded fonts. Default rendering uses the unchanged PT Sans/OCR-B pair.
pub struct Fonts {
    /// Monospaced OCR-B-style font for the MRZ band.
    pub mrz: FontArc,
    /// Proportional sans font for the human-readable VIZ fields.
    pub viz: FontArc,
}

impl Fonts {
    /// Access an embedded VIZ family without changing the default rendering font.
    pub fn viz_font(&self, font: VizFont) -> Result<&FontArc, FontError> {
        if font == VizFont::PtSans {
            return Ok(&self.viz);
        }
        #[cfg(feature = "embedded-fonts")]
        {
            use std::sync::OnceLock;
            static LIBERATION_SANS: OnceLock<Result<FontArc, FontError>> = OnceLock::new();
            static SOURCE_SANS: OnceLock<Result<FontArc, FontError>> = OnceLock::new();
            static LIBERATION_SERIF: OnceLock<Result<FontArc, FontError>> = OnceLock::new();
            static LIBERATION_MONO: OnceLock<Result<FontArc, FontError>> = OnceLock::new();
            let (cache, bytes): (&OnceLock<Result<FontArc, FontError>>, &'static [u8]) = match font
            {
                VizFont::PtSans => return Ok(&self.viz),
                VizFont::LiberationSans => (
                    &LIBERATION_SANS,
                    include_bytes!("../fonts/liberation-sans.ttf"),
                ),
                VizFont::SourceSans3 => {
                    (&SOURCE_SANS, include_bytes!("../fonts/source-sans-3.ttf"))
                }
                VizFont::LiberationSerif => (
                    &LIBERATION_SERIF,
                    include_bytes!("../fonts/liberation-serif.ttf"),
                ),
                VizFont::LiberationMono => (
                    &LIBERATION_MONO,
                    include_bytes!("../fonts/liberation-mono.ttf"),
                ),
            };
            cache
                .get_or_init(|| FontArc::try_from_slice(bytes).map_err(|_| FontError::NotEmbedded))
                .as_ref()
                .map_err(|error| *error)
        }
        #[cfg(not(feature = "embedded-fonts"))]
        {
            Err(FontError::NotEmbedded)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontError {
    /// The `embedded-fonts` feature is off, or the embedded font bytes failed
    /// to parse. Either way, no real glyphs are available this build.
    NotEmbedded,
}

impl core::fmt::Display for FontError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FontError::NotEmbedded => write!(
                f,
                "fonts not embedded (build with --features embedded-fonts, see fonts/README.md)"
            ),
        }
    }
}

impl std::error::Error for FontError {}

/// Attempt to load the MRZ + VIZ fonts. Fails only when the `embedded-fonts`
/// feature is off (`--no-default-features`) or an embedded font fails to parse.
pub fn load_fonts() -> Result<Fonts, FontError> {
    #[cfg(feature = "embedded-fonts")]
    {
        let mrz = FontArc::try_from_slice(OCR_B_BYTES).map_err(|_| FontError::NotEmbedded)?;
        let viz = FontArc::try_from_slice(SANS_BYTES).map_err(|_| FontError::NotEmbedded)?;
        Ok(Fonts { mrz, viz })
    }
    #[cfg(not(feature = "embedded-fonts"))]
    {
        Err(FontError::NotEmbedded)
    }
}

#[cfg(all(test, feature = "embedded-fonts"))]
mod admission;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(feature = "embedded-fonts"))]
    fn not_embedded_without_feature() {
        // `--no-default-features` build: the fonts are vendored but not
        // compiled in, so this must degrade gracefully rather than panic.
        assert!(matches!(load_fonts(), Err(FontError::NotEmbedded)));
    }

    #[test]
    #[cfg(feature = "embedded-fonts")]
    fn embedded_fonts_parse_successfully() {
        // With the feature on, the vendored OFL fonts must actually parse —
        // a corrupt or mismatched TTF would silently fall back to placeholder
        // bars instead of failing loudly, which is worse than a build error.
        let fonts = load_fonts().expect("primary embedded fonts");
        for font in VizFont::ALL {
            assert!(fonts.viz_font(font).is_ok(), "{}", font.name());
        }
    }
}
