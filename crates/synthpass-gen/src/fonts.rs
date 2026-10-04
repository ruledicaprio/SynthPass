//! Glyph loading, gated behind the `embedded-fonts` Cargo feature — **on by
//! default**, because all OFL fonts are vendored under `fonts/` (see
//! `fonts/README.md`). With the feature on, the fonts are baked into the
//! binary via `include_bytes!`. Under `--no-default-features` it is off:
//! [`load_fonts`] returns [`FontError::NotEmbedded`] and `render` degrades
//! gracefully to placeholder bars — see `render.rs`.

use ab_glyph::FontArc;

/// The closed VIZ font set admitted by ADR-0030. Rendering still uses PT Sans.
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

    /// Stable name reserved for the future opt-in interface.
    pub const fn name(self) -> &'static str {
        match self {
            Self::PtSans => "pt-sans",
            Self::LiberationSans => "liberation-sans",
            Self::SourceSans3 => "source-sans-3",
            Self::LiberationSerif => "liberation-serif",
            Self::LiberationMono => "liberation-mono",
        }
    }

    /// Admission factor relative to the renderer's 70%-height scale.
    pub const fn scale(self) -> f32 {
        match self {
            Self::PtSans => 1.0,
            Self::LiberationSans => 0.7510,
            Self::SourceSans3 => 0.9965,
            Self::LiberationSerif => 0.5690,
            // Avoid a one-pixel floating-point top escape for Cyrillic Й.
            // Within 0.000102 of the bisection limit; tested at every builtin.
            Self::LiberationMono => 0.34634,
        }
    }
}

#[cfg(feature = "embedded-fonts")]
static OCR_B_BYTES: &[u8] = include_bytes!("../fonts/ocr-b.ttf");
#[cfg(feature = "embedded-fonts")]
static SANS_BYTES: &[u8] = include_bytes!("../fonts/sans.ttf");

/// Embedded fonts. The renderer still uses the unchanged PT Sans/OCR-B pair.
pub struct Fonts {
    /// Monospaced OCR-B-style font for the MRZ band.
    pub mrz: FontArc,
    /// Proportional sans font for the human-readable VIZ fields.
    pub viz: FontArc,
    alternatives: [FontArc; 4],
}

impl Fonts {
    /// Access an embedded VIZ family without changing the default rendering font.
    pub fn viz_font(&self, font: VizFont) -> &FontArc {
        match font {
            VizFont::PtSans => &self.viz,
            VizFont::LiberationSans => &self.alternatives[0],
            VizFont::SourceSans3 => &self.alternatives[1],
            VizFont::LiberationSerif => &self.alternatives[2],
            VizFont::LiberationMono => &self.alternatives[3],
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
        let parse = |bytes: &'static [u8]| {
            FontArc::try_from_slice(bytes).map_err(|_| FontError::NotEmbedded)
        };
        let alternatives = [
            parse(include_bytes!("../fonts/liberation-sans.ttf"))?,
            parse(include_bytes!("../fonts/source-sans-3.ttf"))?,
            parse(include_bytes!("../fonts/liberation-serif.ttf"))?,
            parse(include_bytes!("../fonts/liberation-mono.ttf"))?,
        ];
        Ok(Fonts {
            mrz,
            viz,
            alternatives,
        })
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
        // With the feature on, both vendored OFL fonts must actually parse —
        // a corrupt or mismatched TTF would silently fall back to placeholder
        // bars instead of failing loudly, which is worse than a build error.
        assert!(load_fonts().is_ok());
    }
}
