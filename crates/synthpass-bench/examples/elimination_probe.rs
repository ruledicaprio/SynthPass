//! Elimination probe: the first two measurements that
//! `knowledge/research/mrz-geometric-elimination.md` asks for before any of it
//! earns a decision ("What would have to be measured", items 1 and 2), taken on
//! the public real specimens against their reviewed fixtures.
//!
//! 1. **Top edge, digit against letter.** Within one MRZ line, does the top
//!    edge of a cell's ink separate printed digits from printed letters? The
//!    answer is a distribution over real lines at their real resolutions, plus
//!    the resolution below which the separation breaks: the resolution floor.
//! 2. **Ink profile, `<` against `K`.** Does the share of a cell's ink in the
//!    left quarter of its glyph separate the filler from `K`? That is the one
//!    pair no check digit can tell apart.
//!
//! This is a **bench-only** measurement. Nothing in the product calls it, no
//! default changes, and it decides nothing: ADR-0014 stays Proposed. **The
//! evidence label for anything derived from it is "Observed on public real
//! specimens, ink-only segmentation, reviewed-fixture truth".**
//!
//! # It reads no text
//!
//! The probe never runs OCR. It finds a zone from its ink alone. Each cell's
//! truth comes from the reviewed fixture (`samples/ocr_fixtures/<stem>.json`),
//! reduced to one of four classes: `digit`, `letter` (any capital except `K`),
//! `K`, or `filler`. Everything the probe prints or writes is classes and
//! geometry, never a character string.
//!
//! # Population
//!
//! A `samples/corpus.jsonl` row is used when all of these hold:
//!
//! - its `ground_truth_stem` names a reviewed fixture directly under
//!   `samples/ocr_fixtures/`. The derived fixtures under `derived/` are never
//!   used, because their names are one OCR pass's reading;
//! - its MRZ is present and not redacted;
//! - its image, read from `--samples-root`, matches the row's sha256. The
//!   default root is `samples/`, which `scripts/sync-samples.ps1` fills from
//!   the `samples-data` branch.
//!
//! The corpus lists public specimens only, and the probe reads nothing else.
//!
//! # Finding a zone without reading it
//!
//! A pixel's grey level is `max(R, G, B)`. MRZ ink is black, so it stays dark,
//! while the coloured security print under it turns light. The steps are:
//!
//! 1. **Binarise.** Sauvola with a window of 15, then 31, then 61 px, then
//!    Otsu over the whole image. The first attempt that yields a zone is kept.
//! 2. **Chain.** The image is cut into 8-connected components, and the
//!    glyph-like ones are chained left to right. Each next component sits 0.55
//!    to 1.7 heights to the right and within 0.35 heights vertically.
//! 3. **Split.** A chain is split wherever a centre spacing strays more than
//!    25% from the chain's median spacing.
//! 4. **Line.** A line is a regular run of exactly the fixture's line width
//!    (44, 36 or 30).
//! 5. **Zone.** A zone is the fixture's number of such lines, stacked in order.
//!    Their pitches must agree within 10%, their ends must align to half a
//!    cell, and consecutive lines must be 1.1 to 3.5 glyph heights apart. When
//!    more than one stack qualifies, the lowest wins.
//!
//! So a line is accepted only when every cell holds exactly one component. Two
//! touching glyphs, a broken stroke or a missing glyph rejects it. This
//! self-check does not depend on either measured signal.
//!
//! An image rotated by more than a few degrees is not found. A zone turned 180
//! degrees would be found with its cells mapped backwards. Check the overlays
//! (`--overlays`) by eye before quoting any number.
//!
//! # What is measured
//!
//! Each cell is measured on its component's own box, widened by a quarter
//! pitch above and below.
//!
//! - **Ink** is darkness measured against the band's own paper and ink. Otsu
//!   splits the band, and `paper` and `ink` are the medians of the two sides.
//!   `d = (paper - g) / (paper - ink)`, clamped to `[0, 1]`. The 90th
//!   percentile of `d` on the paper side (the paper's own texture) is then
//!   subtracted and the result rescaled to `[0, 1]`. Each pixel is weighted by
//!   how much of it the window covers.
//! - **Top edge at fraction `f`** is the row at which the ink accumulated from
//!   the window's top first reaches `f` of the cell's ink, interpolated inside
//!   that row. `f` is 0.02, 0.05, 0.10 and 0.20, since the note asks to "report
//!   the fraction alongside the separation". The **bottom edge** is the same,
//!   counted from the bottom.
//! - **Rise** is the top edge measured upward against the line's skew, in cells
//!   of the line's pitch. The skew is the Theil-Sen slope of the component
//!   centres. No baseline is fitted, so only differences within one line mean
//!   anything.
//! - **Profile** is the cell's ink in eight equal columns of the box, each as a
//!   fraction of the cell's ink. The **left quarter** is the first two
//!   columns: `K`'s stem puts ink there, and a filler's vertex barely does.
//!
//! # Separation and the resolution floor
//!
//! A line with at least two digits and two letters (`K` counts as a letter)
//! gets three numbers for each `f`:
//!
//! - the **gap**: the median digit rise minus the median letter rise;
//! - the **margin**: the lowest digit rise minus the highest letter rise. Above
//!   zero, one threshold separates every digit from every letter in that line;
//! - the **AUC**: the share of digit-letter pairs in which the digit sits
//!   higher, with ties counting half.
//!
//! Each accepted line is then measured again, resampled to each step of 20, 16,
//! 13, 11, 9, 8, 7, 6, 5 and 4 px per cell that is below its own pitch. The
//! resampling uses `image`'s `Triangle` filter on the cell boxes found at
//! native resolution. The segmentation is held fixed, so only the signal
//! degrades.
//!
//! A line's **floor** is the finest step it reaches from native with the margin
//! above zero at every step along the way. The report gives the distribution of
//! floors and the worst line. For `<` against `K`, the cells of all lines are
//! pooled at each step. The pooled floor is the finest step reached from native
//! with the pooled margin above zero all the way.
//!
//! # Running it
//!
//! ```text
//! cargo run --release -p synthpass-bench --example elimination_probe -- \
//!     [--samples-root DIR] [--out DIR] [--overlays] [--locate-only] [--only STEM]
//! ```
//!
//! - **`--locate-only`** stops after finding the zones and prints the outcome
//!   for each document. Without it, the tables go to stdout and every cell's
//!   measurement goes to `<out>/cells.jsonl`. The default `<out>` is
//!   `artifacts/elimination-probe/<short sha>/`. An in-tree directory that git
//!   does not ignore is refused, and an existing `cells.jsonl` is never
//!   overwritten.
//! - **`--overlays`** also writes `<out>/overlays/<file>.png`: each zone with
//!   its cell boxes coloured by class and the 5% top edge drawn in red.
//! - **`--only`** keeps the rows of one fixture stem.

use image::imageops::FilterType;
use image::{GrayImage, Luma, Rgb, RgbImage};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// The fractions of a cell's ink at which its top and bottom edges are read.
const FRACTIONS: [f64; 4] = [0.02, 0.05, 0.10, 0.20];
/// The index of 0.05 in [`FRACTIONS`]: the edge drawn on the overlays.
const OVERLAY_FRACTION: usize = 1;
/// The resampling steps in px per cell. A line is measured only at the steps
/// below its own pitch.
const STEPS: [f64; 10] = [20.0, 16.0, 13.0, 11.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0];
/// Equal-width columns of a glyph's box in its ink profile.
const BINS: usize = 8;
/// The Sauvola windows tried in order, in px. Otsu over the whole image is
/// tried last.
const SAUVOLA_WINDOWS: [u32; 3] = [15, 31, 61];
/// Sauvola's `k`, against a dynamic range `R` of 128.
const SAUVOLA_K: f64 = 0.2;
/// The smallest component height taken for a glyph, in px.
const MIN_GLYPH_PX: u32 = 6;
/// The least difference, in grey levels, between a band's paper and ink
/// medians for the band to be measured.
const MIN_CONTRAST: f64 = 16.0;
const DEFAULT_OUT: &str = "artifacts/elimination-probe";

// ---------------------------------------------------------------------
// Truth: the reviewed fixture's zone, reduced to classes
// ---------------------------------------------------------------------

/// What the reviewed fixture says a cell holds, reduced to the classes the two
/// measurements compare. `K` is kept apart from the other letters for item 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Digit,
    Letter,
    K,
    Filler,
}

impl Class {
    /// `None` for anything outside the 37 printed MRZ characters.
    fn of(c: char) -> Option<Class> {
        match c {
            '0'..='9' => Some(Class::Digit),
            'K' => Some(Class::K),
            'A'..='Z' => Some(Class::Letter),
            '<' => Some(Class::Filler),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Class::Digit => "digit",
            Class::Letter => "letter",
            Class::K => "K",
            Class::Filler => "filler",
        }
    }

    /// Item 1 compares digits with letters, `K` included.
    fn is_letter(self) -> bool {
        matches!(self, Class::Letter | Class::K)
    }
}

/// The zone's classes, line by line, or why the fixture cannot be used. Only
/// the three ICAO shapes (two lines of 44 or 36, three of 30) are taken.
fn fixture_classes(zone: &str) -> Result<Vec<Vec<Class>>, &'static str> {
    let lines: Vec<&str> = zone.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let width = lines.first().map_or(0, |l| l.chars().count());
    let shape = matches!((lines.len(), width), (2, 44) | (2, 36) | (3, 30));
    if !shape || lines.iter().any(|l| l.chars().count() != width) {
        return Err("fixture_shape");
    }
    lines
        .iter()
        .map(|l| {
            l.chars()
                .map(Class::of)
                .collect::<Option<Vec<_>>>()
                .ok_or("fixture_alphabet")
        })
        .collect()
}

/// The rows whose fixture cannot be used, each with the reason.
type Refused = Vec<(String, &'static str)>;

/// One document of the population: its image and its zone's classes.
struct Specimen {
    /// `dir/filename` under the samples root: the document's name in reports.
    name: String,
    image: PathBuf,
    sha256: String,
    zone: Vec<Vec<Class>>,
}

impl Specimen {
    fn shape(&self) -> String {
        format!("{}x{}", self.zone.len(), self.zone[0].len())
    }
}

/// Reads the population from `samples/corpus.jsonl` (see the module doc). The
/// second list holds the rows whose fixture cannot be used, with the reason.
fn population(
    root: &Path,
    samples_root: &Path,
    only: Option<&str>,
) -> Result<(Vec<Specimen>, Refused), String> {
    let corpus = root.join("samples").join("corpus.jsonl");
    let text = fs::read_to_string(&corpus).map_err(|e| format!("{}: {e}", corpus.display()))?;
    let mut specimens = Vec::new();
    let mut refused = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let at = || format!("{}:{}", corpus.display(), n + 1);
        let row: Value = serde_json::from_str(line).map_err(|e| format!("{}: {e}", at()))?;
        let Some(stem) = row["ground_truth_stem"].as_str() else {
            continue;
        };
        if only.is_some_and(|o| o != stem) {
            continue;
        }
        let fixture = root
            .join("samples")
            .join("ocr_fixtures")
            .join(format!("{stem}.json"));
        if !fixture.is_file() {
            continue;
        }
        if row["mrz"]["present"] != Value::Bool(true) || row["mrz"]["redacted"] == Value::Bool(true)
        {
            continue;
        }
        let (Some(dir), Some(file), Some(sha)) = (
            row["dir"].as_str(),
            row["filename"].as_str(),
            row["sha256"].as_str(),
        ) else {
            return Err(format!("{}: a row without dir, filename or sha256", at()));
        };
        let name = format!("{dir}/{file}");
        let fixture_json: Value = fs::read_to_string(&fixture)
            .map_err(|e| format!("{}: {e}", fixture.display()))
            .and_then(|t| {
                serde_json::from_str(&t).map_err(|e| format!("{}: {e}", fixture.display()))
            })?;
        match fixture_json["mrz_line"]
            .as_str()
            .ok_or("fixture_shape")
            .and_then(fixture_classes)
        {
            Ok(zone) => specimens.push(Specimen {
                name,
                image: samples_root.join(dir).join(file),
                sha256: sha.to_ascii_lowercase(),
                zone,
            }),
            Err(reason) => refused.push((name, reason)),
        }
    }
    Ok((specimens, refused))
}

// ---------------------------------------------------------------------
// Finding the zone from ink alone
// ---------------------------------------------------------------------

/// `max(R, G, B)` per pixel: black ink stays dark, coloured print turns light.
fn grey_of(rgb: &RgbImage) -> GrayImage {
    GrayImage::from_fn(rgb.width(), rgb.height(), |x, y| {
        let [r, g, b] = rgb.get_pixel(x, y).0;
        Luma([r.max(g).max(b)])
    })
}

/// Otsu's threshold over `pixels`: the grey level `t` maximising the
/// between-class variance of `<= t` against `> t`. `None` for no pixels or a
/// single grey level.
fn otsu(pixels: impl Iterator<Item = u8>) -> Option<u8> {
    let mut hist = [0u64; 256];
    let mut n = 0u64;
    for p in pixels {
        hist[usize::from(p)] += 1;
        n += 1;
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

/// Sauvola's local threshold: a pixel is ink when it is no lighter than
/// `m * (1 + k * (s / 128 - 1))`, with `m` and `s` the mean and standard
/// deviation of the `window`-square around it (cut at the image's edges).
fn sauvola(gray: &GrayImage, window: u32) -> Vec<bool> {
    let (w, h) = (gray.width() as usize, gray.height() as usize);
    let stride = w + 1;
    // Integral images of g and g squared, with a zero first row and column.
    let mut s1 = vec![0f64; stride * (h + 1)];
    let mut s2 = vec![0f64; stride * (h + 1)];
    for y in 0..h {
        let (mut r1, mut r2) = (0f64, 0f64);
        for x in 0..w {
            let g = f64::from(gray.get_pixel(x as u32, y as u32)[0]);
            r1 += g;
            r2 += g * g;
            s1[(y + 1) * stride + x + 1] = s1[y * stride + x + 1] + r1;
            s2[(y + 1) * stride + x + 1] = s2[y * stride + x + 1] + r2;
        }
    }
    let area = |s: &[f64], x0: usize, y0: usize, x1: usize, y1: usize| {
        s[y1 * stride + x1] - s[y0 * stride + x1] - s[y1 * stride + x0] + s[y0 * stride + x0]
    };
    let half = (window / 2) as usize;
    let mut ink = vec![false; w * h];
    for y in 0..h {
        let (y0, y1) = (y.saturating_sub(half), (y + half + 1).min(h));
        for x in 0..w {
            let (x0, x1) = (x.saturating_sub(half), (x + half + 1).min(w));
            let n = ((x1 - x0) * (y1 - y0)) as f64;
            let mean = area(&s1, x0, y0, x1, y1) / n;
            let var = (area(&s2, x0, y0, x1, y1) / n - mean * mean).max(0.0);
            let t = mean * (1.0 + SAUVOLA_K * (var.sqrt() / 128.0 - 1.0));
            ink[y * w + x] = f64::from(gray.get_pixel(x as u32, y as u32)[0]) <= t;
        }
    }
    ink
}

/// One global Otsu threshold over the whole image.
fn otsu_ink(gray: &GrayImage) -> Vec<bool> {
    let t = otsu(gray.pixels().map(|p| p[0])).unwrap_or(0);
    gray.pixels().map(|p| p[0] <= t).collect()
}

/// A connected component of ink: its box `[x0, x1) x [y0, y1)` and its pixel
/// count.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Blob {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    area: u32,
}

impl Blob {
    fn width(self) -> f64 {
        f64::from(self.x1 - self.x0)
    }

    fn height(self) -> f64 {
        f64::from(self.y1 - self.y0)
    }

    fn cx(self) -> f64 {
        f64::from(self.x0 + self.x1) / 2.0
    }

    fn cy(self) -> f64 {
        f64::from(self.y0 + self.y1) / 2.0
    }

    /// Sized and filled like one glyph: at least [`MIN_GLYPH_PX`] tall, at
    /// most a quarter of the image's height, at most 1.6 times as wide as it
    /// is tall, and inked over at least 8% of its box. A `<` fills about a
    /// quarter of its box, and a ruled line or a frame far less.
    fn glyph_like(self, image_height: u32) -> bool {
        let (w, h) = (self.width(), self.height());
        self.y1 - self.y0 >= MIN_GLYPH_PX
            && h <= f64::from(image_height) / 4.0
            && w <= 1.6 * h
            && f64::from(self.area) >= 0.08 * w * h
    }
}

/// The 8-connected components of `ink`, a row-major mask `width` wide, in the
/// raster order of their first pixel.
fn components(ink: &[bool], width: usize) -> Vec<Blob> {
    if width == 0 {
        return Vec::new();
    }
    let height = ink.len() / width;
    let mut seen = vec![false; ink.len()];
    let mut stack = Vec::new();
    let mut blobs = Vec::new();
    for (start, &is_ink) in ink.iter().enumerate() {
        if !is_ink || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let (mut x0, mut y0, mut x1, mut y1, mut area) = (usize::MAX, usize::MAX, 0, 0, 0u32);
        while let Some(i) = stack.pop() {
            let (x, y) = (i % width, i / width);
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x + 1);
            y1 = y1.max(y + 1);
            area += 1;
            for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                    let j = ny * width + nx;
                    if ink[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        blobs.push(Blob {
            x0: x0 as u32,
            y0: y0 as u32,
            x1: x1 as u32,
            y1: y1 as u32,
            area,
        });
    }
    blobs
}

/// Chains of blobs that step right one glyph at a time, as indices into
/// `blobs`, left to right. Each blob links to its nearest blob to the right
/// whose centre lies 0.55 to 1.7 heights away and within 0.35 heights
/// vertically, with heights within a factor of 1.6 (heights taken as the
/// larger of the two). A blob claimed by two links keeps the one from the
/// nearer blob.
fn chains(blobs: &[Blob]) -> Vec<Vec<usize>> {
    let mut order: Vec<usize> = (0..blobs.len()).collect();
    order.sort_by(|&a, &b| blobs[a].cx().total_cmp(&blobs[b].cx()).then(a.cmp(&b)));
    let mut next: Vec<Option<usize>> = vec![None; blobs.len()];
    for (k, &a) in order.iter().enumerate() {
        let ba = blobs[a];
        // The ratio bound caps the other blob's height at 1.6 times this one's.
        let reach = 1.7 * 1.6 * ba.height();
        let mut best: Option<(f64, f64, usize)> = None;
        for &b in &order[k + 1..] {
            let bb = blobs[b];
            let dx = bb.cx() - ba.cx();
            if dx > reach {
                break;
            }
            let h = ba.height().max(bb.height());
            let dy = (bb.cy() - ba.cy()).abs();
            let ratio = bb.height() / ba.height();
            if dx < 0.55 * h || dx > 1.7 * h || dy > 0.35 * h || !(1.0 / 1.6..=1.6).contains(&ratio)
            {
                continue;
            }
            if best.is_none_or(|(bdx, bdy, _)| (dx, dy) < (bdx, bdy)) {
                best = Some((dx, dy, b));
            }
        }
        next[a] = best.map(|(_, _, b)| b);
    }
    // `order` runs left to right, so a later claimant is the nearer one.
    let mut prev: Vec<Option<usize>> = vec![None; blobs.len()];
    for &a in &order {
        if let Some(b) = next[a] {
            prev[b] = Some(a);
        }
    }
    let mut out = Vec::new();
    for &a in &order {
        if prev[a].is_some() {
            continue;
        }
        let mut chain = vec![a];
        let mut cur = a;
        while let Some(b) = next[cur] {
            if prev[b] != Some(cur) {
                break;
            }
            chain.push(b);
            cur = b;
        }
        out.push(chain);
    }
    out
}

/// Whether every centre spacing of `run` lies within 25% of the run's median
/// spacing.
fn is_regular(run: &[usize], blobs: &[Blob]) -> bool {
    let gaps: Vec<f64> = run
        .windows(2)
        .map(|p| blobs[p[1]].cx() - blobs[p[0]].cx())
        .collect();
    median(&gaps).is_some_and(|pitch| gaps.iter().all(|g| (g / pitch - 1.0).abs() <= 0.25))
}

/// `chain` cut wherever a centre spacing strays more than 25% from the chain's
/// median spacing.
fn regular_runs(chain: &[usize], blobs: &[Blob]) -> Vec<Vec<usize>> {
    let gaps: Vec<f64> = chain
        .windows(2)
        .map(|p| blobs[p[1]].cx() - blobs[p[0]].cx())
        .collect();
    let Some(pitch) = median(&gaps) else {
        return vec![chain.to_vec()];
    };
    let mut runs = Vec::new();
    let mut cur = vec![chain[0]];
    for (i, g) in gaps.iter().enumerate() {
        if (g / pitch - 1.0).abs() > 0.25 {
            runs.push(std::mem::take(&mut cur));
        }
        cur.push(chain[i + 1]);
    }
    runs.push(cur);
    runs
}

/// One located MRZ line: a box per cell, left to right.
#[derive(Debug, Clone)]
struct Line {
    cells: Vec<Blob>,
    /// The Theil-Sen slope of the centres' x over the cell index: px per
    /// cell, to a fraction of a pixel.
    pitch: f64,
    /// The Theil-Sen slope of the centres, px down per px right.
    slope: f64,
}

impl Line {
    /// `None` for fewer than two cells.
    fn new(cells: Vec<Blob>) -> Option<Line> {
        let along: Vec<(f64, f64)> = cells
            .iter()
            .enumerate()
            .map(|(i, b)| (i as f64, b.cx()))
            .collect();
        let pitch = theil_sen(&along);
        let centres: Vec<(f64, f64)> = cells.iter().map(|b| (b.cx(), b.cy())).collect();
        (pitch > 0.0).then(|| Line {
            pitch,
            slope: theil_sen(&centres),
            cells,
        })
    }

    fn height(&self) -> f64 {
        let heights: Vec<f64> = self.cells.iter().map(|b| b.height()).collect();
        median(&heights).unwrap_or(0.0)
    }

    fn mid_y(&self) -> f64 {
        let ys: Vec<f64> = self.cells.iter().map(|b| b.cy()).collect();
        median(&ys).unwrap_or(0.0)
    }

    fn left(&self) -> f64 {
        self.cells.first().map_or(0.0, |b| b.cx())
    }

    fn right(&self) -> f64 {
        self.cells.last().map_or(0.0, |b| b.cx())
    }
}

/// The lowest stack of `count` candidate lines that sit together like one
/// zone (see the module doc), and how many stacks qualified.
fn zone(mut candidates: Vec<Line>, count: usize) -> Option<(Vec<Line>, usize)> {
    if count == 0 || candidates.len() < count {
        return None;
    }
    candidates.sort_by(|a, b| a.mid_y().total_cmp(&b.mid_y()));
    let fits = |u: &Line, v: &Line| {
        let pitch = u.pitch.max(v.pitch);
        let h = u.height().max(v.height());
        let gap = v.mid_y() - u.mid_y();
        (0.9..=1.0 / 0.9).contains(&(u.pitch / v.pitch))
            && (u.left() - v.left()).abs() <= 0.5 * pitch
            && (u.right() - v.right()).abs() <= 0.5 * pitch
            && (1.1 * h..=3.5 * h).contains(&gap)
    };
    let stacks: Vec<usize> = (0..=candidates.len() - count)
        .filter(|&s| (s..s + count - 1).all(|i| fits(&candidates[i], &candidates[i + 1])))
        .collect();
    let &lowest = stacks.last()?;
    Some((candidates[lowest..lowest + count].to_vec(), stacks.len()))
}

/// A zone found from ink alone.
#[derive(Debug, Clone)]
struct Located {
    lines: Vec<Line>,
    /// Which binarisation found it.
    binarisation: String,
    /// How many stacks qualified; the lowest was kept.
    stacks: usize,
}

/// Finds a zone of `count` lines of `width` cells (see the module doc), or
/// says how close the best attempt came.
fn locate(gray: &GrayImage, count: usize, width: usize) -> Result<Located, String> {
    let attempts = SAUVOLA_WINDOWS
        .iter()
        .map(|&w| (format!("sauvola{w}"), Some(w)))
        .chain(std::iter::once(("otsu".to_string(), None)));
    let mut most = 0;
    for (name, window) in attempts {
        let ink = match window {
            Some(w) => sauvola(gray, w),
            None => otsu_ink(gray),
        };
        let blobs: Vec<Blob> = components(&ink, gray.width() as usize)
            .into_iter()
            .filter(|b| b.glyph_like(gray.height()))
            .collect();
        let candidates: Vec<Line> = chains(&blobs)
            .iter()
            .flat_map(|c| regular_runs(c, &blobs))
            .filter(|run| run.len() == width && is_regular(run, &blobs))
            .filter_map(|run| Line::new(run.iter().map(|&i| blobs[i]).collect()))
            .collect();
        most = most.max(candidates.len());
        if let Some((lines, stacks)) = zone(candidates, count) {
            return Ok(Located {
                lines,
                binarisation: name,
                stacks,
            });
        }
    }
    Err(format!(
        "no zone: at best {most} regular line(s) of exactly {width} components"
    ))
}

// ---------------------------------------------------------------------
// Measuring a cell
// ---------------------------------------------------------------------

/// One cell's measurements. Edges are in the original image's rows, one per
/// entry of [`FRACTIONS`]. `bins` holds fractions of the cell's ink.
#[derive(Debug, Clone, PartialEq)]
struct CellInk {
    top: [f64; FRACTIONS.len()],
    bottom: [f64; FRACTIONS.len()],
    bins: [f64; BINS],
}

impl CellInk {
    fn left_quarter(&self) -> f64 {
        self.bins[..BINS / 4].iter().sum()
    }
}

/// A window `[x0, x1) x [y0, y1)` in continuous pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Window {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

/// Darkness per pixel of `img`, row-major, as the module doc defines it.
fn darkness(img: &GrayImage) -> Result<Vec<f64>, &'static str> {
    let t = otsu(img.pixels().map(|p| p[0])).ok_or("flat_band")?;
    let (mut light, mut dark): (Vec<u8>, Vec<u8>) =
        img.pixels().map(|p| p[0]).partition(|&g| g > t);
    if light.is_empty() || dark.is_empty() {
        return Err("flat_band");
    }
    light.sort_unstable();
    dark.sort_unstable();
    let paper = f64::from(light[light.len() / 2]);
    let ink = f64::from(dark[dark.len() / 2]);
    if paper - ink < MIN_CONTRAST {
        return Err("low_contrast");
    }
    let raw = |g: u8| ((paper - f64::from(g)) / (paper - ink)).clamp(0.0, 1.0);
    let texture: Vec<f64> = light.iter().map(|&g| raw(g)).collect();
    let floor = percentile(&texture, 90.0).unwrap_or(0.0);
    if floor >= 0.5 {
        return Err("textured_paper");
    }
    Ok(img
        .pixels()
        .map(|p| ((raw(p[0]) - floor) / (1.0 - floor)).clamp(0.0, 1.0))
        .collect())
}

/// One row of a cell's ink: the part of the pixel row the window covers, as
/// `(top, bottom)`, and the ink inside it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RowInk {
    top: f64,
    bottom: f64,
    ink: f64,
}

/// The ink of `dark` (row-major, `width` wide) inside `win`: one entry per
/// pixel row the window touches, top to bottom, and the ink in each of
/// [`BINS`] equal columns of the window. A pixel counts in proportion to how
/// much of it the window, or the bin, covers.
fn cell_profile(dark: &[f64], width: usize, win: Window) -> (Vec<RowInk>, [f64; BINS]) {
    let height = dark.len() / width.max(1);
    let x0 = win.x0.clamp(0.0, width as f64);
    let x1 = win.x1.clamp(0.0, width as f64);
    let y0 = win.y0.clamp(0.0, height as f64);
    let y1 = win.y1.clamp(0.0, height as f64);
    let mut rows = Vec::new();
    let mut bins = [0.0; BINS];
    if x1 <= x0 || y1 <= y0 {
        return (rows, bins);
    }
    let bin_width = (x1 - x0) / BINS as f64;
    for r in (y0.floor() as usize)..(y1.ceil() as usize) {
        let (top, bottom) = ((r as f64).max(y0), ((r + 1) as f64).min(y1));
        let wy = bottom - top;
        let mut ink = 0.0;
        for c in (x0.floor() as usize)..(x1.ceil() as usize) {
            let (a, b) = ((c as f64).max(x0), ((c + 1) as f64).min(x1));
            let d = dark[r * width + c] * wy;
            ink += d * (b - a);
            for (j, bin) in bins.iter_mut().enumerate() {
                let (e0, e1) = (x0 + j as f64 * bin_width, x0 + (j + 1) as f64 * bin_width);
                let overlap = b.min(e1) - a.max(e0);
                if overlap > 0.0 {
                    *bin += d * overlap;
                }
            }
        }
        rows.push(RowInk { top, bottom, ink });
    }
    (rows, bins)
}

/// The y at which ink accumulated from the top first reaches `target`,
/// interpolated linearly inside the row that crosses it.
fn edge_from_top(rows: &[RowInk], target: f64) -> Option<f64> {
    let mut acc = 0.0;
    for row in rows {
        if row.ink > 0.0 && acc + row.ink >= target {
            return Some(row.top + (target - acc) / row.ink * (row.bottom - row.top));
        }
        acc += row.ink;
    }
    None
}

/// The same edge counted from the bottom.
fn edge_from_bottom(rows: &[RowInk], target: f64) -> Option<f64> {
    let mut acc = 0.0;
    for row in rows.iter().rev() {
        if row.ink > 0.0 && acc + row.ink >= target {
            return Some(row.bottom - (target - acc) / row.ink * (row.bottom - row.top));
        }
        acc += row.ink;
    }
    None
}

/// Measures one line's cell `boxes` (found at native resolution) on `gray`,
/// resampled by `scale` (1.0 is native; below 1.0 the band is shrunk with
/// `Triangle`). Edges come back in native rows. The band is cut with a margin
/// of one pitch, so the resampling filter sees the same surroundings at every
/// cell.
fn measure_cells(
    gray: &GrayImage,
    boxes: &[Blob],
    pitch: f64,
    scale: f64,
) -> Result<Vec<CellInk>, &'static str> {
    let pad = 0.25 * pitch;
    let lo = |f: fn(&Blob) -> u32| boxes.iter().map(f).min().map(f64::from);
    let hi = |f: fn(&Blob) -> u32| boxes.iter().map(f).max().map(f64::from);
    let (Some(bx0), Some(by0), Some(bx1), Some(by1)) =
        (lo(|b| b.x0), lo(|b| b.y0), hi(|b| b.x1), hi(|b| b.y1))
    else {
        return Err("no_cells");
    };
    let cx0 = (bx0 - pitch).floor().max(0.0) as u32;
    let cy0 = (by0 - pad - pitch).floor().max(0.0) as u32;
    let cx1 = ((bx1 + pitch).ceil() as u32).min(gray.width());
    let cy1 = ((by1 + pad + pitch).ceil() as u32).min(gray.height());
    if cx1 <= cx0 || cy1 <= cy0 {
        return Err("no_cells");
    }
    let crop = image::imageops::crop_imm(gray, cx0, cy0, cx1 - cx0, cy1 - cy0).to_image();
    let (band, sx, sy) = if scale < 1.0 {
        let nw = ((f64::from(crop.width()) * scale).round() as u32).max(1);
        let nh = ((f64::from(crop.height()) * scale).round() as u32).max(1);
        let resized = image::imageops::resize(&crop, nw, nh, FilterType::Triangle);
        let sx = f64::from(nw) / f64::from(crop.width());
        let sy = f64::from(nh) / f64::from(crop.height());
        (resized, sx, sy)
    } else {
        (crop, 1.0, 1.0)
    };
    let dark = darkness(&band)?;
    let (ox, oy) = (f64::from(cx0), f64::from(cy0));
    boxes
        .iter()
        .map(|b| {
            let win = Window {
                x0: (f64::from(b.x0) - ox) * sx,
                x1: (f64::from(b.x1) - ox) * sx,
                y0: (f64::from(b.y0) - pad - oy) * sy,
                y1: (f64::from(b.y1) + pad - oy) * sy,
            };
            let (rows, bins) = cell_profile(&dark, band.width() as usize, win);
            let total: f64 = rows.iter().map(|r| r.ink).sum();
            if total <= 1e-9 {
                return Err("empty_cell");
            }
            let mut top = [0.0; FRACTIONS.len()];
            let mut bottom = [0.0; FRACTIONS.len()];
            for (k, f) in FRACTIONS.iter().enumerate() {
                top[k] = edge_from_top(&rows, f * total).ok_or("no_edge")? / sy + oy;
                bottom[k] = edge_from_bottom(&rows, f * total).ok_or("no_edge")? / sy + oy;
            }
            Ok(CellInk {
                top,
                bottom,
                bins: bins.map(|v| v / total),
            })
        })
        .collect()
}

/// Each cell's rise at fraction index `k`: its top edge measured upward against
/// the line's skew, in cells of the line's pitch. Only differences within one
/// line are meaningful.
fn rises(line: &Line, cells: &[CellInk], k: usize) -> Vec<f64> {
    line.cells
        .iter()
        .zip(cells)
        .map(|(b, c)| (line.slope * b.cx() - c.top[k]) / line.pitch)
        .collect()
}

// ---------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------

/// The median, averaging the two middle values of an even count.
fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    Some(if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    })
}

/// The `p`-th percentile (0-100) of `values` by nearest rank.
fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted.get(rank.min(sorted.len() - 1)).copied()
}

/// The Theil-Sen slope of `points`: the median of the slopes between every
/// pair with distinct x. 0 when no pair has.
fn theil_sen(points: &[(f64, f64)]) -> f64 {
    let mut slopes = Vec::new();
    for (i, &(xa, ya)) in points.iter().enumerate() {
        for &(xb, yb) in &points[i + 1..] {
            if xb != xa {
                slopes.push((yb - ya) / (xb - xa));
            }
        }
    }
    median(&slopes).unwrap_or(0.0)
}

/// How far a class expected to measure higher stands above one expected to
/// measure lower.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Separation {
    gap: f64,
    margin: f64,
    auc: f64,
}

/// `high` against `low`: the gap of medians, the margin (lowest `high` minus
/// highest `low`) and the AUC with ties counting half. `None` unless each side
/// has at least two values.
fn separation(high: &[f64], low: &[f64]) -> Option<Separation> {
    if high.len() < 2 || low.len() < 2 {
        return None;
    }
    let min_high = high.iter().copied().fold(f64::INFINITY, f64::min);
    let max_low = low.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut wins = 0.0;
    for &h in high {
        for &l in low {
            wins += if h > l {
                1.0
            } else if h == l {
                0.5
            } else {
                0.0
            };
        }
    }
    Some(Separation {
        gap: median(high)? - median(low)?,
        margin: min_high - max_low,
        auc: wins / (high.len() * low.len()) as f64,
    })
}

/// The finest step reached from native with `ok` holding at every step on the
/// way. `by_step` runs from native (first) towards fewer px per cell. `None`
/// when `ok` fails at native.
fn floor(by_step: &[(f64, bool)]) -> Option<f64> {
    let mut reached = None;
    for &(step, ok) in by_step {
        if !ok {
            break;
        }
        reached = Some(step);
    }
    reached
}

// ---------------------------------------------------------------------
// One run
// ---------------------------------------------------------------------

/// One line's cells at one step, or why its band could not be measured there.
type Measured = Result<Vec<CellInk>, &'static str>;

/// One located line measured at native resolution and at every step below
/// its pitch.
struct LineRecord {
    doc: usize,
    line: usize,
    geometry: Line,
    /// `None` is native; otherwise px per cell. Native comes first.
    steps: Vec<(Option<f64>, Measured)>,
}

impl LineRecord {
    fn at(&self, step: Option<f64>) -> Option<&Measured> {
        self.steps.iter().find(|(s, _)| *s == step).map(|(_, r)| r)
    }
}

fn measure_line(doc: usize, line: usize, gray: &GrayImage, geometry: &Line) -> LineRecord {
    let mut steps = vec![(
        None,
        measure_cells(gray, &geometry.cells, geometry.pitch, 1.0),
    )];
    for &step in STEPS.iter().filter(|&&s| s < geometry.pitch) {
        let scale = step / geometry.pitch;
        steps.push((
            Some(step),
            measure_cells(gray, &geometry.cells, geometry.pitch, scale),
        ));
    }
    LineRecord {
        doc,
        line,
        geometry: geometry.clone(),
        steps,
    }
}

/// Whether a line's truth gives item 1 something to compare: at least two
/// digits and two letters.
fn qualifies(classes: &[Class]) -> bool {
    classes.iter().filter(|&&c| c == Class::Digit).count() >= 2
        && classes.iter().filter(|c| c.is_letter()).count() >= 2
}

/// Item 1 on one line at one step: digit rises against letter rises.
fn line_separation(
    rec: &LineRecord,
    classes: &[Class],
    cells: &[CellInk],
    k: usize,
) -> Option<Separation> {
    let rise = rises(&rec.geometry, cells, k);
    let pick = |want: fn(Class) -> bool| -> Vec<f64> {
        classes
            .iter()
            .zip(&rise)
            .filter(|(c, _)| want(**c))
            .map(|(_, r)| *r)
            .collect()
    };
    separation(&pick(|c| c == Class::Digit), &pick(Class::is_letter))
}

fn fmt(v: Option<f64>) -> String {
    v.map_or("-".to_string(), |v| format!("{v:.3}"))
}

fn step_name(step: Option<f64>) -> String {
    step.map_or("native".to_string(), |s| format!("{s}"))
}

/// The stdout report for item 1.
fn report_item1(specimens: &[Specimen], records: &[LineRecord]) -> String {
    let mut out = String::new();
    let line_name = |r: &LineRecord| format!("{} line {}", specimens[r.doc].name, r.line + 1);
    let qualifying: Vec<&LineRecord> = records
        .iter()
        .filter(|r| qualifies(&specimens[r.doc].zone[r.line]))
        .collect();
    let separation_at = |rec: &LineRecord, step: Option<f64>, k: usize| {
        let classes = &specimens[rec.doc].zone[rec.line];
        rec.at(step).map(|result| {
            result
                .as_ref()
                .ok()
                .and_then(|cells| line_separation(rec, classes, cells, k))
        })
    };
    out.push_str("\n## Item 1: top edge, digit against letter, within one line\n\n");
    out.push_str(&format!(
        "{} lines have at least two digits and two letters. Rises are in cells of the line's own pitch; \
         a line is refused at a step when its band cannot be measured there.\n\n",
        qualifying.len()
    ));
    out.push_str("| f | Step (px per cell) | Lines | Refused | Margin > 0 | Median gap | Lowest margin | Median AUC | Lowest AUC |\n");
    out.push_str("| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    let all_steps: Vec<Option<f64>> = std::iter::once(None)
        .chain(STEPS.iter().map(|&s| Some(s)))
        .collect();
    for (k, f) in FRACTIONS.iter().enumerate() {
        for &step in &all_steps {
            let measured: Vec<Option<Separation>> = qualifying
                .iter()
                .filter_map(|rec| separation_at(rec, step, k))
                .collect();
            if measured.is_empty() {
                continue;
            }
            let seps: Vec<Separation> = measured.iter().flatten().copied().collect();
            let gaps: Vec<f64> = seps.iter().map(|s| s.gap).collect();
            let margins: Vec<f64> = seps.iter().map(|s| s.margin).collect();
            let aucs: Vec<f64> = seps.iter().map(|s| s.auc).collect();
            out.push_str(&format!(
                "| {f} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                step_name(step),
                seps.len(),
                measured.len() - seps.len(),
                seps.iter().filter(|s| s.margin > 0.0).count(),
                fmt(median(&gaps)),
                fmt(margins.iter().copied().reduce(f64::min)),
                fmt(median(&aucs)),
                fmt(aucs.iter().copied().reduce(f64::min)),
            ));
        }
    }
    out.push_str("\nFloors: the finest step each line reaches from native with its margin above zero at every step.\n\n");
    out.push_str(
        "| f | Lines | Margin > 0 at native | Median floor | Worst floor | Worst line |\n",
    );
    out.push_str("| ---: | ---: | ---: | ---: | ---: | --- |\n");
    for (k, f) in FRACTIONS.iter().enumerate() {
        let mut floors: Vec<(f64, String)> = Vec::new();
        for rec in &qualifying {
            let by_step: Vec<(f64, bool)> = rec
                .steps
                .iter()
                .map(|(step, _)| {
                    let ok = separation_at(rec, *step, k)
                        .flatten()
                        .is_some_and(|s| s.margin > 0.0);
                    (step.unwrap_or(rec.geometry.pitch), ok)
                })
                .collect();
            if let Some(px) = floor(&by_step) {
                floors.push((px, line_name(rec)));
            }
        }
        let values: Vec<f64> = floors.iter().map(|(px, _)| *px).collect();
        let worst = floors.iter().max_by(|a, b| a.0.total_cmp(&b.0));
        out.push_str(&format!(
            "| {f} | {} | {} | {} | {} | {} |\n",
            qualifying.len(),
            floors.len(),
            fmt(median(&values)),
            fmt(worst.map(|w| w.0)),
            worst.map_or("-".to_string(), |w| w.1.clone()),
        ));
    }
    out.push_str("\nLines whose margin is not above zero at native for some f (margin at each f; `*` marks the ones not above zero):\n\n");
    out.push_str("| Line | px per cell | f 0.02 | f 0.05 | f 0.1 | f 0.2 |\n");
    out.push_str("| --- | ---: | ---: | ---: | ---: | ---: |\n");
    for rec in &qualifying {
        let margins: Vec<Option<f64>> = (0..FRACTIONS.len())
            .map(|k| separation_at(rec, None, k).flatten().map(|s| s.margin))
            .collect();
        if margins.iter().all(|m| m.is_some_and(|m| m > 0.0)) {
            continue;
        }
        let cells: Vec<String> = margins
            .iter()
            .map(|m| match m {
                Some(m) if *m > 0.0 => format!("{m:.3}"),
                Some(m) => format!("{m:.3}*"),
                None => "refused".to_string(),
            })
            .collect();
        out.push_str(&format!(
            "| {} | {:.1} | {} |\n",
            line_name(rec),
            rec.geometry.pitch,
            cells.join(" | ")
        ));
    }
    out
}

/// The stdout report for item 2: left-quarter ink of `K` against the filler,
/// pooled over every line at each step.
fn report_item2(specimens: &[Specimen], records: &[LineRecord]) -> String {
    let mut out = String::new();
    out.push_str("\n## Item 2: left-quarter ink, K against filler, pooled over lines\n\n");
    out.push_str("| Step (px per cell) | K cells | K min / median / max | Filler cells | Filler min / median / max | Margin | AUC |\n");
    out.push_str("| --- | ---: | --- | ---: | --- | ---: | ---: |\n");
    let mut by_step = Vec::new();
    for step in std::iter::once(None).chain(STEPS.iter().map(|&s| Some(s))) {
        let (mut k_cells, mut fillers) = (Vec::new(), Vec::new());
        for rec in records {
            let Some(Ok(cells)) = rec.at(step) else {
                continue;
            };
            for (class, cell) in specimens[rec.doc].zone[rec.line].iter().zip(cells) {
                match class {
                    Class::K => k_cells.push(cell.left_quarter()),
                    Class::Filler => fillers.push(cell.left_quarter()),
                    _ => {}
                }
            }
        }
        if k_cells.is_empty() && fillers.is_empty() {
            continue;
        }
        let spread = |v: &[f64]| {
            format!(
                "{} / {} / {}",
                fmt(v.iter().copied().reduce(f64::min)),
                fmt(median(v)),
                fmt(v.iter().copied().reduce(f64::max))
            )
        };
        let sep = separation(&k_cells, &fillers);
        by_step.push((step, sep.is_some_and(|s| s.margin > 0.0)));
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            step_name(step),
            k_cells.len(),
            spread(&k_cells),
            fillers.len(),
            spread(&fillers),
            fmt(sep.map(|s| s.margin)),
            fmt(sep.map(|s| s.auc)),
        ));
    }
    let reached = by_step
        .iter()
        .take_while(|(_, ok)| *ok)
        .last()
        .map(|(step, _)| step_name(*step));
    out.push_str(&format!(
        "\nPooled floor (the finest step reached from native with the pooled margin above zero): {}\n",
        reached.unwrap_or_else(|| "none: the margin is not above zero at native".to_string())
    ));
    out
}

/// The cells of one line at one step, as JSONL rows: classes and geometry
/// only.
fn cell_rows(
    spec: &Specimen,
    rec: &LineRecord,
    step: Option<f64>,
    cells: &[CellInk],
) -> Vec<Value> {
    let rise: Vec<Vec<f64>> = (0..FRACTIONS.len())
        .map(|k| rises(&rec.geometry, cells, k))
        .collect();
    cells
        .iter()
        .enumerate()
        .map(|(i, c)| {
            json!({
                "document": spec.name,
                "shape": spec.shape(),
                "line": rec.line + 1,
                "column": i,
                "class": spec.zone[rec.line][i].name(),
                "step_px_per_cell": step,
                "pitch_px": rec.geometry.pitch,
                "top": c.top,
                "bottom": c.bottom,
                "rise": rise.iter().map(|r| r[i]).collect::<Vec<_>>(),
                "bins": c.bins,
                "left_quarter": c.left_quarter(),
            })
        })
        .collect()
}

/// Draws the zone of one document, its cell boxes coloured by class and the
/// 5% top edge in red, magnified so a cell is at least 24 px wide.
fn write_overlay(
    rgb: &RgbImage,
    spec: &Specimen,
    records: &[&LineRecord],
    path: &Path,
) -> Result<(), String> {
    let boxes = || records.iter().flat_map(|r| r.geometry.cells.iter());
    let pitch = records.first().map_or(1.0, |r| r.geometry.pitch);
    let margin = pitch.ceil() as u32;
    let x0 = boxes()
        .map(|b| b.x0)
        .min()
        .unwrap_or(0)
        .saturating_sub(margin);
    let y0 = boxes()
        .map(|b| b.y0)
        .min()
        .unwrap_or(0)
        .saturating_sub(margin);
    let x1 = (boxes().map(|b| b.x1).max().unwrap_or(0) + margin).min(rgb.width());
    let y1 = (boxes().map(|b| b.y1).max().unwrap_or(0) + margin).min(rgb.height());
    if x1 <= x0 || y1 <= y0 {
        return Ok(());
    }
    let k = ((24.0 / pitch).ceil() as u32).clamp(1, 4);
    let crop = image::imageops::crop_imm(rgb, x0, y0, x1 - x0, y1 - y0).to_image();
    let mut img = image::imageops::resize(
        &crop,
        crop.width() * k,
        crop.height() * k,
        FilterType::Nearest,
    );
    let (w, h) = img.dimensions();
    let mut put = |x: u32, y: u32, c: Rgb<u8>| {
        if x < w && y < h {
            img.put_pixel(x, y, c);
        }
    };
    for rec in records {
        let Some(Ok(cells)) = rec.at(None) else {
            continue;
        };
        for ((b, cell), class) in rec
            .geometry
            .cells
            .iter()
            .zip(cells)
            .zip(&spec.zone[rec.line])
        {
            let colour = match class {
                Class::Digit => Rgb([0, 90, 255]),
                Class::Letter => Rgb([0, 160, 0]),
                Class::K => Rgb([200, 0, 200]),
                Class::Filler => Rgb([255, 140, 0]),
            };
            let (bx0, by0) = ((b.x0 - x0) * k, (b.y0 - y0) * k);
            let (bx1, by1) = ((b.x1 - x0) * k, (b.y1 - y0) * k);
            for x in bx0..bx1 {
                put(x, by0, colour);
                put(x, by1.saturating_sub(1), colour);
            }
            for y in by0..by1 {
                put(bx0, y, colour);
                put(bx1.saturating_sub(1), y, colour);
            }
            let top = ((cell.top[OVERLAY_FRACTION] - f64::from(y0)) * f64::from(k)).round();
            if top >= 0.0 {
                for x in bx0..bx1 {
                    put(x, top as u32, Rgb([230, 0, 0]));
                }
            }
        }
    }
    img.save(path)
        .map_err(|e| format!("{}: {e}", path.display()))
}

// ---------------------------------------------------------------------
// Arguments, paths and the run
// ---------------------------------------------------------------------

#[derive(Debug, Default, PartialEq)]
struct Args {
    samples_root: Option<PathBuf>,
    out: Option<PathBuf>,
    overlays: bool,
    locate_only: bool,
    only: Option<String>,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut parsed = Args::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match arg.as_str() {
            "--samples-root" => parsed.samples_root = Some(PathBuf::from(value(arg)?)),
            "--out" => parsed.out = Some(PathBuf::from(value(arg)?)),
            "--only" => parsed.only = Some(value(arg)?),
            "--overlays" => parsed.overlays = true,
            "--locate-only" => parsed.locate_only = true,
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(parsed)
}

fn repo_root() -> PathBuf {
    absolutize(&Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(".."))
}

/// `path` made absolute against the current directory, with `.` and `..`
/// resolved lexically.
fn absolutize(path: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    let mut out = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Refuses an `out` inside `root` unless `is_ignored` says git ignores a file
/// in it: the cell records must never reach a commit by accident. A directory
/// outside the repository is always fine.
fn check_out_dir(
    out: &Path,
    root: &Path,
    is_ignored: impl Fn(&Path) -> bool,
) -> Result<(), String> {
    if !out.starts_with(root) || is_ignored(&out.join("cells.jsonl")) {
        return Ok(());
    }
    Err(format!(
        "refusing --out {}: it is inside the repository and git does not ignore it; use a \
         directory under artifacts/ or outside the tree",
        out.display()
    ))
}

fn git_ignores(root: &Path, path: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["check-ignore", "-q", "--"])
        .arg(path)
        .status()
        .is_ok_and(|status| status.success())
}

fn git_head(root: &Path) -> String {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map_or_else(
            || "unknown".to_string(),
            |o| String::from_utf8_lossy(&o.stdout).trim().to_string(),
        )
}

/// Reads, checks and locates one document. The error is a reason key and a
/// detail.
fn open_and_locate(
    spec: &Specimen,
) -> Result<(RgbImage, GrayImage, Located), (&'static str, String)> {
    let bytes = fs::read(&spec.image).map_err(|e| ("image_missing", e.to_string()))?;
    let sha = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if sha != spec.sha256 {
        return Err((
            "sha_mismatch",
            format!("{sha} is not the corpus's {}", spec.sha256),
        ));
    }
    let rgb = synthpass_ocr::decode_image(&spec.image)
        .map_err(|e| ("decode_failed", e))?
        .into_rgb8();
    let gray = grey_of(&rgb);
    let located = locate(&gray, spec.zone.len(), spec.zone[0].len()).map_err(|e| ("no_zone", e))?;
    Ok((rgb, gray, located))
}

fn run(args: &Args) -> Result<(), String> {
    let root = repo_root();
    let samples_root = absolutize(
        args.samples_root
            .as_deref()
            .unwrap_or(&root.join("samples")),
    );
    let head = git_head(&root);
    let out = absolutize(
        &args
            .out
            .clone()
            .unwrap_or_else(|| root.join(DEFAULT_OUT).join(&head)),
    );
    let writes = !args.locate_only;
    if writes {
        check_out_dir(&out, &root, |p| git_ignores(&root, p))?;
        if out.join("cells.jsonl").exists() {
            return Err(format!(
                "{} already exists; refusing to overwrite a run",
                out.join("cells.jsonl").display()
            ));
        }
        fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
        if args.overlays {
            fs::create_dir_all(out.join("overlays"))
                .map_err(|e| format!("{}: {e}", out.display()))?;
        }
    }
    let (specimens, refused) = population(&root, &samples_root, args.only.as_deref())?;
    println!("# Elimination probe\n");
    println!(
        "MAIN `{head}`. Samples root `{}`. {} documents with a reviewed fixture; {} refused for their fixture.",
        samples_root.display(),
        specimens.len(),
        refused.len()
    );
    println!(
        "Fractions {FRACTIONS:?}; steps {STEPS:?} px per cell; {BINS} profile columns; grey = max(R, G, B).\n"
    );
    println!("| Document | Shape | Outcome | Binarisation | Stacks | Pitch, px per cell |");
    println!("| --- | --- | --- | --- | ---: | --- |");
    for (name, reason) in &refused {
        println!("| {name} | - | {reason} | - | - | - |");
    }
    let mut records = Vec::new();
    let mut accepted = 0;
    let mut reasons: Vec<(&'static str, usize)> = Vec::new();
    for (d, spec) in specimens.iter().enumerate() {
        match open_and_locate(spec) {
            Err((reason, detail)) => {
                println!(
                    "| {} | {} | {reason}: {detail} | - | - | - |",
                    spec.name,
                    spec.shape()
                );
                match reasons.iter_mut().find(|(r, _)| *r == reason) {
                    Some((_, n)) => *n += 1,
                    None => reasons.push((reason, 1)),
                }
            }
            Ok((rgb, gray, located)) => {
                accepted += 1;
                let pitches: Vec<String> = located
                    .lines
                    .iter()
                    .map(|l| format!("{:.2}", l.pitch))
                    .collect();
                println!(
                    "| {} | {} | located | {} | {} | {} |",
                    spec.name,
                    spec.shape(),
                    located.binarisation,
                    located.stacks,
                    pitches.join(", ")
                );
                if args.locate_only {
                    continue;
                }
                let first = records.len();
                for (li, line) in located.lines.iter().enumerate() {
                    records.push(measure_line(d, li, &gray, line));
                }
                if args.overlays {
                    let file = Path::new(&spec.name)
                        .file_stem()
                        .map_or_else(|| format!("doc{d}"), |s| s.to_string_lossy().into_owned());
                    let mine: Vec<&LineRecord> = records[first..].iter().collect();
                    write_overlay(
                        &rgb,
                        spec,
                        &mine,
                        &out.join("overlays").join(format!("{file}.png")),
                    )?;
                }
            }
        }
    }
    let refusals: Vec<String> = reasons.iter().map(|(r, n)| format!("{r} {n}")).collect();
    println!(
        "\nLocated {accepted} of {} documents. Not located: {}.",
        specimens.len(),
        if refusals.is_empty() {
            "none".to_string()
        } else {
            refusals.join(", ")
        }
    );
    if !writes {
        return Ok(());
    }
    print!("{}", report_item1(&specimens, &records));
    print!("{}", report_item2(&specimens, &records));
    let path = out.join("cells.jsonl");
    let mut file = fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let header = json!({"header": {
        "tool": "elimination_probe",
        "main": head,
        "fractions": FRACTIONS,
        "steps_px_per_cell": STEPS,
        "bins": BINS,
        "grey": "max(R, G, B)",
        "documents": specimens.len(),
        "located": accepted,
    }});
    writeln!(file, "{header}").map_err(|e| e.to_string())?;
    for rec in &records {
        for (step, result) in &rec.steps {
            let Ok(cells) = result else { continue };
            for row in cell_rows(&specimens[rec.doc], rec, *step, cells) {
                writeln!(file, "{row}").map_err(|e| e.to_string())?;
            }
        }
    }
    println!("\nCells: {}", path.display());
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = parse_args(&args).and_then(|a| run(&a)) {
        eprintln!("elimination_probe: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A white image with each `(x0, y0, x1, y1)` filled black.
    fn boxes_image(width: u32, height: u32, rects: &[(u32, u32, u32, u32)]) -> GrayImage {
        let mut img = GrayImage::from_pixel(width, height, Luma([255]));
        for &(x0, y0, x1, y1) in rects {
            for y in y0..y1 {
                for x in x0..x1 {
                    img.put_pixel(x, y, Luma([0]));
                }
            }
        }
        img
    }

    /// Two lines of 44 boxes 10 px wide at a 16 px pitch. A box is tall (rows
    /// `top - 2 .. top + 20`) where `tall(line, cell)` and short (rows `top ..
    /// top + 20`) otherwise; line 2 sits 50 px below line 1. Every `skew`
    /// cells a box drops one more row.
    fn zone_rects(
        tall: impl Fn(usize, usize) -> bool,
        skew: Option<usize>,
    ) -> Vec<(u32, u32, u32, u32)> {
        let mut rects = Vec::new();
        for line in 0..2 {
            for cell in 0..44 {
                let x0 = 20 + 16 * cell as u32;
                let drop = skew.map_or(0, |s| (cell / s) as u32);
                let base = 22 + 50 * line as u32 + drop;
                let top = if tall(line, cell) { base - 2 } else { base };
                rects.push((x0, top, x0 + 10, base + 20));
            }
        }
        rects
    }

    #[test]
    fn otsu_splits_a_two_level_image_between_its_levels() {
        let pixels = [10u8, 10, 10, 200, 200, 200, 200];
        let t = otsu(pixels.iter().copied()).expect("two levels");
        assert!((10..200).contains(&t), "{t}");
        assert_eq!(otsu([7u8, 7, 7].iter().copied()), None);
    }

    #[test]
    fn diagonal_pixels_are_one_component_and_separate_boxes_are_two() {
        // A diagonal of four pixels, and a 2x2 box apart from it.
        let width = 8;
        let mut ink = vec![false; width * 4];
        for i in 0..4 {
            ink[i * width + i] = true;
        }
        for (x, y) in [(6, 0), (7, 0), (6, 1), (7, 1)] {
            ink[y * width + x] = true;
        }
        let blobs = components(&ink, width);
        assert_eq!(blobs.len(), 2);
        assert_eq!(
            (
                blobs[0].x0,
                blobs[0].y0,
                blobs[0].x1,
                blobs[0].y1,
                blobs[0].area
            ),
            (0, 0, 4, 4, 4)
        );
        assert_eq!(
            (
                blobs[1].x0,
                blobs[1].y0,
                blobs[1].x1,
                blobs[1].y1,
                blobs[1].area
            ),
            (6, 0, 8, 2, 4)
        );
    }

    #[test]
    fn a_zone_of_regular_boxes_is_found_and_two_touching_glyphs_lose_it() {
        let rects = zone_rects(|_, _| false, None);
        let img = boxes_image(760, 120, &rects);
        let located = locate(&img, 2, 44).expect("a clean zone is found");
        assert_eq!(located.lines.len(), 2);
        assert_eq!(located.stacks, 1);
        for (line, found) in located.lines.iter().enumerate() {
            assert_eq!(found.cells.len(), 44);
            assert!((found.pitch - 16.0).abs() < 1e-9, "{}", found.pitch);
            for (cell, b) in found.cells.iter().enumerate() {
                let r = rects[line * 44 + cell];
                assert_eq!(
                    (b.x0, b.y0, b.x1, b.y1),
                    (r.0, r.1, r.2, r.3),
                    "line {line} cell {cell}"
                );
            }
        }
        // Bridge cells 10 and 11 of line 2: one component spans two cells.
        let mut bridged = rects.clone();
        bridged.push((rects[44 + 10].2, 82, rects[44 + 11].0, 84));
        let error = locate(&boxes_image(760, 120, &bridged), 2, 44).expect_err("no zone");
        assert!(error.contains("no zone"), "{error}");
        // A missing glyph, and a mark off the grid in its gap: 44 components
        // again with both ends in place, but the spacing gives the mark away.
        let mut strayed = rects.clone();
        strayed.remove(44 + 20);
        strayed.push((335, 72, 345, 92));
        let error = locate(&boxes_image(760, 120, &strayed), 2, 44).expect_err("no zone");
        assert!(error.contains("no zone"), "{error}");
        // Line 2 one cell to the right: both lines are regular, but their
        // ends do not align.
        let shifted: Vec<(u32, u32, u32, u32)> = rects
            .iter()
            .enumerate()
            .map(|(i, &(x0, y0, x1, y1))| {
                if i < 44 {
                    (x0, y0, x1, y1)
                } else {
                    (x0 + 16, y0, x1 + 16, y1)
                }
            })
            .collect();
        let error = locate(&boxes_image(760, 120, &shifted), 2, 44).expect_err("no zone");
        assert!(error.contains("no zone"), "{error}");
    }

    #[test]
    fn the_lowest_of_two_qualifying_stacks_is_kept() {
        // Three regular lines 50 px apart: lines 1-2 and lines 2-3 both
        // qualify as a two-line zone.
        let mut rects = zone_rects(|_, _| false, None);
        rects.extend((0..44).map(|cell| {
            let x0 = 20 + 16 * cell;
            (x0, 122, x0 + 10, 142)
        }));
        let located = locate(&boxes_image(760, 170, &rects), 2, 44).expect("zone");
        assert_eq!(located.stacks, 2);
        assert_eq!(located.lines[0].cells[0].y0, 72);
        assert_eq!(located.lines[1].cells[0].y0, 122);
    }

    #[test]
    fn a_skewed_line_is_still_chained_and_its_slope_is_measured() {
        // One row lower every 4 cells of 16 px: a slope of 1/64, and 10 rows
        // over the line against the 2 rows a tall box stands above a short one.
        let tall = |_: usize, cell: usize| !cell.is_multiple_of(3);
        let img = boxes_image(760, 140, &zone_rects(tall, Some(4)));
        let located = locate(&img, 2, 44).expect("a skewed zone is found");
        for line in &located.lines {
            assert!((line.slope - 1.0 / 64.0).abs() < 0.003, "{}", line.slope);
        }
        // Measured against the skew, every tall box still rises above every
        // short one.
        let line = &located.lines[1];
        let classes: Vec<Class> = (0..44)
            .map(|c| {
                if tall(1, c) {
                    Class::Digit
                } else {
                    Class::Letter
                }
            })
            .collect();
        let cells = measure_cells(&img, &line.cells, line.pitch, 1.0).expect("measured");
        let rec = LineRecord {
            doc: 0,
            line: 1,
            geometry: line.clone(),
            steps: vec![],
        };
        let sep = line_separation(&rec, &classes, &cells, 1).expect("both classes");
        assert!(sep.margin > 0.0 && sep.auc == 1.0, "{sep:?}");
    }

    #[test]
    fn edges_interpolate_inside_the_crossing_row() {
        let rows = [
            RowInk {
                top: 0.0,
                bottom: 1.0,
                ink: 0.0,
            },
            RowInk {
                top: 1.0,
                bottom: 2.0,
                ink: 4.0,
            },
            RowInk {
                top: 2.0,
                bottom: 2.5,
                ink: 4.0,
            },
        ];
        assert_eq!(edge_from_top(&rows, 2.0), Some(1.5));
        assert_eq!(edge_from_top(&rows, 4.0), Some(2.0));
        assert_eq!(edge_from_top(&rows, 6.0), Some(2.25));
        assert_eq!(edge_from_bottom(&rows, 2.0), Some(2.25));
        assert_eq!(edge_from_bottom(&rows, 6.0), Some(1.5));
        assert_eq!(edge_from_top(&rows, 9.0), None);
    }

    #[test]
    fn cell_profile_weights_partially_covered_pixels() {
        let dark = vec![1.0; 10 * 10];
        let win = Window {
            x0: 1.5,
            y0: 2.25,
            x1: 3.5,
            y1: 4.0,
        };
        let (rows, bins) = cell_profile(&dark, 10, win);
        let total: f64 = rows.iter().map(|r| r.ink).sum();
        assert!((total - 2.0 * 1.75).abs() < 1e-12, "{total}");
        assert_eq!(rows.first().map(|r| (r.top, r.bottom)), Some((2.25, 3.0)));
        for bin in bins {
            assert!((bin - total / BINS as f64).abs() < 1e-12, "{bin}");
        }
        // A window off the image measures nothing.
        let (rows, _) = cell_profile(
            &dark,
            10,
            Window {
                x0: 12.0,
                y0: 0.0,
                x1: 14.0,
                y1: 3.0,
            },
        );
        assert!(rows.is_empty());
    }

    // Tall boxes play digits, short ones letters. On solid boxes every row of
    // a box holds the same ink, so the edge at fraction f is exact.
    #[test]
    fn solid_boxes_measure_their_known_edges_and_keep_their_difference_resampled() {
        let tall = |_: usize, cell: usize| !cell.is_multiple_of(3);
        let img = boxes_image(760, 120, &zone_rects(tall, None));
        let located = locate(&img, 2, 44).expect("zone");
        let line = &located.lines[1];
        let classes: Vec<Class> = (0..44)
            .map(|c| {
                if tall(1, c) {
                    Class::Digit
                } else {
                    Class::Letter
                }
            })
            .collect();
        let native = measure_cells(&img, &line.cells, line.pitch, 1.0).expect("native");
        for (b, cell) in line.cells.iter().zip(&native) {
            let (top, bottom) = (f64::from(b.y0), f64::from(b.y1));
            for (k, f) in FRACTIONS.iter().enumerate() {
                assert!((cell.top[k] - (top + f * (bottom - top))).abs() < 1e-9);
                assert!((cell.bottom[k] - (bottom - f * (bottom - top))).abs() < 1e-9);
            }
        }
        let rec = LineRecord {
            doc: 0,
            line: 1,
            geometry: line.clone(),
            steps: vec![],
        };
        for scale in [1.0, 0.5] {
            let cells = measure_cells(&img, &line.cells, line.pitch, scale).expect("measured");
            for (k, f) in FRACTIONS.iter().enumerate() {
                let sep = line_separation(&rec, &classes, &cells, k).expect("both classes");
                // Tall tops sit 2 rows higher over 22 rows against 20.
                let want = (2.0 - 2.0 * f) / line.pitch;
                assert!(
                    (sep.gap - want).abs() < 0.35 / line.pitch,
                    "scale {scale} f {f}: {} vs {want}",
                    sep.gap
                );
                assert!(
                    sep.margin > 0.0 && sep.auc == 1.0,
                    "scale {scale} f {f}: {sep:?}"
                );
            }
        }
    }

    // Line 1 is all short boxes (so every left quarter is the same number),
    // line 2 tall at two cells in three. Document 0 calls the tall cells digits;
    // document 1, the same pixels, calls them letters, so its line 2 cannot
    // separate.
    #[test]
    fn the_reports_count_qualifying_lines_and_pooled_cells() {
        let tall = |line: usize, cell: usize| line == 1 && !cell.is_multiple_of(3);
        let img = boxes_image(760, 120, &zone_rects(tall, None));
        let located = locate(&img, 2, 44).expect("zone");
        let line1: Vec<Class> = (0..44)
            .map(|c| match c % 4 {
                0 => Class::K,
                1 | 2 => Class::Filler,
                _ => Class::Letter,
            })
            .collect();
        let line2 = |digit_when_tall: bool| -> Vec<Class> {
            (0..44)
                .map(|c| {
                    if tall(1, c) == digit_when_tall {
                        Class::Digit
                    } else {
                        Class::Letter
                    }
                })
                .collect()
        };
        let spec = |name: &str, digit_when_tall: bool| Specimen {
            name: name.to_string(),
            image: PathBuf::new(),
            sha256: String::new(),
            zone: vec![line1.clone(), line2(digit_when_tall)],
        };
        let specimens = [
            spec("passports/Right.png", true),
            spec("passports/Wrong.png", false),
        ];
        let records: Vec<LineRecord> = (0..2)
            .flat_map(|doc| {
                located
                    .lines
                    .iter()
                    .enumerate()
                    .map(move |(i, l)| (doc, i, l))
            })
            .map(|(doc, i, l)| measure_line(doc, i, &img, l))
            .collect();
        let item1 = report_item1(&specimens, &records);
        assert!(
            item1.contains("\n2 lines have at least two digits and two letters."),
            "{item1}"
        );
        assert!(item1.contains("| 0.02 | native | 2 | 0 | 1 |"), "{item1}");
        assert!(item1.contains("| 0.02 | 2 | 1 |"), "{item1}");
        assert!(
            item1.contains("| passports/Wrong.png line 2 | 16.0 | -"),
            "{item1}"
        );
        let not_separated = item1
            .split("Lines whose margin is not above zero")
            .nth(1)
            .expect("the table of lines not separated");
        assert!(!not_separated.contains("Right.png"), "{item1}");
        let item2 = report_item2(&specimens, &records);
        // 11 `K` and 22 fillers per document; a solid box holds a quarter of
        // its ink in its left quarter.
        assert!(
            item2.contains("| native | 22 | 0.250 / 0.250 / 0.250 | 44 | 0.250 / 0.250 / 0.250 | 0.000 | 0.500 |"),
            "{item2}"
        );
        assert!(
            item2.contains("none: the margin is not above zero at native"),
            "{item2}"
        );
    }

    /// A 2 px wide line from `a` to `b`, stamped as squares along it.
    fn stroke(img: &mut GrayImage, a: (f64, f64), b: (f64, f64)) {
        let steps = 200;
        for i in 0..=steps {
            let t = f64::from(i) / f64::from(steps);
            let (x, y) = (a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1));
            for dy in 0..2 {
                for dx in 0..2 {
                    img.put_pixel(x as u32 + dx, y as u32 + dy, Luma([0]));
                }
            }
        }
    }

    #[test]
    fn the_left_quarter_tells_a_stem_from_a_vertex() {
        let mut img = GrayImage::from_pixel(80, 40, Luma([255]));
        // `K`: a stem, and two arms from its middle to the right corners.
        stroke(&mut img, (8.0, 6.0), (8.0, 31.0));
        stroke(&mut img, (9.0, 19.0), (26.0, 6.0));
        stroke(&mut img, (9.0, 19.0), (26.0, 31.0));
        // `<`: two arms from a vertex at the middle of the left side.
        stroke(&mut img, (48.0, 19.0), (66.0, 6.0));
        stroke(&mut img, (48.0, 19.0), (66.0, 31.0));
        let ink: Vec<bool> = img.pixels().map(|p| p[0] < 128).collect();
        let blobs = components(&ink, 80);
        assert_eq!(blobs.len(), 2);
        let cells = measure_cells(&img, &blobs, 40.0, 1.0).expect("measured");
        let (k, filler) = (cells[0].left_quarter(), cells[1].left_quarter());
        assert!(k - filler > 0.15, "K {k} filler {filler}");
        let sum: f64 = cells[0].bins.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9, "{sum}");
    }

    // Paper at 240 with a fifth of it textured at 218, ink at 20: the texture
    // is a tenth of the way to ink, and comes off before any ink is counted.
    #[test]
    fn paper_texture_is_taken_off_before_ink_is_counted() {
        let img = GrayImage::from_fn(10, 10, |x, y| match (x, y) {
            (_, 0..=1) => Luma([218]),
            (4..=5, _) => Luma([20]),
            _ => Luma([240]),
        });
        let dark = darkness(&img).expect("measurable");
        assert_eq!(dark[0], 0.0, "texture");
        assert_eq!(dark[5 * 10 + 4], 1.0, "ink");
        assert_eq!(dark[5 * 10 + 8], 0.0, "paper");
        let flat = GrayImage::from_pixel(4, 4, Luma([200]));
        assert_eq!(darkness(&flat), Err("flat_band"));
        let faint = GrayImage::from_fn(4, 4, |x, _| Luma([if x < 2 { 200 } else { 190 }]));
        assert_eq!(darkness(&faint), Err("low_contrast"));
    }

    #[test]
    fn separation_counts_ties_as_half_and_needs_two_of_each() {
        let s = separation(&[3.0, 2.0], &[2.0, 1.0]).expect("two of each");
        assert_eq!(s.margin, 0.0);
        assert_eq!(s.auc, 3.5 / 4.0);
        assert_eq!(s.gap, 1.0);
        assert_eq!(separation(&[3.0], &[2.0, 1.0]), None);
        assert_eq!(separation(&[3.0, 4.0], &[]), None);
    }

    #[test]
    fn the_floor_is_the_finest_step_of_the_unbroken_run_from_native() {
        assert_eq!(
            floor(&[(12.5, true), (11.0, true), (9.0, false), (8.0, true)]),
            Some(11.0)
        );
        assert_eq!(floor(&[(12.5, true)]), Some(12.5));
        assert_eq!(floor(&[(12.5, false), (11.0, true)]), None);
        assert_eq!(floor(&[]), None);
    }

    #[test]
    fn medians_percentiles_and_slopes() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), Some(2.5));
        assert_eq!(median(&[]), None);
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0, 5.0], 90.0), Some(5.0));
        let line: Vec<(f64, f64)> = (0..10)
            .map(|x| (f64::from(x), 0.5 * f64::from(x) + 3.0))
            .collect();
        assert!((theil_sen(&line) - 0.5).abs() < 1e-12);
        assert_eq!(theil_sen(&[(1.0, 1.0), (1.0, 5.0)]), 0.0);
    }

    #[test]
    fn fixtures_reduce_to_classes_and_odd_shapes_or_characters_are_refused() {
        let td3 = format!(
            "{}\n{}",
            "P<UTO".to_string() + &"<".repeat(39),
            "K".repeat(9) + &"7".repeat(35)
        );
        let zone = fixture_classes(&td3).expect("TD3 shape");
        assert_eq!(zone.len(), 2);
        assert_eq!(
            &zone[0][..3],
            &[Class::Letter, Class::Filler, Class::Letter]
        );
        assert_eq!(zone[1][0], Class::K);
        assert_eq!(zone[1][43], Class::Digit);
        assert!(fixture_classes(&format!("{td3}\r")).is_ok());
        assert_eq!(fixture_classes(&td3[..88]), Err("fixture_shape"));
        assert_eq!(
            fixture_classes(&td3.replacen('P', "p", 1)),
            Err("fixture_alphabet")
        );
        let td1 = vec!["I".repeat(30); 3].join("\n");
        assert!(fixture_classes(&td1).is_ok());
        assert_eq!(
            fixture_classes(&vec!["I".repeat(30); 2].join("\n")),
            Err("fixture_shape")
        );
    }

    #[test]
    fn arguments_parse_and_unknown_ones_are_refused() {
        let args: Vec<String> = ["--samples-root", "/s", "--overlays", "--only", "X_mrz"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let parsed = parse_args(&args).expect("valid");
        assert_eq!(parsed.samples_root, Some(PathBuf::from("/s")));
        assert!(parsed.overlays && !parsed.locate_only);
        assert_eq!(parsed.only.as_deref(), Some("X_mrz"));
        assert!(parse_args(&["--out".to_string()]).is_err());
        assert!(parse_args(&["--fast".to_string()]).is_err());
    }

    #[test]
    fn an_in_tree_out_dir_is_refused_unless_git_ignores_it() {
        let root = Path::new("/repo");
        let inside = Path::new("/repo/results/run1");
        assert!(check_out_dir(inside, root, |_| false).is_err());
        assert!(check_out_dir(inside, root, |_| true).is_ok());
        let asked = std::cell::RefCell::new(None);
        let _ = check_out_dir(inside, root, |p| {
            *asked.borrow_mut() = Some(p.to_path_buf());
            true
        });
        assert_eq!(
            asked.into_inner(),
            Some(PathBuf::from("/repo/results/run1/cells.jsonl"))
        );
        assert!(check_out_dir(Path::new("/tmp/probe"), root, |_| panic!("not asked")).is_ok());
        assert!(check_out_dir(Path::new("/repo-other/x"), root, |_| false).is_ok());
        let cleaned = absolutize(Path::new("/tmp/../repo/results/./run"));
        assert!(check_out_dir(&cleaned, &absolutize(root), |_| false).is_err());
    }

    // The positive control: a page the generator draws with the vendored OCR-B
    // is located on its exact grid, and its digits stand higher than its
    // letters on line 2 in the median, at every fraction.
    #[test]
    fn a_generated_td3_page_is_located_on_its_exact_grid() {
        use synthpass_gen::layout::{for_format, mrz_char_rect_for_line};
        use synthpass_gen::{generate_from_seed, DocumentType, GeneratorConfig};
        let (image, labels, _) =
            generate_from_seed(&GeneratorConfig::with_document_type(7, DocumentType::TD3));
        let gray = grey_of(&image.into_rgb8());
        let located = locate(&gray, 2, 44).expect("the generated zone is found");
        let page = for_format(DocumentType::TD3);
        for (li, line) in located.lines.iter().enumerate() {
            for (ci, b) in line.cells.iter().enumerate() {
                let r = mrz_char_rect_for_line(page.mrz_lines[li], 44, ci as u32);
                let inside = (f64::from(r.x)..f64::from(r.x + r.width)).contains(&b.cx())
                    && (f64::from(r.y)..f64::from(r.y + r.height)).contains(&b.cy());
                assert!(inside, "line {li} cell {ci}: {b:?} outside {r:?}");
            }
        }
        let zone = fixture_classes(&labels.mrz_lines.join("\n")).expect("generated zone");
        let line = &located.lines[1];
        let cells = measure_cells(&gray, &line.cells, line.pitch, 1.0).expect("measured");
        let rec = LineRecord {
            doc: 0,
            line: 1,
            geometry: line.clone(),
            steps: vec![],
        };
        for (k, f) in FRACTIONS.iter().enumerate() {
            let sep = line_separation(&rec, &zone[1], &cells, k).expect("digits and letters");
            assert!(sep.gap > 0.0, "f {f}: {sep:?}");
        }
    }
}
