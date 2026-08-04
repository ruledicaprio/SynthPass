//! Fixed pixel geometry for ICAO 9303 document data pages.
//!
//! Every rectangle here is a deterministic constant — the same layout is used
//! for every render, which is what lets [`crate::labels::Labels`] be 100%
//! accurate by construction: the generator knows exactly where it is about to
//! draw a field before it draws it.
//!
//! Supports TD1 (3×30 chars), TD2 (2×36 chars), and TD3 (2×44 chars) MRZ
//! formats, each on its **own** ICAO card canvas via [`for_format`] — not the
//! TD3 passport page with a shorter MRZ band swapped in. See [`PageLayout`].
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

use crate::model::DocumentType;

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

    /// Whether this rectangle lies entirely within a `width x height` image.
    pub fn within_bounds(self, width: u32, height: u32) -> bool {
        self.x + self.width <= width && self.y + self.height <= height
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
    let mrz_width = 1080;
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

/// The MRZ band's per-line height/spacing — shared across all three formats
/// so the MRZ font size (which `render.rs` derives from line height) is
/// consistent regardless of which card the line sits on.
pub const MRZ_LINE_HEIGHT: u32 = 50;
pub const MRZ_LINE_SPACING: u32 = 5;

// ---------------------------------------------------------------------
// Per-format page geometry.
// ---------------------------------------------------------------------

/// Everything needed to render or label one document, as a single bundle
/// returned by [`for_format`]. Replaces piecemeal lookups against the
/// TD3-only module consts above for any caller that needs to work across
/// formats (`labels::build_labels`, `render::render`).
#[derive(Debug, Clone)]
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

/// Real ICAO 9303 card geometry per format, all at TD3's existing
/// ~9.6 px/mm so the three canvases are directly comparable:
///
/// - TD3 (ID-3, 125mm x 88mm) → 1200x840 — unchanged from before this
///   function existed.
/// - TD2 (ID-2, 105mm x 74mm) → 1008x710.
/// - TD1 (ID-1, 85.6mm x 54mm) → 822x518, landscape card proportions.
///
/// TD1 and TD2 are genuinely different canvases, not the TD3 passport page
/// with a shorter MRZ band — each gets its own VIZ arrangement: portrait at
/// the left edge (ICAO 9303-5 §3.3 / 9303-6 §3.3: the portrait's left edge
/// is coincident with the card's left edge on both formats), VIZ text
/// fields in a column to its right, and the MRZ spanning the full card
/// width in a band at the bottom (9303-6 Figure 3's "Zone VII at bottom";
/// see this module's doc comment for the TD1 front/back simplification).
pub fn for_format(doc_type: DocumentType) -> PageLayout {
    match doc_type {
        DocumentType::TD3 => td3_layout(),
        DocumentType::TD2 => td2_layout(),
        DocumentType::TD1 => td1_layout(),
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

    let mrz_width = WIDTH - 2 * MARGIN;
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
        watermark: Rect::new(MARGIN, 360, mrz_width, 50),
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

/// TD1 (ID-1, 85.6mm x 54mm) → 822x518. Same arrangement as TD2, but a
/// third MRZ line (30 chars/line vs TD2's 36) needs more of the card's
/// proportionally smaller height, so rows are packed tighter.
fn td1_layout() -> PageLayout {
    const WIDTH: u32 = 822;
    const HEIGHT: u32 = 518;
    const MARGIN: u32 = 40;

    let portrait = Rect::new(MARGIN, MARGIN, 150, 210);
    let viz_x = portrait.x + portrait.width + 24;
    let viz_right = WIDTH - MARGIN;
    let viz_width = viz_right - viz_x;

    let row_h = 28;
    let row_y = |i: u32| MARGIN + i * 34;

    let mrz_width = WIDTH - 2 * MARGIN;
    let mrz_height = 3 * MRZ_LINE_HEIGHT + 2 * MRZ_LINE_SPACING;
    let mrz_start_y = HEIGHT - MARGIN - mrz_height;

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
        watermark: Rect::new(MARGIN, 282, mrz_width, 26),
        mrz_lines: vec![
            Rect::new(MARGIN, mrz_start_y, mrz_width, MRZ_LINE_HEIGHT),
            Rect::new(
                MARGIN,
                mrz_start_y + MRZ_LINE_HEIGHT + MRZ_LINE_SPACING,
                mrz_width,
                MRZ_LINE_HEIGHT,
            ),
            Rect::new(
                MARGIN,
                mrz_start_y + 2 * (MRZ_LINE_HEIGHT + MRZ_LINE_SPACING),
                mrz_width,
                MRZ_LINE_HEIGHT,
            ),
        ],
        mrz_chars: TD1_MRZ_CHARS,
    }
}

/// Get MRZ character rectangle for a specific line and index.
pub fn mrz_char_rect_for_line(line: Rect, mrz_chars: u32, index: u32) -> Rect {
    debug_assert!(index < mrz_chars);
    let cell_width = line.width / mrz_chars;
    Rect::new(line.x + index * cell_width, line.y, cell_width, line.height)
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
    fn mrz_char_cells_fit_within_the_line() {
        for doc_type in [DocumentType::TD1, DocumentType::TD2, DocumentType::TD3] {
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
        for doc_type in [DocumentType::TD1, DocumentType::TD2, DocumentType::TD3] {
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
    }
}
