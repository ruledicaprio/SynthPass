//! `synthpass-export` — turn a deterministic `synthpass-gen` corpus into a
//! training dataset on disk.
//!
//! The formats and the coordinate convention are fixed by
//! [`knowledge/decisions/ADR-0007-dataset-export-format.md`] and specified in
//! [`knowledge/EXPORTS.md`]: record-based exports (JSONL, Hugging Face) use the
//! DeepSeek-OCR / Unlimited-OCR data-engine convention — coordinates normalised
//! to integer `0..=1000` relative to the rendered image, each block's box and
//! content concatenated into one end-to-end ground-truth string, `<page>` as
//! the separator between packed documents.
//!
//! `synthpass-gen`'s labels are 100% accurate by construction (M2's Definition
//! of Done), so a corpus exported from them is cleaner ground truth than the
//! OCR-pseudo-labelled corpora most training recipes were built on.
//!
//! v1 scope: `--profile clean` only (the `Labels` rects are computed on the
//! pristine render; degrade applies no transform to them), JSONL and Hugging
//! Face formats. COCO / YOLO are deferred — see `EXPORTS.md`.
//!
//! [`knowledge/decisions/ADR-0007-dataset-export-format.md`]: https://github.com/ruledicaprio/SynthPass/blob/main/knowledge/decisions/ADR-0007-dataset-export-format.md
//! [`knowledge/EXPORTS.md`]: https://github.com/ruledicaprio/SynthPass/blob/main/knowledge/EXPORTS.md

use std::path::PathBuf;

mod record;
mod writer;

pub use record::{
    image_rel_path, normalize_box, Block, DocumentRecord, GeneratedDoc, MrzRecord, Row,
};
pub use writer::{Manifest, ManifestFile};

use synthpass_gen::DocumentType;

/// Output format for [`run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// One `data.jsonl` of [`Row`]s plus an `images/` directory and a
    /// `manifest.json`.
    Jsonl,
    /// A directory the `datasets` library can load from disk with no network:
    /// `data/train.jsonl` + `data/images/`, `dataset_infos.json`, `README.md`
    /// (dataset card), `manifest.json`. Never a Hub push — ADR-0007 decision 3.
    Hf,
}

impl ExportFormat {
    /// The `"format"` string recorded in the manifest and accepted on the CLI.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Jsonl => "jsonl",
            Self::Hf => "hf",
        }
    }

    /// Parse a case-insensitive CLI value. `coco`/`yolo` are named explicitly
    /// as "not yet" rather than "unknown" — they are on the roadmap
    /// (`EXPORTS.md`, "Deferred: COCO / YOLO").
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "jsonl" => Ok(Self::Jsonl),
            "hf" => Ok(Self::Hf),
            "coco" | "yolo" => Err(format!(
                "--format {s}: not implemented yet — v1 ships jsonl and hf (see knowledge/EXPORTS.md)"
            )),
            other => Err(format!(
                "--format: unknown format '{other}' (valid: jsonl, hf)"
            )),
        }
    }
}

/// Which document format(s) to generate across the corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocTypeChoice {
    /// One fixed format for the whole corpus.
    One(DocumentType),
    /// Round-robin all five formats across the corpus by document index,
    /// matching `synthpass-bench`'s `--profile all`.
    All,
}

impl DocTypeChoice {
    /// The five formats in round-robin order.
    pub const ROUND_ROBIN: [DocumentType; 5] = [
        DocumentType::TD1,
        DocumentType::TD2,
        DocumentType::TD3,
        DocumentType::MrvA,
        DocumentType::MrvB,
    ];

    /// The document type for the `i`-th document in the corpus.
    pub fn for_index(self, i: u64) -> DocumentType {
        match self {
            Self::One(t) => t,
            Self::All => Self::ROUND_ROBIN[(i % 5) as usize],
        }
    }

    /// The `"document_type"` string recorded in the manifest.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::One(t) => match t {
                DocumentType::TD1 => "td1",
                DocumentType::TD2 => "td2",
                DocumentType::TD3 => "td3",
                DocumentType::MrvA => "mrva",
                DocumentType::MrvB => "mrvb",
            },
            Self::All => "all",
        }
    }

    /// Parse a case-insensitive CLI value (`td1`..`mrvb` or `all`).
    pub fn parse(s: &str) -> Result<Self, String> {
        if s.eq_ignore_ascii_case("all") {
            return Ok(Self::All);
        }
        DocumentType::parse(s).map(Self::One)
    }
}

/// Everything [`run`] needs. The CLI builds this from hand-rolled flags; tests
/// and `synthpass-bench` build it directly.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    pub format: ExportFormat,
    /// Number of documents to generate and export.
    pub count: u64,
    /// Base seed; document `i` uses `seed_base + i`.
    pub seed_base: u64,
    pub document_type: DocTypeChoice,
    /// Documents concatenated per JSONL row, joined with `<page>`. `1` = one
    /// document per row.
    pub pack_pages: u32,
    pub out_dir: PathBuf,
}

impl ExportConfig {
    /// The exact `synthpass export …` command line that reproduces this export,
    /// recorded in the manifest.
    pub fn command_line(&self) -> String {
        format!(
            "synthpass export --format {} --count {} --seed {} --document-type {} --profile clean --pack-pages {} --out-dir {}",
            self.format.as_str(),
            self.count,
            self.seed_base,
            self.document_type.as_str(),
            self.pack_pages,
            self.out_dir.display(),
        )
    }
}

/// What [`run`] wrote.
#[derive(Debug, Clone)]
pub struct ExportSummary {
    pub rows: usize,
    pub documents: u64,
    pub out_dir: PathBuf,
    pub manifest: Manifest,
}

/// Errors [`run`] can return. Generation itself is infallible (a pure function
/// of the seed); everything here is I/O or a bad config.
#[derive(Debug)]
pub enum ExportError {
    /// `count` was 0, or `pack_pages` was 0.
    EmptyRequest(&'static str),
    Io(std::io::Error),
    /// A row failed to serialise (should not happen — the row types are plain
    /// data — but not worth an `unwrap`).
    Serialize(serde_json::Error),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyRequest(what) => write!(f, "{what}"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Serialize(e) => write!(f, "serialization error: {e}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<std::io::Error> for ExportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for ExportError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialize(e)
    }
}

/// Generate the corpus and write the dataset under `cfg.out_dir`.
///
/// Deterministic: same `cfg` (seed, count, document type, packing) always
/// produces byte-identical output, because `synthpass_gen::generate_from_seed`
/// is a pure function of the seed (`crates/synthpass-gen/tests/determinism.rs`).
pub fn run(cfg: &ExportConfig) -> Result<ExportSummary, ExportError> {
    if cfg.count == 0 {
        return Err(ExportError::EmptyRequest("--count must be at least 1"));
    }
    if cfg.pack_pages == 0 {
        return Err(ExportError::EmptyRequest("--pack-pages must be at least 1"));
    }

    // 1. Generate every document + its record (pure, deterministic).
    let docs: Vec<GeneratedDoc> = (0..cfg.count)
        .map(|i| {
            let seed = cfg.seed_base + i;
            GeneratedDoc::build(seed, cfg.document_type.for_index(i))
        })
        .collect();

    // 2. Pack the records into rows.
    let records: Vec<DocumentRecord> = docs.iter().map(|d| d.record.clone()).collect();
    let rows = record::pack_rows(&records, cfg.pack_pages);

    // 3. Write.
    writer::write_dataset(cfg, &docs, &rows)
}
