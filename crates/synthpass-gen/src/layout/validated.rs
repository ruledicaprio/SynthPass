//! The validated layout model (ADR-0022 Decisions 3-7).
//!
//! A [`LayoutSpec`] is everything a layout can say: one ICAO format and eleven
//! rectangles. A [`FormatFrame`] is everything it cannot: the canvas, frame,
//! watermark band and MRZ, owned by the engine per format. A
//! [`ValidatedLayout`] is a spec that passed [`ValidatedLayout::try_from_spec`],
//! the only way to build one — the renderer and the labels accept nothing else,
//! so a library caller cannot skip the checks.
//!
//! The checks fail closed, in a fixed order, and report the first violation
//! (see [`LayoutError`]):
//!
//! 1. per rectangle, in [`LayoutField::ALL`] order: not empty, no arithmetic
//!    overflow, inside the permitted area (the frame, down to the lowest bottom
//!    edge among the format's built-in rectangles), and (visual-zone fields) at least as
//!    tall as the shortest visual-zone row among the five built-ins;
//! 2. no two rectangles overlap;
//! 3. every visual-zone rectangle holds the widest value the generator can draw
//!    into that field, measured with the renderer's own text extent (see
//!    `fit.rs`).

use std::fmt;
use std::sync::OnceLock;

use super::{for_format, PageLayout, Rect, FRAME_THICKNESS};
use crate::fonts::load_fonts;
use crate::model::DocumentType;

pub(crate) mod fit;

/// The eleven placed rectangles of a layout, in the fixed order used by every
/// check, error and [`ValidatedLayout::canonical_bytes`]: the portrait, then the
/// ten visual-zone text fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutField {
    Portrait,
    DocumentType,
    IssuingCountry,
    Surname,
    GivenNames,
    DocumentNumber,
    Nationality,
    DateOfBirth,
    Sex,
    DateOfExpiry,
    PersonalNumber,
}

impl LayoutField {
    /// All eleven fields in canonical order.
    pub const ALL: [LayoutField; 11] = [
        LayoutField::Portrait,
        LayoutField::DocumentType,
        LayoutField::IssuingCountry,
        LayoutField::Surname,
        LayoutField::GivenNames,
        LayoutField::DocumentNumber,
        LayoutField::Nationality,
        LayoutField::DateOfBirth,
        LayoutField::Sex,
        LayoutField::DateOfExpiry,
        LayoutField::PersonalNumber,
    ];

    /// The field's snake_case name — the key a layout file will use, and the
    /// name every [`LayoutError`] cites.
    pub fn name(self) -> &'static str {
        match self {
            LayoutField::Portrait => "portrait",
            LayoutField::DocumentType => "document_type",
            LayoutField::IssuingCountry => "issuing_country",
            LayoutField::Surname => "surname",
            LayoutField::GivenNames => "given_names",
            LayoutField::DocumentNumber => "document_number",
            LayoutField::Nationality => "nationality",
            LayoutField::DateOfBirth => "date_of_birth",
            LayoutField::Sex => "sex",
            LayoutField::DateOfExpiry => "date_of_expiry",
            LayoutField::PersonalNumber => "personal_number",
        }
    }

    /// Whether the field draws text (every field but the portrait).
    pub fn is_visual_zone(self) -> bool {
        self != LayoutField::Portrait
    }
}

impl fmt::Display for LayoutField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// What a layout file can say: the format plus eleven rectangles. There is no
/// key for the canvas, the frame, the watermark or the MRZ, and none for any
/// text (ADR-0022 Decisions 1-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutSpec {
    pub format: DocumentType,
    pub portrait: Rect,
    pub document_type: Rect,
    pub issuing_country: Rect,
    pub surname: Rect,
    pub given_names: Rect,
    pub document_number: Rect,
    pub nationality: Rect,
    pub date_of_birth: Rect,
    pub sex: Rect,
    pub date_of_expiry: Rect,
    pub personal_number: Rect,
}

impl LayoutSpec {
    /// The eleven rectangles of `format`'s built-in layout.
    pub fn builtin(format: DocumentType) -> Self {
        let page = for_format(format);
        Self {
            format,
            portrait: page.portrait,
            document_type: page.document_type,
            issuing_country: page.issuing_country,
            surname: page.surname,
            given_names: page.given_names,
            document_number: page.document_number,
            nationality: page.nationality,
            date_of_birth: page.date_of_birth,
            sex: page.sex,
            date_of_expiry: page.date_of_expiry,
            personal_number: page.personal_number,
        }
    }

    /// The rectangle of `field`.
    pub fn rect(&self, field: LayoutField) -> Rect {
        match field {
            LayoutField::Portrait => self.portrait,
            LayoutField::DocumentType => self.document_type,
            LayoutField::IssuingCountry => self.issuing_country,
            LayoutField::Surname => self.surname,
            LayoutField::GivenNames => self.given_names,
            LayoutField::DocumentNumber => self.document_number,
            LayoutField::Nationality => self.nationality,
            LayoutField::DateOfBirth => self.date_of_birth,
            LayoutField::Sex => self.sex,
            LayoutField::DateOfExpiry => self.date_of_expiry,
            LayoutField::PersonalNumber => self.personal_number,
        }
    }

    /// The eleven rectangles in [`LayoutField::ALL`] order.
    pub fn rects(&self) -> [Rect; 11] {
        LayoutField::ALL.map(|field| self.rect(field))
    }
}

/// The part of a page the engine owns, per format (ADR-0022 Decision 3): exactly
/// the values [`for_format`] returns, which are what the synthetic benchmarks
/// and the M4 gate measure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatFrame {
    pub width: u32,
    pub height: u32,
    /// Thickness of the plain frame drawn around the canvas.
    pub frame_thickness: u32,
    /// The unconditional "SYNTHETIC / SPECIMEN" band, drawn last.
    pub watermark: Rect,
    /// MRZ line rectangles, top to bottom: 2 for TD2/TD3/MRV, 3 for TD1.
    pub mrz_lines: Vec<Rect>,
    /// MRZ cells per line.
    pub mrz_chars: u32,
    /// Where a layout rectangle may sit: inside the frame, and no lower than
    /// the lowest bottom edge among the format's eleven built-in rectangles
    /// (portrait included), so no layout brings the visual zone closer to the
    /// MRZ than a built-in does (ADR-0022 Decision 3). That edge is derived
    /// from [`for_format`] and never lies below the watermark top, so nothing
    /// lands on the watermark or the MRZ either.
    pub permitted: Rect,
}

/// The lowest bottom edge (`y + height`, exclusive) among the eleven built-in
/// rectangles of `format`, portrait included, clamped to the watermark top.
fn permitted_bottom(format: DocumentType) -> u32 {
    let spec = LayoutSpec::builtin(format);
    let lowest = spec
        .rects()
        .into_iter()
        .filter_map(|rect| rect.bottom())
        .max()
        .unwrap_or(0);
    lowest.min(for_format(format).watermark.y)
}

/// The engine-owned frame of `format`.
pub fn frame_for(format: DocumentType) -> FormatFrame {
    let page = for_format(format);
    let t = FRAME_THICKNESS;
    FormatFrame {
        width: page.width,
        height: page.height,
        frame_thickness: t,
        permitted: Rect::new(
            t,
            t,
            page.width.saturating_sub(2 * t),
            permitted_bottom(format).saturating_sub(t),
        ),
        watermark: page.watermark,
        mrz_lines: page.mrz_lines,
        mrz_chars: page.mrz_chars,
    }
}

/// The first rule a [`LayoutSpec`] broke. `Display` names the field and the
/// rule, e.g. `surname: overlaps given_names`; [`rule`](Self::rule) and
/// [`field`](Self::field) give the same two facts to code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    /// The rectangle has zero width or height.
    EmptyRect { field: LayoutField },
    /// `x + width` or `y + height` does not fit in `u32`.
    Overflow { field: LayoutField },
    /// The rectangle leaves the permitted area (frame, or below the lowest
    /// built-in rectangle of its format).
    OutsidePermittedArea { field: LayoutField, permitted: Rect },
    /// The two rectangles share a pixel. `field` is the earlier in
    /// [`LayoutField::ALL`] order.
    Overlap {
        field: LayoutField,
        other: LayoutField,
    },
    /// A visual-zone rectangle shorter than the shortest built-in row.
    TooShort {
        field: LayoutField,
        height: u32,
        minimum: u32,
    },
    /// The widest value the generator can draw into the field puts ink
    /// outside the rectangle.
    TextDoesNotFit {
        field: LayoutField,
        /// Ink bounds of the widest value, `[min_x, min_y, max_x, max_y]`
        /// (`max` exclusive), in canvas pixels.
        ink: [i32; 4],
        rect: Rect,
    },
    /// The fit check cannot measure glyphs in this build (no embedded font),
    /// and the spec is not a built-in layout.
    FitCheckUnavailable,
    /// The build embeds the fonts, but loading them failed: the fit check
    /// cannot run, and the feature is not what is missing.
    FontUnavailable(String),
}

impl LayoutError {
    /// A stable kebab-case id of the rule that failed.
    pub fn rule(&self) -> &'static str {
        match self {
            LayoutError::EmptyRect { .. } => "empty-rect",
            LayoutError::Overflow { .. } => "overflow",
            LayoutError::OutsidePermittedArea { .. } => "outside-permitted-area",
            LayoutError::Overlap { .. } => "overlap",
            LayoutError::TooShort { .. } => "too-short",
            LayoutError::TextDoesNotFit { .. } => "text-fit",
            LayoutError::FitCheckUnavailable => "fit-check-unavailable",
            LayoutError::FontUnavailable(_) => "font-unavailable",
        }
    }

    /// The field the rule failed on; `None` for [`Self::FitCheckUnavailable`]
    /// and [`Self::FontUnavailable`], which concern the build, not a rectangle.
    pub fn field(&self) -> Option<LayoutField> {
        match self {
            LayoutError::EmptyRect { field }
            | LayoutError::Overflow { field }
            | LayoutError::OutsidePermittedArea { field, .. }
            | LayoutError::Overlap { field, .. }
            | LayoutError::TooShort { field, .. }
            | LayoutError::TextDoesNotFit { field, .. } => Some(*field),
            LayoutError::FitCheckUnavailable | LayoutError::FontUnavailable(_) => None,
        }
    }
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutError::EmptyRect { field } => {
                write!(f, "{field}: empty rectangle (zero width or height)")
            }
            LayoutError::Overflow { field } => {
                write!(f, "{field}: arithmetic overflow (x + width or y + height exceeds u32)")
            }
            LayoutError::OutsidePermittedArea { field, permitted } => write!(
                f,
                "{field}: outside the permitted area (x {}..{}, y {}..{})",
                permitted.x,
                u64::from(permitted.x) + u64::from(permitted.width),
                permitted.y,
                u64::from(permitted.y) + u64::from(permitted.height),
            ),
            LayoutError::Overlap { field, other } => write!(f, "{field}: overlaps {other}"),
            LayoutError::TooShort {
                field,
                height,
                minimum,
            } => write!(
                f,
                "{field}: height {height} is below the shortest built-in visual-zone row ({minimum})"
            ),
            LayoutError::TextDoesNotFit { field, ink, rect } => write!(
                f,
                "{field}: text does not fit (the widest value inks x {}..{}, y {}..{}; \
                 the rectangle spans x {}..{}, y {}..{})",
                ink[0],
                ink[2],
                ink[1],
                ink[3],
                rect.x,
                u64::from(rect.x) + u64::from(rect.width),
                rect.y,
                u64::from(rect.y) + u64::from(rect.height),
            ),
            LayoutError::FontUnavailable(reason) => write!(
                f,
                "text-fit: the embedded fonts failed to load ({reason}), so the fit check cannot run"
            ),
            LayoutError::FitCheckUnavailable => f.write_str(
                "text-fit: the fit check needs the `embedded-fonts` feature to measure glyphs; \
                 without it only the built-in layouts are accepted",
            ),
        }
    }
}

impl std::error::Error for LayoutError {}

/// A [`LayoutSpec`] that passed every check, assembled with its format's
/// [`FormatFrame`] into the [`PageLayout`] the renderer and the labels use.
/// The fields are private and the only constructors are
/// [`try_from_spec`](Self::try_from_spec) and [`builtin`](Self::builtin).
#[derive(Debug, Clone)]
pub struct ValidatedLayout {
    spec: LayoutSpec,
    page: PageLayout,
    font_scales: [OnceLock<Result<crate::fonts::FieldScales, String>>; 5],
}

impl PartialEq for ValidatedLayout {
    fn eq(&self, other: &Self) -> bool {
        self.spec == other.spec && self.page == other.page
    }
}
impl Eq for ValidatedLayout {}

/// The five built-ins, validated once each. `builtin` is on every `generate`
/// call, and the fit check shapes text, so it must not run per render.
static BUILTINS: [OnceLock<ValidatedLayout>; 5] = [const { OnceLock::new() }; 5];

fn builtin_slot(format: DocumentType) -> usize {
    match format {
        DocumentType::TD1 => 0,
        DocumentType::TD2 => 1,
        DocumentType::TD3 => 2,
        DocumentType::MrvA => 3,
        DocumentType::MrvB => 4,
    }
}

/// The shortest visual-zone row among the five built-ins, in pixels — the
/// floor ADR-0022 Decision 4 sets for every visual-zone rectangle. Derived from
/// [`for_format`], never a literal, so a built-in row can never sit below it.
fn min_visual_zone_height() -> u32 {
    [
        DocumentType::TD1,
        DocumentType::TD2,
        DocumentType::TD3,
        DocumentType::MrvA,
        DocumentType::MrvB,
    ]
    .into_iter()
    .flat_map(|format| {
        let spec = LayoutSpec::builtin(format);
        LayoutField::ALL
            .into_iter()
            .filter(|field| field.is_visual_zone())
            .map(move |field| spec.rect(field).height)
    })
    .min()
    .unwrap_or(0)
}

impl ValidatedLayout {
    /// Cached admission result for this layout and embedded font.
    pub fn viz_scales(
        &self,
        font: crate::fonts::VizFont,
    ) -> Result<&crate::fonts::FieldScales, String> {
        self.font_scales[font as usize]
            .get_or_init(|| crate::fonts::scaling::compute_layout_scales(self, font))
            .as_ref()
            .map_err(Clone::clone)
    }
    /// `format`'s built-in layout. It goes through [`Self::try_from_spec`] like
    /// any other spec (Decision 6), once per process: the result is cached.
    ///
    /// The `expect` cannot fire for as long as the five built-ins satisfy the
    /// checks; `every_builtin_passes_the_checks_and_equals_builtin` proves it for
    /// all five, so a built-in that stops passing fails that test, not a render.
    pub fn builtin(format: DocumentType) -> &'static ValidatedLayout {
        BUILTINS[builtin_slot(format)].get_or_init(|| {
            Self::try_from_spec(LayoutSpec::builtin(format)).expect(match format {
                DocumentType::TD1 => "the built-in TD1 layout passes the layout checks",
                DocumentType::TD2 => "the built-in TD2 layout passes the layout checks",
                DocumentType::TD3 => "the built-in TD3 layout passes the layout checks",
                DocumentType::MrvA => "the built-in MRV-A layout passes the layout checks",
                DocumentType::MrvB => "the built-in MRV-B layout passes the layout checks",
            })
        })
    }

    /// Validate `spec` (see the module docs for the rules and their order).
    ///
    /// The fit rule needs glyph metrics. Without an embedded font
    /// (`--no-default-features`) it cannot run, so the other rules still apply
    /// and a spec equal to its format's built-in layout is accepted — the fit
    /// of the built-ins is proven by tests in a build that has the font — while
    /// every other spec is rejected with [`LayoutError::FitCheckUnavailable`].
    /// With the feature on, a font that fails to load is
    /// [`LayoutError::FontUnavailable`] instead.
    pub fn try_from_spec(spec: LayoutSpec) -> Result<Self, LayoutError> {
        let frame = frame_for(spec.format);
        let minimum_height = min_visual_zone_height();

        for field in LayoutField::ALL {
            let rect = spec.rect(field);
            if rect.is_empty() {
                return Err(LayoutError::EmptyRect { field });
            }
            if rect.right().is_none() || rect.bottom().is_none() {
                return Err(LayoutError::Overflow { field });
            }
            if !frame.permitted.contains(rect) {
                return Err(LayoutError::OutsidePermittedArea {
                    field,
                    permitted: frame.permitted,
                });
            }
            if field.is_visual_zone() && rect.height < minimum_height {
                return Err(LayoutError::TooShort {
                    field,
                    height: rect.height,
                    minimum: minimum_height,
                });
            }
        }

        let fields = LayoutField::ALL;
        for (i, &field) in fields.iter().enumerate() {
            for &other in &fields[i + 1..] {
                if spec.rect(field).intersects(spec.rect(other)) {
                    return Err(LayoutError::Overlap { field, other });
                }
            }
        }

        match load_fonts() {
            Ok(fonts) => fit::check(&spec, &fonts)?,
            Err(_) if spec == LayoutSpec::builtin(spec.format) => {}
            Err(_) if cfg!(not(feature = "embedded-fonts")) => {
                return Err(LayoutError::FitCheckUnavailable)
            }
            Err(e) => return Err(LayoutError::FontUnavailable(e.to_string())),
        }

        let page = PageLayout {
            width: frame.width,
            height: frame.height,
            portrait: spec.portrait,
            document_type: spec.document_type,
            issuing_country: spec.issuing_country,
            surname: spec.surname,
            given_names: spec.given_names,
            document_number: spec.document_number,
            nationality: spec.nationality,
            date_of_birth: spec.date_of_birth,
            sex: spec.sex,
            date_of_expiry: spec.date_of_expiry,
            personal_number: spec.personal_number,
            watermark: frame.watermark,
            mrz_lines: frame.mrz_lines,
            mrz_chars: frame.mrz_chars,
        };
        Ok(Self {
            spec,
            page,
            font_scales: std::array::from_fn(|_| OnceLock::new()),
        })
    }

    /// The format this layout is for.
    pub fn format(&self) -> DocumentType {
        self.spec.format
    }

    /// The validated rectangles as the `PageLayout` the renderer and the labels
    /// use: this layout's eleven rectangles on the engine-owned canvas.
    pub fn page(&self) -> &PageLayout {
        &self.page
    }

    /// The validated spec.
    pub fn spec(&self) -> &LayoutSpec {
        &self.spec
    }

    /// The canonical encoding of this layout's identity (ADR-0022 Decision 7),
    /// 197 bytes: SHA-256 over it is the layout hash, computed by the caller.
    ///
    /// | bytes | content |
    /// |---|---|
    /// | 16 | the ASCII tag `synthpass-layout` |
    /// | 4 | `schema_version`, `1`, `u32` little-endian |
    /// | 1 | the format: `1` TD1, `2` TD2, `3` TD3, `4` MRV-A, `5` MRV-B |
    /// | 11 × 16 | per rectangle `x`, `y`, `width`, `height`, each `u32` little-endian, in [`LayoutField::ALL`] order: portrait, document_type, issuing_country, surname, given_names, document_number, nationality, date_of_birth, sex, date_of_expiry, personal_number |
    ///
    /// A layout's name and description are not part of its identity, and the
    /// engine-owned frame follows from the format.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(CANONICAL_LEN);
        bytes.extend_from_slice(CANONICAL_TAG);
        bytes.extend_from_slice(&SCHEMA_VERSION.to_le_bytes());
        bytes.push(match self.spec.format {
            DocumentType::TD1 => 1,
            DocumentType::TD2 => 2,
            DocumentType::TD3 => 3,
            DocumentType::MrvA => 4,
            DocumentType::MrvB => 5,
        });
        for rect in self.spec.rects() {
            for value in [rect.x, rect.y, rect.width, rect.height] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }
}

const CANONICAL_TAG: &[u8; 16] = b"synthpass-layout";
const SCHEMA_VERSION: u32 = 1;
const CANONICAL_LEN: usize = 16 + 4 + 1 + 11 * 16;

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    pub(crate) const FORMATS: [DocumentType; 5] = [
        DocumentType::TD1,
        DocumentType::TD2,
        DocumentType::TD3,
        DocumentType::MrvA,
        DocumentType::MrvB,
    ];

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Whether `spec` passes every rule this build can run: without an
    /// embedded font the fit rule cannot, and says so with
    /// [`LayoutError::FitCheckUnavailable`], which is not a geometry failure.
    fn passes_but_maybe_fit(spec: LayoutSpec) -> bool {
        matches!(
            ValidatedLayout::try_from_spec(spec),
            Ok(_) | Err(LayoutError::FitCheckUnavailable)
        )
    }

    fn reject(spec: LayoutSpec) -> LayoutError {
        ValidatedLayout::try_from_spec(spec).expect_err("the spec must be rejected")
    }

    /// TD3's built-in with one field's rectangle replaced.
    fn td3_with(field: LayoutField, rect: Rect) -> LayoutSpec {
        let mut spec = LayoutSpec::builtin(DocumentType::TD3);
        match field {
            LayoutField::Portrait => spec.portrait = rect,
            LayoutField::DocumentType => spec.document_type = rect,
            LayoutField::IssuingCountry => spec.issuing_country = rect,
            LayoutField::Surname => spec.surname = rect,
            LayoutField::GivenNames => spec.given_names = rect,
            LayoutField::DocumentNumber => spec.document_number = rect,
            LayoutField::Nationality => spec.nationality = rect,
            LayoutField::DateOfBirth => spec.date_of_birth = rect,
            LayoutField::Sex => spec.sex = rect,
            LayoutField::DateOfExpiry => spec.date_of_expiry = rect,
            LayoutField::PersonalNumber => spec.personal_number = rect,
        }
        spec
    }

    #[test]
    fn every_builtin_passes_the_checks_and_equals_builtin() {
        for format in FORMATS {
            let validated = ValidatedLayout::try_from_spec(LayoutSpec::builtin(format))
                .unwrap_or_else(|e| panic!("{format:?}: {e}"));
            assert_eq!(&validated, ValidatedLayout::builtin(format), "{format:?}");
            assert_eq!(validated.page(), &for_format(format), "{format:?}");
            assert_eq!(validated.format(), format);
        }
    }

    #[test]
    fn the_frame_is_exactly_the_builtin_geometry() {
        for format in FORMATS {
            let page = for_format(format);
            let frame = frame_for(format);
            assert_eq!((frame.width, frame.height), (page.width, page.height));
            assert_eq!(frame.watermark, page.watermark);
            assert_eq!(frame.mrz_lines, page.mrz_lines);
            assert_eq!(frame.mrz_chars, page.mrz_chars);
            assert_eq!(frame.frame_thickness, FRAME_THICKNESS);
            // The permitted area is inside the frame and entirely above the
            // watermark band, so it cannot touch the watermark or the MRZ.
            // It ends at the lowest bottom edge of the built-in's eleven
            // rectangles, so no layout reaches closer to the MRZ than that.
            let lowest = LayoutSpec::builtin(format)
                .rects()
                .iter()
                .map(|r| r.bottom().unwrap())
                .max()
                .unwrap();
            assert_eq!(frame.permitted.bottom().unwrap(), lowest, "{format:?}");
            assert!(lowest <= frame.watermark.y, "{format:?}");
            assert!(Rect::new(0, 0, frame.width, frame.height).contains(frame.permitted));
            assert!(frame.permitted.x >= FRAME_THICKNESS && frame.permitted.y >= FRAME_THICKNESS);
            assert!(frame.permitted.bottom().unwrap() <= frame.watermark.y);
            assert!(!frame.permitted.intersects(frame.watermark));
            for line in &frame.mrz_lines {
                assert!(!frame.permitted.intersects(*line), "{format:?}");
            }
        }
    }

    #[test]
    fn the_minimum_row_height_is_derived_from_the_builtins() {
        // The plan measured 28 px on TD1; it is the shortest of the five.
        assert_eq!(min_visual_zone_height(), 28);
        assert_eq!(LayoutSpec::builtin(DocumentType::TD1).surname.height, 28);
        for format in FORMATS {
            let spec = LayoutSpec::builtin(format);
            for field in LayoutField::ALL.into_iter().filter(|f| f.is_visual_zone()) {
                assert!(spec.rect(field).height >= min_visual_zone_height());
            }
        }
    }

    #[test]
    fn an_empty_rectangle_is_rejected() {
        let e = reject(td3_with(LayoutField::Surname, Rect::new(60, 120, 0, 34)));
        assert_eq!(
            (e.rule(), e.field()),
            ("empty-rect", Some(LayoutField::Surname))
        );
        assert!(e.to_string().starts_with("surname: empty rectangle"), "{e}");
        let e = reject(td3_with(LayoutField::Portrait, Rect::new(860, 100, 260, 0)));
        assert_eq!(
            (e.rule(), e.field()),
            ("empty-rect", Some(LayoutField::Portrait))
        );
    }

    #[test]
    fn arithmetic_overflow_is_rejected_not_a_panic() {
        for rect in [
            Rect::new(u32::MAX, 100, 1, 34),
            Rect::new(100, u32::MAX, 34, 1),
            Rect::new(u32::MAX - 5, u32::MAX - 5, 10, 10),
            Rect::new(u32::MAX, u32::MAX, u32::MAX, u32::MAX),
        ] {
            let e = reject(td3_with(LayoutField::Sex, rect));
            assert_eq!(
                (e.rule(), e.field()),
                ("overflow", Some(LayoutField::Sex)),
                "{rect:?}"
            );
            assert!(e.to_string().starts_with("sex: arithmetic overflow"), "{e}");
        }
    }

    #[test]
    fn a_rectangle_outside_the_permitted_area_is_rejected() {
        // The lowest bottom among TD3's built-in rectangles (the portrait's).
        let edge = LayoutSpec::builtin(DocumentType::TD3)
            .rects()
            .iter()
            .map(|r| r.bottom().unwrap())
            .max()
            .unwrap();
        for (field, rect) in [
            // One pixel below the lowest built-in rectangle.
            (
                LayoutField::PersonalNumber,
                Rect::new(60, edge - 33, 400, 34),
            ),
            // Reaching the watermark band.
            (
                LayoutField::PersonalNumber,
                Rect::new(60, frame_for(DocumentType::TD3).watermark.y - 10, 400, 34),
            ),
            // Inside the frame stroke.
            (LayoutField::DocumentType, Rect::new(2, 60, 120, 34)),
            // Past the right edge of the canvas.
            (LayoutField::Portrait, Rect::new(1100, 100, 260, 340)),
            // On the MRZ.
            (LayoutField::GivenNames, Rect::new(60, 720, 700, 34)),
        ] {
            let e = reject(td3_with(field, rect));
            assert_eq!(
                (e.rule(), e.field()),
                ("outside-permitted-area", Some(field)),
                "{rect:?}"
            );
            assert!(
                e.to_string()
                    .starts_with(&format!("{field}: outside the permitted area")),
                "{e}"
            );
        }
        // A rectangle ending exactly at the lowest built-in bottom is allowed.
        let ok = td3_with(
            LayoutField::PersonalNumber,
            Rect::new(60, edge - 34, 400, 34),
        );
        assert!(passes_but_maybe_fit(ok));
    }

    #[test]
    fn overlapping_rectangles_are_rejected() {
        let given = LayoutSpec::builtin(DocumentType::TD3).given_names;
        let e = reject(td3_with(LayoutField::Surname, given));
        assert_eq!(e.rule(), "overlap");
        assert_eq!(e.to_string(), "surname: overlaps given_names");
        // One pixel of overlap is enough; touching edges are not.
        let mut spec = LayoutSpec::builtin(DocumentType::TD3);
        spec.surname.height = spec.given_names.y - spec.surname.y + 1;
        assert_eq!(reject(spec).to_string(), "surname: overlaps given_names");
        spec.surname.height -= 1;
        assert!(passes_but_maybe_fit(spec));
        // The portrait is one of the eleven.
        let e = reject(td3_with(
            LayoutField::Portrait,
            Rect::new(500, 100, 300, 200),
        ));
        assert_eq!(e.field(), Some(LayoutField::Portrait));
        assert_eq!(e.rule(), "overlap");
    }

    #[test]
    fn a_row_shorter_than_the_shortest_builtin_row_is_rejected() {
        let mut spec = LayoutSpec::builtin(DocumentType::TD3);
        spec.surname.height = min_visual_zone_height() - 1;
        let e = reject(spec);
        assert_eq!(
            (e.rule(), e.field()),
            ("too-short", Some(LayoutField::Surname))
        );
        assert_eq!(
            e.to_string(),
            "surname: height 27 is below the shortest built-in visual-zone row (28)"
        );
        spec.surname.height = min_visual_zone_height();
        assert!(passes_but_maybe_fit(spec));
        // The portrait is not a text row.
        let portrait = Rect::new(860, 100, 260, 20);
        assert!(passes_but_maybe_fit(td3_with(
            LayoutField::Portrait,
            portrait
        )));
    }

    #[test]
    fn canonical_bytes_are_pinned_for_td3() {
        let bytes = ValidatedLayout::builtin(DocumentType::TD3).canonical_bytes();
        assert_eq!(bytes.len(), 197);
        assert_eq!(&bytes[..16], b"synthpass-layout");
        assert_eq!(&bytes[16..20], &1u32.to_le_bytes());
        assert_eq!(bytes[20], 3);
        // Portrait (860, 100, 260, 340), then document_type (60, 60, 120, 34).
        let mut head = Vec::new();
        for v in [860u32, 100, 260, 340, 60, 60, 120, 34] {
            head.extend_from_slice(&v.to_le_bytes());
        }
        assert_eq!(&bytes[21..21 + 32], head.as_slice());
        assert_eq!(
            hex(&Sha256::digest(&bytes)),
            "3ee385f55691249dcde2b4e2243f3501f8a0ad012e57eefb7f94cd2947abbc06"
        );
    }

    #[test]
    fn canonical_bytes_change_with_the_format_and_with_any_coordinate() {
        #[cfg(feature = "embedded-fonts")]
        {
            let base = ValidatedLayout::builtin(DocumentType::TD3).canonical_bytes();
            let mut spec = LayoutSpec::builtin(DocumentType::TD3);
            spec.nationality.x += 1;
            let moved = ValidatedLayout::try_from_spec(spec).unwrap();
            assert_ne!(moved.canonical_bytes(), base);
        }
        let all: Vec<_> = FORMATS
            .iter()
            .map(|&f| ValidatedLayout::builtin(f).canonical_bytes())
            .collect();
        for (i, a) in all.iter().enumerate() {
            assert_eq!(a.len(), 197);
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[cfg(feature = "embedded-fonts")]
    mod fit {
        use super::*;

        #[test]
        fn the_widest_value_must_fit_and_one_pixel_less_must_not() {
            for field in [
                LayoutField::IssuingCountry,
                LayoutField::Surname,
                LayoutField::DocumentNumber,
                LayoutField::DateOfBirth,
                LayoutField::PersonalNumber,
            ] {
                // The narrowest accepted width is a sharp edge: one less
                // pixel is rejected as a fit failure naming that field.
                let base = LayoutSpec::builtin(DocumentType::TD3).rect(field);
                let accepted = |width: u32| {
                    ValidatedLayout::try_from_spec(td3_with(
                        field,
                        Rect::new(base.x, base.y, width, base.height),
                    ))
                };
                let mut width = 1;
                while accepted(width).is_err() {
                    width += 1;
                }
                assert!(width > 8 && width < base.width, "{field}: {width}");
                let e = accepted(width - 1).unwrap_err();
                assert_eq!((e.rule(), e.field()), ("text-fit", Some(field)), "{field}");
                assert!(
                    e.to_string()
                        .starts_with(&format!("{field}: text does not fit")),
                    "{e}"
                );
            }
        }

        #[test]
        fn a_rectangle_flush_with_the_frame_is_accepted_because_the_pen_start_shifts_ink_in() {
            // A rectangle flush against the frame inner edge would let a
            // left-hanging glyph reach into the frame: the start shift
            // (`pen_start`) keeps the ink inside, so this stays valid.
            let t = FRAME_THICKNESS;
            let mut spec = LayoutSpec::builtin(DocumentType::TD3);
            spec.document_type = Rect::new(t, 60, 120, 34);
            assert!(ValidatedLayout::try_from_spec(spec).is_ok());
        }
    }

    #[cfg(not(feature = "embedded-fonts"))]
    #[test]
    fn without_a_font_only_the_builtins_pass_and_the_error_says_why() {
        for format in FORMATS {
            assert!(ValidatedLayout::try_from_spec(LayoutSpec::builtin(format)).is_ok());
        }
        let mut spec = LayoutSpec::builtin(DocumentType::TD3);
        spec.surname.width -= 1;
        let e = reject(spec);
        assert_eq!(e, LayoutError::FitCheckUnavailable);
        assert!(e.to_string().contains("embedded-fonts"), "{e}");
        // The other rules still run first, with their own errors.
        let e = reject(td3_with(LayoutField::Surname, Rect::new(60, 120, 0, 34)));
        assert_eq!(e.rule(), "empty-rect");
    }
}
