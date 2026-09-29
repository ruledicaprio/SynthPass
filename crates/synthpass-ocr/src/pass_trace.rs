//! Per-pass OCR readings, for the benchmarks only (ADR-0024, amendment 1).
//!
//! The retry loop in [`crate::NativeOcr::recognize_detailed`] appends every
//! pass's MRZ-shaped lines to one page text, so which pass read a line is gone
//! before Tier 1 sees the text. [`crate::NativeOcr::recognize_detailed_traced`]
//! returns the same [`crate::OcrPage`] plus one [`PassRecord`] per executed
//! pass, which keeps that provenance for a benchmark that has to attribute a
//! reading to a pass.
//!
//! Nothing on the extraction path calls the traced entry point, none of these
//! types derives `Serialize` (the benchmarks write their own JSON, so the wire
//! format is theirs to own), and `OcrPage` does not carry them.
//!
//! # What is recorded
//!
//! Every **MRZ-shaped** line: exactly the lines the retry loop's own filter
//! keeps (at least 20 non-whitespace characters), for every pass that ran, the
//! general pass included. This is an observation, not a classification — which
//! of those lines is "line 1" is interpretation, and this crate does not
//! interpret fields. Lines that the chargrid arm adds to the page text
//! (`SYNTHPASS_OCR_CHARGRID`) are not pass readings and are not in
//! [`PassRecord::readings`]; the attempt that produced them is recorded on the
//! accepted pass as a [`ChargridRecord`] (see below).
//!
//! Passes that never ran (the pass cap or the wall-clock budget cut the loop
//! off first) have no record; [`crate::OcrPage::retry_stop`] says why.
//!
//! Bounding boxes are in each pass's **own** image space (crop, upscale, turn),
//! stated by [`PassRecord::image_width`] and [`PassRecord::image_height`], so
//! they compare within a pass and not across passes.
//!
//! # Chargrid capture
//!
//! When `SYNTHPASS_OCR_CHARGRID` is `on` or `control`, the chargrid name-line
//! repair (see [`crate::chargrid`]) runs once after the accepting pass, on that
//! pass's pixels. [`PassRecord::chargrid`] holds what that attempt did, on the
//! `Accepted` record only, so a benchmark can say *why* a repaired name line
//! came out as it did, not only that it was wrong (ADR-0024, amendment 2):
//!
//! - always: the arm and the verdict [`crate::OcrPage::chargrid`] reports;
//! - once a recognized line matched the name line: the line's box, its glyphs'
//!   characters and edges, the dark-pixel count of every column of the band the
//!   ink is measured on, a hash of that band, and the ink floor;
//! - once a grid was fitted: its origin and pitch, each glyph's cell, each
//!   cell's ink and the repair outcome.
//!
//! The record is enough to replay the grid fit, the alignment, the ink
//! measurement and both gates without re-running OCR, with any one input varied.
//! Outputs are recorded beside inputs so a replay is checked against what ran.
//! The band's pixels are never recorded, only their hash. The capture records
//! what `apply_chargrid` already did and adds no interpretation; the attempt is
//! identical with and without it.
//!
//! The glyph characters, the lines and the column profile are document
//! content, under the same handling as [`LineReading::text`].

use crate::chargrid::{Glyph, Grid, Rejected, Repair};
use crate::{mrz_shaped_lines, BBox};
use std::fmt;

/// What produced one pass's pixels, independent of which OCR arms are on.
///
/// The indices are **structural**, not treatment names: `MrzVariant(k)` is the
/// `k`-th entry of the `mrz_variants` tier as it came out of the preprocessor,
/// and the label follows those pixels through `SYNTHPASS_OCR_ORDER=control`'s
/// swap of the first two entries, so the same pixels carry the same label under
/// every arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassTransform {
    /// The full-page general pass (`order` 0).
    General,
    /// The untreated band crop moved to the front of the chain by
    /// `SYNTHPASS_OCR_ORDER=band-first`.
    BandFirstPlain,
    /// The `k`-th `preprocess::mrz_variants` entry, counted before any reorder.
    MrzVariant(usize),
    /// The `k`-th `preprocess::geometry_band_variants` entry.
    GeometryBand(usize),
    /// The untreated band crop in the trailing texture tier.
    TexturePlain,
    /// The median-filtered band crop in the trailing texture tier.
    TextureMedian,
    /// The untreated band crop of the page turned by this many degrees
    /// (90 or 270), the chain's outermost tier.
    QuarterTurn(u16),
}

impl fmt::Display for PassTransform {
    /// The wire label the benchmarks write: `general`, `band_first:plain_band`,
    /// `mrz_variants:<k>`, `geometry_band:<k>`, `texture:plain_band`,
    /// `texture:median`, `quarter_turn:<90|270>`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::General => f.write_str("general"),
            Self::BandFirstPlain => f.write_str("band_first:plain_band"),
            Self::MrzVariant(k) => write!(f, "mrz_variants:{k}"),
            Self::GeometryBand(k) => write!(f, "geometry_band:{k}"),
            Self::TexturePlain => f.write_str("texture:plain_band"),
            Self::TextureMedian => f.write_str("texture:median"),
            Self::QuarterTurn(turn) => write!(f, "quarter_turn:{turn}"),
        }
    }
}

/// What the retry loop did with one executed pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassOutcome {
    /// The recognizer returned an error. The loop skipped the pass; the page
    /// text gained nothing from it.
    Failed,
    /// The pass ran but read no MRZ-shaped line, so it added nothing.
    NoMrzShapedLines,
    /// The pass's MRZ-shaped lines are part of the page text, and the loop went
    /// on. For the general pass (`order` 0) they are the base of the page text
    /// rather than an addition to it.
    Appended,
    /// The pass's MRZ-shaped lines held a checksum-valid MRZ and stopped the
    /// loop. The record with this outcome is the pass
    /// [`crate::OcrPage::retry_variant_id`] names, and it is the last one.
    Accepted,
}

impl PassOutcome {
    /// The snake_case label the benchmarks write.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Failed => "failed",
            Self::NoMrzShapedLines => "no_mrz_shaped_lines",
            Self::Appended => "appended",
            Self::Accepted => "accepted",
        }
    }
}

/// One MRZ-shaped line one pass read.
#[derive(Debug, Clone, PartialEq)]
pub struct LineReading {
    /// Position in this pass's own recognized-line order, counting **every**
    /// line the pass emitted, not only the MRZ-shaped ones, so it points back
    /// at the source line.
    pub line_index: usize,
    /// The line's bounding rectangle in the pass image's pixel space.
    pub bbox: BBox,
    /// The trimmed line, byte-identical to what went into `OcrPage::text`.
    pub text: String,
}

/// One executed OCR pass, in execution order.
#[derive(Debug, Clone, PartialEq)]
pub struct PassRecord {
    /// 0 for the general pass, `i + 1` for retry variant `i`.
    pub order: usize,
    /// `general` or `pass-NN`, the vocabulary of
    /// [`crate::OcrPage::retry_variant_id`].
    pub id: String,
    /// What produced this pass's pixels.
    pub transform: PassTransform,
    /// The extra cardinal turn this pass's pixels carry: non-zero only for the
    /// quarter-turn tier.
    pub turn: u16,
    /// Width of the image this pass read; [`LineReading::bbox`] is in this space.
    pub image_width: u32,
    /// Height of the image this pass read.
    pub image_height: u32,
    /// What the loop did with the pass.
    pub outcome: PassOutcome,
    /// Every MRZ-shaped line the pass read, in the pass's own line order.
    pub readings: Vec<LineReading>,
    /// The chargrid attempt that read this pass's pixels. `Some` only on the
    /// `Accepted` record, and only when `SYNTHPASS_OCR_CHARGRID` is `on` or
    /// `control`.
    pub chargrid: Option<ChargridRecord>,
}

/// One chargrid attempt, as observed. See the module's `# Chargrid capture`.
#[derive(Debug, Clone, PartialEq)]
pub struct ChargridRecord {
    /// The arm that ran: `on` or `control`, [`crate::OcrArms::chargrid`]'s
    /// vocabulary.
    pub mode: &'static str,
    /// Byte-identical to the [`crate::OcrPage::chargrid`] this attempt produced.
    pub verdict: String,
    /// `None` when the attempt stopped before a recognized line matched the
    /// name line.
    pub line: Option<ChargridLineCapture>,
}

/// The matched line and what was measured on it. Every position is in the
/// **working image**'s pixel space: the accepted pass's image, or that image
/// downscaled when [`Self::downscaled`] says the width cap fired.
#[derive(Debug, Clone, PartialEq)]
pub struct ChargridLineCapture {
    /// The MRZ line the attempt repairs: 0, or 2 for TD1.
    pub name_line_index: usize,
    /// Cells in that line: 44, 36 or 30.
    pub width: usize,
    /// The parsed name line the repair targets.
    pub raw_name_line: String,
    /// Index of the matched line in the attempt's own recognition output,
    /// counting every line. That is a separate `ocrs` pass from the one the
    /// pass record's readings came from, so the two indices are not comparable.
    pub recognized_line_index: usize,
    /// The matched line's text, as `ocrs` read it.
    pub recognized_text: String,
    /// The line was wider than the width cap, so the image was downscaled and
    /// recognized again; all geometry here is in the downscaled image.
    pub downscaled: bool,
    /// Working image width.
    pub image_width: u32,
    /// Working image height.
    pub image_height: u32,
    /// The matched line's box, as the fit and the ink measurement read it.
    pub line_box: LineBox,
    /// Lowercase hex SHA-256 of the grayscale band the ink is measured on: the
    /// working image width as `u32` little-endian, the row count
    /// (`bottom.min(h) - top.min(h)`) as `u32` little-endian, then those rows of
    /// the grayscale image, every column, row-major, one byte per pixel. The
    /// band itself is never recorded.
    pub band_sha256: String,
    /// The matched line's glyphs in recognition order.
    pub glyphs: Vec<Glyph>,
    /// Dark-pixel count of every column of the working image, over the line
    /// box's rows: [`Self::image_width`] entries.
    pub column_ink: Vec<u32>,
    /// The ink floor the repair's gate used.
    pub ink_floor: f32,
    /// `None` when the grid fit returned nothing (no glyphs, or more glyphs than
    /// cells).
    pub fit: Option<ChargridFit>,
}

/// A line box in edges, the form the fit and the ink measurement consume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineBox {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

/// The fitted grid and what the repair made of it.
#[derive(Debug, Clone, PartialEq)]
pub struct ChargridFit {
    /// The fitted origin, pitch and cell count, after the refit rounds.
    pub grid: Grid,
    /// `chargrid::align(glyphs, grid)`: glyph `i` sits in cell `cells[i]`.
    pub cells: Option<Vec<usize>>,
    /// Exactly the ink slice the repair's gate received, one value per cell.
    pub cell_ink: Vec<f32>,
    /// Exactly what the repair returned. It differs from the page verdict under
    /// `control` (which never repairs), and when the repaired block is not
    /// re-verified.
    pub repair: Result<Repair, Rejected>,
}

/// The pass's recognized text: its lines joined with `\n`.
///
/// This is the text the retry loop has always worked on, and it is what
/// `OcrEngine::get_text` returns in `ocrs` 0.13.1 (see `run_pass`).
pub(crate) fn joined_text(lines: &[(String, BBox)]) -> String {
    lines
        .iter()
        .map(|(text, _)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The MRZ-shaped lines of a pass, with the position and box of each.
///
/// The filter is the retry loop's own (`mrz_shaped_lines`): trim, then keep a
/// line with at least 20 non-whitespace characters. Deriving it from the
/// structured lines rather than from the joined text is what carries
/// `line_index` and `bbox`; the retry loop's text is still built from
/// `mrz_shaped_lines`, unchanged, and the two agree by construction — the
/// tests pin that they do, and the loop `debug_assert`s it on every traced run.
///
/// Assumes no recognized line contains a newline, which holds for `ocrs`: its
/// recognition alphabet has none.
pub(crate) fn mrz_readings(lines: &[(String, BBox)]) -> Vec<LineReading> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(line_index, (text, bbox))| {
            let trimmed = text.trim();
            (trimmed.chars().filter(|c| !c.is_whitespace()).count() >= 20).then(|| LineReading {
                line_index,
                bbox: *bbox,
                text: trimmed.to_string(),
            })
        })
        .collect()
}

/// The text the retry loop appends for one pass, rebuilt from its readings.
/// Equal to `mrz_shaped_lines` of the pass's joined text; used to assert it.
pub(crate) fn readings_text(readings: &[LineReading]) -> String {
    readings
        .iter()
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether the two derivations of a pass's MRZ-shaped text agree. Kept as a
/// function so the loop's `debug_assert!` and the tests state one rule.
pub(crate) fn readings_match_candidates(lines: &[(String, BBox)]) -> bool {
    readings_text(&mrz_readings(lines)) == mrz_shaped_lines(&joined_text(lines))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bbox(n: usize) -> BBox {
        BBox {
            x: n as f32,
            y: (n * 2) as f32,
            w: 100.0,
            h: 10.0,
        }
    }

    fn lines(texts: &[&str]) -> Vec<(String, BBox)> {
        texts
            .iter()
            .enumerate()
            .map(|(i, t)| ((*t).to_string(), bbox(i)))
            .collect()
    }

    const NINETEEN: &str = "ABCDEFGHIJKLMNOPQRS";
    const TWENTY: &str = "ABCDEFGHIJKLMNOPQRST";

    #[test]
    fn readings_agree_with_the_retry_loops_own_filter() {
        let cases: Vec<Vec<&str>> = vec![
            vec![],
            vec![""],
            vec![NINETEEN, TWENTY, NINETEEN],
            // Inner spaces count toward neither side of the threshold.
            vec!["ABCDEFGHIJ KLMNOPQRS", "ABCDEFGHIJ KLMNOPQRST"],
            vec!["  ABCDEFGHIJKLMNOPQRST  ", "\tABCDEFGHIJKLMNOPQRS\t"],
            // Padding must not lift a short line over the threshold.
            vec!["    ABCDEFGHIJKLMNOPQRS    "],
            vec!["", TWENTY, "", "", TWENTY, ""],
            vec![TWENTY, TWENTY, ""],
            vec!["P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<", "short", ""],
        ];
        for texts in cases {
            let lines = lines(&texts);
            let pass_text = joined_text(&lines);
            assert_eq!(
                pass_text,
                texts.join("\n"),
                "the pass text is the lines joined with a newline: {texts:?}"
            );
            let readings = mrz_readings(&lines);
            assert_eq!(
                readings_text(&readings),
                mrz_shaped_lines(&pass_text),
                "the readings must rebuild exactly what the loop appends: {texts:?}"
            );
            assert!(readings_match_candidates(&lines), "{texts:?}");
        }
    }

    #[test]
    fn the_twenty_character_threshold_is_inclusive_at_twenty_and_not_at_nineteen() {
        let readings = mrz_readings(&lines(&[NINETEEN, TWENTY]));
        assert_eq!(readings.len(), 1);
        assert_eq!(readings[0].text, TWENTY);
    }

    #[test]
    fn line_index_counts_every_line_the_pass_emitted() {
        let source = lines(&["", NINETEEN, TWENTY, "", "  ABCDEFGHIJKLMNOPQRSTU  ", ""]);
        let readings = mrz_readings(&source);
        let indexes: Vec<usize> = readings.iter().map(|r| r.line_index).collect();
        assert_eq!(indexes, [2, 4]);
        for r in &readings {
            let (text, bbox) = &source[r.line_index];
            assert_eq!(
                r.text,
                text.trim(),
                "the reading is its source line, trimmed"
            );
            assert_eq!(&r.bbox, bbox, "the box travels with the line");
        }
    }

    #[test]
    fn readings_are_trimmed_but_keep_inner_spaces() {
        let readings = mrz_readings(&lines(&["  ABCDEFGHIJ KLMNOPQRST \t"]));
        assert_eq!(readings.len(), 1);
        assert_eq!(readings[0].text, "ABCDEFGHIJ KLMNOPQRST");
    }

    #[test]
    fn a_pass_with_no_lines_has_no_readings() {
        assert!(mrz_readings(&[]).is_empty());
        assert_eq!(joined_text(&[]), "");
    }

    #[test]
    fn transform_wire_labels() {
        let cases = [
            (PassTransform::General, "general"),
            (PassTransform::BandFirstPlain, "band_first:plain_band"),
            (PassTransform::MrzVariant(0), "mrz_variants:0"),
            (PassTransform::MrzVariant(6), "mrz_variants:6"),
            (PassTransform::GeometryBand(1), "geometry_band:1"),
            (PassTransform::TexturePlain, "texture:plain_band"),
            (PassTransform::TextureMedian, "texture:median"),
            (PassTransform::QuarterTurn(90), "quarter_turn:90"),
            (PassTransform::QuarterTurn(270), "quarter_turn:270"),
        ];
        for (transform, label) in cases {
            assert_eq!(transform.to_string(), label);
        }
    }

    #[test]
    fn outcome_wire_labels() {
        assert_eq!(PassOutcome::Failed.as_str(), "failed");
        assert_eq!(
            PassOutcome::NoMrzShapedLines.as_str(),
            "no_mrz_shaped_lines"
        );
        assert_eq!(PassOutcome::Appended.as_str(), "appended");
        assert_eq!(PassOutcome::Accepted.as_str(), "accepted");
    }
}
