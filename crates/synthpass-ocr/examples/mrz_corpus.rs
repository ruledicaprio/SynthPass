//! Tier-1 accuracy harness: runs the in-process OCR engine over every
//! MRZ-bearing specimen in `samples/` and reports how many produce a
//! checksum-valid MRZ (the "Tier-1 hit rate" from knowledge/ARCHITECTURE.md §8).
//!
//! Run from the repo root (models auto-resolve there):
//! ```powershell
//! cargo run -p synthpass-ocr --release --example mrz_corpus
//! ```
//! Pass `--dump` to print the raw OCR text for each miss.
//!
//! # Where the corpus comes from
//!
//! Both sets are derived from `samples/corpus.jsonl` rather than hard-coded:
//!
//! * **Positive** — every row carrying an `expected_document_number`, which is
//!   normally the `document_number` from that specimen's hand-labelled `.json`.
//! * **Negative** — every row with `mrz.present == false`: a document that
//!   physically has no machine-readable zone, and so must never yield a valid
//!   MRZ.
//!
//! The two lists used to be `const` arrays of basenames, which meant a rename
//! stranded an entry silently: the harness reported a lower hit rate instead of
//! an error, and 16 of 21 entries were stale by the time this changed. Deriving
//! them removes that failure mode entirely — a renamed specimen keeps its link
//! because the manifest is regenerated from the corpus itself.

use std::path::{Path, PathBuf};
use synthpass_ocr::NativeOcr;

/// One expected result: a specimen basename and the document number its
/// hand-verified ground truth records.
struct CorpusEntry {
    file: String,
    expected_doc: String,
}

/// Loads the positive set: every manifest row that records an expected
/// document number.
///
/// That field is normally the `document_number` from a hand-labelled `.json`,
/// but it can also be filled in by hand for a specimen whose number is known
/// without a full `Extraction` label. Three entries are in that second group
/// (Oman 2004, Viet Nam 2023) or deliberately excluded (Israel 2003, whose MRZ
/// is physically redacted) — see their `notes`. Keying on ground truth alone
/// would have silently dropped the first two, and since both are *known
/// misses* that would have moved the reported rate from 18/21 to 18/18 purely
/// by shrinking the denominator.
fn load_corpus(root: &Path) -> Vec<CorpusEntry> {
    let mut out: Vec<CorpusEntry> = manifest_rows(root)
        .into_iter()
        .filter_map(|row| {
            let file = row["filename"].as_str()?.to_string();
            let expected_doc = row["expected_document_number"].as_str()?.to_string();
            Some(CorpusEntry { file, expected_doc })
        })
        .collect();
    out.sort_by(|a, b| a.file.cmp(&b.file));
    out
}

/// Loads the negative set: every specimen the manifest records as carrying no
/// MRZ at all.
fn load_negative(root: &Path) -> Vec<String> {
    let mut out: Vec<String> = manifest_rows(root)
        .into_iter()
        .filter(|row| row["mrz"]["present"].as_bool() == Some(false))
        .filter_map(|row| row["filename"].as_str().map(str::to_string))
        .collect();
    out.sort();
    out
}

fn manifest_rows(root: &Path) -> Vec<serde_json::Value> {
    let path = root.join("samples").join("corpus.jsonl");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}) — it is tracked on main; regenerate it with the \
             corpus_manifest example",
            path.display()
        )
    });
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

fn main() {
    let dump = std::env::args().any(|a| a == "--dump");
    let root = repo_root();
    let ocr = NativeOcr::load(
        &root.join("text-detection.rten"),
        &root.join("text-recognition.rten"),
    )
    .expect("failed to load OCR models — run from the repo root");

    let corpus = load_corpus(&root);
    let negative = load_negative(&root);

    let mut hits = 0usize;
    let mut missing = 0usize;
    for CorpusEntry { file, expected_doc } in &corpus {
        let Some(path) = find_sample(file) else {
            println!("SKIP  {file}: not found under samples/ — stale corpus entry");
            missing += 1;
            continue;
        };
        let started = std::time::Instant::now();
        let text = match ocr.recognize(&path) {
            Ok(t) => t,
            Err(e) => {
                println!("MISS  {file}: OCR error: {e}");
                continue;
            }
        };
        let elapsed = started.elapsed();
        match mrz::find_and_parse(&text) {
            Ok(d) if d.valid() => {
                let doc_ok = d.document_number == *expected_doc;
                if doc_ok {
                    hits += 1;
                    println!(
                        "HIT   {file}: {} ({:?}, {elapsed:.1?})",
                        d.document_number, d.format
                    );
                } else {
                    println!(
                        "MISS  {file}: checksum-valid but WRONG doc number {} (expected {expected_doc})",
                        d.document_number
                    );
                }
            }
            Ok(d) => {
                println!(
                    "MISS  {file}: parsed but checksums failed: {:?} ({elapsed:.1?})",
                    d.checks
                );
                if dump {
                    println!(
                        "--- candidate ---\n{}\n--- ocr text ---\n{text}\n---",
                        d.mrz_lines
                    );
                }
            }
            Err(e) => {
                println!("MISS  {file}: {e} ({elapsed:.1?})");
                if dump {
                    println!("--- ocr text ---\n{text}\n---");
                }
            }
        }
    }
    // Denominator is the whole declared corpus, not just what was found: a
    // stale entry is a gap in coverage, and hiding it by shrinking the
    // denominator would quietly inflate the rate this harness exists to report.
    let total = corpus.len();
    let pct = 100.0 * hits as f64 / total as f64;
    println!("\nTier-1 hit rate: {hits}/{total} = {pct:.0}%");
    if missing > 0 {
        println!(
            "warning: {missing} corpus entr(y/ies) not found under samples/ — counted as misses"
        );
    }

    println!("\nNegative controls (no MRZ present — must not validate):");
    for file in &negative {
        let Some(path) = find_sample(file) else {
            println!("SKIP  {file}: not found under samples/ — stale corpus entry");
            continue;
        };
        let started = std::time::Instant::now();
        let text = match ocr.recognize(&path) {
            Ok(t) => t,
            Err(e) => {
                println!("ERROR {file}: {e}");
                continue;
            }
        };
        let elapsed = started.elapsed();
        match mrz::find_and_parse(&text) {
            Ok(d) if d.valid() => println!(
                "FALSE-POSITIVE {file}: hallucinated valid MRZ {} ({elapsed:.1?})",
                d.document_number
            ),
            _ => println!("OK    {file}: no valid MRZ ({elapsed:.1?})"),
        }
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/synthpass-ocr → repo root is two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

/// Locates `name` (a bare filename, no path) anywhere under `samples/`,
/// searching recursively — so this survives `samples/` being reorganized into
/// continent/class subfolders without any entry needing its exact subpath.
///
/// Returns `None` rather than panicking on a name that no longer exists.
/// Panicking made the first stranded name abort the whole run — including,
/// because the positive loop runs first, the negative-control sweep that never
/// got reached. A skipped specimen should cost one line of output and one entry
/// off the denominator, not the entire harness.
///
/// A *rename* used to strand an entry here, because the two sets were `const`
/// arrays of basenames written by hand: the recursive search absorbed a
/// specimen being moved but not renamed, and 16 of 21 entries had gone stale
/// that way. Both sets now come from `samples/corpus.jsonl`, which is
/// regenerated from the corpus itself, so a rename carries the link with it.
/// A `SKIP` now means the manifest is genuinely out of date — regenerate it
/// with the `corpus_manifest` example.
fn find_sample(name: &str) -> Option<PathBuf> {
    fn search(dir: &Path, name: &str) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = search(&path, name) {
                    return Some(found);
                }
            } else if path.file_name().and_then(|f| f.to_str()) == Some(name) {
                return Some(path);
            }
        }
        None
    }
    search(&repo_root().join("samples"), name)
}
