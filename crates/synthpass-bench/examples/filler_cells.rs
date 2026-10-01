//! Filler or letter, per MRZ name cell (#575 item C) — a **bench-only**
//! prototype. Nothing here is wired into the product, and no default changes.
//!
//! Each name cell of a Tier-1 hit is classed `filler`, `letter`, `uncertain`
//! or `occluded` from its own pixels on a grid, and the class is compared with
//! the reviewed fixture's zone (`samples/ocr_fixtures/<stem>.json`), never with
//! OCR. A cell classed `occluded` is left exactly as OCR read it and is never
//! reconstructed (ADR-0026).
//!
//! # The decision
//!
//! A printed `<` has no vertical stroke and less ink than most letters. It is
//! *not* shorter: the filler spans about one cap height in the standard and
//! in print (`knowledge/benchmarks/ocrb-filler-geometry-2026-09-23.md`), so
//! inked height separates nothing. What does separate it is its shape: two
//! steep arms meeting at the middle of its left side. Per cell, on a band
//! binarised with Otsu's threshold:
//!
//! - `v`: the longest vertical run of ink in any one column, over the line's
//!   90th-percentile `v` (a stemmed letter's height). A stem makes a letter.
//! - The glyph's ink box is cut into thirds each way, and each zone's ink
//!   density is taken over the box's mean: `corner` is the denser of the
//!   top-left and bottom-left zones, `apex` the middle-left zone, `ends` the
//!   sparser of the top-right and bottom-right zones. A chevron has empty left
//!   corners, a dense apex and inked arm ends; every letter fills at least one
//!   left corner (`S`, `Z` and `V`, which have no stem, included).
//! - `r`, the cell's dark fraction over the line's 90th-percentile cell, and
//!   the absolute dark fraction, only guard against blank and covered cells.
//! - `h`, the longest horizontal run over the cell width, is measured and
//!   reported by `tune` but not used: blur merges neighbouring fillers.
//!
//! [`Thresholds::SYNTHETIC_TUNED`] turns them into a class. Its numbers come
//! from a search on synthetic renders (`tune --search`, seeds 100-199 at every
//! sweep step), never from the real fixtures the gate is measured on, and the
//! sweep (seeds 0-99) is held out from it; see its doc for the derivation.
//!
//! # Grids, kept apart
//!
//! - **(a) ink grid, hand-verified.** Fitted from the line's ink alone: the
//!   format's cell count, the first and last inked columns, and the pitch and
//!   origin whose cell boundaries cross the least ink ([`ink_grid`]). Each
//!   fitted line is written as an overlay with the fixture's filler cells
//!   marked (`real-analyze --overlays`) and checked by eye.
//! - **(b) production's chargrid fit**, as `SYNTHPASS_OCR_CHARGRID=control`
//!   captures it on the accepted pass (`ChargridLineCapture::fit`). The
//!   working image is rebuilt from the variant dump and checked against the
//!   capture's band hash before any cell is read.
//!
//! # Applying the classes
//!
//! Only the `off` read of a Tier-1 hit is touched, and only name cells. A
//! `filler` cell read as a letter becomes `<`; nothing else changes (see
//! [`apply`]). The line is left alone when a `letter` cell was read as `<`,
//! the signature of a read shifted against the printed cells, and the change
//! is dropped unless the re-parse is checksum-valid with every non-name field
//! unchanged (see [`reparse`]).
//!
//! # Subcommands
//!
//! ```text
//! filler_cells real --arm off|control|on [--out DIR]
//!     OCR the 45 name-scorable documents of the committed real-specimen
//!     baseline with the shipped pipeline. SYNTHPASS_OCR_CHARGRID must equal
//!     --arm. `control` also needs SYNTHPASS_OCR_DUMP_VARIANTS=<DIR>/dumps so
//!     the accepted pass's pixels can be read back. Writes <DIR>/real-<arm>.jsonl.
//! filler_cells real-analyze [--out DIR] [--overlays]
//!     Classify every name cell of every Tier-1 hit on grids (a) and (b), apply
//!     the classes to the `off` read, and print per-cell and per-document tables.
//! filler_cells tune [--seeds N] [--seed-start S] [--clean-only] [--search]
//!     Feature distributions on synthetic TD3 renders (default seeds 100-199,
//!     every sweep step), the committed thresholds' confusion there, and with
//!     --search the threshold search that chose them.
//! filler_cells sweep [--seeds N] [--seed-start S]
//!     The classifier on synthetic TD3 renders under one degradation at a time
//!     (resolution, contrast, rotation, blur; #433's axes), on the exact grid.
//!     Default seeds 0-99, disjoint from the tuning seeds.
//! filler_cells render [--seed N] [--axis A] [--step S]
//!     One degraded synthetic line with its exact grid drawn in.
//! ```
//!
//! Everything a run writes goes under `artifacts/filler-cells/` by default,
//! which is gitignored. **The real subcommands write document text** (the read
//! zone, for re-parsing) into that directory; keep it local. What they print
//! to stdout is counts only, named by document stem.

use image::{GrayImage, Rgb, RgbImage};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use synthpass_ocr::chargrid::{self, Grid};
use synthpass_ocr::{NativeOcr, PassOutcome, PassRecord};

const DEFAULT_OUT: &str = "artifacts/filler-cells";

// ---------------------------------------------------------------------
// The classifier (pure)
// ---------------------------------------------------------------------

/// A cell's class. `Occluded` is never reconstructed (ADR-0026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CellClass {
    Filler,
    Letter,
    Uncertain,
    Occluded,
}

impl CellClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::Filler => "filler",
            Self::Letter => "letter",
            Self::Uncertain => "uncertain",
            Self::Occluded => "occluded",
        }
    }

    const ALL: [CellClass; 4] = [
        CellClass::Filler,
        CellClass::Letter,
        CellClass::Uncertain,
        CellClass::Occluded,
    ];
}

/// An integer pixel window `[x0, x1) x [y0, y1)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PxRect {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

impl PxRect {
    /// The window covering `(left, top, right, bottom)`, clamped to a `w x h`
    /// image. Edges round to the nearest pixel boundary, as `chargrid::cell_ink`
    /// rounds its cell edges.
    fn clamped(left: f64, top: f64, right: f64, bottom: f64, w: u32, h: u32) -> PxRect {
        let clamp = |v: f64, max: u32| v.round().clamp(0.0, f64::from(max)) as u32;
        PxRect {
            x0: clamp(left, w),
            y0: clamp(top, h),
            x1: clamp(right, w),
            y1: clamp(bottom, h),
        }
    }

    fn is_empty(&self) -> bool {
        self.x0 >= self.x1 || self.y0 >= self.y1
    }
}

/// One cell's measurements. `ink` is absolute; `r` and `v` are relative to
/// the line and `h` to the cell's width (see the module doc).
#[derive(Debug, Clone, Copy, PartialEq)]
struct CellFeatures {
    ink: f64,
    r: f64,
    v: f64,
    h: f64,
    /// Whether the cell held a glyph ink box at all; the zone features are
    /// 0 when it did not.
    has_box: bool,
    /// The denser of the ink box's top-left and bottom-left zones.
    corner: f64,
    /// The ink box's middle-left zone: where a chevron's apex sits.
    apex: f64,
    /// The sparser of the ink box's top-right and bottom-right zones: where a
    /// chevron's arms end.
    ends: f64,
}

/// Otsu's threshold over `pixels`: the grey level `t` maximising the
/// between-class variance of `<= t` against `> t`. `None` for no pixels or a
/// single grey level.
fn otsu(pixels: impl Iterator<Item = u8>) -> Option<u8> {
    let mut hist = [0u64; 256];
    let mut n = 0u64;
    for p in pixels {
        hist[p as usize] += 1;
        n += 1;
    }
    if n == 0 {
        return None;
    }
    let total: f64 = hist
        .iter()
        .enumerate()
        .map(|(i, &c)| i as f64 * c as f64)
        .sum();
    let (mut w0, mut sum0) = (0f64, 0f64);
    let mut best: Option<(f64, u8)> = None;
    // Threshold `t` splits `<= t` from `> t`, so 255 is never a split.
    for (t, &count) in hist.iter().enumerate().take(255) {
        w0 += count as f64;
        sum0 += t as f64 * count as f64;
        let w1 = n as f64 - w0;
        if w0 == 0.0 || w1 == 0.0 {
            continue;
        }
        let m0 = sum0 / w0;
        let m1 = (total - sum0) / w1;
        let between = w0 * w1 * (m0 - m1) * (m0 - m1);
        // Strictly greater: ties keep the lower threshold.
        if best.is_none_or(|(b, _)| between > b) {
            best = Some((between, t as u8));
        }
    }
    best.map(|(_, t)| t)
}

/// The `p`-th percentile (0-100) of `values` by nearest rank on the sorted
/// copy. `None` when empty.
fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted.get(rank.min(sorted.len() - 1)).copied()
}

/// Raw per-cell measurements on a binary mask.
struct RawCell {
    /// Dark fraction of the cell.
    ink: f64,
    /// Longest vertical run of ink in any one column, px.
    vrun: u32,
    /// Longest horizontal run of ink in any one row, px.
    hrun: u32,
    /// Ink density of each third-by-third zone of the glyph's ink box
    /// (row-major: top-left, top-centre, ..., bottom-right), over the box's
    /// mean density. `None` when the cell holds no glyph box.
    zones: Option<[f64; 9]>,
}

fn raw_cell(mask: &dyn Fn(u32, u32) -> bool, cell: PxRect) -> RawCell {
    let empty = RawCell {
        ink: 0.0,
        vrun: 0,
        hrun: 0,
        zones: None,
    };
    if cell.is_empty() {
        return empty;
    }
    let (cw, ch) = (cell.x1 - cell.x0, cell.y1 - cell.y0);
    let mut dark = 0u64;
    let mut hrun = 0u32;
    let mut row_ink = Vec::with_capacity(ch as usize);
    for y in cell.y0..cell.y1 {
        let (mut run, mut count) = (0u32, 0u32);
        for x in cell.x0..cell.x1 {
            if mask(x, y) {
                count += 1;
                run += 1;
                hrun = hrun.max(run);
            } else {
                run = 0;
            }
        }
        dark += u64::from(count);
        row_ink.push(count);
    }
    let mut vrun = 0u32;
    let mut col_ink = Vec::with_capacity(cw as usize);
    for x in cell.x0..cell.x1 {
        let (mut run, mut count) = (0u32, 0u32);
        for y in cell.y0..cell.y1 {
            if mask(x, y) {
                count += 1;
                run += 1;
                vrun = vrun.max(run);
            } else {
                run = 0;
            }
        }
        col_ink.push(count);
    }
    let ink = dark as f64 / (f64::from(cw) * f64::from(ch));
    // The glyph's ink box: rows holding at least 8% of the cell's width in
    // ink, columns holding at least 8% of its height, so a stray pixel or a
    // neighbour's sliver does not stretch it.
    let min_row = ((f64::from(cw) * 0.08).round() as u32).max(1);
    let min_col = ((f64::from(ch) * 0.08).round() as u32).max(1);
    let rows = (
        row_ink.iter().position(|&c| c >= min_row),
        row_ink.iter().rposition(|&c| c >= min_row),
    );
    let cols = (
        col_ink.iter().position(|&c| c >= min_col),
        col_ink.iter().rposition(|&c| c >= min_col),
    );
    let zones = match (rows, cols) {
        ((Some(r0), Some(r1)), (Some(c0), Some(c1))) if r1 >= r0 + 2 && c1 >= c0 + 2 => {
            let (bh, bw) = ((r1 - r0 + 1) as f64, (c1 - c0 + 1) as f64);
            let mut ink_z = [0f64; 9];
            let mut area_z = [0f64; 9];
            for yi in r0..=r1 {
                let zy = ((3.0 * (yi - r0) as f64 / bh) as usize).min(2);
                for xi in c0..=c1 {
                    let zx = ((3.0 * (xi - c0) as f64 / bw) as usize).min(2);
                    area_z[zy * 3 + zx] += 1.0;
                    if mask(cell.x0 + xi as u32, cell.y0 + yi as u32) {
                        ink_z[zy * 3 + zx] += 1.0;
                    }
                }
            }
            let mean = ink_z.iter().sum::<f64>() / (bh * bw);
            (mean > 0.0).then(|| {
                let mut d = [0f64; 9];
                for z in 0..9 {
                    d[z] = if area_z[z] > 0.0 {
                        ink_z[z] / area_z[z] / mean
                    } else {
                        0.0
                    };
                }
                d
            })
        }
        _ => None,
    };
    RawCell {
        ink,
        vrun,
        hrun,
        zones,
    }
}

/// Features for every cell of one line. `band` is the region the Otsu
/// threshold is computed over (the line's band across the grid's span);
/// `cells` are the cell windows, in cell order.
fn line_features(gray: &GrayImage, band: PxRect, cells: &[PxRect]) -> Option<Vec<CellFeatures>> {
    if band.is_empty() {
        return None;
    }
    let threshold = otsu(
        (band.y0..band.y1).flat_map(|y| (band.x0..band.x1).map(move |x| gray.get_pixel(x, y)[0])),
    )?;
    let mask = |x: u32, y: u32| gray.get_pixel(x, y)[0] <= threshold;
    let raw: Vec<RawCell> = cells.iter().map(|&c| raw_cell(&mask, c)).collect();
    let inks: Vec<f64> = raw.iter().map(|r| r.ink).collect();
    let vruns: Vec<f64> = raw.iter().map(|r| f64::from(r.vrun)).collect();
    let ink_ref = percentile(&inks, 90.0)?;
    let height_ref = percentile(&vruns, 90.0)?;
    if ink_ref <= 0.0 || height_ref <= 0.0 {
        return None;
    }
    Some(
        raw.iter()
            .zip(cells)
            .map(|(c, cell)| {
                let z = c.zones.unwrap_or([0.0; 9]);
                CellFeatures {
                    ink: c.ink,
                    r: c.ink / ink_ref,
                    v: f64::from(c.vrun) / height_ref,
                    h: f64::from(c.hrun) / f64::from((cell.x1 - cell.x0).max(1)),
                    has_box: c.zones.is_some(),
                    corner: z[0].max(z[6]),
                    apex: z[3],
                    ends: z[2].min(z[8]),
                }
            })
            .collect(),
    )
}

/// The text rows of a band: Otsu over the band, then the contiguous run of
/// rows around the inkiest row whose ink count is at least 10% of that row's,
/// widened by one row each side. It drops a neighbouring MRZ line, or VIZ
/// text, that a padded line box takes in. `None` when the band has no ink.
fn text_rows(gray: &GrayImage, band: PxRect) -> Option<(u32, u32)> {
    if band.is_empty() {
        return None;
    }
    let threshold = otsu(
        (band.y0..band.y1).flat_map(|y| (band.x0..band.x1).map(move |x| gray.get_pixel(x, y)[0])),
    )?;
    let profile: Vec<u32> = (band.y0..band.y1)
        .map(|y| {
            (band.x0..band.x1)
                .filter(|&x| gray.get_pixel(x, y)[0] <= threshold)
                .count() as u32
        })
        .collect();
    let (peak, &max) = profile
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(&a.0)))?;
    if max == 0 {
        return None;
    }
    let floor = (max / 10).max(1);
    let mut lo = peak;
    while lo > 0 && profile[lo - 1] >= floor {
        lo -= 1;
    }
    let mut hi = peak;
    while hi + 1 < profile.len() && profile[hi + 1] >= floor {
        hi += 1;
    }
    let top = (band.y0 + lo as u32).saturating_sub(1).max(band.y0);
    let bottom = (band.y0 + hi as u32 + 2).min(band.y1);
    Some((top, bottom))
}

/// The decision's numbers. A cell is:
///
/// - `occluded` when its dark fraction is at least `occluded_ink` (absolute)
///   or at least `occluded_r` times the line's reference cell;
/// - `uncertain` when its dark fraction is below `blank_ink` (nothing
///   printed where a name line always prints something);
/// - `filler` when it is chevron-shaped and stemless: it has a glyph box,
///   `corner <= filler_corner`, `apex >= filler_apex`, `ends >= filler_ends`
///   and `v <= filler_v`, and no letter bound is reached;
/// - `letter` when `corner >= letter_corner` or `v >= letter_v` (a filled
///   left corner, or a stem) and the cell is not filler-like;
/// - `uncertain` otherwise (both, or neither).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Thresholds {
    blank_ink: f64,
    occluded_ink: f64,
    occluded_r: f64,
    filler_corner: f64,
    filler_apex: f64,
    filler_ends: f64,
    filler_v: f64,
    letter_corner: f64,
    letter_v: f64,
}

/// The letter bounds sit this far beyond the filler bounds, so a cell
/// between them is `uncertain`. Chosen, not fitted.
const LETTER_CORNER_MARGIN: f64 = 0.25;
const LETTER_V_MARGIN: f64 = 0.15;

impl Thresholds {
    /// Found by `tune --search` on the synthetic tuning set: TD3 seeds
    /// 100-199, every step of the four sweep axes (3,700 lines; 92,500 filler
    /// and 51,800 letter cells). Never on the real fixtures, and never on the
    /// sweep's seeds (0-99).
    ///
    /// The search walks `filler_corner` 0.30-0.90, `filler_apex` 0.8-1.6,
    /// `filler_ends` 0.0-1.2 (steps of 0.05, 0.1, 0.1) and `filler_v`
    /// 0.45-0.90 (0.05), with the letter bounds at the margins above, and keeps
    /// the combination that classes the most tuning fillers `filler` while
    /// classing **no** tuning letter `filler`. It walks from the strictest
    /// combination outward and keeps only strict improvements, so ties go to
    /// the stricter bounds. Result: `filler_corner` 0.75, `filler_apex` 1.0,
    /// `filler_ends` 0.3, `filler_v` 0.80 (so `letter_corner` 1.00 and
    /// `letter_v` 0.95): 86,821 of 92,500 tuning fillers classed `filler`
    /// (93.9%), 1,652 classed `letter`, and 0 of 51,800 tuning letters
    /// classed `filler`. Every chosen value is inside its searched range.
    ///
    /// `blank_ink` 0.01, `occluded_ink` 0.60 and `occluded_r` 2.5 are not
    /// tuned: on the tuning set letters reach `ink` 0.566 and `r` 1.510.
    /// They are stated guards.
    const SYNTHETIC_TUNED: Thresholds = Thresholds::with_filler_bounds(0.75, 1.0, 0.3, 0.80);

    const fn with_filler_bounds(corner: f64, apex: f64, ends: f64, v: f64) -> Thresholds {
        Thresholds {
            blank_ink: 0.01,
            occluded_ink: 0.60,
            occluded_r: 2.5,
            filler_corner: corner,
            filler_apex: apex,
            filler_ends: ends,
            filler_v: v,
            letter_corner: corner + LETTER_CORNER_MARGIN,
            letter_v: v + LETTER_V_MARGIN,
        }
    }

    fn to_json(self) -> Value {
        let r3 = |v: f64| (v * 1000.0).round() / 1000.0;
        json!({
            "blank_ink": r3(self.blank_ink),
            "occluded_ink": r3(self.occluded_ink),
            "occluded_r": r3(self.occluded_r),
            "filler_corner": r3(self.filler_corner),
            "filler_apex": r3(self.filler_apex),
            "filler_ends": r3(self.filler_ends),
            "filler_v": r3(self.filler_v),
            "letter_corner": r3(self.letter_corner),
            "letter_v": r3(self.letter_v),
        })
    }
}

fn classify(f: &CellFeatures, t: &Thresholds) -> CellClass {
    if f.ink >= t.occluded_ink || f.r >= t.occluded_r {
        return CellClass::Occluded;
    }
    if f.ink < t.blank_ink {
        return CellClass::Uncertain;
    }
    let filler_like = f.has_box
        && f.corner <= t.filler_corner
        && f.apex >= t.filler_apex
        && f.ends >= t.filler_ends
        && f.v <= t.filler_v;
    let letter_like = f.corner >= t.letter_corner || f.v >= t.letter_v;
    match (filler_like, letter_like) {
        (true, false) => CellClass::Filler,
        (false, true) => CellClass::Letter,
        _ => CellClass::Uncertain,
    }
}

/// Cell windows for a fitted horizontal grid over rows `top..bottom`.
fn grid_cells(grid: &Grid, top: u32, bottom: u32, w: u32, h: u32) -> Vec<PxRect> {
    (0..grid.cells)
        .map(|k| {
            let x0 = f64::from(grid.origin) + k as f64 * f64::from(grid.pitch);
            let x1 = x0 + f64::from(grid.pitch);
            PxRect::clamped(x0, f64::from(top), x1, f64::from(bottom), w, h)
        })
        .collect()
}

/// Grid (a): the pitch and origin whose `cells + 1` boundaries cross the least
/// ink, constrained so the line's first and last inked columns fall in cells 0
/// and `cells - 1`. Each boundary is charged the ink of every column within
/// 20% of a pitch of it, so a boundary centred in the gap between two glyphs
/// costs less than one grazing a glyph's edge; a single column would tie every
/// grid whose boundaries all land somewhere in the gaps. Searched over pitch
/// `[(R-L)/cells, (R-L)/(cells-1)]` in 0.02 px steps and origin
/// `[L - pitch/2, L]` in 0.25 px steps; exact ties keep the first candidate
/// (smallest pitch, then smallest origin). `None` when the band has no ink or
/// the constraint admits nothing.
fn ink_grid(gray: &GrayImage, band: PxRect, cells: usize) -> Option<Grid> {
    if band.is_empty() || cells < 2 {
        return None;
    }
    let threshold = otsu(
        (band.y0..band.y1).flat_map(|y| (band.x0..band.x1).map(move |x| gray.get_pixel(x, y)[0])),
    )?;
    let rows = band.y1 - band.y0;
    let profile: Vec<u32> = (band.x0..band.x1)
        .map(|x| {
            (band.y0..band.y1)
                .filter(|&y| gray.get_pixel(x, y)[0] <= threshold)
                .count() as u32
        })
        .collect();
    // A column is inked when at least 6% of the band's rows are dark there.
    let min_col = ((f64::from(rows) * 0.06).round() as u32).max(1);
    let first = profile.iter().position(|&c| c >= min_col)?;
    let last = profile.iter().rposition(|&c| c >= min_col)?;
    let left = f64::from(band.x0) + first as f64;
    let right = f64::from(band.x0) + last as f64 + 1.0;
    let span = right - left;
    let n = cells as f64;
    let at = |x: f64| -> u64 {
        let i = x.round() - f64::from(band.x0);
        if i < 0.0 || i >= profile.len() as f64 {
            0
        } else {
            u64::from(profile[i as usize])
        }
    };
    let mut best: Option<(u64, f64, f64)> = None;
    let mut pitch = span / n;
    while pitch <= span / (n - 1.0) + 1e-9 {
        let mut origin = left - pitch / 2.0;
        while origin <= left + 1e-9 {
            let end = origin + n * pitch;
            if end + 1e-9 >= right && origin + (n - 1.0) * pitch < right {
                let half = (0.2 * pitch).round().max(1.0) as i64;
                let cost: u64 = (0..=cells)
                    .map(|k| {
                        let x = origin + k as f64 * pitch;
                        (-half..=half).map(|d| at(x + d as f64)).sum::<u64>()
                    })
                    .sum();
                if best.is_none_or(|(c, _, _)| cost < c) {
                    best = Some((cost, origin, pitch));
                }
            }
            origin += 0.25;
        }
        pitch += 0.02;
    }
    best.map(|(_, origin, pitch)| Grid {
        origin: origin as f32,
        pitch: pitch as f32,
        cells,
    })
}

// ---------------------------------------------------------------------
// Argument handling and small helpers
// ---------------------------------------------------------------------

fn flag_value<'a>(args: &'a [String], name: &str) -> Result<Option<&'a str>, String> {
    match args.iter().position(|a| a == name) {
        None => Ok(None),
        Some(i) => args
            .get(i + 1)
            .map(|v| Some(v.as_str()))
            .ok_or_else(|| format!("{name} needs a value")),
    }
}

fn parse_u64(args: &[String], name: &str, default: u64) -> Result<u64, String> {
    match flag_value(args, name)? {
        None => Ok(default),
        Some(v) => v.parse().map_err(|_| format!("{name}: not a number: {v}")),
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn out_dir(args: &[String]) -> Result<PathBuf, String> {
    let dir = match flag_value(args, "--out")? {
        Some(v) => PathBuf::from(v),
        None => repo_root().join(DEFAULT_OUT),
    };
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(dir)
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn git_head() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_root())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn read_jsonl(path: &Path) -> Result<Vec<Value>, String> {
    let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    std::io::BufReader::new(file)
        .lines()
        .map(|line| {
            let line = line.map_err(|e| e.to_string())?;
            serde_json::from_str(&line).map_err(|e| format!("{}: {e}", path.display()))
        })
        .collect()
}

fn write_jsonl(path: &Path, rows: &[Value]) -> Result<(), String> {
    let mut file = fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    for row in rows {
        writeln!(file, "{row}").map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// `(rows, cells)` for a zone's line count and length, or `None` for a shape
/// no format has.
fn format_of_zone(lines: &[&str]) -> Option<mrz::Format> {
    let len = lines.first()?.chars().count();
    match (lines.len(), len) {
        (2, 44) => Some(mrz::Format::Td3),
        (2, 36) => Some(mrz::Format::Td2),
        (3, 30) => Some(mrz::Format::Td1),
        _ => None,
    }
}

/// The first name cell of the name line: 5 on a two-line format (document
/// code and issuing state come first), 0 on TD1's third line.
fn first_name_cell(format: mrz::Format) -> usize {
    match format {
        mrz::Format::Td1 => 0,
        _ => 5,
    }
}

// ---------------------------------------------------------------------
// `real`: OCR the name-scorable documents under one chargrid arm
// ---------------------------------------------------------------------

/// The 45 name-scorable documents of the committed baseline: the scored
/// population (a Tier-1 hit, `checksum_failed` or `no_mrz_found`) whose
/// outcome row carries a `names_exact` verdict.
fn name_scorable(root: &Path) -> Result<Vec<Value>, String> {
    let rows = read_jsonl(&root.join("knowledge/benchmarks/real-specimen-outcomes.jsonl"))?;
    Ok(rows
        .into_iter()
        .filter(|r| {
            !r["names_exact"].is_null()
                && matches!(
                    r["outcome"].as_str(),
                    Some("hit" | "checksum_failed" | "no_mrz_found")
                )
        })
        .collect())
}

fn field_values(e: &synthpass_core::Extraction) -> BTreeMap<&'static str, Option<String>> {
    BTreeMap::from([
        ("document_type", e.document_type.clone()),
        ("issuing_country", e.issuing_country.clone()),
        ("document_number", e.document_number.clone()),
        ("surname", e.surname.clone()),
        ("given_names", e.given_names.clone()),
        ("nationality", e.nationality.clone()),
        ("date_of_birth", e.date_of_birth.clone()),
        ("sex", e.sex.clone()),
        ("date_of_expiry", e.date_of_expiry.clone()),
        ("personal_number", e.personal_number.clone()),
    ])
}

fn capture_json(record: &PassRecord) -> Value {
    let Some(cg) = &record.chargrid else {
        return Value::Null;
    };
    let line = cg.line.as_ref().map(|l| {
        json!({
            "name_line_index": l.name_line_index,
            "width": l.width,
            "downscaled": l.downscaled,
            "image_width": l.image_width,
            "image_height": l.image_height,
            "line_box": [l.line_box.left, l.line_box.top, l.line_box.right, l.line_box.bottom],
            "band_sha256": l.band_sha256,
            "glyphs": l.glyphs.len(),
            "fit": l.fit.as_ref().map(|f| json!({
                "origin": f.grid.origin,
                "pitch": f.grid.pitch,
                "cells": f.grid.cells,
            })),
        })
    });
    json!({ "mode": cg.mode, "verdict": cg.verdict, "line": line })
}

fn run_real(args: &[String]) -> Result<(), String> {
    let arm = flag_value(args, "--arm")?.ok_or("real needs --arm off|control|on")?;
    if !matches!(arm, "off" | "control" | "on") {
        return Err(format!("--arm must be off, control or on, not {arm}"));
    }
    let env_arm = std::env::var("SYNTHPASS_OCR_CHARGRID").unwrap_or_else(|_| "off".into());
    if env_arm.trim().to_ascii_lowercase() != arm {
        return Err(format!(
            "SYNTHPASS_OCR_CHARGRID is {env_arm:?} but --arm is {arm}; set them equal"
        ));
    }
    let dump = std::env::var("SYNTHPASS_OCR_DUMP_VARIANTS").ok();
    if arm == "control" && dump.as_deref().is_none_or(|d| d.trim().is_empty()) {
        return Err("--arm control needs SYNTHPASS_OCR_DUMP_VARIANTS set".into());
    }
    let root = repo_root();
    let out = out_dir(args)?;
    let samples = root.join("samples");
    let models = synthpass_bench::resolve_model_dir(&root, |k| std::env::var_os(k));
    let det = models.join("text-detection.rten");
    let rec = models.join("text-recognition.rten");
    let ocr = NativeOcr::load(&det, &rec)?;
    let det_sha = sha256_hex(&fs::read(&det).map_err(|e| e.to_string())?);
    let rec_sha = sha256_hex(&fs::read(&rec).map_err(|e| e.to_string())?);
    let input_dir = out.join(format!("input-{arm}"));
    fs::create_dir_all(&input_dir).map_err(|e| e.to_string())?;

    let docs = name_scorable(&root)?;
    let mut rows = vec![json!({
        "header": true,
        "arm": arm,
        "git_head": git_head(),
        "documents": docs.len(),
        "models": { "detection_sha256": det_sha, "recognition_sha256": rec_sha },
        "env": {
            "SYNTHPASS_OCR_CHARGRID": env_arm,
            "SYNTHPASS_OCR_MAX_SECONDS": std::env::var("SYNTHPASS_OCR_MAX_SECONDS").ok(),
            "RTEN_NUM_THREADS": std::env::var("RTEN_NUM_THREADS").ok(),
        },
    })];
    for (i, doc) in docs.iter().enumerate() {
        let stem = doc["name"].as_str().ok_or("outcome row without a name")?;
        let asset = doc["asset_id"]
            .as_str()
            .ok_or("outcome row without asset_id")?;
        let truth = synthpass_bench::load_ground_truth(&samples, stem)
            .ok_or_else(|| format!("{stem}: no fixture"))?;
        // The pipeline's own decode, then a PNG the OCR reads, exactly as
        // provider-bench hands a specimen to `recognize_detailed_traced`. The
        // PNG is named by stem so the variant dump is too.
        let image = synthpass_ocr::decode_image(&samples.join(asset))?;
        let tmp = input_dir.join(format!("{stem}.png"));
        image.save(&tmp).map_err(|e| e.to_string())?;
        let started = std::time::Instant::now();
        let traced = ocr.recognize_detailed_traced(&tmp);
        let elapsed = started.elapsed().as_secs_f64();
        let _ = fs::remove_file(&tmp);
        let (page, records) = match traced {
            Ok(v) => v,
            Err(e) => {
                rows.push(json!({ "name": stem, "asset_id": asset, "ocr_error": e }));
                continue;
            }
        };
        let read = synthpass_die::read_tier1(&page.text);
        let (valid, zone, fields, format) = match &read.parsed {
            Ok(data) => {
                let fields = field_values(&synthpass_die::mrz_reader::extraction_from_mrz(data));
                (
                    data.valid(),
                    Some(data.mrz_lines.clone()),
                    Some(fields),
                    Some(format!("{:?}", data.format)),
                )
            }
            Err(_) => (false, None, None, None),
        };
        let truth_fields = field_values(&truth);
        let hit = valid
            && fields
                .as_ref()
                .is_some_and(|f| f["document_number"] == truth_fields["document_number"]);
        let accepted = records.iter().find(|r| r.outcome == PassOutcome::Accepted);
        let readings: Vec<Value> = accepted
            .map(|r| {
                r.readings
                    .iter()
                    .map(|l| json!({ "text": l.text, "bbox": [l.bbox.x, l.bbox.y, l.bbox.w, l.bbox.h] }))
                    .collect()
            })
            .unwrap_or_default();
        eprintln!(
            "[{arm} {}/{}] {stem}: {} ({elapsed:.1}s)",
            i + 1,
            docs.len(),
            if hit { "hit" } else { "miss" }
        );
        rows.push(json!({
            "name": stem,
            "asset_id": asset,
            "baseline_outcome": doc["outcome"],
            "baseline_names_exact": doc["names_exact"],
            "hit": hit,
            "checksums_valid": valid,
            "format": format,
            "zone": zone,
            "fields": fields,
            "retry_variant_id": page.retry_variant_id,
            "chargrid_verdict": page.chargrid,
            "accepted_pass": accepted.map(|r| json!({
                "id": r.id,
                "transform": r.transform.to_string(),
                "image_width": r.image_width,
                "image_height": r.image_height,
                "readings": readings,
                "chargrid": capture_json(r),
            })),
            "elapsed_s": elapsed,
        }));
    }
    let path = out.join(format!("real-{arm}.jsonl"));
    write_jsonl(&path, &rows)?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

// ---------------------------------------------------------------------
// `real-analyze`: classify, apply, score
// ---------------------------------------------------------------------

/// SHA-256 of the band `chargrid` measured on, as
/// `ChargridLineCapture::band_sha256` documents it: width and row count as
/// `u32` little-endian, then those rows of the grayscale image.
fn band_sha256(gray: &GrayImage, top: u32, bottom: u32) -> String {
    let (w, h) = gray.dimensions();
    let top = top.min(h);
    let bottom = bottom.min(h);
    let rows = bottom.saturating_sub(top);
    let start = top as usize * w as usize;
    let end = start + rows as usize * w as usize;
    let mut bytes = Vec::with_capacity(8 + (end - start));
    bytes.extend_from_slice(&w.to_le_bytes());
    bytes.extend_from_slice(&rows.to_le_bytes());
    bytes.extend_from_slice(&gray.as_raw()[start..end]);
    sha256_hex(&bytes)
}

/// The accepted pass's pixels, read back from the variant dump, and resized
/// the way `fit_chargrid_name_line` resizes when its capture says so.
fn working_image(dump_dir: &Path, stem: &str, pass: &Value) -> Result<RgbImage, String> {
    let id = pass["id"].as_str().ok_or("accepted pass without id")?;
    let label = match id.strip_prefix("pass-") {
        Some(n) => format!("variant{n}"),
        None => id.to_string(),
    };
    let path = dump_dir.join(format!("{stem}__{label}.png"));
    let image = image::open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .into_rgb8();
    let line = &pass["chargrid"]["line"];
    if line["downscaled"].as_bool() == Some(true) {
        let w = line["image_width"].as_u64().ok_or("no image_width")? as u32;
        let h = line["image_height"].as_u64().ok_or("no image_height")? as u32;
        return Ok(image::imageops::resize(
            &image,
            w,
            h,
            image::imageops::FilterType::Lanczos3,
        ));
    }
    Ok(image)
}

/// Where the name line sits in the working image: `(left, top, right,
/// bottom)`. From the chargrid capture when there is one; otherwise from the
/// accepted pass's reading closest (by CER) to the read name line.
fn name_line_box(pass: &Value, name_line: &str) -> Option<([f64; 4], &'static str)> {
    if let Some(b) = pass["chargrid"]["line"]["line_box"].as_array() {
        let v: Vec<f64> = b.iter().filter_map(Value::as_f64).collect();
        if v.len() == 4 {
            return Some(([v[0], v[1], v[2], v[3]], "chargrid_capture"));
        }
    }
    let readings = pass["readings"].as_array()?;
    let best = readings
        .iter()
        .filter_map(|r| {
            let text = r["text"].as_str()?;
            let b: Vec<f64> = r["bbox"]
                .as_array()?
                .iter()
                .filter_map(Value::as_f64)
                .collect();
            (b.len() == 4).then(|| (synthpass_bench::cer(name_line, text), b))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))?;
    let b = best.1;
    Some(([b[0], b[1], b[0] + b[2], b[1] + b[3]], "pass_reading"))
}

/// One document's classification on one grid.
struct GridRun {
    grid: Grid,
    classes: Vec<CellClass>,
}

fn classify_on_grid(
    gray: &GrayImage,
    grid: Grid,
    top: u32,
    bottom: u32,
    t: &Thresholds,
) -> Option<GridRun> {
    let (w, h) = gray.dimensions();
    let cells = grid_cells(&grid, top, bottom, w, h);
    let first = cells.first()?;
    let last = cells.last()?;
    let band = PxRect {
        x0: first.x0,
        y0: top.min(h),
        x1: last.x1,
        y1: bottom.min(h),
    };
    let features = line_features(gray, band, &cells)?;
    Some(GridRun {
        grid,
        classes: features.iter().map(|f| classify(f, t)).collect(),
    })
}

/// What the prototype does to one read name line: every name cell classed
/// `filler` that OCR read as a letter becomes `<`. Nothing else changes: a
/// `letter` cell is never rewritten (the class does not say which letter),
/// and `occluded` and `uncertain` cells keep OCR's read. The whole line is
/// left alone when any name cell classed `letter` was read as `<`, because
/// that disagreement is what a misaligned grid looks like.
fn apply(read: &str, classes: &[CellClass], first: usize) -> (String, &'static str) {
    apply_with(read, classes, first, true)
}

/// [`apply`], with the misalignment guard switchable. `guard: false` is a
/// counterfactual for the note only, never the prototype's rule.
fn apply_with(
    read: &str,
    classes: &[CellClass],
    first: usize,
    guard: bool,
) -> (String, &'static str) {
    let chars: Vec<char> = read.chars().collect();
    if chars.len() != classes.len() {
        return (read.to_string(), "abstain:width");
    }
    let disagrees =
        (first..chars.len()).any(|i| classes[i] == CellClass::Letter && chars[i] == '<');
    if guard && disagrees {
        return (read.to_string(), "abstain:letter_read_as_filler");
    }
    let out: String = chars
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            if i >= first && classes[i] == CellClass::Filler && c != '<' {
                '<'
            } else {
                c
            }
        })
        .collect();
    let verdict = if out == read { "unchanged" } else { "changed" };
    (out, verdict)
}

/// Re-parse the zone with a new name line. The change is kept only when the
/// result is still checksum-valid and every field but the two names is
/// identical to the original read; otherwise the original fields stand.
fn reparse(
    zone: &str,
    name_idx: usize,
    new_line: &str,
    original: &BTreeMap<String, Option<String>>,
) -> Option<BTreeMap<String, Option<String>>> {
    let mut lines: Vec<String> = zone.lines().map(str::to_string).collect();
    *lines.get_mut(name_idx)? = new_line.to_string();
    let read = synthpass_die::read_tier1(&lines.join("\n"));
    let data = read.parsed.ok().filter(|d| d.valid())?;
    let fields: BTreeMap<String, Option<String>> =
        field_values(&synthpass_die::mrz_reader::extraction_from_mrz(&data))
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
    let same_rest = fields
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), "surname" | "given_names"))
        .all(|(k, v)| original.get(k) == Some(v));
    same_rest.then_some(fields)
}

fn fields_of(v: &Value) -> BTreeMap<String, Option<String>> {
    v.as_object()
        .map(|o| {
            o.iter()
                .map(|(k, v)| (k.clone(), v.as_str().map(str::to_string)))
                .collect()
        })
        .unwrap_or_default()
}

fn names_exact(
    fields: &BTreeMap<String, Option<String>>,
    truth: &synthpass_core::Extraction,
) -> bool {
    let get = |k: &str| fields.get(k).cloned().flatten().unwrap_or_default();
    let (ts, tg) = (
        truth.surname.clone().unwrap_or_default(),
        truth.given_names.clone().unwrap_or_default(),
    );
    synthpass_bench::classify_names(&ts, &tg, &get("surname"), &get("given_names")).is_none()
}

/// Per-field correctness against the fixture, over the fields the fixture
/// states. `true` = the read equals the fixture.
fn field_correct(
    fields: &BTreeMap<String, Option<String>>,
    truth: &synthpass_core::Extraction,
) -> BTreeMap<String, bool> {
    field_values(truth)
        .into_iter()
        .filter_map(|(k, expected)| {
            let expected = expected?;
            let got = fields.get(k).cloned().flatten();
            Some((k.to_string(), got.as_deref() == Some(expected.as_str())))
        })
        .collect()
}

#[derive(Default)]
struct Confusion {
    /// `(truth is filler, class) -> cells`.
    cells: BTreeMap<(bool, CellClass), u64>,
}

impl Confusion {
    fn add(&mut self, truth_filler: bool, class: CellClass) {
        *self.cells.entry((truth_filler, class)).or_default() += 1;
    }

    fn get(&self, truth_filler: bool, class: CellClass) -> u64 {
        self.cells.get(&(truth_filler, class)).copied().unwrap_or(0)
    }

    fn row(&self, truth_filler: bool) -> String {
        let total: u64 = CellClass::ALL
            .iter()
            .map(|&c| self.get(truth_filler, c))
            .sum();
        let parts: Vec<String> = CellClass::ALL
            .iter()
            .map(|&c| format!("{}", self.get(truth_filler, c)))
            .collect();
        format!("{} | {total}", parts.join(" | "))
    }

    fn to_json(&self) -> Value {
        let side = |f: bool| {
            let m: serde_json::Map<String, Value> = CellClass::ALL
                .iter()
                .map(|&c| (c.as_str().to_string(), json!(self.get(f, c))))
                .collect();
            Value::Object(m)
        };
        json!({ "truth_filler": side(true), "truth_letter": side(false) })
    }
}

/// Draws a crop of the band with grid (a) (green, top half) and grid (b)
/// (red, bottom half) boundaries, and a blue bar over each cell the fixture
/// says is a filler. Scaled so the crop is `target_w` wide.
fn overlay(
    image: &RgbImage,
    band: PxRect,
    a: Option<&Grid>,
    b: Option<&Grid>,
    truth: &[bool],
    target_w: u32,
) -> RgbImage {
    let crop = image::imageops::crop_imm(
        image,
        band.x0,
        band.y0,
        band.x1 - band.x0,
        band.y1 - band.y0,
    )
    .to_image();
    let scale = f64::from(target_w) / f64::from(crop.width().max(1));
    let th = ((f64::from(crop.height()) * scale).round() as u32).max(1);
    let big = image::imageops::resize(&crop, target_w, th, image::imageops::FilterType::Triangle);
    let bar = 6u32;
    let mut out = RgbImage::from_pixel(target_w, th + bar, Rgb([255, 255, 255]));
    image::imageops::overlay(&mut out, &big, 0, i64::from(bar));
    let to_x = |x: f64| ((x - f64::from(band.x0)) * scale).round();
    let vline = |img: &mut RgbImage, x: f64, y0: u32, y1: u32, colour: Rgb<u8>| {
        if x >= 0.0 && x < f64::from(target_w) {
            for y in y0..y1.min(img.height()) {
                img.put_pixel(x as u32, y, colour);
            }
        }
    };
    let h = out.height();
    if let Some(g) = a {
        for k in 0..=g.cells {
            let x = to_x(f64::from(g.origin) + k as f64 * f64::from(g.pitch));
            vline(&mut out, x, bar, bar + (h - bar) / 2, Rgb([0, 170, 0]));
        }
        for (k, &filler) in truth.iter().enumerate() {
            if filler {
                let x0 = to_x(f64::from(g.origin) + k as f64 * f64::from(g.pitch)).max(0.0) as u32;
                let x1 = to_x(f64::from(g.origin) + (k as f64 + 1.0) * f64::from(g.pitch))
                    .clamp(0.0, f64::from(target_w)) as u32;
                for x in x0..x1 {
                    for y in 0..bar - 1 {
                        out.put_pixel(x, y, Rgb([0, 90, 255]));
                    }
                }
            }
        }
    }
    if let Some(g) = b {
        for k in 0..=g.cells {
            let x = to_x(f64::from(g.origin) + k as f64 * f64::from(g.pitch));
            vline(&mut out, x, bar + (h - bar) / 2, h, Rgb([220, 0, 0]));
        }
    }
    out
}

fn run_real_analyze(args: &[String]) -> Result<(), String> {
    let root = repo_root();
    let out = out_dir(args)?;
    let samples = root.join("samples");
    let dump_dir = out.join("dumps");
    let want_overlays = args.iter().any(|a| a == "--overlays");
    let t = Thresholds::SYNTHETIC_TUNED;
    let load = |arm: &str| -> Result<BTreeMap<String, Value>, String> {
        Ok(read_jsonl(&out.join(format!("real-{arm}.jsonl")))?
            .into_iter()
            .filter(|r| r["header"].is_null())
            .filter_map(|r| Some((r["name"].as_str()?.to_string(), r)))
            .collect())
    };
    let off = load("off")?;
    let control = load("control")?;
    let on = load("on")?;

    let mut conf_a = Confusion::default();
    let mut conf_b = Confusion::default();
    // OCR's own read of the same cells, for comparison: was the cell read as
    // `<` when it is a filler, and as a letter when it is a letter.
    let mut ocr_read = Confusion::default();
    let mut overlays: Vec<(String, RgbImage)> = Vec::new();
    let mut doc_rows: Vec<Value> = Vec::new();
    let mut table: Vec<String> = Vec::new();
    let mut strict = BTreeMap::from([
        ("off", 0u32),
        ("control", 0),
        ("on", 0),
        ("proto_a", 0),
        ("proto_b", 0),
    ]);
    let mut regressions = BTreeMap::from([("proto_a", 0u32), ("proto_b", 0)]);
    let mut field_regressions = BTreeMap::from([("proto_a", 0u32), ("proto_b", 0)]);
    let mut disagreements: Vec<String> = Vec::new();
    // Counterfactuals, reported beside the arms and never gated: the strict
    // count of the truth-driven ceiling, and of each grid without the
    // misalignment guard (strict, correct->wrong).
    let mut ceiling_total = 0u32;
    let ceiling_strict = &mut ceiling_total;
    let mut no_guard: BTreeMap<&str, (u32, u32)> = BTreeMap::new();

    for (stem, off_row) in &off {
        let truth = synthpass_bench::load_ground_truth(&samples, stem)
            .ok_or_else(|| format!("{stem}: no fixture"))?;
        let truth_zone = truth.mrz_line.clone().unwrap_or_default();
        let hit_of = |r: Option<&Value>| r.is_some_and(|r| r["hit"].as_bool() == Some(true));
        let strict_of = |r: Option<&Value>| {
            r.is_some_and(|r| hit_of(Some(r)) && names_exact(&fields_of(&r["fields"]), &truth))
        };
        let s_off = strict_of(Some(off_row));
        let s_control = strict_of(control.get(stem));
        let s_on = strict_of(on.get(stem));
        if s_off != s_control || hit_of(Some(off_row)) != hit_of(control.get(stem)) {
            disagreements.push(format!("{stem}: off and control differ"));
        }
        let baseline_strict = off_row["baseline_outcome"] == "hit"
            && off_row["baseline_names_exact"].as_bool() == Some(true);
        if baseline_strict != s_off {
            disagreements.push(format!(
                "{stem}: local off strict={s_off}, committed baseline strict={baseline_strict}"
            ));
        }
        *strict.get_mut("off").ok_or("")? += u32::from(s_off);
        *strict.get_mut("control").ok_or("")? += u32::from(s_control);
        *strict.get_mut("on").ok_or("")? += u32::from(s_on);

        let mut row = json!({
            "name": stem,
            "hit": hit_of(Some(off_row)),
            "strict_off": s_off,
            "strict_control": s_control,
            "strict_on": s_on,
        });
        let mut proto_strict = BTreeMap::from([("proto_a", s_off), ("proto_b", s_off)]);
        // A document the counterfactual never touches keeps its `off` verdict.
        let mut no_guard_strict = proto_strict.clone();

        // Only a Tier-1 hit read under `off` is ever touched, and its pixels
        // come from the `control` run of the same document.
        let prepared = (|| -> Result<Option<Value>, String> {
            if !hit_of(Some(off_row)) {
                return Ok(None);
            }
            let Some(ctl) = control.get(stem) else {
                return Ok(Some(json!({ "skip": "no control record" })));
            };
            if ctl["zone"] != off_row["zone"] {
                return Ok(Some(json!({ "skip": "control read a different zone" })));
            }
            let zone = off_row["zone"].as_str().ok_or("hit without zone")?;
            let zone_lines: Vec<&str> = zone.lines().collect();
            let truth_lines: Vec<&str> = truth_zone.lines().collect();
            let Some(format) = format_of_zone(&zone_lines) else {
                return Ok(Some(json!({ "skip": "zone shape" })));
            };
            if format_of_zone(&truth_lines) != Some(format) {
                return Ok(Some(json!({ "skip": "fixture zone shape differs" })));
            }
            let (_, width) = chargrid::format_geometry(format).ok_or("format geometry")?;
            let name_idx = chargrid::name_line_index(format).ok_or("name line")?;
            let read_line = zone_lines[name_idx];
            let truth_line: Vec<char> = truth_lines[name_idx].chars().collect();
            let pass = &ctl["accepted_pass"];
            if pass.is_null() {
                // A Tier-1 read assembled from the page text with no single
                // accepting pass (the retry loop ran out): no pixels to read.
                return Ok(Some(json!({ "skip": "no accepted pass" })));
            }
            let image = working_image(&dump_dir, stem, pass)?;
            let gray = image::imageops::grayscale(&image);
            let (w, h) = gray.dimensions();
            let Some((lb, box_source)) = name_line_box(pass, read_line) else {
                return Ok(Some(json!({ "skip": "no line box" })));
            };
            let capture = &pass["chargrid"]["line"];
            let (top, bottom) = (lb[1].max(0.0) as u32, lb[3].max(0.0) as u32);
            let sha_ok = capture["band_sha256"]
                .as_str()
                .map(|want| band_sha256(&gray, top, bottom) == want);
            // Vertical padding of 15% of the box height each side.
            let pad_y = (lb[3] - lb[1]) * 0.15;
            let (ptop, pbottom) = (
                (lb[1] - pad_y).max(0.0).round() as u32,
                ((lb[3] + pad_y).round() as u32).min(h),
            );
            let pitch_guess = (lb[2] - lb[0]) / width as f64;
            let (ptop, pbottom) = text_rows(
                &gray,
                PxRect::clamped(lb[0], f64::from(ptop), lb[2], f64::from(pbottom), w, h),
            )
            .unwrap_or((ptop, pbottom));
            let band_a = PxRect::clamped(
                lb[0] - pitch_guess,
                f64::from(ptop),
                lb[2] + pitch_guess,
                f64::from(pbottom),
                w,
                h,
            );
            let grid_a = ink_grid(&gray, band_a, width);
            let grid_b = capture["fit"].as_object().and_then(|f| {
                Some(Grid {
                    origin: f.get("origin")?.as_f64()? as f32,
                    pitch: f.get("pitch")?.as_f64()? as f32,
                    cells: f.get("cells")?.as_u64()? as usize,
                })
            });
            let run_a = grid_a.and_then(|g| classify_on_grid(&gray, g, ptop, pbottom, &t));
            let run_b = grid_b.and_then(|g| classify_on_grid(&gray, g, ptop, pbottom, &t));
            let first = first_name_cell(format);
            let truth_filler: Vec<bool> = truth_line.iter().map(|&c| c == '<').collect();
            if want_overlays {
                // Two halves of the line, each 1,600 px wide, cut on grid (a)
                // when there is one.
                let (x_lo, x_hi) = match &grid_a {
                    Some(g) => (
                        f64::from(g.origin),
                        f64::from(g.origin) + width as f64 * f64::from(g.pitch),
                    ),
                    None => (lb[0], lb[2]),
                };
                let mid = (x_lo + x_hi) / 2.0;
                let margin = pitch_guess * 0.6;
                for (a, b) in [(x_lo - margin, mid + margin), (mid - margin, x_hi + margin)] {
                    let band = PxRect::clamped(a, f64::from(ptop), b, f64::from(pbottom), w, h);
                    if !band.is_empty() {
                        overlays.push((
                            stem.clone(),
                            overlay(
                                &image,
                                band,
                                grid_a.as_ref(),
                                grid_b.as_ref(),
                                &truth_filler,
                                1600,
                            ),
                        ));
                    }
                }
            }
            Ok(Some(json!({
                "format": format!("{format:?}"),
                "name_idx": name_idx,
                "first": first,
                "box_source": box_source,
                "band_sha_ok": sha_ok,
                "read_line": read_line,
                "grid_a": run_a.as_ref().map(|r| json!([r.grid.origin, r.grid.pitch])),
                "grid_b": run_b.as_ref().map(|r| json!([r.grid.origin, r.grid.pitch])),
                "classes_a": run_a.map(|r| r.classes.iter().map(|c| c.as_str()).collect::<Vec<_>>()),
                "classes_b": run_b.map(|r| r.classes.iter().map(|c| c.as_str()).collect::<Vec<_>>()),
                "truth_filler": truth_filler,
                "truth_line": truth_lines[name_idx],
            })))
        })()?;

        if let Some(p) = prepared {
            if let Some(skip) = p["skip"].as_str() {
                row["skip"] = json!(skip);
                *ceiling_strict += u32::from(s_off);
            } else {
                let first = p["first"].as_u64().unwrap_or(0) as usize;
                let name_idx = p["name_idx"].as_u64().unwrap_or(0) as usize;
                let read_line = p["read_line"].as_str().unwrap_or_default();
                let read_chars: Vec<char> = read_line.chars().collect();
                let truth_filler: Vec<bool> = p["truth_filler"]
                    .as_array()
                    .map(|a| a.iter().filter_map(Value::as_bool).collect())
                    .unwrap_or_default();
                for i in first..truth_filler.len().min(read_chars.len()) {
                    let read_class = if read_chars[i] == '<' {
                        CellClass::Filler
                    } else {
                        CellClass::Letter
                    };
                    ocr_read.add(truth_filler[i], read_class);
                }
                let original = fields_of(&off_row["fields"]);
                let zone = off_row["zone"].as_str().unwrap_or_default();
                // How OCR's read of the name line differs from the fixture,
                // on an edit alignment (a dropped or added cell is one
                // deletion or insertion, not a shifted tail of substitutions).
                let truth_line = p["truth_line"].as_str().unwrap_or_default();
                let (mut filler_read_letter, mut letter_read_filler, mut letter_misread) =
                    (0u32, 0u32, 0u32);
                let (mut inserted, mut deleted) = (0u32, 0u32);
                // The ceiling of any filler-or-letter decision on this read:
                // every name cell the alignment pairs a printed `<` with a read
                // letter is set back to `<`. Truth-driven; a bound, not an arm.
                let mut oracle = String::with_capacity(read_line.len());
                let mut ti = 0usize;
                for edit in synthpass_bench::levenshtein_backtrace(truth_line, read_line) {
                    use synthpass_bench::LevenshteinEdit as E;
                    match edit {
                        E::Match(c) => {
                            oracle.push(c);
                            ti += 1;
                        }
                        E::Substitution { expected, got } => {
                            let in_names = ti >= first;
                            match (expected == '<', got == '<') {
                                (true, false) if in_names => {
                                    filler_read_letter += 1;
                                    oracle.push('<');
                                }
                                (false, true) if in_names => {
                                    letter_read_filler += 1;
                                    oracle.push(got);
                                }
                                (false, false) if in_names => {
                                    letter_misread += 1;
                                    oracle.push(got);
                                }
                                _ => oracle.push(got),
                            }
                            ti += 1;
                        }
                        E::Insertion(c) => {
                            inserted += 1;
                            oracle.push(c);
                        }
                        E::Deletion(_) => {
                            deleted += 1;
                            ti += 1;
                        }
                    }
                }
                row["read_diff"] = json!({
                    "filler_read_letter": filler_read_letter,
                    "letter_read_filler": letter_read_filler,
                    "letter_misread": letter_misread,
                    "inserted": inserted,
                    "deleted": deleted,
                });
                let ceiling = if oracle == read_line {
                    s_off
                } else {
                    reparse(zone, name_idx, &oracle, &original)
                        .is_some_and(|f| names_exact(&f, &truth))
                };
                row["ceiling_strict"] = json!(ceiling);
                *ceiling_strict += u32::from(ceiling);
                row["format"] = p["format"].clone();
                row["box_source"] = p["box_source"].clone();
                row["band_sha_ok"] = p["band_sha_ok"].clone();
                for (key, conf) in [("a", &mut conf_a), ("b", &mut conf_b)] {
                    let Some(classes) = p[format!("classes_{key}")].as_array() else {
                        row[format!("proto_{key}")] = json!("no_grid");
                        continue;
                    };
                    let classes: Vec<CellClass> = classes
                        .iter()
                        .map(|c| match c.as_str() {
                            Some("filler") => CellClass::Filler,
                            Some("letter") => CellClass::Letter,
                            Some("occluded") => CellClass::Occluded,
                            _ => CellClass::Uncertain,
                        })
                        .collect();
                    for i in first..truth_filler.len().min(classes.len()) {
                        conf.add(truth_filler[i], classes[i]);
                    }
                    let arm = if key == "a" { "proto_a" } else { "proto_b" };
                    {
                        let (line, verdict) = apply_with(read_line, &classes, first, false);
                        let fields = if verdict == "changed" {
                            reparse(zone, name_idx, &line, &original)
                        } else {
                            None
                        }
                        .unwrap_or_else(|| original.clone());
                        let s = names_exact(&fields, &truth);
                        no_guard_strict.insert(arm, s);
                        row[format!("{arm}_no_guard_strict")] = json!(s);
                    }
                    let (new_line, verdict) = apply(read_line, &classes, first);
                    let mut final_fields = original.clone();
                    let mut outcome = verdict.to_string();
                    if verdict == "changed" {
                        match reparse(zone, name_idx, &new_line, &original) {
                            Some(f) => final_fields = f,
                            None => outcome = "abstain:reparse".to_string(),
                        }
                    }
                    let s = names_exact(&final_fields, &truth);
                    proto_strict.insert(arm, s);
                    let before = field_correct(&original, &truth);
                    let after = field_correct(&final_fields, &truth);
                    let broken: Vec<&String> = before
                        .iter()
                        .filter(|(k, &ok)| ok && after.get(*k) == Some(&false))
                        .map(|(k, _)| k)
                        .collect();
                    if !broken.is_empty() {
                        *field_regressions.get_mut(arm).ok_or("")? += 1;
                    }
                    if s_off && !s {
                        *regressions.get_mut(arm).ok_or("")? += 1;
                    }
                    let changed_cells = new_line
                        .chars()
                        .zip(read_line.chars())
                        .filter(|(a, b)| a != b)
                        .count();
                    row[arm] = json!({
                        "outcome": outcome,
                        "cells_changed": changed_cells,
                        "strict": s,
                        "fields_broken": broken,
                        "grid": p[format!("grid_{key}")].clone(),
                    });
                }
            }
        }
        for (arm, s) in &proto_strict {
            *strict.get_mut(arm).ok_or("")? += u32::from(*s);
        }
        for (arm, s) in no_guard_strict {
            let e = no_guard.entry(arm).or_insert((0u32, 0u32));
            e.0 += u32::from(s);
            e.1 += u32::from(s_off && !s);
        }
        let cell = |v: &Value| -> String {
            if let Some(s) = v.as_str() {
                return s.to_string();
            }
            match (v["outcome"].as_str(), v["strict"].as_bool()) {
                (Some(o), Some(s)) => format!("{o} / {}", if s { "exact" } else { "wrong" }),
                _ => "—".to_string(),
            }
        };
        table.push(format!(
            "| {stem} | {} | {} | {} | {} | {} |",
            if row["hit"] == true { "hit" } else { "miss" },
            if s_off { "exact" } else { "wrong" },
            if s_on { "exact" } else { "wrong" },
            cell(&row["proto_a"]),
            cell(&row["proto_b"]),
        ));
        doc_rows.push(row);
    }

    println!("## Per-cell (name cells of Tier-1 hits; truth from the fixture)\n");
    println!("| Source | Truth | filler | letter | uncertain | occluded | cells |");
    println!("| --- | --- | ---: | ---: | ---: | ---: | ---: |");
    for (label, conf) in [("grid (a)", &conf_a), ("grid (b)", &conf_b)] {
        println!("| {label} | `<` | {} |", conf.row(true));
        println!("| {label} | letter | {} |", conf.row(false));
    }
    println!("| OCR read | `<` | {} |", ocr_read.row(true));
    println!("| OCR read | letter | {} |", ocr_read.row(false));
    println!("\n## Per-document ({} name-scorable)\n", off.len());
    println!(
        "| Document | Tier-1 | off | chargrid on | prototype, grid (a) | prototype, grid (b) |"
    );
    println!("| --- | --- | --- | --- | --- | --- |");
    for line in &table {
        println!("{line}");
    }
    println!("\nstrict names: {strict:?}");
    println!("counterfactual, ceiling (truth-driven): {ceiling_total}");
    println!("counterfactual, no misalignment guard (strict, correct->wrong): {no_guard:?}");
    println!("name correct->wrong: {regressions:?}");
    println!("documents with any field correct->wrong: {field_regressions:?}");
    for d in &disagreements {
        println!("note: {d}");
    }

    let summary = json!({
        "thresholds": t.to_json(),
        "documents": off.len(),
        "strict": strict,
        "counterfactual_ceiling_strict": ceiling_total,
        "counterfactual_no_guard": no_guard,
        "name_regressions": regressions,
        "field_regressions": field_regressions,
        "confusion_a": conf_a.to_json(),
        "confusion_b": conf_b.to_json(),
        "confusion_ocr_read": ocr_read.to_json(),
        "disagreements": disagreements,
        "documents_detail": doc_rows,
    });
    let path = out.join("real-analysis.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    eprintln!("wrote {}", path.display());

    if want_overlays {
        let dir = out.join("overlays");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        for (i, chunk) in overlays.chunks(8).enumerate() {
            let height: u32 = chunk.iter().map(|(_, im)| im.height() + 4).sum();
            let mut sheet = RgbImage::from_pixel(1600, height.max(1), Rgb([128, 128, 128]));
            let mut y = 0i64;
            let mut names = Vec::new();
            for (stem, im) in chunk {
                image::imageops::overlay(&mut sheet, im, 0, y);
                y += i64::from(im.height()) + 4;
                if names.last() != Some(stem) {
                    names.push(stem.clone());
                }
            }
            let path = dir.join(format!("sheet-{i:02}.png"));
            sheet.save(&path).map_err(|e| e.to_string())?;
            fs::write(dir.join(format!("sheet-{i:02}.txt")), names.join("\n"))
                .map_err(|e| e.to_string())?;
        }
        eprintln!("wrote overlays to {}", dir.display());
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Synthetic: `tune` and `sweep`
// ---------------------------------------------------------------------

/// Native cell width of the generator's MRZ, px.
const NATIVE_PX: f64 = synthpass_gen::layout::MRZ_CELL_WIDTH as f64;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Axis {
    Resolution,
    Contrast,
    Rotation,
    Blur,
}

impl Axis {
    const ALL: [Axis; 4] = [Axis::Resolution, Axis::Contrast, Axis::Rotation, Axis::Blur];

    fn name(self) -> &'static str {
        match self {
            Axis::Resolution => "resolution",
            Axis::Contrast => "contrast",
            Axis::Rotation => "rotation",
            Axis::Blur => "blur",
        }
    }

    /// The glyph atlas's steps for the same axes (px per cell, ink scale,
    /// degrees clockwise, Gaussian sigma in native px).
    fn steps(self) -> &'static [f64] {
        match self {
            Axis::Resolution => &[
                22.0, 18.0, 15.0, 13.0, 11.0, 10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0,
            ],
            Axis::Contrast => &[1.0, 0.8, 0.6, 0.5, 0.4, 0.3, 0.2, 0.15, 0.1],
            Axis::Rotation => &[0.0, 0.25, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0],
            Axis::Blur => &[0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0],
        }
    }
}

/// A rendered page, the line's rectangle and each cell's rectangle, mapped
/// into the page's pixel space, and the line's truth.
struct SyntheticLine {
    image: RgbImage,
    /// `(left, top, right, bottom)` per cell, in page pixels.
    cells: Vec<(f64, f64, f64, f64)>,
    truth: Vec<char>,
}

fn render_td3(seed: u64) -> Result<(RgbImage, synthpass_gen::layout::Rect, Vec<char>), String> {
    use synthpass_gen::{generate_from_seed, DocumentType, GeneratorConfig};
    let page = synthpass_gen::layout::for_format(DocumentType::TD3);
    let line = *page
        .mrz_lines
        .first()
        .ok_or("TD3 layout without MRZ lines")?;
    let (image, labels, _) = generate_from_seed(&GeneratorConfig::with_document_type(
        seed,
        DocumentType::TD3,
    ));
    let truth: Vec<char> = labels
        .mrz_lines
        .first()
        .ok_or("generator produced no MRZ line")?
        .chars()
        .collect();
    Ok((image.into_rgb8(), line, truth))
}

/// Rotate `(x, y)` clockwise as displayed by `deg` about `(cx, cy)`.
fn rotate_point((x, y): (f64, f64), deg: f64, cx: f64, cy: f64) -> (f64, f64) {
    let (s, c) = deg.to_radians().sin_cos();
    let (dx, dy) = (x - cx, y - cy);
    (cx + dx * c - dy * s, cy + dx * s + dy * c)
}

/// Bilinear rotation, as the glyph atlas does it: each output pixel centre is
/// mapped back into the source; outside the page is the paper colour.
fn rotate_about(clean: &RgbImage, deg: f64, cx: f64, cy: f64, paper: [u8; 3]) -> RgbImage {
    let (w, h) = clean.dimensions();
    let (s, c) = deg.to_radians().sin_cos();
    let fetch = |x: i64, y: i64, ch: usize| -> f64 {
        if x < 0 || y < 0 || x >= i64::from(w) || y >= i64::from(h) {
            f64::from(paper[ch])
        } else {
            f64::from(clean.get_pixel(x as u32, y as u32).0[ch])
        }
    };
    let mut out = RgbImage::new(w, h);
    for oy in 0..h {
        for ox in 0..w {
            let dx = f64::from(ox) + 0.5 - cx;
            let dy = f64::from(oy) + 0.5 - cy;
            let u = cx + dx * c + dy * s - 0.5;
            let v = cy - dx * s + dy * c - 0.5;
            let (x0, y0) = (u.floor(), v.floor());
            let (fx, fy) = (u - x0, v - y0);
            let (x0, y0) = (x0 as i64, y0 as i64);
            let mut px = [0u8; 3];
            for (ch, value) in px.iter_mut().enumerate() {
                let top = fetch(x0, y0, ch) * (1.0 - fx) + fetch(x0 + 1, y0, ch) * fx;
                let bot = fetch(x0, y0 + 1, ch) * (1.0 - fx) + fetch(x0 + 1, y0 + 1, ch) * fx;
                *value = (top * (1.0 - fy) + bot * fy).round().clamp(0.0, 255.0) as u8;
            }
            out.put_pixel(ox, oy, Rgb(px));
        }
    }
    out
}

/// One degraded line: the page degraded on one axis, and the true cell
/// rectangles mapped through that degradation (the exact grid).
fn degraded_line(seed: u64, axis: Axis, step: f64) -> Result<SyntheticLine, String> {
    let (clean, line, truth) = render_td3(seed)?;
    let n = truth.len();
    let rect = |k: usize| {
        let r = synthpass_gen::layout::mrz_char_rect_for_line(line, n as u32, k as u32);
        (
            f64::from(r.x),
            f64::from(r.y),
            f64::from(r.x + r.width),
            f64::from(r.y + r.height),
        )
    };
    let paper = {
        let x = (line.x / 2).min(clean.width().saturating_sub(1));
        let y = (line.y + line.height / 2).min(clean.height().saturating_sub(1));
        clean.get_pixel(x, y).0
    };
    let identity = |image: RgbImage| SyntheticLine {
        image,
        cells: (0..n).map(rect).collect(),
        truth: truth.clone(),
    };
    Ok(match axis {
        Axis::Resolution => {
            if step >= NATIVE_PX {
                identity(clean)
            } else {
                let s = step / NATIVE_PX;
                let (w, h) = clean.dimensions();
                let nw = ((f64::from(w) * s).round() as u32).max(1);
                let nh = ((f64::from(h) * s).round() as u32).max(1);
                let (sx, sy) = (f64::from(nw) / f64::from(w), f64::from(nh) / f64::from(h));
                let image =
                    image::imageops::resize(&clean, nw, nh, image::imageops::FilterType::Triangle);
                SyntheticLine {
                    image,
                    cells: (0..n)
                        .map(|k| {
                            let (a, b, c, d) = rect(k);
                            (a * sx, b * sy, c * sx, d * sy)
                        })
                        .collect(),
                    truth: truth.clone(),
                }
            }
        }
        Axis::Contrast => {
            let mut image = clean;
            for p in image.pixels_mut() {
                for (ch, v) in p.0.iter_mut().enumerate() {
                    let pv = f64::from(paper[ch]);
                    *v = (pv - step * (pv - f64::from(*v))).round().clamp(0.0, 255.0) as u8;
                }
            }
            identity(image)
        }
        Axis::Rotation => {
            if step == 0.0 {
                identity(clean)
            } else {
                let cx = f64::from(line.x) + f64::from(line.width) / 2.0;
                let cy = f64::from(line.y) + f64::from(line.height) / 2.0;
                let image = rotate_about(&clean, step, cx, cy, paper);
                SyntheticLine {
                    image,
                    // Each cell's rotated quad, as its axis-aligned bounds.
                    cells: (0..n)
                        .map(|k| {
                            let (a, b, c, d) = rect(k);
                            let pts = [(a, b), (c, b), (c, d), (a, d)]
                                .map(|p| rotate_point(p, step, cx, cy));
                            let xs = pts.map(|p| p.0);
                            let ys = pts.map(|p| p.1);
                            (
                                xs.iter().copied().fold(f64::MAX, f64::min),
                                ys.iter().copied().fold(f64::MAX, f64::min),
                                xs.iter().copied().fold(f64::MIN, f64::max),
                                ys.iter().copied().fold(f64::MIN, f64::max),
                            )
                        })
                        .collect(),
                    truth: truth.clone(),
                }
            }
        }
        Axis::Blur => {
            if step == 0.0 {
                identity(clean)
            } else {
                identity(image::imageops::blur(&clean, step as f32))
            }
        }
    })
}

/// Features of the name cells (5..44) of one synthetic line, on the exact
/// grid. For a rotation each cell is its rotated quad's bounds, so a
/// neighbour's ink can enter it; that is part of what the axis measures.
fn synthetic_features(line: &SyntheticLine) -> Option<Vec<(char, CellFeatures)>> {
    let gray = image::imageops::grayscale(&line.image);
    let (w, h) = gray.dimensions();
    let rects: Vec<PxRect> = line
        .cells
        .iter()
        .map(|&(a, b, c, d)| PxRect::clamped(a, b, c, d, w, h))
        .collect();
    let band = PxRect {
        x0: rects.iter().map(|r| r.x0).min()?,
        y0: rects.iter().map(|r| r.y0).min()?,
        x1: rects.iter().map(|r| r.x1).max()?,
        y1: rects.iter().map(|r| r.y1).max()?,
    };
    // Cells that share their rows (every axis but rotation) are cut to the
    // band's text rows, as on real crops.
    let shared = rects.iter().all(|r| r.y0 == band.y0 && r.y1 == band.y1);
    let (band, rects) = match text_rows(&gray, band).filter(|_| shared) {
        Some((top, bottom)) => (
            PxRect {
                y0: top,
                y1: bottom,
                ..band
            },
            rects
                .iter()
                .map(|r| PxRect {
                    y0: top,
                    y1: bottom,
                    ..*r
                })
                .collect(),
        ),
        None => (band, rects),
    };
    let feats = line_features(&gray, band, &rects)?;
    Some(line.truth.iter().copied().zip(feats).skip(5).collect())
}

/// Feature names, in [`feature_values`]'s order.
const FEATURES: [&str; 7] = ["ink", "r", "v", "h", "corner", "apex", "ends"];

fn feature_values(f: &CellFeatures) -> [f64; 7] {
    [f.ink, f.r, f.v, f.h, f.corner, f.apex, f.ends]
}

/// The tuning population: `--seeds` seeds from `--seed-start` (default
/// 100-199, disjoint from the sweep's 0-99), at every step of every axis
/// (`--clean-only` for the clean render alone).
fn tuning_lines(args: &[String]) -> Result<Vec<(Axis, f64, SyntheticLine)>, String> {
    let seeds = parse_u64(args, "--seeds", 100)?;
    let start = parse_u64(args, "--seed-start", 100)?;
    let clean_only = args.iter().any(|a| a == "--clean-only");
    let mut lines = Vec::new();
    for seed in start..start + seeds {
        for axis in Axis::ALL {
            for &step in axis.steps() {
                let identity = matches!(
                    (axis, step),
                    (Axis::Resolution, 22.0)
                        | (Axis::Contrast, 1.0)
                        | (Axis::Rotation, 0.0)
                        | (Axis::Blur, 0.0)
                );
                // The clean render appears once, under contrast 1.
                if (clean_only && !(axis == Axis::Contrast && step == 1.0))
                    || (identity && axis != Axis::Contrast)
                {
                    continue;
                }
                lines.push((axis, step, degraded_line(seed, axis, step)?));
            }
        }
    }
    Ok(lines)
}

fn run_tune(args: &[String]) -> Result<(), String> {
    synthpass_gen::fonts::load_fonts().map_err(|e| format!("fonts: {e:?}"))?;
    let lines = tuning_lines(args)?;
    let mut filler: Vec<Vec<f64>> = vec![Vec::new(); FEATURES.len()];
    let mut letter: Vec<Vec<f64>> = vec![Vec::new(); FEATURES.len()];
    let mut per_letter: BTreeMap<char, Vec<[f64; 7]>> = BTreeMap::new();
    let t = Thresholds::SYNTHETIC_TUNED;
    let mut conf = Confusion::default();
    let mut per_step: BTreeMap<(&'static str, String), Confusion> = BTreeMap::new();
    for (axis, step, line) in &lines {
        let Some(feats) = synthetic_features(line) else {
            continue;
        };
        for (ch, f) in feats {
            let values = feature_values(&f);
            let side = if ch == '<' { &mut filler } else { &mut letter };
            for (i, v) in values.iter().enumerate() {
                side[i].push(*v);
            }
            if ch != '<' {
                per_letter.entry(ch).or_default().push(values);
            }
            let c = classify(&f, &t);
            conf.add(ch == '<', c);
            per_step
                .entry((axis.name(), format!("{step}")))
                .or_default()
                .add(ch == '<', c);
        }
    }
    println!(
        "tuning set: {} lines, name cells 5-43: {} filler, {} letter",
        lines.len(),
        filler[0].len(),
        letter[0].len()
    );
    println!("| Feature | Class | min | p1 | p5 | p50 | p95 | p99 | max |");
    println!("| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
    for (i, name) in FEATURES.iter().enumerate() {
        for (class, v) in [("`<`", &filler[i]), ("letter", &letter[i])] {
            let p = |q| percentile(v, q).unwrap_or(f64::NAN);
            println!(
                "| {name} | {class} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} |",
                p(0.0),
                p(1.0),
                p(5.0),
                p(50.0),
                p(95.0),
                p(99.0),
                p(100.0)
            );
        }
    }
    println!("\nper letter, p1 of each feature:");
    println!("| Letter | n | {} |", FEATURES.join(" | "));
    for (ch, rows) in &per_letter {
        let cols: Vec<String> = (0..FEATURES.len())
            .map(|i| {
                let v: Vec<f64> = rows.iter().map(|r| r[i]).collect();
                format!("{:.3}", percentile(&v, 1.0).unwrap_or(f64::NAN))
            })
            .collect();
        println!("| {ch} | {} | {} |", rows.len(), cols.join(" | "));
    }
    if args.iter().any(|a| a == "--search") {
        let cells: Vec<(bool, CellFeatures)> = lines
            .iter()
            .filter_map(|(_, _, line)| synthetic_features(line))
            .flatten()
            .map(|(ch, f)| (ch == '<', f))
            .collect();
        let steps = |lo: f64, hi: f64, step: f64| -> Vec<f64> {
            let n = ((hi - lo) / step).round() as usize;
            (0..=n)
                .map(|i| ((lo + i as f64 * step) * 100.0).round() / 100.0)
                .collect()
        };
        let mut best: Option<(u64, u64, Thresholds)> = None;
        // Strictest first: low corner and v bounds, high apex and ends bounds.
        for corner in steps(0.30, 0.90, 0.05) {
            for apex in steps(0.8, 1.6, 0.1).into_iter().rev() {
                for ends in steps(0.0, 1.2, 0.1).into_iter().rev() {
                    for v in steps(0.45, 0.90, 0.05) {
                        let t = Thresholds::with_filler_bounds(corner, apex, ends, v);
                        let (mut hits, mut false_fillers, mut filler_as_letter) =
                            (0u64, 0u64, 0u64);
                        for (is_filler, f) in &cells {
                            match (classify(f, &t), is_filler) {
                                (CellClass::Filler, true) => hits += 1,
                                (CellClass::Filler, false) => false_fillers += 1,
                                (CellClass::Letter, true) => filler_as_letter += 1,
                                _ => {}
                            }
                        }
                        if false_fillers == 0 && best.is_none_or(|(b, _, _)| hits > b) {
                            best = Some((hits, filler_as_letter, t));
                        }
                    }
                }
            }
        }
        match best {
            Some((hits, as_letter, t)) => println!(
                "\nsearch: best {:?}\n  fillers classed filler {hits} of {}, fillers classed letter {as_letter}, letters classed filler 0",
                t.to_json(),
                filler[0].len()
            ),
            None => println!("\nsearch: no combination keeps letters out of `filler`"),
        }
    }
    println!("\nwith the committed thresholds, on the tuning set (filler | letter | uncertain | occluded | n):");
    println!("`<`: {}", conf.row(true));
    println!("letter: {}", conf.row(false));
    for ((axis, step), c) in &per_step {
        println!(
            "  {axis} {step}: `<` {} ; letter {}",
            c.row(true),
            c.row(false)
        );
    }
    Ok(())
}

fn run_sweep(args: &[String]) -> Result<(), String> {
    let seeds = parse_u64(args, "--seeds", 100)?;
    let start = parse_u64(args, "--seed-start", 0)?;
    let out = out_dir(args)?;
    synthpass_gen::fonts::load_fonts().map_err(|e| format!("fonts: {e:?}"))?;
    let t = Thresholds::SYNTHETIC_TUNED;
    let mut rows = Vec::new();
    println!(
        "sweep: TD3, seeds {start}-{}, name cells 5-43, exact grid, thresholds as committed",
        start + seeds - 1
    );
    println!(
        "| Axis | Step | `<` cells | `<` → filler | `<` → letter | `<` → uncertain | `<` → occluded | letter cells | letter → filler | letter → letter | letter → uncertain | letter → occluded | lines all-correct |"
    );
    println!("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
    for axis in Axis::ALL {
        for &step in axis.steps() {
            let mut conf = Confusion::default();
            let mut all_correct = 0u64;
            let mut no_features = 0u64;
            for seed in start..start + seeds {
                let line = degraded_line(seed, axis, step)?;
                let Some(feats) = synthetic_features(&line) else {
                    no_features += 1;
                    continue;
                };
                let mut ok = true;
                for (ch, f) in feats {
                    let c = classify(&f, &t);
                    conf.add(ch == '<', c);
                    let want = if ch == '<' {
                        CellClass::Filler
                    } else {
                        CellClass::Letter
                    };
                    ok &= c == want;
                }
                all_correct += u64::from(ok);
            }
            let g = |f, c| conf.get(f, c);
            let nf: u64 = CellClass::ALL.iter().map(|&c| g(true, c)).sum();
            let nl: u64 = CellClass::ALL.iter().map(|&c| g(false, c)).sum();
            println!(
                "| {} | {step} | {nf} | {} | {} | {} | {} | {nl} | {} | {} | {} | {} | {all_correct} / {seeds} |",
                axis.name(),
                g(true, CellClass::Filler),
                g(true, CellClass::Letter),
                g(true, CellClass::Uncertain),
                g(true, CellClass::Occluded),
                g(false, CellClass::Filler),
                g(false, CellClass::Letter),
                g(false, CellClass::Uncertain),
                g(false, CellClass::Occluded),
            );
            rows.push(json!({
                "axis": axis.name(),
                "step": step,
                "confusion": conf.to_json(),
                "lines_all_correct": all_correct,
                "lines_without_features": no_features,
                "seeds": seeds,
            }));
        }
    }
    let path = out.join("sweep.json");
    let body = json!({
        "seed_start": start,
        "seeds": seeds,
        "thresholds": t.to_json(),
        "git_head": git_head(),
        "rows": rows,
    });
    fs::write(
        &path,
        serde_json::to_string_pretty(&body).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

/// Writes one degraded synthetic line with its exact grid drawn in, for
/// looking at what the features measure.
fn run_render(args: &[String]) -> Result<(), String> {
    let seed = parse_u64(args, "--seed", 100)?;
    let axis_name = flag_value(args, "--axis")?.unwrap_or("contrast");
    let axis = Axis::ALL
        .into_iter()
        .find(|a| a.name() == axis_name)
        .ok_or_else(|| format!("unknown axis {axis_name}"))?;
    let step: f64 = match flag_value(args, "--step")? {
        Some(v) => v.parse().map_err(|_| format!("--step: {v}"))?,
        None => 1.0,
    };
    synthpass_gen::fonts::load_fonts().map_err(|e| format!("fonts: {e:?}"))?;
    let line = degraded_line(seed, axis, step)?;
    let (w, h) = line.image.dimensions();
    let left = line.cells.iter().map(|c| c.0).fold(f64::MAX, f64::min);
    let right = line.cells.iter().map(|c| c.2).fold(f64::MIN, f64::max);
    let top = line.cells.iter().map(|c| c.1).fold(f64::MAX, f64::min);
    let bottom = line.cells.iter().map(|c| c.3).fold(f64::MIN, f64::max);
    let band = PxRect::clamped(left, top, right, bottom, w, h);
    let pitch = (right - left) / line.cells.len() as f64;
    let grid = Grid {
        origin: left as f32,
        pitch: pitch as f32,
        cells: line.cells.len(),
    };
    let truth: Vec<bool> = line.truth.iter().map(|&c| c == '<').collect();
    let img = overlay(&line.image, band, Some(&grid), None, &truth, 1600);
    let path = out_dir(args)?.join(format!("render-{seed}-{axis_name}-{step}.png"));
    img.save(&path).map_err(|e| e.to_string())?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("real") => run_real(&args[1..]),
        Some("real-analyze") => run_real_analyze(&args[1..]),
        Some("tune") => run_tune(&args[1..]),
        Some("sweep") => run_sweep(&args[1..]),
        Some("render") => run_render(&args[1..]),
        _ => Err(
            "usage: filler_cells real|real-analyze|tune|sweep [options] (see the module doc)"
                .into(),
        ),
    };
    if let Err(e) = result {
        eprintln!("filler_cells: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    /// A white band with black rectangles drawn in, for the pure functions.
    fn band_with(w: u32, h: u32, boxes: &[(u32, u32, u32, u32)]) -> GrayImage {
        let mut img = GrayImage::from_pixel(w, h, Luma([240]));
        for &(x0, y0, x1, y1) in boxes {
            for y in y0..y1 {
                for x in x0..x1 {
                    img.put_pixel(x, y, Luma([20]));
                }
            }
        }
        img
    }

    #[test]
    fn otsu_splits_two_levels_between_them() {
        let t = otsu([20u8, 20, 20, 240, 240, 240].into_iter());
        assert!(t.is_some_and(|t| (20..240).contains(&t)));
        assert_eq!(otsu(std::iter::empty()), None);
    }

    /// Draws a 3 px-thick straight stroke from `(x0, y0)` to `(x1, y1)`.
    fn stroke(img: &mut GrayImage, (x0, y0): (f64, f64), (x1, y1): (f64, f64)) {
        let steps = 200;
        for i in 0..=steps {
            let t = f64::from(i) / f64::from(steps);
            let (x, y) = (x0 + t * (x1 - x0), y0 + t * (y1 - y0));
            for dx in -1..=1 {
                let px = (x.round() as i64 + dx) as u32;
                if px < img.width() && (y.round() as u32) < img.height() {
                    img.put_pixel(px, y.round() as u32, Luma([20]));
                }
            }
        }
    }

    #[test]
    fn a_stemmed_letter_is_a_letter_and_a_chevron_is_a_filler() {
        // Cell 0 (x 0-20): an `L`, a full-height stem and a foot. Cell 1
        // (x 20-40): a `<`, two steep arms meeting at the middle left.
        let mut img = band_with(40, 30, &[(4, 3, 8, 27), (4, 23, 16, 27)]);
        stroke(&mut img, (35.0, 3.0), (25.0, 15.0));
        stroke(&mut img, (25.0, 15.0), (35.0, 27.0));
        let cells = [
            PxRect {
                x0: 0,
                y0: 0,
                x1: 20,
                y1: 30,
            },
            PxRect {
                x0: 20,
                y0: 0,
                x1: 40,
                y1: 30,
            },
        ];
        let band = PxRect {
            x0: 0,
            y0: 0,
            x1: 40,
            y1: 30,
        };
        let f = line_features(&img, band, &cells).expect("features");
        let t = Thresholds::SYNTHETIC_TUNED;
        assert_eq!(classify(&f[0], &t), CellClass::Letter, "{:?}", f[0]);
        assert_eq!(classify(&f[1], &t), CellClass::Filler, "{:?}", f[1]);
    }

    #[test]
    fn a_solid_cell_is_occluded_and_an_empty_one_uncertain() {
        let t = Thresholds::SYNTHETIC_TUNED;
        let solid = CellFeatures {
            ink: 0.9,
            r: 1.0,
            v: 1.0,
            h: 1.0,
            has_box: true,
            corner: 1.0,
            apex: 1.0,
            ends: 1.0,
        };
        let empty = CellFeatures {
            ink: 0.0,
            r: 0.0,
            v: 0.0,
            h: 0.0,
            has_box: false,
            corner: 0.0,
            apex: 0.0,
            ends: 0.0,
        };
        assert_eq!(classify(&solid, &t), CellClass::Occluded);
        assert_eq!(classify(&empty, &t), CellClass::Uncertain);
    }

    #[test]
    fn apply_only_turns_filler_cells_read_as_letters_into_fillers() {
        use CellClass::*;
        let classes = [Letter, Letter, Filler, Filler, Uncertain, Occluded];
        assert_eq!(
            apply("ABSC<D", &classes, 0),
            ("AB<<<D".to_string(), "changed")
        );
        // A letter cell read as `<` means the grid and the read disagree.
        assert_eq!(
            apply("<BSCXD", &classes, 0).1,
            "abstain:letter_read_as_filler"
        );
        // Cells before `first` are never touched or checked.
        assert_eq!(
            apply("<BSC<D", &classes, 1),
            ("<B<<<D".to_string(), "changed")
        );
        assert_eq!(apply("AB", &classes, 0).1, "abstain:width");
    }

    #[test]
    fn the_ink_grid_recovers_a_regular_grid() {
        // Ten cells of 20 px from x = 13; each glyph 12 px wide, 4 px in.
        let boxes: Vec<(u32, u32, u32, u32)> = (0..10u32)
            .map(|k| (13 + 20 * k + 4, 3, 13 + 20 * k + 16, 27))
            .collect();
        let img = band_with(240, 30, &boxes);
        let band = PxRect {
            x0: 0,
            y0: 0,
            x1: 240,
            y1: 30,
        };
        let g = ink_grid(&img, band, 10).expect("grid");
        assert!((g.pitch - 20.0).abs() < 0.5, "pitch {}", g.pitch);
        // Every true glyph centre falls inside its own fitted cell.
        for k in 0..10u32 {
            let centre = (13 + 20 * k + 10) as f32;
            let cell = ((centre - g.origin) / g.pitch).floor() as u32;
            assert_eq!(cell, k);
        }
    }
}
