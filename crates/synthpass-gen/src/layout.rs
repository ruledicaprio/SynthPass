//! Pixel geometry for ICAO 9303 document data pages.
//!
//! Every rectangle is known before anything is drawn — the layout is never
//! inferred from the image — which is what lets [`crate::labels::Labels`] be
//! 100% accurate by construction: the generator knows exactly where it is
//! about to draw a field before it draws it.
//!
//! The five built-in layouts are deterministic constants ([`for_format`]).
//! Alongside them, a [`ValidatedLayout`] is the geometry the renderer and the
//! labels actually consume: a [`LayoutSpec`] (the format plus the portrait and
//! the ten visual-zone rectangles) that passed every check in
//! [`ValidatedLayout::try_from_spec`], on top of the engine-owned
//! [`FormatFrame`] (canvas, frame, watermark band and MRZ), which a layout
//! cannot change. The built-ins go through the same checks
//! ([`ValidatedLayout::builtin`]); ADR-0022 Decisions 3-7 are the contract.
//!
//! Supports TD1 (3×30 chars), TD2 (2×36 chars), TD3 (2×44 chars), MRV-A
//! (2×44 chars), and MRV-B (2×36 chars) MRZ formats, each on its **own**
//! ICAO card/label canvas via [`for_format`] — not the TD3 passport page
//! with a shorter MRZ band swapped in. See [`PageLayout`].
//!
//! This generator renders one composite image per document — portrait, VIZ
//! fields, and MRZ together — the same convention TD3's own passport data
//! page already uses. That is accurate for TD3 (a real passport data page
//! genuinely carries VIZ and MRZ on one physical page) and for TD2 (ICAO
//! 9303-6 §4.2: Zone VII, the MRZ, is on the *front* of a TD2, alongside the
//! VIZ). It is a deliberate simplification for TD1: ICAO 9303-5 §3.1 puts
//! the MRZ (Zone VII) on the *back* of a TD1-size card, opposite the VIZ
//! zones on the front — two physical sides this single-image generator does
//! not model separately. Chosen for consistency with TD2/TD3 and because
//! nothing downstream (the bench corpus, the Tier-1 gate) needs a two-sided
//! artifact to grade the MRZ.
//!
//! MRV-A/MRV-B are a further simplification in kind: ICAO 9303-7 describes a
//! visa **label/sticker** affixed into a passport page, not a standalone
//! card, but this generator renders it the same way it renders TD1/TD2/TD3
//! — as its own single, self-contained canvas — since nothing downstream
//! needs it composited onto a passport-page background to grade the MRZ.

use crate::model::DocumentType;

mod validated;

#[cfg(test)]
mod properties;

pub use validated::{
    frame_for, FormatFrame, LayoutError, LayoutField, LayoutSpec, ValidatedLayout,
};

/// A pixel-space bounding box, `(x, y)` top-left plus `width`/`height`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The right edge, `x + width`; `None` when it overflows `u32`.
    pub fn right(self) -> Option<u32> {
        self.x.checked_add(self.width)
    }

    /// The bottom edge, `y + height`; `None` when it overflows `u32`.
    pub fn bottom(self) -> Option<u32> {
        self.y.checked_add(self.height)
    }

    /// Whether the rectangle covers no pixel (zero width or zero height).
    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Whether this rectangle lies entirely within a `width x height` image.
    /// A rectangle whose edges overflow `u32` is not within any image.
    pub fn within_bounds(self, width: u32, height: u32) -> bool {
        matches!(
            (self.right(), self.bottom()),
            (Some(right), Some(bottom)) if right <= width && bottom <= height
        )
    }

    /// Whether `inner` lies entirely within this rectangle. An overflowing
    /// rectangle, on either side, is never contained or containing.
    pub fn contains(self, inner: Rect) -> bool {
        matches!(
            (self.right(), self.bottom(), inner.right(), inner.bottom()),
            (Some(right), Some(bottom), Some(inner_right), Some(inner_bottom))
                if inner.x >= self.x
                    && inner.y >= self.y
                    && inner_right <= right
                    && inner_bottom <= bottom
        )
    }

    /// Whether the two rectangles share at least one pixel. Rectangles that
    /// only touch along an edge do not intersect, and an empty rectangle
    /// intersects nothing. The edges are summed in `u64`, where two `u32`
    /// values cannot overflow, so this holds for any input.
    pub fn intersects(self, other: Rect) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        let edge = |origin: u32, extent: u32| u64::from(origin) + u64::from(extent);
        u64::from(self.x) < edge(other.x, other.width)
            && u64::from(other.x) < edge(self.x, self.width)
            && u64::from(self.y) < edge(other.y, other.height)
            && u64::from(other.y) < edge(self.y, self.height)
    }
}

// ---------------------------------------------------------------------
// TD3 (ID-3, 125mm x 88mm passport data page) — unchanged from before this
// module gained per-format geometry. Kept as free constants, both for
// backward compatibility (nothing outside this crate consumed them per the
// M6 plan's audit, but `tests/watermark.rs` does) and because they are the
// literal values `for_format(DocumentType::TD3)` returns — verified
// byte-identical by `tests::td3_page_layout_matches_the_legacy_consts`, so
// TD3 output is unaffected by this module gaining TD1/TD2 support.
// ---------------------------------------------------------------------

/// Overall data-page canvas size, roughly the ID-3 aspect ratio (125mm x 88mm)
/// at ~9.6 px/mm.
pub const IMAGE_WIDTH: u32 = 1200;
pub const IMAGE_HEIGHT: u32 = 840;

/// Portrait photo placeholder box.
pub const PORTRAIT: Rect = Rect::new(860, 100, 260, 340);

/// VIZ (visual inspection zone) text fields, top half of the page.
pub const DOCUMENT_TYPE: Rect = Rect::new(60, 60, 120, 34);
pub const ISSUING_COUNTRY: Rect = Rect::new(220, 60, 120, 34);
pub const SURNAME: Rect = Rect::new(60, 120, 700, 34);
pub const GIVEN_NAMES: Rect = Rect::new(60, 170, 700, 34);
pub const DOCUMENT_NUMBER: Rect = Rect::new(60, 220, 300, 34);
pub const NATIONALITY: Rect = Rect::new(60, 270, 200, 34);
pub const DATE_OF_BIRTH: Rect = Rect::new(280, 270, 240, 34);
pub const SEX: Rect = Rect::new(540, 270, 80, 34);
pub const DATE_OF_EXPIRY: Rect = Rect::new(60, 320, 240, 34);
pub const PERSONAL_NUMBER: Rect = Rect::new(60, 370, 400, 34);

/// The unconditional "SYNTHETIC / SPECIMEN" watermark band (see
/// `render::draw_watermark`) — always drawn, independent of the
/// `embedded-fonts` feature.
pub const WATERMARK: Rect = Rect::new(60, 470, 1080, 60);

/// TD3's own two MRZ lines, bottom-anchored.
fn td3_mrz_lines() -> Vec<Rect> {
    let mrz_width = TD3_MRZ_CHARS * MRZ_CELL_WIDTH;
    let start_y = 720;
    vec![
        Rect::new(60, start_y, mrz_width, MRZ_LINE_HEIGHT),
        Rect::new(
            60,
            start_y + MRZ_LINE_HEIGHT + MRZ_LINE_SPACING,
            mrz_width,
            MRZ_LINE_HEIGHT,
        ),
    ]
}

/// MRZ character counts by document type.
pub const TD1_MRZ_CHARS: u32 = 30;
pub const TD2_MRZ_CHARS: u32 = 36;
pub const TD3_MRZ_CHARS: u32 = 44;
pub const MRVA_MRZ_CHARS: u32 = 44;
pub const MRVB_MRZ_CHARS: u32 = 36;

/// The MRZ band's per-line height/spacing — shared across all three formats
/// so the MRZ font size (which `render.rs` derives from line height) is
/// consistent regardless of which card the line sits on.
pub const MRZ_LINE_HEIGHT: u32 = 50;
pub const MRZ_LINE_SPACING: u32 = 5;

/// The pixel font size `render.rs` rasterizes the MRZ glyphs at, independent
/// of any one format's MRZ line rect height.
///
/// Before [#411](https://github.com/ruledicaprio/SynthPass/issues/411), the
/// font size was derived as `line_rect.height as f32 * 0.8`, which tied
/// glyph size to whatever height a format's MRZ line rects happened to use.
/// TD1's line rects shrink independently of the other formats (see
/// [`TD1_MRZ_LINE_PITCH`]), so the font size is now this free-standing
/// constant instead: today's `MRZ_LINE_HEIGHT as f32 * 0.8 = 40.0` px, the
/// same value every format rendered at before. At `MRZ_FONT_PX` = 40 px, 1
/// font unit = `40 / 1651` px (`hhea` ascender 1319 minus descender −332),
/// so the font's 885-unit flat-capital median rasterizes to a **21.44 px
/// cap** — the same cap height [`MRZ_CELL_WIDTH`]'s derivation uses.
pub const MRZ_FONT_PX: f32 = 40.0;

/// The MRZ character pitch, in pixels, shared by every format's MRZ cells —
/// one physical constant instead of each line rect's width divided by its
/// character count.
///
/// **Derivation** (`knowledge/ocrb/line-and-pitch.md`,
/// [#411](https://github.com/ruledicaprio/SynthPass/issues/411)):
/// ICAO's fixed MRZ pitch is 2.54 mm (Doc 9303-3 §4.4 / ISO 1073-2 §3.8)
/// against OCR-B's 2.46 mm nominal capital height, a ratio of 1.033 cap.
/// `render.rs` rasterizes the vendored font at [`MRZ_FONT_PX`] = 40 px,
/// where 1 font unit = `40 / 1651` px (`hhea` ascender 1319 minus descender
/// −332); at the font's 885-unit flat-capital median that is a rendered cap
/// of 21.44 px. 2.54 / 2.46 × 21.44 px = 22.15 px, and
/// the nearest integer cell is **22 px = 1.026 cap** — inside ISO 1831's
/// 0.935 cap pitch floor and the measured real-TD3 range, and closer to
/// nominal than the next integer up (23 px = 1.073 cap).
pub const MRZ_CELL_WIDTH: u32 = 22;

/// Thickness, in pixels, of the plain frame `render.rs` draws around every
/// canvas. The permitted placement area of a [`ValidatedLayout`] starts
/// inside it.
pub const FRAME_THICKNESS: u32 = 6;

// ---------------------------------------------------------------------
// Per-format page geometry.
// ---------------------------------------------------------------------

/// Everything needed to render or label one document, as a single bundle
/// returned by [`for_format`] or [`ValidatedLayout::page`]. Replaces
/// piecemeal lookups against the TD3-only module consts above for any caller
/// that needs to work across formats (`labels::build_labels`,
/// `render::render`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageLayout {
    pub width: u32,
    pub height: u32,
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
    pub watermark: Rect,
    /// MRZ line rectangles, top to bottom: 2 for TD2/TD3, 3 for TD1.
    pub mrz_lines: Vec<Rect>,
    pub mrz_chars: u32,
}

/// Real ICAO 9303 card/label geometry per format, all at TD3's existing
/// ~9.6 px/mm so the canvases are directly comparable:
///
/// - TD3 (ID-3, 125mm x 88mm) → 1200x840 — unchanged from before this
///   function existed.
/// - TD2 (ID-2, 105mm x 74mm) → 1008x710.
/// - TD1 (ID-1, 85.6mm x 54mm) → 822x518, landscape card proportions.
/// - MRV-A (9303-7 §3.3, 120mm x 80mm) → 1152x768.
/// - MRV-B (9303-7 §3.3, 105mm x 74mm — explicitly "based on ISO/IEC 7810,
///   ID-2 Type Card") → 1008x710, the same canvas as TD2, deliberately: the
///   physical size is identical per that ICAO citation, not a copy-paste
///   duplicate.
///
/// Every format gets its own VIZ arrangement: portrait at the left edge
/// (ICAO 9303-5 §3.3 / 9303-6 §3.3 / 9303-7's Zone V rule all state the
/// portrait's left edge is coincident with the card/label's own left edge),
/// VIZ text fields in a column to its right, and the MRZ spanning the full
/// width in a band at the bottom (9303-6 Figure 3 / 9303-7 Zone VII; see
/// this module's doc comment for the TD1 front/back and MRV
/// label-vs-standalone-canvas simplifications).
pub fn for_format(doc_type: DocumentType) -> PageLayout {
    match doc_type {
        DocumentType::TD3 => td3_layout(),
        DocumentType::TD2 => td2_layout(),
        DocumentType::TD1 => td1_layout(),
        DocumentType::MrvA => mrva_layout(),
        DocumentType::MrvB => mrvb_layout(),
    }
}

fn td3_layout() -> PageLayout {
    PageLayout {
        width: IMAGE_WIDTH,
        height: IMAGE_HEIGHT,
        portrait: PORTRAIT,
        document_type: DOCUMENT_TYPE,
        issuing_country: ISSUING_COUNTRY,
        surname: SURNAME,
        given_names: GIVEN_NAMES,
        document_number: DOCUMENT_NUMBER,
        nationality: NATIONALITY,
        date_of_birth: DATE_OF_BIRTH,
        sex: SEX,
        date_of_expiry: DATE_OF_EXPIRY,
        personal_number: PERSONAL_NUMBER,
        watermark: WATERMARK,
        mrz_lines: td3_mrz_lines(),
        mrz_chars: TD3_MRZ_CHARS,
    }
}

/// TD2 (ID-2, 105mm x 74mm) → 1008x710. Portrait at the left edge, a VIZ
/// column to its right, MRZ band (2 lines) spanning the full card width at
/// the bottom.
fn td2_layout() -> PageLayout {
    const WIDTH: u32 = 1008;
    const HEIGHT: u32 = 710;
    const MARGIN: u32 = 50;

    let portrait = Rect::new(MARGIN, MARGIN, 190, 260);
    let viz_x = portrait.x + portrait.width + 30;
    let viz_right = WIDTH - MARGIN;
    let viz_width = viz_right - viz_x;

    let row_h = 32;
    let row_y = |i: u32| MARGIN + i * 42;

    // `band_width` and `mrz_width` used to be one shared local
    // (`WIDTH - 2 * MARGIN`) until #411 narrowed the MRZ cell pitch. Kept
    // split so the watermark band — an ethics guardrail, see `render.rs`'s
    // module doc comment — never shrinks with the MRZ.
    let band_width = WIDTH - 2 * MARGIN;
    let mrz_width = TD2_MRZ_CHARS * MRZ_CELL_WIDTH;
    let mrz_height = 2 * MRZ_LINE_HEIGHT + MRZ_LINE_SPACING;
    let mrz_start_y = HEIGHT - MARGIN - mrz_height;

    PageLayout {
        width: WIDTH,
        height: HEIGHT,
        portrait,
        document_type: Rect::new(viz_x, row_y(0), 120, row_h),
        issuing_country: Rect::new(viz_x + 140, row_y(0), 120, row_h),
        surname: Rect::new(viz_x, row_y(1), viz_width, row_h),
        given_names: Rect::new(viz_x, row_y(2), viz_width, row_h),
        document_number: Rect::new(viz_x, row_y(3), 260, row_h),
        nationality: Rect::new(viz_x, row_y(4), 150, row_h),
        date_of_birth: Rect::new(viz_x + 170, row_y(4), 200, row_h),
        sex: Rect::new(viz_x + 390, row_y(4), 70, row_h),
        date_of_expiry: Rect::new(viz_x, row_y(5), 200, row_h),
        personal_number: Rect::new(viz_x, row_y(6), 400, row_h),
        watermark: Rect::new(MARGIN, 360, band_width, 50),
        mrz_lines: vec![
            Rect::new(MARGIN, mrz_start_y, mrz_width, MRZ_LINE_HEIGHT),
            Rect::new(
                MARGIN,
                mrz_start_y + MRZ_LINE_HEIGHT + MRZ_LINE_SPACING,
                mrz_width,
                MRZ_LINE_HEIGHT,
            ),
        ],
        mrz_chars: TD2_MRZ_CHARS,
    }
}

/// TD1's own MRZ line pitch, in pixels — denser than every other format's
/// [`MRZ_LINE_HEIGHT`] + [`MRZ_LINE_SPACING`] (55 px = 2.565 cap), because
/// Doc 9303-5 Figure 6 sets TD1's line spacing at 4.23 mm, the densest
/// packing ISO 1831 permits.
///
/// **Derivation** (`knowledge/ocrb/line-and-pitch.md`, the 2026-09-28
/// amendment to `knowledge/decisions/ADR-0015-geometric-mrz-band-location.md`,
/// [#411](https://github.com/ruledicaprio/SynthPass/issues/411)): 4.23 mm ÷
/// 2.46 mm nominal cap = 1.72 cap, which is 36.88 px at the 21.44 px cap
/// [`MRZ_CELL_WIDTH`]'s derivation computes. **37 px = 1.726 cap** is the
/// nearest integer. The conforming band, from TD1's 2.95 mm printing zone
/// against the 2.66 mm constant-strokewidth digit, is 4.20-4.52 mm =
/// 1.71-1.84 cap = 36.6-39.4 px; 37 px sits inside it. TD1's three MRZ line
/// rects are exactly this pitch tall, stacked with **no gap** — unlike every
/// other format's rects, which are shorter than their pitch and leave a gap
/// (`MRZ_LINE_SPACING`) between them. Keeping the same 50 px rects and
/// stepping them by 37 was rejected: overlapping line rects would make
/// per-character label boxes overlap across lines.
pub const TD1_MRZ_LINE_PITCH: u32 = 37;

/// TD1 (ID-1, 85.6mm x 54mm) → 822x518. Same arrangement as TD2, but a
/// third MRZ line (30 chars/line vs TD2's 36) needs more of the card's
/// proportionally smaller height, so rows are packed tighter, at TD1's own
/// [`TD1_MRZ_LINE_PITCH`] rather than the shared [`MRZ_LINE_HEIGHT`] +
/// [`MRZ_LINE_SPACING`] every other format uses.
fn td1_layout() -> PageLayout {
    const WIDTH: u32 = 822;
    const HEIGHT: u32 = 518;
    const MARGIN: u32 = 40;
    // The MRZ band's own bottom margin, deliberately smaller than the top/
    // VIZ MARGIN above. The M6 TD1 root-cause diagnosis (knowledge/ROADMAP.md
    // and this crate's --dump-ocr probe) found the watermark and MRZ line 1
    // separated by only 8px: `draw_watermark` draws a 28px-tall glyph
    // (`scale = 4` in render.rs) into what was a 26px rect starting at
    // y=282, and MRZ line 1 started at y=318 — an 8px gap on a canvas where
    // TD2 gets ~145px and TD3 gets ~190px. Across OCR's internal multi-pass
    // retry loop that gap was tight enough that some passes' text detector
    // merged the watermark into (or dropped) the region where line 3 (the
    // name line) should have been its own detected line — see the probe's
    // recorded line dumps. Reclaiming a smaller bottom margin here (instead
    // of shrinking the VIZ rows, which would cost every other field's OCR
    // legibility) pushes MRZ start down and buys real separation without
    // touching row_h/row_y below.
    const MRZ_BOTTOM_MARGIN: u32 = 16;

    let portrait = Rect::new(MARGIN, MARGIN, 150, 210);
    let viz_x = portrait.x + portrait.width + 24;
    let viz_right = WIDTH - MARGIN;
    let viz_width = viz_right - viz_x;

    let row_h = 28;
    let row_y = |i: u32| MARGIN + i * 34;
    // Bottom of the last VIZ row (personal_number, row index 6) — the
    // watermark band must start at or after this, same as it always has.
    let viz_bottom = row_y(6) + row_h;

    // Split per #411, same reasoning as `td2_layout`'s: `band_width` keeps
    // the watermark at its full, unchanged width; `mrz_width` is the
    // narrower MRZ cell width.
    let band_width = WIDTH - 2 * MARGIN;
    let mrz_width = TD1_MRZ_CHARS * MRZ_CELL_WIDTH;
    let mrz_height = 3 * TD1_MRZ_LINE_PITCH;
    let mrz_start_y = HEIGHT - MRZ_BOTTOM_MARGIN - mrz_height;

    // Center the watermark band in the window between the VIZ rows and the
    // MRZ band. `draw_watermark`'s glyphs are 28px tall (`scale = 4` in
    // render.rs); this rect is taller than that on purpose, so the glyph
    // never touches either edge of its own rect the way it did at the old
    // 26px height.
    const WATERMARK_HEIGHT: u32 = 32;
    let watermark_y = viz_bottom + (mrz_start_y - viz_bottom - WATERMARK_HEIGHT) / 2;

    PageLayout {
        width: WIDTH,
        height: HEIGHT,
        portrait,
        document_type: Rect::new(viz_x, row_y(0), 100, row_h),
        issuing_country: Rect::new(viz_x + 110, row_y(0), 100, row_h),
        surname: Rect::new(viz_x, row_y(1), viz_width, row_h),
        given_names: Rect::new(viz_x, row_y(2), viz_width, row_h),
        document_number: Rect::new(viz_x, row_y(3), 220, row_h),
        nationality: Rect::new(viz_x, row_y(4), 120, row_h),
        date_of_birth: Rect::new(viz_x + 130, row_y(4), 170, row_h),
        sex: Rect::new(viz_x + 310, row_y(4), 60, row_h),
        date_of_expiry: Rect::new(viz_x, row_y(5), 170, row_h),
        personal_number: Rect::new(viz_x, row_y(6), 320, row_h),
        watermark: Rect::new(MARGIN, watermark_y, band_width, WATERMARK_HEIGHT),
        mrz_lines: vec![
            Rect::new(MARGIN, mrz_start_y, mrz_width, TD1_MRZ_LINE_PITCH),
            Rect::new(
                MARGIN,
                mrz_start_y + TD1_MRZ_LINE_PITCH,
                mrz_width,
                TD1_MRZ_LINE_PITCH,
            ),
            Rect::new(
                MARGIN,
                mrz_start_y + 2 * TD1_MRZ_LINE_PITCH,
                mrz_width,
                TD1_MRZ_LINE_PITCH,
            ),
        ],
        mrz_chars: TD1_MRZ_CHARS,
    }
}

/// MRV-A (ICAO 9303-7 §3.3, nominal 120mm x 80mm) → 1152x768. Portrait at
/// the left edge — Zone V's own rule ("the left edge of the identification
/// feature shall be coincident with the left edge of the MRV"), not just
/// this module's TD1/TD2 convention — VIZ column to its right, 2-line MRZ
/// band (Zone VII) at the bottom, same arrangement as `td2_layout()` since
/// MRV-A is also a 2-line MRZ format.
fn mrva_layout() -> PageLayout {
    const WIDTH: u32 = 1152;
    const HEIGHT: u32 = 768;
    const MARGIN: u32 = 50;

    let portrait = Rect::new(MARGIN, MARGIN, 240, 320);
    let viz_x = portrait.x + portrait.width + 30;
    let viz_right = WIDTH - MARGIN;
    let viz_width = viz_right - viz_x;

    let row_h = 34;
    let row_y = |i: u32| MARGIN + i * 46;

    // Split per #411, same reasoning as `td2_layout`'s: `band_width` keeps
    // the watermark at its full, unchanged width; `mrz_width` is the
    // narrower MRZ cell width.
    let band_width = WIDTH - 2 * MARGIN;
    let mrz_width = MRVA_MRZ_CHARS * MRZ_CELL_WIDTH;
    let mrz_height = 2 * MRZ_LINE_HEIGHT + MRZ_LINE_SPACING;
    let mrz_start_y = HEIGHT - MARGIN - mrz_height;

    PageLayout {
        width: WIDTH,
        height: HEIGHT,
        portrait,
        document_type: Rect::new(viz_x, row_y(0), 130, row_h),
        issuing_country: Rect::new(viz_x + 150, row_y(0), 130, row_h),
        surname: Rect::new(viz_x, row_y(1), viz_width, row_h),
        given_names: Rect::new(viz_x, row_y(2), viz_width, row_h),
        document_number: Rect::new(viz_x, row_y(3), 280, row_h),
        nationality: Rect::new(viz_x, row_y(4), 160, row_h),
        date_of_birth: Rect::new(viz_x + 180, row_y(4), 210, row_h),
        sex: Rect::new(viz_x + 410, row_y(4), 75, row_h),
        date_of_expiry: Rect::new(viz_x, row_y(5), 210, row_h),
        personal_number: Rect::new(viz_x, row_y(6), 420, row_h),
        watermark: Rect::new(MARGIN, row_y(7), band_width, 50),
        mrz_lines: vec![
            Rect::new(MARGIN, mrz_start_y, mrz_width, MRZ_LINE_HEIGHT),
            Rect::new(
                MARGIN,
                mrz_start_y + MRZ_LINE_HEIGHT + MRZ_LINE_SPACING,
                mrz_width,
                MRZ_LINE_HEIGHT,
            ),
        ],
        mrz_chars: MRVA_MRZ_CHARS,
    }
}

/// MRV-B (ICAO 9303-7 §3.3, nominal 105mm x 74mm — explicitly "based on
/// ISO/IEC 7810, ID-2 Type Card") → 1008x710, the same canvas
/// [`td2_layout`] returns. That is deliberate, not accidental duplication:
/// MRV-B is physically an ID-2-sized label per that ICAO citation. It stays
/// a distinct function (not a `td2_layout()` alias) because its field
/// semantics differ — visa fields, no personal-number check digit, no
/// composite check digit (see `mrz::emit`'s MRV-B docs) — even though the
/// canvas dimensions coincide.
fn mrvb_layout() -> PageLayout {
    const WIDTH: u32 = 1008;
    const HEIGHT: u32 = 710;
    const MARGIN: u32 = 50;

    let portrait = Rect::new(MARGIN, MARGIN, 190, 260);
    let viz_x = portrait.x + portrait.width + 30;
    let viz_right = WIDTH - MARGIN;
    let viz_width = viz_right - viz_x;

    let row_h = 32;
    let row_y = |i: u32| MARGIN + i * 42;

    // Split per #411, same reasoning as `td2_layout`'s: `band_width` keeps
    // the watermark at its full, unchanged width; `mrz_width` is the
    // narrower MRZ cell width.
    let band_width = WIDTH - 2 * MARGIN;
    let mrz_width = MRVB_MRZ_CHARS * MRZ_CELL_WIDTH;
    let mrz_height = 2 * MRZ_LINE_HEIGHT + MRZ_LINE_SPACING;
    let mrz_start_y = HEIGHT - MARGIN - mrz_height;

    PageLayout {
        width: WIDTH,
        height: HEIGHT,
        portrait,
        document_type: Rect::new(viz_x, row_y(0), 120, row_h),
        issuing_country: Rect::new(viz_x + 140, row_y(0), 120, row_h),
        surname: Rect::new(viz_x, row_y(1), viz_width, row_h),
        given_names: Rect::new(viz_x, row_y(2), viz_width, row_h),
        document_number: Rect::new(viz_x, row_y(3), 260, row_h),
        nationality: Rect::new(viz_x, row_y(4), 150, row_h),
        date_of_birth: Rect::new(viz_x + 170, row_y(4), 200, row_h),
        sex: Rect::new(viz_x + 390, row_y(4), 70, row_h),
        date_of_expiry: Rect::new(viz_x, row_y(5), 200, row_h),
        personal_number: Rect::new(viz_x, row_y(6), 400, row_h),
        watermark: Rect::new(MARGIN, 360, band_width, 50),
        mrz_lines: vec![
            Rect::new(MARGIN, mrz_start_y, mrz_width, MRZ_LINE_HEIGHT),
            Rect::new(
                MARGIN,
                mrz_start_y + MRZ_LINE_HEIGHT + MRZ_LINE_SPACING,
                mrz_width,
                MRZ_LINE_HEIGHT,
            ),
        ],
        mrz_chars: MRVB_MRZ_CHARS,
    }
}

/// Get MRZ character rectangle for a specific line and index, one
/// [`MRZ_CELL_WIDTH`]-px cell at `index`'s position along the line.
pub fn mrz_char_rect_for_line(line: Rect, mrz_chars: u32, index: u32) -> Rect {
    debug_assert!(index < mrz_chars);
    Rect::new(
        line.x + index * MRZ_CELL_WIDTH,
        line.y,
        MRZ_CELL_WIDTH,
        line.height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_named_rects_fit_the_canvas() {
        let rects = [
            PORTRAIT,
            DOCUMENT_TYPE,
            ISSUING_COUNTRY,
            SURNAME,
            GIVEN_NAMES,
            DOCUMENT_NUMBER,
            NATIONALITY,
            DATE_OF_BIRTH,
            SEX,
            DATE_OF_EXPIRY,
            PERSONAL_NUMBER,
            WATERMARK,
        ];
        for r in rects {
            assert!(
                r.within_bounds(IMAGE_WIDTH, IMAGE_HEIGHT),
                "{r:?} escapes the {IMAGE_WIDTH}x{IMAGE_HEIGHT} canvas"
            );
        }
    }

    #[test]
    fn rect_edges_are_checked_never_wrapped() {
        let huge = Rect::new(u32::MAX - 1, u32::MAX - 1, 5, 5);
        assert_eq!(huge.right(), None);
        assert_eq!(huge.bottom(), None);
        assert!(!huge.within_bounds(u32::MAX, u32::MAX));
        assert!(!Rect::new(0, 0, 10, 10).contains(huge));
        assert!(!huge.contains(Rect::new(u32::MAX, u32::MAX, 0, 0)));
        let max = Rect::new(u32::MAX, u32::MAX, u32::MAX, u32::MAX);
        assert!(max.intersects(max));
        assert!(!max.intersects(Rect::new(0, 0, 10, 10)));
    }

    #[test]
    fn rect_intersects_is_strict_and_symmetric() {
        let a = Rect::new(10, 10, 10, 10);
        let touching = Rect::new(20, 10, 10, 10);
        let overlapping = Rect::new(19, 19, 10, 10);
        let empty = Rect::new(12, 12, 0, 5);
        assert!(!a.intersects(touching) && !touching.intersects(a));
        assert!(a.intersects(overlapping) && overlapping.intersects(a));
        assert!(!a.intersects(empty) && !empty.intersects(a));
        assert!(a.contains(Rect::new(10, 10, 10, 10)));
        assert!(!a.contains(overlapping));
    }

    #[test]
    fn mrz_char_cells_fit_within_the_line() {
        for doc_type in [
            DocumentType::TD1,
            DocumentType::TD2,
            DocumentType::TD3,
            DocumentType::MrvA,
            DocumentType::MrvB,
        ] {
            let layout = for_format(doc_type);
            for rect in &layout.mrz_lines {
                for i in 0..layout.mrz_chars {
                    let cell = mrz_char_rect_for_line(*rect, layout.mrz_chars, i);
                    assert!(cell.x + cell.width <= rect.x + rect.width);
                }
            }
        }
    }

    /// TD3's `PageLayout` must be byte-identical to the pre-refactor module
    /// consts — the M6 plan's explicit requirement that this refactor leave
    /// TD3 output unchanged.
    #[test]
    fn td3_page_layout_matches_the_legacy_consts() {
        let layout = for_format(DocumentType::TD3);
        assert_eq!(layout.width, IMAGE_WIDTH);
        assert_eq!(layout.height, IMAGE_HEIGHT);
        assert_eq!(layout.portrait, PORTRAIT);
        assert_eq!(layout.document_type, DOCUMENT_TYPE);
        assert_eq!(layout.issuing_country, ISSUING_COUNTRY);
        assert_eq!(layout.surname, SURNAME);
        assert_eq!(layout.given_names, GIVEN_NAMES);
        assert_eq!(layout.document_number, DOCUMENT_NUMBER);
        assert_eq!(layout.nationality, NATIONALITY);
        assert_eq!(layout.date_of_birth, DATE_OF_BIRTH);
        assert_eq!(layout.sex, SEX);
        assert_eq!(layout.date_of_expiry, DATE_OF_EXPIRY);
        assert_eq!(layout.personal_number, PERSONAL_NUMBER);
        assert_eq!(layout.watermark, WATERMARK);
        assert_eq!(layout.mrz_chars, TD3_MRZ_CHARS);
        assert_eq!(layout.mrz_lines, td3_mrz_lines());
    }

    /// Every named rect in every format's `PageLayout` — portrait, VIZ
    /// fields, watermark, and MRZ lines alike — must lie inside that
    /// format's own canvas. The M6 plan's per-format verification step.
    #[test]
    fn every_format_page_layout_fits_its_own_canvas() {
        for doc_type in [
            DocumentType::TD1,
            DocumentType::TD2,
            DocumentType::TD3,
            DocumentType::MrvA,
            DocumentType::MrvB,
        ] {
            let layout = for_format(doc_type);
            let (w, h) = (layout.width, layout.height);
            let mut rects = vec![
                layout.portrait,
                layout.document_type,
                layout.issuing_country,
                layout.surname,
                layout.given_names,
                layout.document_number,
                layout.nationality,
                layout.date_of_birth,
                layout.sex,
                layout.date_of_expiry,
                layout.personal_number,
                layout.watermark,
            ];
            rects.extend(&layout.mrz_lines);
            for r in rects {
                assert!(
                    r.within_bounds(w, h),
                    "{doc_type:?}: {r:?} escapes the {w}x{h} canvas"
                );
            }
            assert_eq!(
                layout.mrz_lines.len(),
                doc_type.mrz_lines(),
                "{doc_type:?}: MRZ line count must match DocumentType::mrz_lines"
            );
        }
    }

    /// Per-format canvas sizes match the M6 plan's ~9.6 px/mm ICAO card
    /// dimensions exactly.
    #[test]
    fn canvas_sizes_match_icao_card_dimensions() {
        assert_eq!(
            (
                for_format(DocumentType::TD3).width,
                for_format(DocumentType::TD3).height
            ),
            (1200, 840)
        );
        assert_eq!(
            (
                for_format(DocumentType::TD2).width,
                for_format(DocumentType::TD2).height
            ),
            (1008, 710)
        );
        assert_eq!(
            (
                for_format(DocumentType::TD1).width,
                for_format(DocumentType::TD1).height
            ),
            (822, 518)
        );
        assert_eq!(
            (
                for_format(DocumentType::MrvA).width,
                for_format(DocumentType::MrvA).height
            ),
            (1152, 768)
        );
        assert_eq!(
            (
                for_format(DocumentType::MrvB).width,
                for_format(DocumentType::MrvB).height
            ),
            (1008, 710)
        );
    }

    /// The #411 trap, pinned: `TD2`, `TD1`, `MRV-A` and `MRV-B`'s watermark
    /// rects are built from `band_width`, a local split off from the MRZ
    /// line width so that narrowing [`MRZ_CELL_WIDTH`] cannot narrow the
    /// watermark along with it (`render.rs`'s module doc comment marks the
    /// watermark as an unconditional ethics guardrail). TD3's `WATERMARK` was
    /// never on that shared local, but is pinned here too so all five
    /// formats are covered by one test. Every value is the same one
    /// `for_format` returned before #411's pitch change — this test would
    /// fail if the split were ever missed or undone.
    ///
    /// **TD1 is the one exception, on purpose.** #411's TD1 line-pitch change
    /// (`TD1_MRZ_LINE_PITCH`) shrinks `mrz_height` from 160 to 111 px, which
    /// moves `mrz_start_y` down and re-centres the watermark in the taller
    /// gap that opens up between the VIZ rows and the MRZ — `watermark_y`
    /// moves from 291 to 315. Only TD1's `y` is updated here; its `x`,
    /// `width` and `height` are pinned exactly as before, same as every
    /// other format's full rect, because the pitch change touches TD1's MRZ
    /// band alone.
    #[test]
    fn watermark_rect_is_unchanged_by_the_mrz_pitch_split() {
        assert_eq!(
            for_format(DocumentType::TD3).watermark,
            Rect::new(60, 470, 1080, 60)
        );
        assert_eq!(
            for_format(DocumentType::TD2).watermark,
            Rect::new(50, 360, 908, 50)
        );
        assert_eq!(
            for_format(DocumentType::TD1).watermark,
            Rect::new(40, 315, 742, 32)
        );
        assert_eq!(
            for_format(DocumentType::MrvA).watermark,
            Rect::new(50, 372, 1052, 50)
        );
        assert_eq!(
            for_format(DocumentType::MrvB).watermark,
            Rect::new(50, 360, 908, 50)
        );
    }

    /// [`MRZ_CELL_WIDTH`]'s doc comment derives 22px as 1.026 cap by hand;
    /// this test checks that arithmetic against the *actual* rendered glyph
    /// instead of trusting the comment. It reads `H`'s outline bounds, in the
    /// real vendored font's own design units, and scales them by the exact
    /// factor `render.rs::draw_mrz_glyphs` and `ab_glyph` itself use
    /// (`px_scale / height_unscaled()`, see `ScaleFont::v_scale_factor`) —
    /// not a rasterized `OutlinedGlyph::px_bounds`, whose pixel-grid rounding
    /// would round a 21.44px cap up to a 22px bounding box and make this
    /// test vacuously pass at exactly `MRZ_CELL_WIDTH`.
    #[cfg(feature = "embedded-fonts")]
    #[test]
    fn mrz_cell_width_is_1_02_to_1_04_of_the_rendered_cap_height() {
        use ab_glyph::Font;

        let fonts = crate::fonts::load_fonts()
            .expect("embedded-fonts is on by default and vendors both fonts");
        let font = &fonts.mrz;

        // Mirrors `render.rs::draw_mrz_glyphs`'s own px_scale derivation.
        let px_scale = MRZ_FONT_PX;
        let outline = font
            .outline(font.glyph_id('H'))
            .expect("'H' must have an outline in the vendored OCR-B font");
        // `Outline::bounds` uses ab_glyph's y-down rasterizer convention
        // (`min.y` holds the font's own `y_max`), so its signed `height()`
        // is negative; the magnitude is the cap height.
        let cap_units = outline.bounds.height().abs();
        let cap_px = cap_units * px_scale / font.height_unscaled();

        let ratio = MRZ_CELL_WIDTH as f32 / cap_px;
        assert!(
            (1.02..=1.04).contains(&ratio),
            "MRZ_CELL_WIDTH ({MRZ_CELL_WIDTH}) / rendered cap ({cap_px} px) = {ratio}, \
             expected within [1.02, 1.04] per the ISO 1073-2/1831 pitch-to-cap \
             derivation in MRZ_CELL_WIDTH's doc comment"
        );
    }

    /// [`TD1_MRZ_LINE_PITCH`]'s doc comment derives 37px as 1.726 cap by
    /// hand; this test checks that arithmetic against the *actual* rendered
    /// glyph, the same way `mrz_cell_width_is_1_02_to_1_04_of_the_rendered_cap_height`
    /// does for [`MRZ_CELL_WIDTH`] — reading `H`'s outline bounds from the
    /// real vendored font rather than trusting a constant the test itself
    /// would otherwise have to write down.
    #[cfg(feature = "embedded-fonts")]
    #[test]
    fn td1_line_pitch_is_1_71_to_1_84_of_the_rendered_cap_height() {
        use ab_glyph::Font;

        let fonts = crate::fonts::load_fonts()
            .expect("embedded-fonts is on by default and vendors both fonts");
        let font = &fonts.mrz;

        // Mirrors `render.rs::draw_mrz_glyphs`'s own px_scale derivation.
        let px_scale = MRZ_FONT_PX;
        let outline = font
            .outline(font.glyph_id('H'))
            .expect("'H' must have an outline in the vendored OCR-B font");
        let cap_units = outline.bounds.height().abs();
        let cap_px = cap_units * px_scale / font.height_unscaled();

        let ratio = TD1_MRZ_LINE_PITCH as f32 / cap_px;
        assert!(
            (1.71..=1.84).contains(&ratio),
            "TD1_MRZ_LINE_PITCH ({TD1_MRZ_LINE_PITCH}) / rendered cap ({cap_px} px) = \
             {ratio}, expected within [1.71, 1.84], TD1's conforming line-spacing band \
             per TD1_MRZ_LINE_PITCH's doc comment"
        );
    }

    /// #411's TD1 line-pitch change (`TD1_MRZ_LINE_PITCH`) touches only
    /// TD1's `PageLayout`. This pins the other four formats' `mrz_lines` to
    /// the exact values `origin/main` at `d489fd3` (after #411 PR 1, before
    /// this PR) produces, so a regression that leaked the TD1 change into a
    /// shared constant would fail here instead of only showing up as a
    /// pixel difference in a benchmark.
    #[test]
    fn non_td1_mrz_lines_are_unchanged_by_the_td1_line_pitch() {
        assert_eq!(
            for_format(DocumentType::TD3).mrz_lines,
            vec![Rect::new(60, 720, 968, 50), Rect::new(60, 775, 968, 50)]
        );
        assert_eq!(
            for_format(DocumentType::TD2).mrz_lines,
            vec![Rect::new(50, 555, 792, 50), Rect::new(50, 610, 792, 50)]
        );
        assert_eq!(
            for_format(DocumentType::MrvA).mrz_lines,
            vec![Rect::new(50, 613, 968, 50), Rect::new(50, 668, 968, 50)]
        );
        assert_eq!(
            for_format(DocumentType::MrvB).mrz_lines,
            vec![Rect::new(50, 555, 792, 50), Rect::new(50, 610, 792, 50)]
        );
    }
}
