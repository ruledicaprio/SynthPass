//! Fixed-grid MRZ name-line repair: pure geometry, no OCR types.
//!
//! # Why
//!
//! The MRZ name line (`P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<`) is a
//! fixed-pitch monospaced OCR-B grid — 44, 36 or 30 cells depending on
//! format — but `ocrs`'s recognizer never emits the isolated `<` glyph at
//! all: three probes run 2026-09-16 (`probe_chargrid.rs`, `probe_spaced.rs`,
//! and a horizontal-stretch sweep, none committed) established that neither
//! stretching the crop nor inserting blank gaps between cells before
//! re-recognizing gets the model to read a lone filler — it simply drops
//! it, so `KOVALENKO<<ANDRII` comes back as `KOVALENKOANDRII` and
//! `mrz::clean_name` has nothing to split the given names on. No check
//! digit covers the name line, so nothing downstream can prove which
//! reading is right; this module recovers the *position* of each dropped
//! filler from the fixed grid instead of asking the recognizer to read it.
//!
//! # What this module does
//!
//! Given the glyphs `ocrs` *did* read, each with a left-edge x position
//! (`ocrs`'s `TextChar.rect` — the CTC step's anchor is the left edge; the
//! right edge is only where the *next* step began, see [`fit_grid`]'s seed
//! comment), [`fit_grid`] estimates the line's cell pitch and origin,
//! [`align`] assigns each glyph to a cell by a monotone dynamic program, and
//! [`regrid`]/[`repair_name_line`] fill every cell that carries no glyph
//! with `<`. [`grid_origin_from_ink`] and [`cell_ink`] read the same grid
//! from the image's raw ink instead of glyph positions; every deficit is
//! corroborated against that ink before it's trusted (see
//! [`repair_name_line`]'s ink gate).
//!
//! # What this module does not do
//!
//! No `ocrs` type appears in any signature here — [`Glyph`] is a plain
//! `(char, left, right)` triple a caller builds from whatever recognizer it
//! used. There is no OCR invocation, no image preprocessing beyond reading
//! pixels a caller already produced, and no environment variable: this is
//! the pure algorithm module. Wiring it into [`crate::NativeOcr`] behind an
//! opt-in env var, and the `ocrs::TextChar` → [`Glyph`] conversion, is
//! PR-1.4.
//!
//! # Two probe findings this module encodes
//!
//! - A horizontal stretch of the crop and re-spacing the cells with blank
//!   gaps were both tried and rejected in the 2026-09-16 probes: neither
//!   changes what the recognizer reads, because the model has no representation
//!   of "a `<` glyph" to produce regardless of how the input is laid out —
//!   the fix has to happen after recognition, on the positions it already
//!   returned.
//! - The left-edge DP ([`align`]) took MRV-B line 1 from 0/10 to 5/10 exact
//!   in the same probes, and never corrupted an already-dense (no missing
//!   glyphs) line — the `n == cells` branch of [`fit_grid`] bypasses the DP
//!   entirely for that case, so a fully-read line is never at risk from it.

use image::GrayImage;

/// A single recognized character with its horizontal extent, in the
/// coordinate space of the line image the caller ran OCR on. `left` is what
/// every algorithm in this module keys on (see [`fit_grid`]'s seed comment
/// for why); `right` is carried through for a caller that wants it (e.g. to
/// crop around a glyph) but nothing here reads it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glyph {
    /// The recognized character.
    pub ch: char,
    /// Left edge of the glyph's bounding box, in pixels.
    pub left: f32,
    /// Right edge of the glyph's bounding box, in pixels.
    pub right: f32,
}

/// A fitted fixed-pitch character grid: cell `k`'s left edge is
/// `origin + pitch * k`, for `k` in `0..cells`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    /// Left edge of cell 0, in pixels.
    pub origin: f32,
    /// Distance between consecutive cells' left edges, in pixels.
    pub pitch: f32,
    /// Number of cells in the line (44/36/30 — see [`format_geometry`]).
    pub cells: usize,
}

/// TD1's name line (line index 2, see [`name_line_index`]) is pure name
/// text starting at position 0 — unlike the 2-line formats' line 0, it
/// carries no document-code/issuing-country prefix. TD1 is the only format
/// in [`format_geometry`]'s table with this width (44/44/36/36/30 for
/// Td3/MrvA/Td2/MrvB/Td1), so a name line of this width identifies TD1
/// without needing a `mrz::Format` argument — see [`repair_name_line`].
const TD1_NAME_LINE_WIDTH: usize = 30;

/// The MRZ line grid (line count, cells per line) for the formats this
/// module knows how to repair. `mrz::Format` is `#[non_exhaustive]`, so
/// unrecognized future variants fall through to `None` rather than failing
/// to compile.
///
/// This table is deliberately local rather than added to `crates/mrz`: see
/// mrz 0.7.2's planned `Format::line_length`, which will let this function
/// forward to it instead of duplicating the numbers (tracked, not done here
/// — editing `crates/mrz` is out of scope for this PR).
pub fn format_geometry(format: mrz::Format) -> Option<(usize, usize)> {
    match format {
        mrz::Format::Td3 => Some((2, 44)),
        mrz::Format::Td2 => Some((2, 36)),
        mrz::Format::Td1 => Some((3, TD1_NAME_LINE_WIDTH)),
        mrz::Format::MrvA => Some((2, 44)),
        mrz::Format::MrvB => Some((2, 36)),
        _ => None,
    }
}

/// Which MRZ line (0-based) carries the name field for `format`: line 0 for
/// every 2-line format, line 2 (the last line) for TD1's 3-line layout.
/// `#[non_exhaustive]`, same caveat as [`format_geometry`].
pub fn name_line_index(format: mrz::Format) -> Option<usize> {
    match format {
        mrz::Format::Td3 | mrz::Format::Td2 | mrz::Format::MrvA | mrz::Format::MrvB => Some(0),
        mrz::Format::Td1 => Some(2),
        _ => None,
    }
}

/// DP cost weight that prefers packing a glyph into the cell right after its
/// predecessor over skipping ahead, when otherwise tied. Ported unchanged
/// from the 2026-09-16 `probe_chargrid.rs`'s `align`, which found the DP's
/// output stable across its corpus without needing to tune this value.
const PACK: f32 = 0.05;

/// Assign each of `glyphs` (in left-to-right reading order) to a cell of
/// `grid`, monotonically increasing, minimizing squared distance from each
/// glyph's `left` to its assigned cell's left edge (in pitch units) plus
/// `PACK` per skipped cell. `None` when there are no glyphs to place, or
/// more glyphs than cells (both signal the caller passed mismatched input,
/// not a fittable line).
///
/// Ported unchanged (aside from taking a [`Grid`] instead of raw `(a, b)`)
/// from `probe_chargrid.rs`'s `align`, which measured it recovering MRV-B
/// line 1 from 0/10 to 5/10 exact and never corrupting an already-dense
/// line over the 2026-09-16 probe corpus.
// The DP fills `cost[i][c]` from `cost[i-1][p]` for `p` in a range that
// depends on `c`, not on position in an iterator -- `c` and `p` are load-
// bearing indices, not incidental ones `enumerate()` could stand in for.
#[allow(clippy::needless_range_loop)]
pub fn align(glyphs: &[Glyph], grid: &Grid) -> Option<Vec<usize>> {
    let n = glyphs.len();
    let cells = grid.cells;
    if n == 0 || n > cells {
        return None;
    }
    let inf = f32::INFINITY;
    // cost[i][c]: best cost of placing glyphs[0..=i] with glyphs[i] at cell c.
    let mut cost = vec![vec![inf; cells]; n];
    let mut back = vec![vec![0usize; cells]; n];
    let unit = |i: usize, c: usize| {
        let d = (glyphs[i].left - (grid.origin + grid.pitch * c as f32)) / grid.pitch;
        d * d
    };
    for c in 0..cells {
        cost[0][c] = unit(0, c);
    }
    for i in 1..n {
        for c in i..cells {
            let mut best = inf;
            let mut arg = 0;
            for p in (i - 1)..c {
                let v = cost[i - 1][p] + if c > p + 1 { PACK } else { 0.0 };
                if v < best {
                    best = v;
                    arg = p;
                }
            }
            cost[i][c] = best + unit(i, c);
            back[i][c] = arg;
        }
    }
    let mut c = (0..cells).min_by(|x, y| cost[n - 1][*x].total_cmp(&cost[n - 1][*y]))?;
    let mut out = vec![0; n];
    for i in (0..n).rev() {
        out[i] = c;
        c = back[i][c];
    }
    Some(out)
}

/// Number of align+refit rounds [`fit_grid`] runs when the glyph count
/// doesn't already match the cell count one-to-one. Ported from
/// `probe_chargrid.rs`'s `regrid`, which found 3 rounds sufficient for the
/// DP assignment and the least-squares refit to settle on its corpus.
const REFIT_ROUNDS: u32 = 3;

/// Least-squares slope/intercept for `x = a + b*i` — `(a, b)` minimizing
/// squared residuals over `(i, x)` pairs. `None` when the points carry no
/// variance in `i` (fewer than two distinct indices), which `fit_grid`
/// cannot itself produce for `n > 1` but a direct caller could.
fn least_squares(points: impl Iterator<Item = (f64, f64)>) -> Option<(f64, f64)> {
    let (mut n, mut si, mut sx, mut sii, mut six) = (0f64, 0f64, 0f64, 0f64, 0f64);
    for (i, x) in points {
        n += 1.0;
        si += i;
        sx += x;
        sii += i * i;
        six += i * x;
    }
    if n < 1.0 {
        return None;
    }
    let den = n * sii - si * si;
    const LS_DEGENERATE_EPS: f64 = 1e-9; // ported from probe_chargrid.rs's `den.abs() < 1e-9`
    if den.abs() < LS_DEGENERATE_EPS {
        return None;
    }
    let b = (n * six - si * sx) / den;
    let a = (sx - b * si) / n;
    Some((a, b))
}

/// Fit a [`Grid`] to `glyphs`, given the line's known left/right image
/// bounds and cell count.
///
/// When `glyphs.len() == cells`, every cell is already accounted for, so the
/// identity assignment (glyph `i` is cell `i`) is exact and a single
/// least-squares fit over `(i, glyph[i].left)` recovers `(origin, pitch)`
/// directly — no DP needed, and a dense line is therefore never at risk of
/// the alignment corrupting it (see the module doc's second probe finding).
///
/// Otherwise, the pitch is seeded from the line's own width
/// (`(line_right - line_left) / cells`) and the **origin from the line's
/// left edge, `line_left` itself** — a cell *boundary* — because
/// [`Glyph::left`] is a left edge too, and origin/positions must share that
/// convention for [`align`]'s DP to start from the right place. The
/// 2026-09-16 probe shipped `a = left + b / 2`, a cell *centre*, as the
/// seed; that is off by half a pitch from every glyph position, which
/// systematically misassigns the tail of the line after the first
/// ambiguous glyph (fixed here — see this module's test
/// `fit_grid_seed_uses_left_edge_not_cell_centre` for a worked example of
/// the corruption). [`align`] then runs, followed by up to
/// [`REFIT_ROUNDS`] rounds of least-squares refit over the DP's own
/// assignment, each round re-running [`align`] against the refit grid.
pub fn fit_grid(glyphs: &[Glyph], line_left: f32, line_right: f32, cells: usize) -> Option<Grid> {
    let n = glyphs.len();
    if n == 0 || n > cells {
        return None;
    }
    if n == cells {
        let (a, b) = least_squares(
            glyphs
                .iter()
                .enumerate()
                .map(|(i, g)| (i as f64, g.left as f64)),
        )?;
        return Some(Grid {
            origin: a as f32,
            pitch: b as f32,
            cells,
        });
    }
    let mut grid = Grid {
        origin: line_left,
        pitch: (line_right - line_left) / cells as f32,
        cells,
    };
    let mut idx = align(glyphs, &grid)?;
    for _ in 0..REFIT_ROUNDS {
        let Some((a, b)) = least_squares(
            idx.iter()
                .zip(glyphs)
                .map(|(&i, g)| (i as f64, g.left as f64)),
        ) else {
            break;
        };
        grid = Grid {
            origin: a as f32,
            pitch: b as f32,
            cells,
        };
        idx = align(glyphs, &grid)?;
    }
    Some(grid)
}

/// Place each of `glyphs` into its [`align`]-assigned cell of `grid`,
/// leaving every other cell `None`. `None` overall when [`align`] itself
/// can't place the glyphs (empty input, or more glyphs than cells).
///
/// [`align`]'s DP assigns strictly increasing cell indices to strictly
/// increasing glyph indices, so the returned cells are never double-booked:
/// `glyphs.len()` cells come back `Some`, the remaining `grid.cells -
/// glyphs.len()` come back `None`.
pub fn regrid(glyphs: &[Glyph], grid: &Grid) -> Option<Vec<Option<char>>> {
    let idx = align(glyphs, grid)?;
    let mut cells: Vec<Option<char>> = vec![None; grid.cells];
    for (glyph, &i) in glyphs.iter().zip(&idx) {
        cells[i] = Some(glyph.ch);
    }
    Some(cells)
}

/// Luma threshold below which a pixel counts as ink. `dark` in
/// [`grid_origin_from_ink`]/[`cell_ink`] is a plain grayscale band (a
/// caller's standard-weights `0.299R + 0.587G + 0.114B` luma conversion,
/// not yet reduced to booleans) — these two functions apply this threshold
/// themselves. Ported from `probe_spaced.rs`'s `to_dark` (`l < 110`).
const INK_LUMA_THRESHOLD: u8 = 110;

fn is_ink(luma: u8) -> bool {
    luma < INK_LUMA_THRESHOLD
}

/// Step size, in pixels, for [`grid_origin_from_ink`]'s search over
/// candidate origins. Ported from `probe_spaced.rs`'s `grid_origin`
/// (`x0 += 0.5`).
const ORIGIN_SEARCH_STEP: f32 = 0.5;

/// Recover a grid's horizontal phase (origin) from the image's own ink,
/// independent of any recognized glyph. Scans rows `top..bottom` of `dark`
/// for a per-column dark-pixel count (the "ink profile"), finds the first
/// column carrying any ink, then searches origins in
/// `[first_dark - pitch, first_dark]` in [`ORIGIN_SEARCH_STEP`] steps for
/// the one whose `cells + 1` cell boundaries cross the least ink — a
/// well-placed grid's boundaries fall between characters, on background.
/// `None` when the row range is empty, `pitch` is non-positive, or no
/// column in `top..bottom` carries any ink.
///
/// Ported from `probe_spaced.rs`'s `grid_origin`; that probe bounded the
/// column search to the line's own left/right box (`l.left - b ..=
/// l.right + b`) to avoid picking up ink from other lines sharing the
/// image. This function instead expects `dark` already cropped to the
/// line's horizontal extent — the `top`/`bottom` row bounds are all it
/// needs from the caller.
pub fn grid_origin_from_ink(
    dark: &GrayImage,
    top: u32,
    bottom: u32,
    pitch: f32,
    cells: usize,
) -> Option<f32> {
    let (w, h) = dark.dimensions();
    let top = top.min(h);
    let bottom = bottom.min(h);
    if top >= bottom || pitch <= 0.0 {
        return None;
    }
    let profile = |x: u32| -> u32 {
        (top..bottom)
            .filter(|&y| is_ink(dark.get_pixel(x, y)[0]))
            .count() as u32
    };
    let first_dark = (0..w).find(|&x| profile(x) > 0)? as f32;
    let mut best_cost = u32::MAX;
    let mut best_x0 = first_dark;
    let mut x0 = first_dark - pitch;
    while x0 <= first_dark {
        let cost: u32 = (0..=cells)
            .map(|k| {
                let x = (x0 + k as f32 * pitch).round();
                if x < 0.0 || x >= w as f32 {
                    0
                } else {
                    profile(x as u32)
                }
            })
            .sum();
        if cost < best_cost {
            best_cost = cost;
            best_x0 = x0;
        }
        x0 += ORIGIN_SEARCH_STEP;
    }
    Some(best_x0)
}

/// Dark-pixel fraction of each cell of `grid`, over rows `top..bottom` of
/// `dark`. A cell whose horizontal span falls (even partially) outside the
/// image, or an empty row range, reports `0.0` for that cell rather than
/// panicking on an out-of-bounds pixel access.
pub fn cell_ink(dark: &GrayImage, top: u32, bottom: u32, grid: &Grid) -> Vec<f32> {
    let (w, h) = dark.dimensions();
    let top = top.min(h);
    let bottom = bottom.min(h);
    (0..grid.cells)
        .map(|k| {
            let x_lo = (grid.origin + k as f32 * grid.pitch).max(0.0).round() as u32;
            let x_hi = ((grid.origin + (k as f32 + 1.0) * grid.pitch)
                .max(0.0)
                .round() as u32)
                .min(w);
            if x_lo >= x_hi || top >= bottom {
                return 0.0;
            }
            let mut dark_px: u32 = 0;
            let mut total: u32 = 0;
            for x in x_lo..x_hi {
                for y in top..bottom {
                    total += 1;
                    if is_ink(dark.get_pixel(x, y)[0]) {
                        dark_px += 1;
                    }
                }
            }
            dark_px as f32 / total as f32
        })
        .collect()
}

/// Default `ink_floor` for [`repair_name_line`]: the minimum dark-pixel
/// fraction (from [`cell_ink`]) an empty cell must show to count as "the
/// image shows something there, the glyph just didn't get read" rather
/// than "this cell really is a blank `<`, correctly regridded." Chosen low
/// deliberately — a printed OCR-B `<` filler is a thin glyph and still
/// leaves a real but modest ink footprint against the checkerboard/photo
/// background printed under many MRZ zones. Not a probe-measured value
/// (the ink-corroboration gate is new in this module); callers may
/// override it per image.
pub const DEFAULT_INK_FLOOR: f32 = 0.05;

/// Number of characters at the start of a 2-line format's name line that
/// are structural (document code + issuing country), not name text —
/// [`repair_name_line`]'s protected prefix. See [`TD1_NAME_LINE_WIDTH`] for
/// why TD1 has none.
const PREFIX_LEN: usize = 5;

/// Why [`repair_name_line`] declined to repair a name line, rather than
/// silently returning a guess a check digit can't verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejected {
    /// `glyphs.len() == width`: the line is already full, nothing to fill.
    NoDeficit,
    /// `glyphs.len() > width`: more glyphs than the line has cells, which
    /// cannot happen for a genuine read of this line and signals the
    /// caller passed mismatched geometry.
    TooManyGlyphs,
    /// [`regrid`] could not place the glyphs on `grid` at all, or the
    /// grid's cell count disagrees with `width`.
    GridFit,
    /// The number of empty cells [`regrid`] left whose measured ink meets
    /// `ink_floor` does not equal the glyph deficit — the fit doesn't match
    /// what the image shows, so it's rejected rather than trusted.
    InkMismatch {
        /// The glyph deficit (`width - glyphs.len()`), i.e. how many
        /// filled-but-inked cells the fit should have found.
        expected: usize,
        /// How many empty cells actually met `ink_floor`.
        found: usize,
    },
    /// The repaired line's protected prefix moved, or — for a 2-line
    /// format — stopped resolving to a real issuing country. See
    /// [`repair_name_line`]'s doc comment.
    PrefixChanged,
}

/// A successfully repaired MRZ name line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repair {
    /// The repaired line: exactly `width` characters, every previously
    /// ungridded cell filled with `<`.
    pub line: String,
    /// How many `<` fillers were placed — always equal to the glyph
    /// deficit (`width - glyphs.len()`).
    pub fillers_placed: usize,
}

/// Fill the gaps [`ocrs`] left in an MRZ name line by fitting `glyphs` onto
/// `grid` and treating every ungridded cell as a dropped `<`.
///
/// # The two constraints
///
/// No check digit covers the name line (see the module doc), so nothing
/// downstream can *prove* a repair correct — the two gates below are this
/// module's substitute, in the same spirit as
/// `crates/mrz/src/parser.rs`'s `shift_or_unshift_line1`, which arbitrates
/// TD3/MRV line 1's insertion-vs-drop ambiguity (also uncovered by any
/// check digit) using `mrz::country_name` as the only available positive
/// signal, accepting a repair only when it newly resolves to a real
/// country rather than merely being shape-plausible (see
/// `knowledge/MRZ_SEQUENCE_COMPLETENESS.md`'s chunk 6). This function
/// applies the same discipline: a repair is accepted only when it has
/// positive corroborating evidence, and rejected — not guessed at — when
/// it doesn't.
///
/// **Budget.** `deficit = width - glyphs.len()`. `deficit == 0` is
/// [`Rejected::NoDeficit`] (nothing to repair); `glyphs.len() > width` is
/// [`Rejected::TooManyGlyphs`]. [`regrid`]'s empty cells are exactly
/// `deficit` many by construction (see [`regrid`]'s doc comment) — so the
/// number of those empty cells whose `ink` (from [`cell_ink`], one value per
/// cell) meets `ink_floor` must equal `deficit` exactly, or the fit is
/// rejected as [`Rejected::InkMismatch`] rather than inventing fillers the
/// image doesn't support. The ink gate is mandatory: glyph positions alone
/// never justify a filler.
///
/// **Arbitration.** The repaired line's first [`PREFIX_LEN`] characters
/// must equal `raw_line`'s first [`PREFIX_LEN`] characters, or the result
/// is [`Rejected::PrefixChanged`] — the DP must not be allowed to shift a
/// glyph the recognizer actually read out of its position. For a 2-line
/// format (`width != `[`TD1_NAME_LINE_WIDTH`]) those five characters are
/// the document code and issuing country (`P<UTO...`), so this function
/// additionally requires `mrz::country_name` to resolve the repaired
/// issuing-country slice (characters 2..5) — a repair that leaves that
/// slice unresolvable has no positive evidence it's right, mirroring
/// `shift_or_unshift_line1`'s gate. TD1's name line (line index 2, see
/// [`name_line_index`]) has no such prefix at all — it is name text from
/// position 0 — so both checks are skipped for it; `width ==
/// `[`TD1_NAME_LINE_WIDTH`]` alone identifies that case, since TD1 is the
/// only format whose name line has this width.
pub fn repair_name_line(
    raw_line: &str,
    glyphs: &[Glyph],
    width: usize,
    ink: &[f32],
    ink_floor: f32,
    grid: &Grid,
) -> Result<Repair, Rejected> {
    let n = glyphs.len();
    if n == width {
        return Err(Rejected::NoDeficit);
    }
    if n > width {
        return Err(Rejected::TooManyGlyphs);
    }
    let deficit = width - n;

    let cells = regrid(glyphs, grid).ok_or(Rejected::GridFit)?;
    if cells.len() != width {
        return Err(Rejected::GridFit);
    }

    if ink.len() != width {
        return Err(Rejected::GridFit);
    }
    let inked_empty = cells
        .iter()
        .zip(ink)
        .filter(|(c, &v)| c.is_none() && v >= ink_floor)
        .count();
    if inked_empty != deficit {
        return Err(Rejected::InkMismatch {
            expected: deficit,
            found: inked_empty,
        });
    }

    let repaired: String = cells.iter().map(|c| c.unwrap_or('<')).collect();

    if width != TD1_NAME_LINE_WIDTH {
        let raw_chars: Vec<char> = raw_line.chars().collect();
        let repaired_chars: Vec<char> = repaired.chars().collect();
        let prefix_len = PREFIX_LEN.min(raw_chars.len()).min(repaired_chars.len());
        if raw_chars[..prefix_len] != repaired_chars[..prefix_len] {
            return Err(Rejected::PrefixChanged);
        }
        if prefix_len == PREFIX_LEN {
            let issuer: String = repaired_chars[2..5].iter().collect();
            if mrz::country_name(&issuer).is_none() {
                return Err(Rejected::PrefixChanged);
            }
        }
    }

    Ok(Repair {
        line: repaired,
        fillers_placed: deficit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(ch: char, left: f32, right: f32) -> Glyph {
        Glyph { ch, left, right }
    }

    /// Ink in every cell: a printed `<` filler carries ink, so a band whose
    /// only defect is dropped fillers reads as fully inked.
    fn all_ink(width: usize) -> Vec<f32> {
        vec![1.0; width]
    }

    // ---- format_geometry / name_line_index -------------------------------

    #[test]
    fn format_geometry_covers_all_five_formats() {
        assert_eq!(format_geometry(mrz::Format::Td3), Some((2, 44)));
        assert_eq!(format_geometry(mrz::Format::Td2), Some((2, 36)));
        assert_eq!(format_geometry(mrz::Format::Td1), Some((3, 30)));
        assert_eq!(format_geometry(mrz::Format::MrvA), Some((2, 44)));
        assert_eq!(format_geometry(mrz::Format::MrvB), Some((2, 36)));
    }

    #[test]
    fn name_line_index_is_zero_except_td1() {
        assert_eq!(name_line_index(mrz::Format::Td3), Some(0));
        assert_eq!(name_line_index(mrz::Format::Td2), Some(0));
        assert_eq!(name_line_index(mrz::Format::MrvA), Some(0));
        assert_eq!(name_line_index(mrz::Format::MrvB), Some(0));
        assert_eq!(name_line_index(mrz::Format::Td1), Some(2));
    }

    // ---- align -------------------------------------------------------------

    #[test]
    fn align_identity_when_positions_sit_on_cell_centres() {
        let grid = Grid {
            origin: 0.0,
            pitch: 10.0,
            cells: 6,
        };
        let glyphs: Vec<Glyph> = (0..6)
            .map(|i| glyph('A', i as f32 * 10.0, i as f32 * 10.0 + 8.0))
            .collect();
        assert_eq!(align(&glyphs, &grid), Some(vec![0, 1, 2, 3, 4, 5]));
    }

    #[test]
    fn align_restores_one_inner_gap() {
        // True line (12 cells): K O V A < < A N D R I I -- OCR drops the two
        // fillers at cells 4,5 (mirrors "KOVALENKO<<ANDRII" collapsing its
        // double separator).
        let grid = Grid {
            origin: 0.0,
            pitch: 10.0,
            cells: 12,
        };
        let true_cells = [0usize, 1, 2, 3, 6, 7, 8, 9, 10, 11];
        let glyphs: Vec<Glyph> = true_cells
            .iter()
            .map(|&c| glyph('X', c as f32 * 10.0, c as f32 * 10.0 + 8.0))
            .collect();
        assert_eq!(align(&glyphs, &grid), Some(true_cells.to_vec()));
    }

    #[test]
    fn align_tolerates_pitch_drift_and_jitter() {
        let a = 3.0f32;
        let b = 12.0f32;
        let cells = 20;
        // positions = a + b*i*1.01 + small jitter
        let glyphs: Vec<Glyph> = (0..cells)
            .map(|i| {
                let jitter = if i % 2 == 0 { 0.3 } else { -0.3 };
                let x = a + b * i as f32 * 1.01 + jitter;
                glyph('X', x, x + 8.0)
            })
            .collect();
        let grid = Grid {
            origin: a,
            pitch: b,
            cells,
        };
        assert_eq!(align(&glyphs, &grid), Some((0..cells).collect::<Vec<_>>()));
    }

    #[test]
    fn align_none_when_more_glyphs_than_cells() {
        let grid = Grid {
            origin: 0.0,
            pitch: 10.0,
            cells: 3,
        };
        let glyphs: Vec<Glyph> = (0..4)
            .map(|i| glyph('X', i as f32 * 10.0, i as f32 * 10.0 + 8.0))
            .collect();
        assert_eq!(align(&glyphs, &grid), None);
    }

    #[test]
    fn align_none_when_no_glyphs() {
        let grid = Grid {
            origin: 0.0,
            pitch: 10.0,
            cells: 3,
        };
        assert_eq!(align(&[], &grid), None);
    }

    // ---- fit_grid ------------------------------------------------------------

    #[test]
    fn fit_grid_ls_fit_recovers_known_grid_when_dense() {
        let true_origin = 5.0f32;
        let true_pitch = 11.0f32;
        let cells = 15;
        let glyphs: Vec<Glyph> = (0..cells)
            .map(|i| {
                let x = true_origin + true_pitch * i as f32;
                glyph('X', x, x + 8.0)
            })
            .collect();
        let line_left = true_origin;
        let line_right = true_origin + true_pitch * cells as f32;
        let grid = fit_grid(&glyphs, line_left, line_right, cells).unwrap();
        assert!(
            (grid.origin - true_origin).abs() < 1e-2,
            "origin={}",
            grid.origin
        );
        assert!(
            (grid.pitch - true_pitch).abs() < 1e-2,
            "pitch={}",
            grid.pitch
        );
    }

    #[test]
    fn fit_grid_refit_converges_from_left_edge_seed() {
        let true_origin = 100.0f32;
        let true_pitch = 12.0f32;
        let cells = 10;
        // Drop one inner glyph (index 5) so the seed/refit path runs.
        let glyphs: Vec<Glyph> = (0..cells)
            .filter(|&i| i != 5)
            .map(|i| {
                let x = true_origin + true_pitch * i as f32;
                glyph('X', x, x + 8.0)
            })
            .collect();
        let line_left = true_origin;
        let line_right = true_origin + true_pitch * cells as f32;
        let grid = fit_grid(&glyphs, line_left, line_right, cells).unwrap();
        assert!(
            (grid.origin - true_origin).abs() < 0.5,
            "origin={}",
            grid.origin
        );
        assert!(
            (grid.pitch - true_pitch).abs() < 0.5,
            "pitch={}",
            grid.pitch
        );
        let idx = align(&glyphs, &grid).unwrap();
        assert_eq!(idx, vec![0, 1, 2, 3, 4, 6, 7, 8, 9]);
    }

    #[test]
    fn fit_grid_seed_uses_left_edge_not_cell_centre() {
        // Same shape as `fit_grid_refit_converges_from_left_edge_seed`, but
        // this test exercises `align` directly at round 0 -- before any
        // refit -- to isolate the seed choice itself. Ground truth: 5 cells,
        // glyphs present at true cells 0,1,3,4 (cell 2 missing).
        let true_origin = 0.0f32;
        let pitch = 10.0f32;
        let cells = 5;
        let true_cells = [0usize, 1, 3, 4];
        let glyphs: Vec<Glyph> = true_cells
            .iter()
            .map(|&c| glyph('X', c as f32 * pitch, c as f32 * pitch + 8.0))
            .collect();

        let correct_seed = Grid {
            origin: true_origin, // left edge -- this module's seed
            pitch,
            cells,
        };
        let buggy_seed = Grid {
            origin: true_origin + pitch / 2.0, // cell centre -- the pre-fix seed
            pitch,
            cells,
        };

        let good = align(&glyphs, &correct_seed).unwrap();
        let buggy = align(&glyphs, &buggy_seed).unwrap();

        assert_eq!(good, true_cells.to_vec());
        assert_ne!(
            buggy,
            true_cells.to_vec(),
            "a half-pitch-off seed should misassign the tail of the line"
        );
    }

    #[test]
    fn fit_grid_none_when_too_many_glyphs() {
        let glyphs: Vec<Glyph> = (0..5)
            .map(|i| glyph('X', i as f32 * 10.0, i as f32 * 10.0 + 8.0))
            .collect();
        assert_eq!(fit_grid(&glyphs, 0.0, 30.0, 3), None);
    }

    // ---- regrid / repair_name_line -----------------------------------------

    /// Build a dense grid + glyphs for `line` (no drops), used as scaffolding
    /// by the tests below that then remove specific glyphs.
    fn dense_glyphs(line: &str, pitch: f32) -> (Vec<Glyph>, Grid) {
        let cells = line.chars().count();
        let glyphs: Vec<Glyph> = line
            .chars()
            .enumerate()
            .map(|(i, ch)| glyph(ch, i as f32 * pitch, i as f32 * pitch + pitch * 0.8))
            .collect();
        let grid = Grid {
            origin: 0.0,
            pitch,
            cells,
        };
        (glyphs, grid)
    }

    #[test]
    fn repair_name_line_no_deficit_when_line_full() {
        let line = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        assert_eq!(line.chars().count(), 44);
        let (glyphs, grid) = dense_glyphs(line, 10.0);
        let err = repair_name_line(line, &glyphs, 44, &all_ink(44), DEFAULT_INK_FLOOR, &grid)
            .unwrap_err();
        assert_eq!(err, Rejected::NoDeficit);
    }

    #[test]
    fn repair_name_line_too_many_glyphs() {
        let line = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        let (glyphs, grid) = dense_glyphs(line, 10.0);
        let err = repair_name_line(line, &glyphs, 40, &all_ink(40), DEFAULT_INK_FLOOR, &grid)
            .unwrap_err();
        assert_eq!(err, Rejected::TooManyGlyphs);
    }

    #[test]
    fn repair_name_line_spends_budget_exactly() {
        let full = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        let width = full.chars().count();
        let (dense, _) = dense_glyphs(full, 10.0);
        // Drop the two fillers between surname and given names -- "...SSON"
        // ends at index 12, so the double separator is index 13,14.
        assert_eq!(&full[12..15], "N<<");
        let glyphs: Vec<Glyph> = dense
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 13 && *i != 14)
            .map(|(_, g)| *g)
            .collect();
        let grid = fit_grid(&glyphs, 0.0, width as f32 * 10.0, width).unwrap();
        let repair = repair_name_line(
            full,
            &glyphs,
            width,
            &all_ink(width),
            DEFAULT_INK_FLOOR,
            &grid,
        )
        .unwrap();
        assert_eq!(repair.fillers_placed, 2);
        assert_eq!(repair.line, full);
    }

    #[test]
    fn repair_name_line_ink_mismatch_when_missing_cell_is_blank() {
        let full = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        let width = full.chars().count();
        let (dense, _) = dense_glyphs(full, 10.0);
        let glyphs: Vec<Glyph> = dense
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 13 && *i != 14)
            .map(|(_, g)| *g)
            .collect();
        let grid = fit_grid(&glyphs, 0.0, width as f32 * 10.0, width).unwrap();
        // Ink says only 1 of the 2 missing cells actually has ink -- the fit
        // should be rejected rather than trusted.
        let mut ink = vec![1.0f32; width];
        ink[13] = 1.0;
        ink[14] = 0.0; // below floor: "this cell is genuinely blank"
        let err =
            repair_name_line(full, &glyphs, width, &ink, DEFAULT_INK_FLOOR, &grid).unwrap_err();
        assert_eq!(
            err,
            Rejected::InkMismatch {
                expected: 2,
                found: 1
            }
        );
    }

    #[test]
    fn repair_name_line_rejects_ink_of_the_wrong_width() {
        let full = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        let width = full.chars().count();
        let (dense, _) = dense_glyphs(full, 10.0);
        let glyphs: Vec<Glyph> = dense
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 13)
            .map(|(_, g)| *g)
            .collect();
        let grid = fit_grid(&glyphs, 0.0, width as f32 * 10.0, width).unwrap();
        let err = repair_name_line(
            full,
            &glyphs,
            width,
            &all_ink(width - 1),
            DEFAULT_INK_FLOOR,
            &grid,
        )
        .unwrap_err();
        assert_eq!(err, Rejected::GridFit);
    }

    #[test]
    fn repair_name_line_ink_match_accepts() {
        let full = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        let width = full.chars().count();
        let (dense, _) = dense_glyphs(full, 10.0);
        let glyphs: Vec<Glyph> = dense
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 13 && *i != 14)
            .map(|(_, g)| *g)
            .collect();
        let grid = fit_grid(&glyphs, 0.0, width as f32 * 10.0, width).unwrap();
        let ink = vec![1.0f32; width]; // every cell, including both missing ones, shows ink
        let repair =
            repair_name_line(full, &glyphs, width, &ink, DEFAULT_INK_FLOOR, &grid).unwrap();
        assert_eq!(repair.line, full);
    }

    #[test]
    fn repair_name_line_prefix_changed_when_dp_moves_a_prefix_glyph() {
        let full = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        let width = full.chars().count();
        let (dense, _) = dense_glyphs(full, 10.0);
        // Drop a glyph from inside the protected prefix (index 2, 'U' of UTO).
        let glyphs: Vec<Glyph> = dense
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 2)
            .map(|(_, g)| *g)
            .collect();
        let grid = fit_grid(&glyphs, 0.0, width as f32 * 10.0, width).unwrap();
        let err = repair_name_line(
            full,
            &glyphs,
            width,
            &all_ink(width),
            DEFAULT_INK_FLOOR,
            &grid,
        )
        .unwrap_err();
        assert_eq!(err, Rejected::PrefixChanged);
    }

    #[test]
    fn repair_name_line_td1_skips_prefix_check() {
        // TD1 name line (30 cells), no document-code/issuer prefix -- a
        // deficit landing in the first 5 cells must not be rejected.
        let full = "ERIKSSON<<ANNA<MARIA<<<<<<<<<<";
        assert_eq!(full.chars().count(), 30);
        let (dense, _) = dense_glyphs(full, 10.0);
        // Drop the filler at index 8 (right after the short surname).
        let glyphs: Vec<Glyph> = dense
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 8)
            .map(|(_, g)| *g)
            .collect();
        let grid = fit_grid(&glyphs, 0.0, 30.0 * 10.0, 30).unwrap();
        let repair =
            repair_name_line(full, &glyphs, 30, &all_ink(30), DEFAULT_INK_FLOOR, &grid).unwrap();
        assert_eq!(repair.line, full);
    }

    #[test]
    fn repair_name_line_restores_compound_surname_single_fillers() {
        // "DE<LA<CRUZ<<JUAN" -- multiple *single* `<` separators, each of
        // which must come back individually, not lumped into one gap. Uses
        // TD1's 30-cell width so the doc-code/issuer prefix check (which
        // this string, being pure name text, would never satisfy) doesn't
        // apply -- see `TD1_NAME_LINE_WIDTH`.
        let full = format!("{:<<30}", "DE<LA<CRUZ<<JUAN");
        assert_eq!(full.chars().count(), 30);
        let (dense, _) = dense_glyphs(&full, 10.0);
        // Drop every single-filler separator: indices of '<' that are not
        // part of the trailing pad and not the double `<<` before JUAN.
        let drop: Vec<usize> = full
            .char_indices()
            .filter(|&(i, c)| c == '<' && (i == 2 || i == 5))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(drop, vec![2, 5]);
        let glyphs: Vec<Glyph> = dense
            .iter()
            .enumerate()
            .filter(|(i, _)| !drop.contains(i))
            .map(|(_, g)| *g)
            .collect();
        let grid = fit_grid(&glyphs, 0.0, 30.0 * 10.0, 30).unwrap();
        let repair =
            repair_name_line(&full, &glyphs, 30, &all_ink(30), DEFAULT_INK_FLOOR, &grid).unwrap();
        assert_eq!(repair.line, full);
        assert_eq!(repair.fillers_placed, 2);
    }

    // ---- ink from a drawn band ----------------------------------------------

    /// Build a `GrayImage` `cells` wide (`pitch` px per cell, rounded) and
    /// `height` px tall, with an ink block covering every cell whose index is
    /// not in `blank`. Each cell leaves a 1px unlit column on its left edge
    /// (`x % pitch == 0`) so the cell boundaries have somewhere genuinely
    /// blank to fall on -- a real printed character never fills its cell
    /// edge-to-edge either.
    fn draw_band(cells: usize, pitch: u32, height: u32, blank: &[usize]) -> GrayImage {
        let width = cells as u32 * pitch;
        GrayImage::from_fn(width, height, |x, y| {
            let cell = (x / pitch) as usize;
            let in_margin = (x % pitch) == 0 || y < 2 || y >= height - 2;
            let ink = !blank.contains(&cell) && !in_margin;
            image::Luma([if ink { 20u8 } else { 250u8 }])
        })
    }

    #[test]
    fn grid_origin_from_ink_recovers_offset_origin() {
        let cells = 44;
        let pitch = 24u32;
        let height = 40u32;
        let band = draw_band(cells, pitch, height, &[]);
        let true_origin = 0.0f32; // draw_band's cell 0 starts at x=0
        let recovered = grid_origin_from_ink(&band, 0, height, pitch as f32, cells).unwrap();
        // The single-pixel blank margin at each boundary leaves a small tie
        // band (a boundary search hitting anywhere on that pixel scores
        // equally); the search picks the leftmost tie, so the recovered
        // origin can land up to ~1px left of the true one.
        assert!(
            (recovered - true_origin).abs() <= 1.5,
            "recovered={recovered}"
        );
    }

    #[test]
    fn cell_ink_reports_ink_in_every_drawn_cell() {
        let cells = 44;
        let pitch = 24u32;
        let height = 40u32;
        let band = draw_band(cells, pitch, height, &[]);
        let grid = Grid {
            origin: 0.0,
            pitch: pitch as f32,
            cells,
        };
        let ink = cell_ink(&band, 0, height, &grid);
        assert_eq!(ink.len(), cells);
        for (k, &v) in ink.iter().enumerate() {
            assert!(v >= DEFAULT_INK_FLOOR, "cell {k} ink={v}");
        }
    }

    #[test]
    fn drawn_band_repairs_two_designated_inner_blank_cells() {
        // A printed `<` filler still has ink (see `DEFAULT_INK_FLOOR`'s doc
        // comment), so the drawn band carries ink in *every* cell -- the two
        // designated cells below are missing only from the *glyph* list
        // (what `ocrs` read), simulating a recognizer that dropped two
        // fillers whose ink is nonetheless visible in the image.
        let cells = 44;
        let pitch = 24u32;
        let height = 40u32;
        let band = draw_band(cells, pitch, height, &[]);
        let grid = Grid {
            origin: 0.0,
            pitch: pitch as f32,
            cells,
        };
        let ink = cell_ink(&band, 0, height, &grid);
        for (k, &v) in ink.iter().enumerate() {
            assert!(v >= DEFAULT_INK_FLOOR, "cell {k} ink={v}");
        }
        // Both designated cells are `<` fillers in `full` (index 14 is the
        // second of the surname/given-names double separator, index 30 is
        // inside the trailing pad) -- the realistic drop this module
        // repairs; see `random_drops_restore_the_original_line_when_prefix_survives`
        // for why dropping a *letter* is a different, DP-ambiguous case.
        let dropped_cells = [14usize, 30];
        let full = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        assert_eq!(full.chars().count(), cells);
        let glyphs: Vec<Glyph> = full
            .chars()
            .enumerate()
            .filter(|(i, _)| !dropped_cells.contains(i))
            .map(|(i, ch)| {
                glyph(
                    ch,
                    i as f32 * pitch as f32,
                    i as f32 * pitch as f32 + pitch as f32 * 0.8,
                )
            })
            .collect();
        let repair =
            repair_name_line(full, &glyphs, cells, &ink, DEFAULT_INK_FLOOR, &grid).unwrap();
        assert_eq!(repair.line, full);
        assert_eq!(repair.fillers_placed, 2);
    }

    // ---- property-style: random drop/restore -------------------------------

    /// Minimal seeded LCG (no `proptest` dependency in this crate -- see
    /// PR-1.3's task description). Not cryptographic; deterministic across
    /// runs given the same seed, which is all a repeatable test needs.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            // Numerical Recipes constants.
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0
        }
        fn range(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    #[test]
    fn random_drops_restore_the_original_line_when_prefix_survives() {
        let full = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        let width = full.chars().count();
        let (dense, _) = dense_glyphs(full, 10.0);
        // Candidate drop positions: `<` cells outside the protected prefix.
        // `ocrs` drops fillers, not letters (see the module doc) -- dropping
        // a *letter* is not the failure mode this module repairs, and the
        // DP's flat per-skip `PACK` cost (ported as-is from the probe, see
        // `align`'s doc comment) is genuinely ambiguous when several
        // consecutive *letters* vanish at once. Restricting drops to `<`
        // positions keeps every letter as an anchor, which is what the real
        // failure mode leaves intact.
        let candidates: Vec<usize> = full
            .char_indices()
            .filter(|&(i, c)| c == '<' && i >= PREFIX_LEN)
            .map(|(i, _)| i)
            .collect();
        let mut rng = Lcg(0x5EED_C0FF_EE12_3456);
        for trial in 0..200 {
            // Drop a random subset (1..=6 of the candidate `<` positions).
            let drop_count = (1 + rng.range(6)).min(candidates.len());
            let mut drop: Vec<usize> = Vec::new();
            while drop.len() < drop_count {
                let idx = candidates[rng.range(candidates.len())];
                if !drop.contains(&idx) {
                    drop.push(idx);
                }
            }
            let glyphs: Vec<Glyph> = dense
                .iter()
                .enumerate()
                .filter(|(i, _)| !drop.contains(i))
                .map(|(_, g)| *g)
                .collect();
            let grid = fit_grid(&glyphs, 0.0, width as f32 * 10.0, width)
                .unwrap_or_else(|| panic!("trial {trial}: fit_grid failed for drop set {drop:?}"));
            let repair = repair_name_line(
                full,
                &glyphs,
                width,
                &all_ink(width),
                DEFAULT_INK_FLOOR,
                &grid,
            )
            .unwrap_or_else(|e| {
                panic!("trial {trial}: repair_name_line failed for drop set {drop:?}: {e:?}")
            });
            assert_eq!(repair.line, full, "trial {trial}: drop set {drop:?}");
        }
    }
}
