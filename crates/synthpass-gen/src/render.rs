//! Compose the rendered `DynamicImage` for a generated document.
//!
//! Two guardrails in this module are **unconditional** — they render
//! regardless of the `embedded-fonts` feature and cannot be disabled through
//! [`crate::GeneratorConfig`]:
//!
//! 1. [`draw_watermark`] stamps a "SYNTHETIC / SPECIMEN" watermark, drawn from
//!    a hand-authored 5x7 bitmap font defined in this file — it never depends
//!    on a TTF being present.
//! 2. The background composed by [`render`] is a generic, non-country
//!    template: a plain frame and neutral fill, no national emblem, coat of
//!    arms, or issuing-country branding of any kind.
//!
//! When `embedded-fonts` is off (`--no-default-features`; it is on by
//! default), VIZ and MRZ text degrade to placeholder bars drawn in the exact
//! [`crate::layout`] rectangles, so bounding boxes stay meaningful even
//! without real glyphs.
//!
//! The renderer draws whatever [`crate::layout::ValidatedLayout`] it is given
//! ([`render_with_layout`]); [`render`] and [`render_with`] use the built-in
//! layout of the requested format. VIZ text is measured by one function,
//! [`text_ink`], which both the glyph drawing and the layout fit check
//! ([`crate::layout::ValidatedLayout::try_from_spec`]) call, so the check
//! measures exactly what is drawn.

use image::{DynamicImage, Rgb, RgbImage};

use crate::fonts::{load_fonts, Fonts};
use crate::labels::Labels;
use crate::layout::{self, PageLayout, Rect, ValidatedLayout};
use crate::model::{DocumentType, Passport};

/// Blur sigma for the uniform MRZ redaction, in cell widths. A sweep seed for #565 PR 6, not a measured value.
pub const REDACT_BLUR_SIGMA_CELLS: f32 = 0.5;
/// First-cell graded blur sigma in cell widths. A sweep seed for #565 PR 6, not a measured value.
pub const REDACT_GRADED_BLUR_MIN_CELLS: f32 = 0.1;
/// Last-cell graded blur sigma in cell widths. A sweep seed for #565 PR 6, not a measured value.
pub const REDACT_GRADED_BLUR_MAX_CELLS: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactStyle {
    FillBlack,
    FillWhite,
    FillGrey,
    Blur,
    GradedBlur,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedactSpan {
    line: usize,
    first: usize,
    last: usize,
    style: RedactStyle,
    doc_type: DocumentType,
}

impl RedactSpan {
    pub fn new(
        doc_type: DocumentType,
        line: usize,
        first: usize,
        last: usize,
        style: RedactStyle,
    ) -> Result<Self, String> {
        let page = layout::for_format(doc_type);
        if line >= page.mrz_lines.len() {
            return Err(format!(
                "line {line} is out of range for {}",
                doc_type.as_str()
            ));
        }
        if first > last {
            return Err(format!("first cell {first} exceeds last cell {last}"));
        }
        if last >= page.mrz_chars as usize {
            return Err(format!(
                "last cell {last} is out of range (line width {})",
                page.mrz_chars
            ));
        }
        Ok(Self {
            line,
            first,
            last,
            style,
            doc_type,
        })
    }
    pub fn line(self) -> usize {
        self.line
    }
    pub fn first(self) -> usize {
        self.first
    }
    pub fn last(self) -> usize {
        self.last
    }
    pub fn style(self) -> RedactStyle {
        self.style
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderOptions {
    pub redact: Option<RedactSpan>,
}

const BACKGROUND: Rgb<u8> = Rgb([244, 243, 236]);
const FRAME: Rgb<u8> = Rgb([70, 72, 90]);
const PORTRAIT_FILL: Rgb<u8> = Rgb([205, 205, 210]);
const PLACEHOLDER_BAR: Rgb<u8> = Rgb([120, 122, 140]);
const MRZ_CELL: Rgb<u8> = Rgb([30, 30, 40]);
const WATERMARK_COLOR: Rgb<u8> = Rgb([176, 48, 48]);

fn fill_rect(img: &mut RgbImage, rect: Rect, color: Rgb<u8>) {
    for y in rect.y..(rect.y + rect.height).min(img.height()) {
        for x in rect.x..(rect.x + rect.width).min(img.width()) {
            img.put_pixel(x, y, color);
        }
    }
}

fn border_rect(img: &mut RgbImage, rect: Rect, color: Rgb<u8>, thickness: u32) {
    let t = thickness.max(1);
    fill_rect(img, Rect::new(rect.x, rect.y, rect.width, t), color);
    fill_rect(
        img,
        Rect::new(
            rect.x,
            (rect.y + rect.height).saturating_sub(t),
            rect.width,
            t,
        ),
        color,
    );
    fill_rect(img, Rect::new(rect.x, rect.y, t, rect.height), color);
    fill_rect(
        img,
        Rect::new(
            (rect.x + rect.width).saturating_sub(t),
            rect.y,
            t,
            rect.height,
        ),
        color,
    );
}

/// A placeholder bar occupying most of `rect`'s height, vertically centered —
/// stands in for VIZ text when no real font is embedded.
fn draw_placeholder_bar(img: &mut RgbImage, rect: Rect) {
    let inset_y = rect.height / 4;
    let bar = Rect::new(
        rect.x,
        rect.y + inset_y,
        rect.width,
        rect.height.saturating_sub(2 * inset_y).max(1),
    );
    fill_rect(img, bar, PLACEHOLDER_BAR);
}

/// Placeholder rendering of one MRZ line: each of the line's `mrz_chars`
/// character cells (30, 36 or 44, by format) is filled when the printed character is not the `<` filler, and left blank
/// otherwise — this keeps the per-character bounding boxes meaningful (filler
/// runs stay visually empty) without needing real glyphs.
fn draw_mrz_placeholder(img: &mut RgbImage, line_rect: Rect, text: &str, mrz_chars: u32) {
    for (i, c) in text.chars().enumerate() {
        if c == '<' {
            continue;
        }
        let cell = layout::mrz_char_rect_for_line(line_rect, mrz_chars, i as u32);
        let inset = cell.height / 5;
        let glyph_box = Rect::new(
            cell.x + 1,
            cell.y + inset,
            cell.width.saturating_sub(2),
            cell.height.saturating_sub(2 * inset).max(1),
        );
        fill_rect(img, glyph_box, MRZ_CELL);
    }
}

// ---------------------------------------------------------------------
// Real glyph rendering (only compiled with `embedded-fonts`).
// ---------------------------------------------------------------------

/// Alpha-blend `fg` over `bg` by the rasterizer's per-pixel coverage — using
/// the full coverage gradient (instead of a hard on/off threshold) preserves
/// the sub-pixel edge shape that OCR models rely on to distinguish
/// similarly-shaped glyphs (e.g. `7`/`Z`, `O`/`0`).
fn blend(bg: Rgb<u8>, fg: Rgb<u8>, coverage: f32) -> Rgb<u8> {
    let a = coverage.clamp(0.0, 1.0);
    let mut out = [0u8; 3];
    for k in 0..3 {
        out[k] = (bg[k] as f32 * (1.0 - a) + fg[k] as f32 * a).round() as u8;
    }
    Rgb(out)
}

#[cfg(feature = "embedded-fonts")]
fn draw_one_glyph(img: &mut RgbImage, font: &ab_glyph::FontArc, glyph: ab_glyph::Glyph) {
    use ab_glyph::Font;

    let Some(outlined) = font.outline_glyph(glyph) else {
        return;
    };
    let bounds = outlined.px_bounds();
    outlined.draw(|gx, gy, coverage| {
        if coverage <= 0.0 {
            return;
        }
        let px = bounds.min.x as i32 + gx as i32;
        let py = bounds.min.y as i32 + gy as i32;
        if px >= 0 && py >= 0 {
            let (px, py) = (px as u32, py as u32);
            if px < img.width() && py < img.height() {
                let bg = *img.get_pixel(px, py);
                img.put_pixel(px, py, blend(bg, MRZ_CELL, coverage));
            }
        }
    });
}

/// The pixel size VIZ text is drawn at inside `rect`: 70% of its height.
pub(crate) fn viz_px_scale(rect: Rect) -> f32 {
    rect.height as f32 * 0.7
}

/// How far right of the rectangle's origin `first` must start so its ink box
/// does not begin left of it: the whole pixels of its negative left side
/// bearing (the sans `J` hooks about one pixel left of its pen position), `0.0`
/// for every glyph whose bounds start at or right of the pen.
pub(crate) fn left_overhang(font: &ab_glyph::FontArc, first: char, px_scale: f32) -> f32 {
    use ab_glyph::{Font, ScaleFont};

    let glyph = font.as_scaled(px_scale).scaled_glyph(first);
    font.outline_glyph(glyph)
        .map_or(0.0, |outlined| (-outlined.px_bounds().min.x).max(0.0))
}

/// Where the pen starts for `text` in `rect`: the rectangle's left edge, moved
/// right by [`left_overhang`] of the first character, so the ink box starts
/// inside the rectangle.
pub(crate) fn pen_start(
    font: &ab_glyph::FontArc,
    text_first: char,
    rect: Rect,
    px_scale: f32,
) -> f32 {
    rect.x as f32 + left_overhang(font, text_first, px_scale)
}

/// The glyphs of `text` as the VIZ drawing places them: flowed left-to-right
/// from [`pen_start`] using the font's own advance widths (no kerning, no
/// clipping), with the baseline at `rect.y` plus the scaled ascent.
/// [`draw_glyph_text`] draws exactly these glyphs, and [`text_ink`] measures
/// exactly these.
fn flow_glyphs(
    font: &ab_glyph::FontArc,
    text: &str,
    rect: Rect,
    px_scale: f32,
) -> Vec<ab_glyph::Glyph> {
    use ab_glyph::{point, Font, ScaleFont};

    let scaled = font.as_scaled(px_scale);
    let Some(first) = text.chars().next() else {
        return Vec::new();
    };
    let mut x = pen_start(font, first, rect, px_scale);
    let y = rect.y as f32 + scaled.ascent();
    let mut glyphs = Vec::with_capacity(text.chars().count());
    for c in text.chars() {
        let id = scaled.glyph_id(c);
        glyphs.push(id.with_scale_and_position(px_scale, point(x, y)));
        x += scaled.h_advance(id);
    }
    glyphs
}

/// The integer pixel box (`min` inclusive, `max` exclusive) that drawing a text
/// can touch: the union of the glyphs' rasterizer bounds, which is also the
/// range [`draw_one_glyph`] writes within.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InkBox {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl InkBox {
    pub(crate) fn union(self, other: InkBox) -> InkBox {
        InkBox {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    /// Whether this box lies within `rect` on all four sides.
    pub(crate) fn within(self, rect: Rect) -> bool {
        let right = i64::from(rect.x) + i64::from(rect.width);
        let bottom = i64::from(rect.y) + i64::from(rect.height);
        i64::from(self.min_x) >= i64::from(rect.x)
            && i64::from(self.min_y) >= i64::from(rect.y)
            && i64::from(self.max_x) <= right
            && i64::from(self.max_y) <= bottom
    }
}

/// The ink box of `text` drawn into `rect` at `px_scale`, or `None` when no
/// glyph has an outline (an empty string, or only spaces). This is the one
/// text-extent function: [`draw_glyph_text`] draws the same glyphs
/// [`flow_glyphs`] yields, and the layout fit check calls this.
pub(crate) fn text_ink(
    font: &ab_glyph::FontArc,
    text: &str,
    rect: Rect,
    px_scale: f32,
) -> Option<InkBox> {
    use ab_glyph::Font;

    flow_glyphs(font, text, rect, px_scale)
        .into_iter()
        .filter_map(|glyph| font.outline_glyph(glyph))
        .map(|outlined| {
            let bounds = outlined.px_bounds();
            InkBox {
                min_x: bounds.min.x as i32,
                min_y: bounds.min.y as i32,
                max_x: bounds.max.x as i32,
                max_y: bounds.max.y as i32,
            }
        })
        .reduce(InkBox::union)
}

/// Draws `text` as [`flow_glyphs`] places it — fine for VIZ fields, which
/// aren't checksum-validated and just need to look plausible within `rect`.
#[cfg(feature = "embedded-fonts")]
fn draw_glyph_text(
    img: &mut RgbImage,
    font: &ab_glyph::FontArc,
    text: &str,
    rect: Rect,
    px_scale: f32,
) {
    for glyph in flow_glyphs(font, text, rect, px_scale) {
        draw_one_glyph(img, font, glyph);
    }
}

/// Draws one MRZ character per fixed-width cell from
/// [`layout::mrz_char_rect_for_line`], centering each glyph within its own cell
/// instead of flowing by the font's natural advance. MRZ text is checksum-
/// validated after OCR, so cross-character drift from an advance/cell-width
/// mismatch (which compounds over all 44 columns) must not be allowed to
/// merge or overlap adjacent glyphs.
#[cfg(feature = "embedded-fonts")]
fn draw_mrz_glyphs(
    img: &mut RgbImage,
    font: &ab_glyph::FontArc,
    text: &str,
    line_rect: Rect,
    mrz_chars: u32,
) {
    use ab_glyph::{point, Font, ScaleFont};

    let px_scale = layout::MRZ_FONT_PX;
    let scaled = font.as_scaled(px_scale);
    let y = line_rect.y as f32 + scaled.ascent();
    for (i, c) in text.chars().enumerate() {
        let cell = layout::mrz_char_rect_for_line(line_rect, mrz_chars, i as u32);
        let id = scaled.glyph_id(c);
        let advance = scaled.h_advance(id);
        let x = cell.x as f32 + ((cell.width as f32 - advance) / 2.0).max(0.0);
        let glyph = id.with_scale_and_position(px_scale, point(x, y));
        draw_one_glyph(img, font, glyph);
    }
}

fn draw_text_field(img: &mut RgbImage, rect: Rect, text: &str, fonts: Option<&Fonts>) {
    #[cfg(feature = "embedded-fonts")]
    if let Some(fonts) = fonts {
        draw_glyph_text(img, &fonts.viz, text, rect, viz_px_scale(rect));
        return;
    }
    #[cfg(not(feature = "embedded-fonts"))]
    let _ = (fonts, text);
    draw_placeholder_bar(img, rect);
}

fn draw_mrz_line(
    img: &mut RgbImage,
    rect: Rect,
    text: &str,
    mrz_chars: u32,
    fonts: Option<&Fonts>,
) {
    #[cfg(feature = "embedded-fonts")]
    if let Some(fonts) = fonts {
        draw_mrz_glyphs(img, &fonts.mrz, text, rect, mrz_chars);
        return;
    }
    #[cfg(not(feature = "embedded-fonts"))]
    let _ = fonts;
    draw_mrz_placeholder(img, rect, text, mrz_chars);
}

// ---------------------------------------------------------------------
// Hand-authored 5x7 bitmap font — used ONLY for the mandatory watermark, so
// the "SYNTHETIC / SPECIMEN" stamp never depends on a TTF being available.
// ---------------------------------------------------------------------

/// 7 rows x 5 columns, `1` = ink. Covers exactly the characters needed to
/// spell "SYNTHETIC / SPECIMEN".
fn glyph_5x7(c: char) -> [[u8; 5]; 7] {
    match c {
        'S' => [
            [0, 1, 1, 1, 1],
            [1, 0, 0, 0, 0],
            [1, 0, 0, 0, 0],
            [0, 1, 1, 1, 0],
            [0, 0, 0, 0, 1],
            [0, 0, 0, 0, 1],
            [1, 1, 1, 1, 0],
        ],
        'Y' => [
            [1, 0, 0, 0, 1],
            [1, 0, 0, 0, 1],
            [0, 1, 0, 1, 0],
            [0, 0, 1, 0, 0],
            [0, 0, 1, 0, 0],
            [0, 0, 1, 0, 0],
            [0, 0, 1, 0, 0],
        ],
        'N' => [
            [1, 0, 0, 0, 1],
            [1, 1, 0, 0, 1],
            [1, 0, 1, 0, 1],
            [1, 0, 0, 1, 1],
            [1, 0, 0, 0, 1],
            [1, 0, 0, 0, 1],
            [1, 0, 0, 0, 1],
        ],
        'T' => [
            [1, 1, 1, 1, 1],
            [0, 0, 1, 0, 0],
            [0, 0, 1, 0, 0],
            [0, 0, 1, 0, 0],
            [0, 0, 1, 0, 0],
            [0, 0, 1, 0, 0],
            [0, 0, 1, 0, 0],
        ],
        'H' => [
            [1, 0, 0, 0, 1],
            [1, 0, 0, 0, 1],
            [1, 0, 0, 0, 1],
            [1, 1, 1, 1, 1],
            [1, 0, 0, 0, 1],
            [1, 0, 0, 0, 1],
            [1, 0, 0, 0, 1],
        ],
        'E' => [
            [1, 1, 1, 1, 1],
            [1, 0, 0, 0, 0],
            [1, 0, 0, 0, 0],
            [1, 1, 1, 1, 0],
            [1, 0, 0, 0, 0],
            [1, 0, 0, 0, 0],
            [1, 1, 1, 1, 1],
        ],
        'I' => [
            [0, 1, 1, 1, 0],
            [0, 0, 1, 0, 0],
            [0, 0, 1, 0, 0],
            [0, 0, 1, 0, 0],
            [0, 0, 1, 0, 0],
            [0, 0, 1, 0, 0],
            [0, 1, 1, 1, 0],
        ],
        'C' => [
            [0, 1, 1, 1, 1],
            [1, 0, 0, 0, 0],
            [1, 0, 0, 0, 0],
            [1, 0, 0, 0, 0],
            [1, 0, 0, 0, 0],
            [1, 0, 0, 0, 0],
            [0, 1, 1, 1, 1],
        ],
        'P' => [
            [1, 1, 1, 1, 0],
            [1, 0, 0, 0, 1],
            [1, 0, 0, 0, 1],
            [1, 1, 1, 1, 0],
            [1, 0, 0, 0, 0],
            [1, 0, 0, 0, 0],
            [1, 0, 0, 0, 0],
        ],
        'M' => [
            [1, 0, 0, 0, 1],
            [1, 1, 0, 1, 1],
            [1, 0, 1, 0, 1],
            [1, 0, 1, 0, 1],
            [1, 0, 0, 0, 1],
            [1, 0, 0, 0, 1],
            [1, 0, 0, 0, 1],
        ],
        '/' => [
            [0, 0, 0, 0, 1],
            [0, 0, 0, 0, 1],
            [0, 0, 0, 1, 0],
            [0, 0, 1, 0, 0],
            [0, 1, 0, 0, 0],
            [1, 0, 0, 0, 0],
            [1, 0, 0, 0, 0],
        ],
        _ => [[0; 5]; 7], // space and anything unrecognized: blank
    }
}

fn draw_bitmap_char(img: &mut RgbImage, c: char, x: u32, y: u32, scale: u32, color: Rgb<u8>) {
    let glyph = glyph_5x7(c);
    for (row_idx, row) in glyph.iter().enumerate() {
        for (col_idx, &on) in row.iter().enumerate() {
            if on == 0 {
                continue;
            }
            let px0 = x + col_idx as u32 * scale;
            let py0 = y + row_idx as u32 * scale;
            for dy in 0..scale {
                for dx in 0..scale {
                    let (px, py) = (px0 + dx, py0 + dy);
                    if px < img.width() && py < img.height() {
                        img.put_pixel(px, py, color);
                    }
                }
            }
        }
    }
}

/// Draw "SYNTHETIC / SPECIMEN" from the hand-authored bitmap font in `rect`
/// (that format's [`PageLayout::watermark`]), tiled to fill the band. This is
/// one of the two mandatory ethics guardrails: it renders unconditionally,
/// independent of the `embedded-fonts` feature, not configurable via
/// [`crate::GeneratorConfig`], and — per this function's signature taking
/// `rect` rather than a hardcoded constant — independent of document format
/// too: every [`PageLayout`] carries its own watermark band, and `render`
/// calls this for TD1/TD2/TD3 alike.
fn draw_watermark(img: &mut RgbImage, rect: Rect) {
    const TEXT: &str = "SYNTHETIC / SPECIMEN ";
    let scale = 4u32;
    let glyph_w = 5 * scale;
    let advance = glyph_w + scale;
    let glyph_h = 7 * scale;
    let y = rect.y + rect.height.saturating_sub(glyph_h) / 2;

    let mut x = rect.x;
    'tiles: loop {
        for c in TEXT.chars() {
            if x + glyph_w > rect.x + rect.width {
                break 'tiles;
            }
            draw_bitmap_char(img, c, x, y, scale, WATERMARK_COLOR);
            x += advance;
        }
    }
}

/// Compose the full data-page image for `passport`, using `labels` as the
/// single source of truth for text content and placement, on `doc_type`'s
/// own card canvas ([`layout::for_format`]) — TD1/TD2/TD3 each get their own
/// `PageLayout` rather than sharing TD3's fixed 1200x840 canvas.
pub fn render(passport: &Passport, labels: &Labels, doc_type: DocumentType) -> DynamicImage {
    render_with(passport, labels, doc_type, &RenderOptions::default())
        .expect("default render options are valid")
}

/// [`render`] with options, on `doc_type`'s built-in layout.
pub fn render_with(
    passport: &Passport,
    labels: &Labels,
    doc_type: DocumentType,
    options: &RenderOptions,
) -> Result<DynamicImage, String> {
    render_with_layout(
        passport,
        labels,
        ValidatedLayout::builtin(doc_type),
        options,
    )
}

/// [`render_with`] on an explicit [`ValidatedLayout`]: the layout's format
/// picks the canvas, watermark band and MRZ, and its rectangles place the
/// portrait and the visual-zone fields. `labels` must have been built from the
/// same layout ([`crate::labels::build_labels_with_layout`]).
pub fn render_with_layout(
    passport: &Passport,
    labels: &Labels,
    layout: &ValidatedLayout,
    options: &RenderOptions,
) -> Result<DynamicImage, String> {
    // `labels` is the single source of drawn text (kept in sync with
    // `passport` by construction, see `labels::build_labels`); `passport` is
    // accepted for API symmetry with `crate::generate` and future per-field
    // render options (e.g. photo synthesis keyed off sex/nationality).
    let _ = passport;
    let doc_type = layout.format();
    let page: &PageLayout = layout.page();
    if let Some(span) = options.redact {
        if span.doc_type != doc_type {
            return Err("redaction document type does not match render document type".into());
        }
        // Revalidate here so no externally composed options can index outside the page.
        RedactSpan::new(doc_type, span.line, span.first, span.last, span.style)?;
    }
    let mut img = RgbImage::from_pixel(page.width, page.height, BACKGROUND);

    // Generic, non-country template: a plain frame only. Deliberately no
    // national emblem, coat of arms, or issuing-country branding — guardrail
    // #2, unconditional regardless of `embedded-fonts`.
    border_rect(
        &mut img,
        Rect::new(0, 0, page.width, page.height),
        FRAME,
        layout::FRAME_THICKNESS,
    );

    // Portrait placeholder: a plain filled box, never a rendered likeness.
    fill_rect(&mut img, page.portrait, PORTRAIT_FILL);
    border_rect(&mut img, page.portrait, FRAME, 2);

    let fonts: Option<Fonts> = load_fonts().ok();
    let fonts_ref = fonts.as_ref();

    draw_text_field(
        &mut img,
        page.document_type,
        &labels.document_type.value,
        fonts_ref,
    );
    draw_text_field(
        &mut img,
        page.issuing_country,
        &labels.issuing_country.value,
        fonts_ref,
    );
    // A real passport prints the name in its native script; the MRZ band
    // (drawn below from `labels.mrz_lines`) carries the Latin transliteration.
    // For a Latin-script identity `*_native` is `None` and `surname`/
    // `given_names` are already the printed form.
    let surname_viz = labels
        .surname_native
        .as_ref()
        .map_or(labels.surname.value.as_str(), |fl| fl.value.as_str());
    let given_names_viz = labels
        .given_names_native
        .as_ref()
        .map_or(labels.given_names.value.as_str(), |fl| fl.value.as_str());
    draw_text_field(&mut img, page.surname, surname_viz, fonts_ref);
    draw_text_field(&mut img, page.given_names, given_names_viz, fonts_ref);
    draw_text_field(
        &mut img,
        page.document_number,
        &labels.document_number.value,
        fonts_ref,
    );
    draw_text_field(
        &mut img,
        page.nationality,
        &labels.nationality.value,
        fonts_ref,
    );
    draw_text_field(
        &mut img,
        page.date_of_birth,
        &labels.date_of_birth.value,
        fonts_ref,
    );
    draw_text_field(&mut img, page.sex, &labels.sex.value, fonts_ref);
    draw_text_field(
        &mut img,
        page.date_of_expiry,
        &labels.date_of_expiry.value,
        fonts_ref,
    );
    if let Some(pn) = &labels.personal_number {
        draw_text_field(&mut img, page.personal_number, &pn.value, fonts_ref);
    }

    // Draw MRZ lines from this format's own PageLayout.
    for (i, mrz_line) in labels.mrz_lines.iter().enumerate() {
        if i < page.mrz_lines.len() {
            draw_mrz_line(
                &mut img,
                page.mrz_lines[i],
                mrz_line,
                page.mrz_chars,
                fonts_ref,
            );
        }
    }

    if let Some(span) = options.redact {
        let line = page.mrz_lines[span.line];
        let x = line.x + span.first as u32 * layout::MRZ_CELL_WIDTH;
        let width = (span.last - span.first + 1) as u32 * layout::MRZ_CELL_WIDTH;
        let region = Rect::new(x, line.y, width, line.height);
        match span.style {
            RedactStyle::FillBlack | RedactStyle::FillWhite | RedactStyle::FillGrey => {
                let tone = match span.style {
                    RedactStyle::FillBlack => Rgb([0, 0, 0]),
                    RedactStyle::FillWhite => Rgb([255, 255, 255]),
                    _ => Rgb([128, 128, 128]),
                };
                fill_rect(&mut img, region, tone);
            }
            RedactStyle::Blur => blur_region(
                &mut img,
                region,
                REDACT_BLUR_SIGMA_CELLS * layout::MRZ_CELL_WIDTH as f32,
            ),
            RedactStyle::GradedBlur => {
                let count = span.last - span.first + 1;
                for offset in 0..count {
                    let t = if count <= 1 {
                        0.0
                    } else {
                        offset as f32 / (count - 1) as f32
                    };
                    let sigma = REDACT_GRADED_BLUR_MIN_CELLS
                        + t * (REDACT_GRADED_BLUR_MAX_CELLS - REDACT_GRADED_BLUR_MIN_CELLS);
                    blur_region(
                        &mut img,
                        Rect::new(
                            x + offset as u32 * layout::MRZ_CELL_WIDTH,
                            line.y,
                            layout::MRZ_CELL_WIDTH,
                            line.height,
                        ),
                        sigma * layout::MRZ_CELL_WIDTH as f32,
                    );
                }
            }
        }
    }

    // Guardrail #1: unconditional synthetic watermark, drawn last so it stays
    // on top of every other element — on every format's own watermark band.
    draw_watermark(&mut img, page.watermark);

    Ok(DynamicImage::ImageRgb8(img))
}

fn blur_region(img: &mut RgbImage, rect: Rect, sigma: f32) {
    let crop = image::imageops::crop_imm(img, rect.x, rect.y, rect.width, rect.height).to_image();
    let blurred = image::imageops::blur(&crop, sigma);
    image::imageops::replace(img, &blurred, i64::from(rect.x), i64::from(rect.y));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_or_mismatched_redaction_spans_are_rejected_without_panicking() {
        assert!(RedactSpan::new(DocumentType::TD1, 3, 0, 1, RedactStyle::Blur).is_err());
        assert!(RedactSpan::new(DocumentType::TD3, 0, 2, 1, RedactStyle::Blur).is_err());
        let config = crate::GeneratorConfig::with_document_type(9, DocumentType::TD1);
        let passport = crate::data::generate_passport(&config);
        let labels = crate::labels::build_labels(&passport, DocumentType::TD1);
        let span = RedactSpan::new(DocumentType::TD3, 0, 0, 1, RedactStyle::Blur).unwrap();
        assert!(render_with(
            &passport,
            &labels,
            DocumentType::TD1,
            &RenderOptions { redact: Some(span) }
        )
        .is_err());
        assert!(
            crate::generate_with(&passport, &config, &RenderOptions { redact: Some(span) })
                .is_err()
        );
    }

    #[test]
    fn redaction_changes_only_the_requested_cells_and_keeps_watermark() {
        let config = crate::GeneratorConfig::with_document_type(17, DocumentType::TD3);
        let passport = crate::data::generate_passport(&config);
        let labels = crate::labels::build_labels(&passport, DocumentType::TD3);
        let plain = render(&passport, &labels, DocumentType::TD3).to_rgb8();
        for style in [
            RedactStyle::FillBlack,
            RedactStyle::FillWhite,
            RedactStyle::FillGrey,
            RedactStyle::Blur,
            RedactStyle::GradedBlur,
        ] {
            let span = RedactSpan::new(DocumentType::TD3, 0, 5, 7, style).unwrap();
            let redacted = render_with(
                &passport,
                &labels,
                DocumentType::TD3,
                &RenderOptions { redact: Some(span) },
            )
            .unwrap()
            .to_rgb8();
            let line = layout::for_format(DocumentType::TD3).mrz_lines[0];
            let x0 = line.x + 5 * layout::MRZ_CELL_WIDTH;
            let x1 = line.x + 8 * layout::MRZ_CELL_WIDTH;
            for y in 0..plain.height() {
                for x in 0..plain.width() {
                    let inside = x >= x0 && x < x1 && y >= line.y && y < line.y + line.height;
                    if !inside {
                        assert_eq!(
                            plain.get_pixel(x, y),
                            redacted.get_pixel(x, y),
                            "outside span ({x},{y}) {style:?}"
                        );
                    }
                }
            }
            if let RedactStyle::FillBlack | RedactStyle::FillWhite | RedactStyle::FillGrey = style {
                let expected = match style {
                    RedactStyle::FillBlack => Rgb([0, 0, 0]),
                    RedactStyle::FillWhite => Rgb([255, 255, 255]),
                    _ => Rgb([128, 128, 128]),
                };
                for y in line.y..line.y + line.height {
                    for x in x0..x1 {
                        assert_eq!(*redacted.get_pixel(x, y), expected);
                    }
                }
            } else {
                for cell in 5..=7 {
                    let left = line.x + cell * layout::MRZ_CELL_WIDTH;
                    assert!(
                        (line.y..line.y + line.height)
                            .any(|y| (left..left + layout::MRZ_CELL_WIDTH)
                                .any(|x| plain.get_pixel(x, y) != redacted.get_pixel(x, y))),
                        "cell {cell} unchanged"
                    );
                }
            }
            let page = layout::for_format(DocumentType::TD3);
            for y in page.watermark.y..page.watermark.y + page.watermark.height {
                for x in page.watermark.x..page.watermark.x + page.watermark.width {
                    assert_eq!(plain.get_pixel(x, y), redacted.get_pixel(x, y));
                }
            }
        }
    }

    #[test]
    fn mrz_cell_geometry_is_contiguous_for_td1_and_td3() {
        use crate::data::generate_passport;
        use crate::labels::build_labels;
        use crate::model::GeneratorConfig;
        for doc_type in [DocumentType::TD1, DocumentType::TD3] {
            let page = layout::for_format(doc_type);
            let passport = generate_passport(&GeneratorConfig::with_document_type(616, doc_type));
            let labels = build_labels(&passport, doc_type);
            let mut blank_labels = labels.clone();
            blank_labels.mrz_lines = labels
                .mrz_lines
                .iter()
                .map(|s| "<".repeat(s.chars().count()))
                .collect();
            let blank = render(&passport, &blank_labels, doc_type).to_rgb8();
            for (line_idx, line) in page.mrz_lines.iter().enumerate() {
                assert_eq!(line.width, page.mrz_chars * layout::MRZ_CELL_WIDTH);
                let chars: Vec<char> = labels.mrz_lines[line_idx].chars().collect();
                for cell in 0..page.mrz_chars {
                    let rect = layout::mrz_char_rect_for_line(*line, page.mrz_chars, cell);
                    assert!(rect.within_bounds(page.width, page.height));
                    if chars[cell as usize] == '<' {
                        continue;
                    }
                    let mut isolated = labels.clone();
                    let mut line_chars = vec!['<'; page.mrz_chars as usize];
                    line_chars[cell as usize] = chars[cell as usize];
                    isolated.mrz_lines[line_idx] = line_chars.into_iter().collect();
                    let isolated = render(&passport, &isolated, doc_type).to_rgb8();
                    let mut ink = false;
                    for y in 0..page.height {
                        for x in 0..page.width {
                            if isolated.get_pixel(x, y) != blank.get_pixel(x, y) {
                                if x >= rect.x
                                    && x < rect.x + rect.width
                                    && y >= rect.y
                                    && y < rect.y + rect.height
                                {
                                    ink = true;
                                } else if x >= line.x
                                    && x < line.x + line.width
                                    && y >= line.y
                                    && y < line.y + line.height
                                {
                                    panic!("{doc_type:?} line {line_idx} cell {cell} ink leaked to ({x},{y})");
                                }
                            }
                        }
                    }
                    assert!(ink, "{doc_type:?} line {line_idx} cell {cell} has no ink");
                }
            }
        }
    }

    #[test]
    fn watermark_and_mrz_cells_are_disjoint_for_every_format() {
        use crate::data::generate_passport;
        use crate::labels::build_labels;
        use crate::model::GeneratorConfig;
        for doc_type in [
            DocumentType::TD1,
            DocumentType::TD2,
            DocumentType::TD3,
            DocumentType::MrvA,
            DocumentType::MrvB,
        ] {
            let config = GeneratorConfig::with_document_type(616, doc_type);
            let passport = generate_passport(&config);
            let labels = build_labels(&passport, doc_type);
            let page = layout::for_format(doc_type);
            let plain = render(&passport, &labels, doc_type).to_rgb8();
            let span = RedactSpan::new(doc_type, 0, 0, 2, RedactStyle::FillGrey).unwrap();
            let covered = render_with(
                &passport,
                &labels,
                doc_type,
                &RenderOptions { redact: Some(span) },
            )
            .unwrap()
            .to_rgb8();
            for y in page.watermark.y..page.watermark.y + page.watermark.height {
                for x in page.watermark.x..page.watermark.x + page.watermark.width {
                    assert_eq!(plain.get_pixel(x, y), covered.get_pixel(x, y));
                }
            }
            for line in &page.mrz_lines {
                for cell in 0..page.mrz_chars {
                    let r = layout::mrz_char_rect_for_line(*line, page.mrz_chars, cell);
                    let overlaps = r.x < page.watermark.x + page.watermark.width
                        && page.watermark.x < r.x + r.width
                        && r.y < page.watermark.y + page.watermark.height
                        && page.watermark.y < r.y + r.height;
                    assert!(!overlaps, "{doc_type:?} cell {cell}");
                }
            }
        }
    }

    #[test]
    fn watermark_renders_without_embedded_fonts() {
        let img = RgbImage::from_pixel(layout::IMAGE_WIDTH, layout::IMAGE_HEIGHT, BACKGROUND);
        let mut watermarked = img.clone();
        draw_watermark(&mut watermarked, layout::WATERMARK);

        let rect = layout::WATERMARK;
        let mut differs = false;
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                if watermarked.get_pixel(x, y) != img.get_pixel(x, y) {
                    differs = true;
                    break;
                }
            }
        }
        assert!(
            differs,
            "watermark region must differ from a blank template"
        );
    }

    /// The M6 plan's per-format watermark guardrail test: every format's
    /// `render` output must carry a visibly different watermark band from a
    /// blank canvas of that format's own size, not just TD3's.
    #[test]
    fn watermark_renders_on_every_document_format() {
        use crate::data::generate_passport;
        use crate::labels::build_labels;
        use crate::model::GeneratorConfig;

        for doc_type in [
            DocumentType::TD1,
            DocumentType::TD2,
            DocumentType::TD3,
            DocumentType::MrvA,
            DocumentType::MrvB,
        ] {
            let passport = generate_passport(&GeneratorConfig::with_document_type(1, doc_type));
            let labels = build_labels(&passport, doc_type);
            let image = render(&passport, &labels, doc_type);
            let rgb = image.to_rgb8();

            let rect = layout::for_format(doc_type).watermark;
            let mut differs = false;
            for y in rect.y..rect.y + rect.height {
                for x in rect.x..rect.x + rect.width {
                    if rgb.get_pixel(x, y).0 != BACKGROUND.0 {
                        differs = true;
                        break;
                    }
                }
            }
            assert!(
                differs,
                "{doc_type:?}: watermark region must differ from the background fill"
            );
        }
    }

    #[cfg(feature = "embedded-fonts")]
    mod text_extent {
        use super::*;
        use crate::fonts::load_fonts;

        /// Bounding box of the pixels `draw_glyph_text` changes.
        fn drawn_box(text: &str, rect: Rect) -> Option<(i32, i32, i32, i32)> {
            let fonts = load_fonts().expect("embedded fonts load");
            let white = Rgb([255, 255, 255]);
            let mut img =
                RgbImage::from_pixel(rect.x + rect.width + 40, rect.y + rect.height + 40, white);
            draw_glyph_text(&mut img, &fonts.viz, text, rect, viz_px_scale(rect));
            let mut found: Option<(i32, i32, i32, i32)> = None;
            for (x, y, px) in img.enumerate_pixels() {
                if *px == white {
                    continue;
                }
                let (x, y) = (x as i32, y as i32);
                found = Some(found.map_or((x, y, x + 1, y + 1), |(a, b, c, d)| {
                    (a.min(x), b.min(y), c.max(x + 1), d.max(y + 1))
                }));
            }
            found
        }

        #[test]
        fn the_ink_box_covers_every_drawn_pixel_and_is_tight() {
            let fonts = load_fonts().expect("embedded fonts load");
            let rect = Rect::new(60, 120, 700, 34);
            for text in [
                "VANTERPOOL",
                "JOHAN",
                "L898902C3",
                "1974-08-12",
                "ЦВЄТКОВ",
                "M",
                "I",
            ] {
                let ink = text_ink(&fonts.viz, text, rect, viz_px_scale(rect)).expect(text);
                let (x0, y0, x1, y1) = drawn_box(text, rect).expect(text);
                // The measured box contains everything drawn ...
                assert!(
                    ink.min_x <= x0 && ink.min_y <= y0 && ink.max_x >= x1 && ink.max_y >= y1,
                    "{text}: drawn ({x0},{y0},{x1},{y1}) escapes {ink:?}"
                );
                // ... and is conservative by at most the rounding pixel.
                assert!(
                    x0 - ink.min_x <= 1
                        && y0 - ink.min_y <= 1
                        && ink.max_x - x1 <= 1
                        && ink.max_y - y1 <= 1,
                    "{text}: {ink:?} is looser than one pixel around ({x0},{y0},{x1},{y1})"
                );
            }
        }

        #[test]
        fn a_one_glyph_text_extent_is_pinned() {
            // 34 px row -> 23.8 px glyphs; pin the numbers so a change in the
            // extent function (or the font) is a visible, reviewed change.
            let fonts = load_fonts().expect("embedded fonts load");
            let rect = Rect::new(60, 120, 700, 34);
            let px = viz_px_scale(rect);
            let m = text_ink(&fonts.viz, "M", rect, px).expect("M has ink");
            let mm = text_ink(&fonts.viz, "MM", rect, px).expect("MM has ink");
            assert_eq!((m.min_x, m.max_x, mm.min_x, mm.max_x), (61, 73, 61, 88));
            assert_eq!((m.min_y, m.max_y), (mm.min_y, mm.max_y));
            // Two glyphs reach exactly one advance further than one.
            use ab_glyph::{Font, ScaleFont};
            let scaled = fonts.viz.as_scaled(px);
            let advance = scaled.h_advance(scaled.glyph_id('M'));
            let delta = (mm.max_x - m.max_x) as f32;
            assert!(
                (delta - advance).abs() <= 1.0,
                "advance {advance}, delta {delta}"
            );
            assert_eq!(text_ink(&fonts.viz, "", rect, px), None);
            assert_eq!(text_ink(&fonts.viz, "   ", rect, px), None);
        }

        #[test]
        fn text_that_hangs_left_of_the_pen_is_moved_inside_its_rectangle() {
            let fonts = load_fonts().expect("embedded fonts load");
            let rect = Rect::new(214, 142, 220, 28);
            let px = viz_px_scale(rect);
            // The sans `J` hooks about one pixel left of its pen position.
            assert!(left_overhang(&fonts.viz, 'J', px) >= 1.0);
            assert_eq!(left_overhang(&fonts.viz, ' ', px), 0.0);
            for text in ["J", "JPN", "JOHAN", "V", "VANTERPOOL", "ЖУКОВ", "UTO"] {
                let ink = text_ink(&fonts.viz, text, rect, px).expect(text);
                assert!(ink.min_x >= rect.x as i32, "{text}: {ink:?}");
                let (x0, ..) = drawn_box(text, rect).expect(text);
                assert!(x0 >= rect.x as i32, "{text}: drawn from {x0}");
            }
        }

        #[test]
        fn ink_box_within_checks_all_four_sides() {
            let rect = Rect::new(10, 10, 20, 20);
            let ok = InkBox {
                min_x: 10,
                min_y: 10,
                max_x: 30,
                max_y: 30,
            };
            assert!(ok.within(rect));
            for bad in [
                InkBox { min_x: 9, ..ok },
                InkBox { min_y: 9, ..ok },
                InkBox { max_x: 31, ..ok },
                InkBox { max_y: 31, ..ok },
            ] {
                assert!(!bad.within(rect), "{bad:?}");
            }
        }
    }
}
