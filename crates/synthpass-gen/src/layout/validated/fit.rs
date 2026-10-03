//! The text-fit rule (ADR-0022 Decision 4): every visual-zone rectangle holds
//! the widest value the generator can draw into that field.
//!
//! "Holds" is measured with the renderer's own function, [`text_ink`], at the
//! renderer's own size ([`viz_px_scale`]) and position (the rectangle's origin),
//! against all four sides of the rectangle: visual-zone text is drawn unclipped
//! from `rect.x` with natural advances, so an over-long value would spill into
//! its neighbour or off the page and the label box would no longer contain the
//! ink.
//!
//! The bound is sound, not sampled. The generator's values are described by
//! [`VizDomain`], a superset of what [`crate::data::generate_passport`] can
//! produce:
//!
//! - **pooled fields** (names, country codes, the document code, sex): every
//!   pool entry is measured, Cyrillic entries included;
//! - **random fields** (document and personal numbers, dates): one value per
//!   position is not enumerable, so the extremes are built from the per-position
//!   alphabets instead. Advances are non-negative, the pen position only grows
//!   along a string, and the glyph bounds are monotone in it, so the glyph at
//!   position `j` of any string has its right edge at or left of the same
//!   glyph placed after a prefix of widest-advance characters, and its left,
//!   top and bottom edges at or inside those of the same glyph placed alone at
//!   the rectangle's origin (the first glyph's start shift is part of the pen
//!   position, so the prefix starts with the character that leaves the pen
//!   furthest right). The check measures both families and takes the union.

use super::{LayoutError, LayoutField, LayoutSpec};
use crate::data::{viz_domain, VizDomain};
use crate::fonts::Fonts;
use crate::layout::Rect;
use crate::render::{pen_start, text_ink, viz_px_scale, InkBox};

/// Fails on the first visual-zone field (in [`LayoutField::ALL`] order) whose
/// widest value puts ink outside its rectangle.
pub(super) fn check(spec: &LayoutSpec, fonts: &Fonts) -> Result<(), LayoutError> {
    for field in LayoutField::ALL {
        let rect = spec.rect(field);
        let Some(domain) = viz_domain(spec.format, field) else {
            continue;
        };
        let Some(ink) = widest_ink(&fonts.viz, &domain, rect) else {
            continue;
        };
        if !ink.within(rect) {
            return Err(LayoutError::TextDoesNotFit {
                field,
                ink: [ink.min_x, ink.min_y, ink.max_x, ink.max_y],
                rect,
            });
        }
    }
    Ok(())
}

/// The union of the ink boxes of every value in `domain`, drawn into `rect`;
/// `None` when no value has any ink.
fn widest_ink(font: &ab_glyph::FontArc, domain: &VizDomain, rect: Rect) -> Option<InkBox> {
    let px_scale = viz_px_scale(rect);
    match domain {
        VizDomain::Pool(values) => union_all(
            values
                .iter()
                .map(|value| text_ink(font, value, rect, px_scale)),
        ),
        VizDomain::Positional(positions) => {
            let mut boxes = Vec::new();
            // Left, top and bottom: each character alone at the origin.
            for &c in positions.iter().flatten() {
                boxes.push(text_ink(font, &c.to_string(), rect, px_scale));
            }
            // Right: each position's character after a prefix whose first
            // character maximises the pen position after it (its start shift
            // included, see `pen_start`), then the widest advances.
            let mut prefix = String::new();
            for (j, set) in positions.iter().enumerate() {
                for &c in set {
                    let mut value = prefix.clone();
                    value.push(c);
                    boxes.push(text_ink(font, &value, rect, px_scale));
                }
                prefix.push(if j == 0 {
                    furthest_first(font, set, rect, px_scale)
                } else {
                    widest_advance(font, set, px_scale)
                });
            }
            union_all(boxes)
        }
    }
}

fn union_all(boxes: impl IntoIterator<Item = Option<InkBox>>) -> Option<InkBox> {
    boxes.into_iter().flatten().reduce(InkBox::union)
}

/// The character of `set` with the widest advance at `px_scale`; the first of
/// equals. `set` is never empty.
fn widest_advance(font: &ab_glyph::FontArc, set: &[char], px_scale: f32) -> char {
    use ab_glyph::{Font, ScaleFont};

    let scaled = font.as_scaled(px_scale);
    let mut widest = set[0];
    let mut widest_advance = scaled.h_advance(scaled.glyph_id(widest));
    for &c in &set[1..] {
        let advance = scaled.h_advance(scaled.glyph_id(c));
        if advance > widest_advance {
            widest = c;
            widest_advance = advance;
        }
    }
    widest
}

/// The character of `set` that leaves the pen furthest right after it when it
/// starts a string in `rect`; the first of equals.
fn furthest_first(font: &ab_glyph::FontArc, set: &[char], rect: Rect, px_scale: f32) -> char {
    use ab_glyph::{Font, ScaleFont};

    let scaled = font.as_scaled(px_scale);
    let after = |c: char| pen_start(font, c, rect, px_scale) + scaled.h_advance(scaled.glyph_id(c));
    let mut best = set[0];
    let mut best_after = after(best);
    for &c in &set[1..] {
        let a = after(c);
        if a > best_after {
            best = c;
            best_after = a;
        }
    }
    best
}
