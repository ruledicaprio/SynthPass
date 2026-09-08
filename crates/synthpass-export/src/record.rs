//! `synthpass_gen::Labels` → a serialisable training row, in the DeepSeek-OCR
//! 0–1000 convention fixed by `knowledge/decisions/ADR-0007-dataset-export-format.md`.

use serde::Serialize;
use synthpass_core::v2::CoreField;
use synthpass_gen::{generate_from_seed, DocumentType, GeneratorConfig, Labels};

/// Sentinel tokens for the concatenated ground-truth string. Pinned here so
/// every export is byte-identical — see `knowledge/EXPORTS.md`, "Ground-truth
/// string".
const REF_OPEN: &str = "<|ref|>";
const REF_CLOSE: &str = "<|/ref|>";
const BOX_TOKEN: &str = "<|box|>";
/// Separator between the documents packed into one row.
pub const PAGE_SEPARATOR: &str = "<page>";

/// Soft cap on a packed row: stop adding documents once the running
/// ground-truth length crosses this many characters. `32_000` tokens at a
/// coarse 4-chars-per-token proxy — the exporter deliberately does not depend
/// on a tokenizer (`EXPORTS.md`).
const PACK_CHAR_BUDGET: usize = 32_000 * 4;

/// One labelled region: a VIZ field (`field` is a [`CoreField`] wire name) or
/// an MRZ line (`field` is `"mrz_line"`, `line` is its 0-based index).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Block {
    pub field: String,
    pub value: String,
    /// `[x0, y0, x1, y1]` — top-left and bottom-right, each normalised to
    /// `0..=1000` relative to the rendered image.
    #[serde(rename = "box")]
    pub bbox: [u32; 4],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// The MRZ, verbatim from [`Labels`], plus the normalised whole-band box.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MrzRecord {
    /// `Labels::mrz_lines` verbatim (2 lines for TD2/TD3/MRV, 3 for TD1).
    pub lines: Vec<String>,
    /// `"TD1"`..`"MRVB"` — `synthpass_gen` `mrz_format.as_str()`.
    pub format: String,
    pub band_box: [u32; 4],
}

/// One generated document as a training row entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocumentRecord {
    pub seed: u64,
    /// `"td1"`..`"mrvb"`.
    pub document_type: String,
    /// `"TD1"`..`"MRVB"` — the MRZ format string.
    pub mrz_format: String,
    /// Path to the rendered PNG, relative to the JSONL file
    /// (`images/<doctype>-<seed:06>.png`). v1 always writes images.
    pub image: String,
    /// Rendered image pixels — informational; every `box` is already 0–1000.
    pub width: u32,
    pub height: u32,
    /// VIZ fields then MRZ lines, in reading order (top-to-bottom, left-to-right).
    pub blocks: Vec<Block>,
    pub mrz: MrzRecord,
    /// This document's blocks concatenated end-to-end — see `EXPORTS.md`.
    pub ground_truth: String,
}

/// One JSONL line: one document by default, or `pack_pages` of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    /// `"<doctype>-<seed:06>"`, or `"<doctype>-<firstSeed>+<n>"` for a packed row.
    pub id: String,
    pub documents: Vec<DocumentRecord>,
    /// `documents[*].ground_truth` joined with `<page>`.
    pub ground_truth: String,
}

/// A generated document: its serialisable [`DocumentRecord`] plus the rendered
/// image the writer saves alongside the JSONL.
pub struct GeneratedDoc {
    pub record: DocumentRecord,
    pub image: image::DynamicImage,
}

impl GeneratedDoc {
    /// Generate document `seed` as `doc_type` and build its record. Pure and
    /// deterministic — the same `(seed, doc_type)` always yields the same
    /// record and the same pixels.
    pub fn build(seed: u64, doc_type: DocumentType) -> Self {
        let config = GeneratorConfig::with_document_type(seed, doc_type);
        let (image, labels, _passport) = generate_from_seed(&config);
        let (width, height) = (image.width(), image.height());
        let record = build_record(seed, doc_type, &labels, width, height);
        Self { record, image }
    }
}

/// Normalise a `synthpass_gen` pixel [`Rect`](synthpass_gen::layout::Rect) to
/// the `[x0, y0, x1, y1]` 0–1000 box (ADR-0007). Rounds half to even so the
/// output is byte-reproducible; clamps into `0..=1000`.
pub fn normalize_box(rect: synthpass_gen::layout::Rect, img_w: u32, img_h: u32) -> [u32; 4] {
    fn scale(v: u32, span: u32) -> u32 {
        if span == 0 {
            return 0;
        }
        ((v as f64) * 1000.0 / (span as f64))
            .round_ties_even()
            .clamp(0.0, 1000.0) as u32
    }
    [
        scale(rect.x, img_w),
        scale(rect.y, img_h),
        scale(rect.x.saturating_add(rect.width), img_w),
        scale(rect.y.saturating_add(rect.height), img_h),
    ]
}

/// The PNG path for a document, relative to the JSONL file. Deterministic in
/// `(doctype, seed)` so `build_record` can compute it without knowing the
/// output directory; the writer saves the file at the matching location.
pub fn image_rel_path(doctype: &str, seed: u64) -> String {
    format!("images/{doctype}-{seed:06}.png")
}

fn doc_type_lower(t: DocumentType) -> &'static str {
    match t {
        DocumentType::TD1 => "td1",
        DocumentType::TD2 => "td2",
        DocumentType::TD3 => "td3",
        DocumentType::MrvA => "mrva",
        DocumentType::MrvB => "mrvb",
    }
}

/// A VIZ field pulled off [`Labels`], paired with the [`CoreField`] whose wire
/// name it carries.
fn viz_fields(labels: &Labels) -> Vec<(CoreField, &synthpass_gen::FieldLabel)> {
    let mut v = vec![
        (CoreField::DocumentType, &labels.document_type),
        (CoreField::IssuingCountry, &labels.issuing_country),
        (CoreField::Surname, &labels.surname),
        (CoreField::GivenNames, &labels.given_names),
        (CoreField::DocumentNumber, &labels.document_number),
        (CoreField::Nationality, &labels.nationality),
        (CoreField::DateOfBirth, &labels.date_of_birth),
        (CoreField::Sex, &labels.sex),
        (CoreField::DateOfExpiry, &labels.date_of_expiry),
    ];
    if let Some(pn) = labels.personal_number.as_ref() {
        v.push((CoreField::PersonalNumber, pn));
    }
    v
}

fn build_record(
    seed: u64,
    doc_type: DocumentType,
    labels: &Labels,
    width: u32,
    height: u32,
) -> DocumentRecord {
    let page = synthpass_gen::layout::for_format(doc_type);

    // Collect every labelled region with its pixel rect, then order by reading
    // position (top-to-bottom, then left-to-right). Geometry, not a hard-coded
    // list, so the order stays correct across the five formats' differing
    // layouts and `personal_number` lands where it is actually drawn.
    let mut staged: Vec<(synthpass_gen::layout::Rect, Block)> = Vec::new();

    for (field, fl) in viz_fields(labels) {
        staged.push((
            fl.rect,
            Block {
                field: field.as_str().to_string(),
                value: fl.value.clone(),
                bbox: normalize_box(fl.rect, width, height),
                line: None,
            },
        ));
    }

    // Cyrillic-script documents paint the name in its native script in the VIZ;
    // the `surname`/`given_names` blocks above and the MRZ carry the ICAO 9303
    // §6 B transliteration. Emit the native strings as their own blocks, on the
    // same VIZ rect, so an exported row carries both scripts (EXPORTS.md).
    for (name, fl) in [
        ("surname_native", labels.surname_native.as_ref()),
        ("given_names_native", labels.given_names_native.as_ref()),
    ] {
        if let Some(fl) = fl {
            staged.push((
                fl.rect,
                Block {
                    field: name.to_string(),
                    value: fl.value.clone(),
                    bbox: normalize_box(fl.rect, width, height),
                    line: None,
                },
            ));
        }
    }

    for (i, line) in labels.mrz_lines.iter().enumerate() {
        let rect = page.mrz_lines[i];
        staged.push((
            rect,
            Block {
                field: "mrz_line".to_string(),
                value: line.clone(),
                bbox: normalize_box(rect, width, height),
                line: Some(i as u32),
            },
        ));
    }

    staged.sort_by_key(|(r, _)| (r.y, r.x));
    let blocks: Vec<Block> = staged.into_iter().map(|(_, b)| b).collect();

    let ground_truth = blocks.iter().map(block_to_ground_truth).collect();

    let doctype = doc_type_lower(doc_type);
    DocumentRecord {
        seed,
        document_type: doctype.to_string(),
        mrz_format: labels.mrz_format.as_str().to_string(),
        image: image_rel_path(doctype, seed),
        width,
        height,
        blocks,
        mrz: MrzRecord {
            lines: labels.mrz_lines.clone(),
            format: labels.mrz_format.as_str().to_string(),
            band_box: normalize_box(labels.mrz_rect, width, height),
        },
        ground_truth,
    }
}

fn block_to_ground_truth(b: &Block) -> String {
    let [x0, y0, x1, y1] = b.bbox;
    format!(
        "{REF_OPEN}{field}{REF_CLOSE}{BOX_TOKEN}{x0},{y0},{x1},{y1}{BOX_TOKEN}{value}",
        field = b.field,
        value = b.value,
    )
}

/// Pack `records` into rows of up to `pack_pages` documents each, respecting
/// the [`PACK_CHAR_BUDGET`] soft cap. A single document always gets its own row
/// even if it exceeds the cap.
pub fn pack_rows(records: &[DocumentRecord], pack_pages: u32) -> Vec<Row> {
    let pack_pages = pack_pages.max(1) as usize;
    let mut rows = Vec::new();
    let mut current: Vec<DocumentRecord> = Vec::new();
    let mut current_chars = 0usize;

    for rec in records {
        let rec_chars = rec.ground_truth.len();
        let would_overflow_budget = !current.is_empty()
            && current_chars + rec_chars + PAGE_SEPARATOR.len() > PACK_CHAR_BUDGET;
        if current.len() >= pack_pages || would_overflow_budget {
            rows.push(finish_row(std::mem::take(&mut current)));
            current_chars = 0;
        }
        current_chars += rec_chars + PAGE_SEPARATOR.len();
        current.push(rec.clone());
    }
    if !current.is_empty() {
        rows.push(finish_row(current));
    }
    rows
}

fn finish_row(documents: Vec<DocumentRecord>) -> Row {
    let ground_truth = documents
        .iter()
        .map(|d| d.ground_truth.as_str())
        .collect::<Vec<_>>()
        .join(PAGE_SEPARATOR);
    let id = match documents.as_slice() {
        [only] => format!("{}-{:06}", only.document_type, only.seed),
        [first, ..] => format!("{}-{}+{}", first.document_type, first.seed, documents.len()),
        [] => unreachable!("finish_row is never called with an empty document list"),
    };
    Row {
        id,
        documents,
        ground_truth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_normalization_is_deterministic_and_clamped() {
        let r = synthpass_gen::layout::Rect::new(60, 720, 1080, 100);
        let a = normalize_box(r, 1200, 840);
        let b = normalize_box(r, 1200, 840);
        assert_eq!(a, b);
        // x: 60/1200 -> 50 ; x1: 1140/1200 -> 950 ; y: 720/840 -> 857 ; y1: 820/840 -> 976
        assert_eq!(a, [50, 857, 950, 976]);
        assert!(a.iter().all(|&v| v <= 1000));
    }

    #[test]
    fn build_is_pure_in_the_seed() {
        let one = GeneratedDoc::build(42, DocumentType::TD3).record;
        let two = GeneratedDoc::build(42, DocumentType::TD3).record;
        assert_eq!(one, two);
        assert_eq!(one.document_type, "td3");
        assert_eq!(one.mrz_format, "TD3");
        assert_eq!(one.mrz.lines.len(), 2);
    }

    #[test]
    fn blocks_are_in_reading_order_and_mrz_comes_last() {
        let rec = GeneratedDoc::build(7, DocumentType::TD3).record;
        // y is non-decreasing when we re-derive it from the normalised boxes.
        let ys: Vec<u32> = rec.blocks.iter().map(|b| b.bbox[1]).collect();
        assert!(
            ys.windows(2).all(|w| w[0] <= w[1]),
            "blocks not top-to-bottom: {ys:?}"
        );
        let mrz_count = rec.blocks.iter().filter(|b| b.field == "mrz_line").count();
        assert_eq!(mrz_count, 2);
        assert!(rec
            .blocks
            .iter()
            .rev()
            .take(2)
            .all(|b| b.field == "mrz_line"));
        assert_eq!(rec.blocks.last().unwrap().line, Some(1));
    }

    #[test]
    fn ground_truth_string_uses_the_pinned_tokens() {
        let rec = GeneratedDoc::build(1, DocumentType::TD3).record;
        assert!(rec
            .ground_truth
            .starts_with("<|ref|>document_type<|/ref|><|box|>"));
        assert!(rec.ground_truth.contains("<|box|>"));
        // Single-document record: no page separator inside it.
        assert!(!rec.ground_truth.contains("<page>"));
    }

    #[test]
    fn td1_has_three_mrz_line_blocks() {
        let rec = GeneratedDoc::build(3, DocumentType::TD1).record;
        let mrz: Vec<u32> = rec
            .blocks
            .iter()
            .filter(|b| b.field == "mrz_line")
            .map(|b| b.line.unwrap())
            .collect();
        assert_eq!(mrz, vec![0, 1, 2]);
        assert_eq!(rec.mrz.lines.len(), 3);
    }

    #[test]
    fn packing_groups_documents_and_ids_reflect_it() {
        let recs: Vec<DocumentRecord> = (0..5)
            .map(|s| GeneratedDoc::build(s, DocumentType::TD3).record)
            .collect();

        let unpacked = pack_rows(&recs, 1);
        assert_eq!(unpacked.len(), 5);
        assert_eq!(unpacked[0].id, "td3-000000");
        assert_eq!(unpacked[0].documents.len(), 1);
        assert!(!unpacked[0].ground_truth.contains("<page>"));

        let packed = pack_rows(&recs, 2);
        assert_eq!(packed.len(), 3); // 2 + 2 + 1
        assert_eq!(packed[0].id, "td3-0+2");
        assert_eq!(packed[0].documents.len(), 2);
        assert!(packed[0].ground_truth.contains("<page>"));
        assert_eq!(packed[2].id, "td3-000004"); // the lone tail document
    }
}
