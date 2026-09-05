//! Proposes country-name vocabulary from the corpus, anchored on each
//! specimen's MRZ-proven issuing state.
//!
//! # The idea
//!
//! `synthpass_core::normalize::country_code` resolves a country *name* through
//! `mrz::code_for_name`. Documents print the *adjectival* form — `CANADIAN`,
//! `ESPAÑOLA`, `HRVATSKO` — which no name table can hold, so those fall through
//! unresolved and score as Tier-2 misses.
//!
//! The corpus can supply the missing forms without anybody guessing them: every
//! specimen carries its own country code in `samples/corpus.jsonl`, and its
//! visual zone prints that country's name in whatever language and inflection
//! the issuing state chose. Join the two and the vocabulary falls out.
//!
//! # The discriminator, and why it is not similarity
//!
//! A token is proposed for code `C` when it appears on **at least two specimens
//! of C and on no specimen of any other country**.
//!
//! Exclusivity, not string similarity, because a native-language demonym has no
//! similarity to its English name — `HRVATSKO` and `Croatia` share nothing, and
//! any edit-distance rule tuned to catch that would also match half the corpus.
//! Exclusivity is what separates `ESPANOLA` (Spanish documents only) from
//! `PASSPORT` (everywhere).
//!
//! Two specimens rather than one because a single appearance cannot be told
//! apart from an OCR artifact that happens to occur once.
//!
//! # This proposes; a person disposes
//!
//! Exclusivity is necessary and **not sufficient**: it equally proposes
//! `MADRID → ESP` and `BUNDESDRUCKEREI → DEU`, which are exclusive to those
//! countries and are not country names. The output is a candidate list for
//! review, exactly as `corpus_manifest.rs` states for the fields it cannot
//! decide. Nothing here writes to `normalize.rs`.
//!
//! Accepted candidates must then clear the measurement gate before they ship —
//! `cargo run -p synthpass-bench --example vocab_replay` requires a candidate to
//! flip at least one real Tier-2 answer and break none.
//!
//! # Output
//!
//! A TSV under `artifacts/` (gitignored), **not** the tracked
//! `knowledge/visual-zone-survey.jsonl`: that file records per-specimen counts
//! and no document text, and this tool has no business changing what the
//! repository publishes about specimen contents.
//!
//! # Usage
//!
//! ```powershell
//! cargo run -p synthpass-ocr --release --example mine_country_vocab
//! cargo run -p synthpass-ocr --release --example mine_country_vocab -- --dir passports
//! ```
//!
//! A full pass OCRs every corpus image and takes tens of minutes. Run it alone:
//! the OCR retry budget is wall-clock gated (`SYNTHPASS_OCR_MAX_SECONDS`), so a
//! contended run silently reads *less* text and mines less vocabulary.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use synthpass_ocr::geometry::{self, OcrLine};
use synthpass_ocr::NativeOcr;

/// Per-line MRZ-likelihood cutoff, matching `visual_zone_survey.rs` so the two
/// surveys agree on which lines are the machine-readable zone.
const MRZ_LINE_SCORE_THRESHOLD: f64 = 0.65;
const MRZ_CHARSET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789<";

/// Shortest token worth proposing. Three letters would sweep in every ICAO code
/// printed on the page (and the codes are already resolved by
/// `looks_like_a_code`), plus most OCR debris.
const MIN_TOKEN_LEN: usize = 4;

/// A token must appear on at least this many specimens of one country before it
/// is proposed — one appearance is indistinguishable from an OCR artifact.
const MIN_SPECIMENS: usize = 2;

const IMAGE_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "gif"];

fn main() {
    let root = repo_root();
    let only_dir = flag("--dir");

    let by_file = issuing_states(&root.join("samples").join("corpus.jsonl"));
    println!(
        "manifest rows with a usable issuing state: {}",
        by_file.len()
    );

    let mut files = Vec::new();
    walk(&root.join("samples"), &mut files);
    files.sort();
    if let Some(dir) = &only_dir {
        files.retain(|p| p.components().any(|c| c.as_os_str() == dir.as_str()));
    }
    files.retain(|p| basename(p).is_some_and(|f| by_file.contains_key(f)));
    println!("specimens to scan: {}\n", files.len());

    let ocr = NativeOcr::load(
        &root.join("text-detection.rten"),
        &root.join("text-recognition.rten"),
    )
    .expect("failed to load OCR models — run from the repo root");

    // token -> country code -> how many distinct specimens showed it.
    let mut seen: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();

    for (i, path) in files.iter().enumerate() {
        let filename = basename(path).unwrap_or("?").to_string();
        let Some(code) = by_file.get(filename.as_str()) else {
            continue;
        };
        let page = match ocr.recognize_detailed(path) {
            Ok(p) => p,
            Err(e) => {
                println!("[{}/{}] SKIP {filename}: {e}", i + 1, files.len());
                continue;
            }
        };
        let tokens = visual_zone_tokens(&page.lines);
        println!(
            "[{}/{}] {filename} ({code}) — {} candidate token(s)",
            i + 1,
            files.len(),
            tokens.len()
        );
        // Counted once per specimen, not once per occurrence: a word repeated
        // down one page is one document's worth of evidence.
        for token in tokens {
            *seen
                .entry(token)
                .or_default()
                .entry(code.clone())
                .or_default() += 1;
        }
    }

    let mut proposals: Vec<(String, String, usize)> = Vec::new();
    for (token, per_country) in &seen {
        // Exclusive to exactly one country, and seen often enough there.
        let [(code, count)] = per_country.iter().collect::<Vec<_>>()[..] else {
            continue;
        };
        if *count < MIN_SPECIMENS {
            continue;
        }
        // Already resolved upstream — proposing it would be dead weight.
        if mrz::code_for_name(token).is_some() {
            continue;
        }
        proposals.push((token.clone(), code.clone(), *count));
    }
    // Most-evidenced first: that is the order a human should review in.
    proposals.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));

    let out = root.join("artifacts").join("country-vocab-candidates.tsv");
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut body = String::from("token\tcode\tspecimens\tcountry\n");
    for (token, code, count) in &proposals {
        let name = mrz::country_name(code).unwrap_or("?");
        body.push_str(&format!("{token}\t{code}\t{count}\t{name}\n"));
    }
    if let Err(e) = std::fs::write(&out, &body) {
        eprintln!("could not write {}: {e}", out.display());
        std::process::exit(1);
    }

    println!(
        "\n=== {} candidate(s) -> {} ===",
        proposals.len(),
        out.display()
    );
    println!("token                code  n  country");
    for (token, code, count) in proposals.iter().take(60) {
        println!(
            "{token:20} {code}   {count}  {}",
            mrz::country_name(code).unwrap_or("?")
        );
    }
    println!(
        "\nThese are proposals, not vocabulary. Exclusivity also selects city and \
         printer names (MADRID, BUNDESDRUCKEREI); read the list before taking any \
         of it, then prove the ones you take with `vocab_replay`."
    );
}

/// Uppercase alphabetic tokens from the specimen's **non-MRZ** lines.
///
/// The MRZ is excluded deliberately: it prints the ICAO code, not the country's
/// own word for itself, so mining it would just relearn `mrz::code_for_name`.
fn visual_zone_tokens(lines: &[OcrLine]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in lines {
        if geometry::mrz_line_score(line, MRZ_CHARSET) >= MRZ_LINE_SCORE_THRESHOLD {
            continue;
        }
        for token in line.text.split(|c: char| !c.is_ascii_alphabetic()) {
            if token.len() >= MIN_TOKEN_LEN {
                out.insert(token.to_ascii_uppercase());
            }
        }
    }
    out
}

/// `filename -> issuing state` for every manifest row whose state this
/// workspace recognises.
///
/// Read from the manifest rather than from the OCR: `samples/corpus.jsonl` is
/// the reviewed source of truth, and its own doc states the filename wins over
/// a disagreeing OCR read of line 1 — which is exactly the field being joined
/// on here.
fn issuing_states(manifest: &Path) -> BTreeMap<String, String> {
    let Ok(text) = std::fs::read_to_string(manifest) else {
        eprintln!("cannot read {}", manifest.display());
        std::process::exit(1);
    };
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let Ok(row) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let (Some(filename), Some(state)) = (
            row.get("filename").and_then(|v| v.as_str()),
            row.pointer("/mrz/issuing_state").and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        let state = state.trim_end_matches('<');
        if mrz::country_name(state).is_some() {
            out.insert(filename.to_string(), state.to_string());
        }
    }
    out
}

fn basename(p: &Path) -> Option<&str> {
    p.file_name().and_then(|f| f.to_str())
}

fn flag(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.as_str()))
        {
            out.push(path);
        }
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}
