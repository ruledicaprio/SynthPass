//! Fixed pixel geometry for ICAO 9303 document data pages.
//!
//! Every rectangle here is a deterministic constant — the same layout is used
//! for every render, which is what lets [`crate::labels::Labels`] be 100%
//! accurate by construction: the generator knows exactly where it is about to
//! draw a field before it draws it.
//!
//! Supports TD1 (3×30 chars), TD2 (2×36 chars), and TD3 (2×44 chars) MRZ formats.

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

/// Overall data-page canvas size, roughly the ID-3 aspect ratio (125mm x 88mm).
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

/// MRZ character counts by document type.
pub const TD1_MRZ_CHARS: u32 = 30;
pub const TD2_MRZ_CHARS: u32 = 36;
pub const TD3_MRZ_CHARS: u32 = 44;

/// The MRZ band at the bottom of the page. Dimensions vary by document type.
pub const MRZ_LINE_HEIGHT: u32 = 50;
pub const MRZ_LINE_SPACING: u32 = 5; // Space between MRZ lines

/// Get MRZ character count for a document type.
pub fn mrz_chars(doc_type: DocumentType) -> u32 {
    match doc_type {
        DocumentType::TD1 => TD1_MRZ_CHARS,
        DocumentType::TD2 => TD2_MRZ_CHARS,
        DocumentType::TD3 => TD3_MRZ_CHARS,
    }
}

/// Get MRZ line rectangles for a document type.
pub fn mrz_lines(doc_type: DocumentType) -> Vec<Rect> {
    let mrz_width = 1080; // Same width for all types

    match doc_type {
        DocumentType::TD1 => {
            // TD1 has 3 lines - start higher to fit within IMAGE_HEIGHT (840px)
            let start_y = 620; // Start higher to fit 3 lines
            vec![
                Rect::new(60, start_y, mrz_width, MRZ_LINE_HEIGHT),
                Rect::new(60, start_y + MRZ_LINE_HEIGHT + MRZ_LINE_SPACING, mrz_width, MRZ_LINE_HEIGHT),
                Rect::new(60, start_y + 2*(MRZ_LINE_HEIGHT + MRZ_LINE_SPACING), mrz_width, MRZ_LINE_HEIGHT),
            ]
        }
        DocumentType::TD2 | DocumentType::TD3 => {
            // TD2 and TD3 have 2 lines - use original position
            let start_y = 720;
            vec![
                Rect::new(60, start_y, mrz_width, MRZ_LINE_HEIGHT),
                Rect::new(60, start_y + MRZ_LINE_HEIGHT + MRZ_LINE_SPACING, mrz_width, MRZ_LINE_HEIGHT),
            ]
        }
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
        // Test for all document types
        for doc_type in [DocumentType::TD1, DocumentType::TD2, DocumentType::TD3] {
            let mrz_rects = mrz_lines(doc_type);
            let chars = mrz_chars(doc_type);
            for rect in mrz_rects {
                for i in 0..chars {
                    let cell = mrz_char_rect_for_line(rect, chars, i);
                    assert!(cell.x + cell.width <= rect.x + rect.width);
                }
            }
        }
    }
}
