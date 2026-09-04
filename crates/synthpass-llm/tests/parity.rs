//! Field-level parity harness: compares `NativeLlm` extraction against
//! MRZ-derived ground truth under `samples/ocr_fixtures/`. Ignored by default —
//! needs the ~1 GB GGUF at the repo root; run explicitly with:
//!
//! ```sh
//! cargo test -p synthpass-llm --test parity -- --ignored --nocapture
//! ```
//!
//! This is a regression smoke check, not an accuracy gate: a small 1.5B model
//! reading OCR'd text will not hit 100% field accuracy (e.g. it may return
//! `"CROATIA"` where the fixture has the ISO code `"HRV"`), so this asserts a
//! minimum per-field match rate across the whole sample set rather than exact
//! equality per file. A regression that tanks the match rate is the signal to
//! watch for — not any single field on any single document.
//!
//! Measured baseline, 2026-09-04, `qwen2.5-1.5b-instruct-q4_k_m`, ~35 min:
//! **reviewed 85/162 (52.5%) over 18 documents, derived 85/162 (52.5%) over
//! 54.** Full record, including the MRZ-holdout arm, in
//! `knowledge/benchmarks/parity-mrz-holdout-2026-09-04.md`.
//!
//! That is up from 78/162 (48.1%) on the same corpus and model earlier the
//! same day. Nothing about the model changed: `synthpass_core::normalize`
//! learned two printed date forms it had been discarding — whitespace-separated
//! numerics and a named month with a two-digit year — and the +14 landed as
//! exactly +7 in each fixture set. See
//! `knowledge/benchmarks/normalize-date-forms-2026-09-04.md`. Both numbers are
//! quoted here because a baseline that silently tracks upward stops being a
//! baseline.
//!
//! # Normalize before comparing, or this harness lies
//!
//! This file previously scored the model's raw output and recorded 26.5% —
//! about 20 points low. Its own doc comment argued for that, and the argument
//! was wrong in one specific way worth preserving:
//!
//! > *The harness deliberately does not normalise them away: doing so would
//! > raise the number without changing what the pipeline actually receives.*
//!
//! The pipeline **does** normalise. `synthpass_core::normalize::extraction`
//! runs on every Tier-2 result in both `process_document` and
//! `process_document_stream`, so `"PASSPORT"` reaches a caller as `"P"` and
//! `"CROATIA"` as `"HRV"`. Declining to normalise here did not measure the
//! shipped path more honestly — it measured a value the product never emits,
//! and reported failures that do not exist. The false premise came from
//! `normalize.rs`'s own module header, which still claimed it was "not wired
//! into the pipeline" long after it was.
//!
//! So: normalise both sides, exactly as production does, and let the remaining
//! disagreements be real ones. The four fields no normaliser touches
//! (`document_number`, `surname`, `given_names`, `sex`) scored identically
//! before and after the fix, which is the control that proves this changed the
//! measurement rather than the answer.
//!
//! # Two fixture sets, because ground truth is not uniform
//!
//! Fixtures come in two kinds, and conflating them would make this harness lie:
//!
//! | Directory | Origin | Fields scored |
//! | :-- | :-- | :-- |
//! | `samples/ocr_fixtures/` | hand-verified against the image | all nine prompt fields |
//! | `samples/ocr_fixtures/derived/` | generated from a checksum-valid MRZ | the check-digited fields only |
//!
//! ICAO 9303 check-digits exactly `document_number`, `date_of_birth`,
//! `date_of_expiry` and `personal_number`. A derived fixture's *name* is
//! therefore not truth — it is one OCR pass's reading of characters no check
//! digit covers, on exactly the positions this corpus shows the recogniser
//! getting wrong. Scoring a model against those would invert the measurement:
//! a model that read the visual zone correctly would be marked wrong for
//! disagreeing with an OCR error, so improving visual-zone handling — the whole
//! point of `knowledge/VIZ_TIER2_DESIGN.md` — would show up here as a
//! regression. The split is what keeps that from happening while still letting
//! the denominator grow from six documents to a hundred.
//!
//! Promoting a candidate is `git mv samples/ocr_fixtures/derived/<stem>.json
//! samples/ocr_fixtures/`, once a person has compared its name fields against
//! the image. Every promotion moves seven more fields into the scored set.
//!
//! # A known bias in this corpus
//!
//! Every fixture here has a checksum-valid MRZ — that is where the ground truth
//! comes from. But Tier 2 only runs when Tier 1 *fails*, so in production this
//! model never sees a document like these. The harness measures "can the model
//! extract fields from an ID document's OCR text", which is the right question
//! for a regression check and the wrong one for predicting escalation accuracy.
//! `knowledge/VIZ_TIER2_DESIGN.md` §5 describes the MRZ-holdout variant that
//! closes the gap: strip the MRZ lines from the input and keep the ground truth,
//! which turns a Tier-1 success into an honest Tier-2 test case.

use std::path::PathBuf;
use std::time::Instant;
use synthpass_core::v2::{CoreField, ExtractionV2, FieldConfidence};
use synthpass_core::Extraction;
use synthpass_llm::NativeLlm;

/// The fields `prompt::build_prompt` actually asks the model for, minus
/// `mrz_line` (a raw zone, not a field). Scoring anything it never requested
/// would measure the harness, not the model.
const SCORED: [CoreField; 9] = [
    CoreField::DocumentType,
    CoreField::IssuingCountry,
    CoreField::DocumentNumber,
    CoreField::Surname,
    CoreField::GivenNames,
    CoreField::Nationality,
    CoreField::DateOfBirth,
    CoreField::Sex,
    CoreField::DateOfExpiry,
];

/// The seven fields this harness scored before the fixture set was split, kept
/// as a second reported figure so the historical baseline stays comparable
/// across the change. Dropping it once a run or two has been recorded against
/// the nine-field number is fine; silently changing what the number means is
/// not.
const LEGACY_SCORED: [CoreField; 7] = [
    CoreField::DocumentNumber,
    CoreField::Surname,
    CoreField::GivenNames,
    CoreField::Nationality,
    CoreField::DateOfBirth,
    CoreField::Sex,
    CoreField::DateOfExpiry,
];

/// True when an ICAO check digit mathematically covers this field, so a
/// generated fixture may assert it without a human having looked.
///
/// Read from [`FieldConfidence::mrz_checksum_scope`] rather than restated:
/// that constructor is the crate's single answer to "what does a check digit
/// prove", and a second copy of the answer here would eventually disagree with
/// it. `PROVEN` is 1.0 and the maximum, so the comparison is exact.
fn is_checksum_proven(field: CoreField) -> bool {
    FieldConfidence::mrz_checksum_scope().score(field) >= 1.0
}

/// One `.md` (the model's input) plus the `.json` it is scored against.
struct Fixture {
    stem: String,
    markdown: PathBuf,
    truth: PathBuf,
    /// A person compared this fixture's unproven fields against the image.
    reviewed: bool,
}

impl Fixture {
    fn scored_fields(&self) -> Vec<CoreField> {
        SCORED
            .into_iter()
            .filter(|f| self.reviewed || is_checksum_proven(*f))
            .collect()
    }
}

/// Collects every fixture pair under `samples/ocr_fixtures/`.
///
/// Read from the directories rather than from `samples/corpus.jsonl`'s
/// `ground_truth_stem` links, which is where this list came from before. Both
/// survive a corpus rename — the reason that link was introduced — but a
/// fixture pair is self-contained: the `.md` is the model's input and the
/// `.json` is the answer, and neither needs the image to exist. Listing the
/// directory also picks up a promotion (`git mv` out of `derived/`) with no
/// second step, where the manifest route would silently keep scoring the
/// promoted fixture as unreviewed until someone regenerated the manifest.
fn fixtures() -> Vec<Fixture> {
    let dir = repo_root().join("samples").join("ocr_fixtures");
    let mut out = Vec::new();
    for (base, reviewed) in [(dir.clone(), true), (dir.join("derived"), false)] {
        let Ok(entries) = std::fs::read_dir(&base) else {
            continue;
        };
        for path in entries.flatten().map(|e| e.path()) {
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let markdown = base.join(format!("{stem}.md"));
            if !markdown.is_file() {
                // A label with no OCR text has nothing to feed the model.
                // Regenerate with `cargo run -p synthpass-bench --release
                // --example ground_truth_candidates`.
                eprintln!("skipping {stem}: no {stem}.md beside the label");
                continue;
            }
            out.push(Fixture {
                stem: stem.to_string(),
                markdown,
                truth: path.clone(),
                reviewed,
            });
        }
    }
    out.sort_by(|a, b| a.stem.cmp(&b.stem));
    out
}

/// Case- and whitespace-insensitive comparison of two already-normalized
/// values.
///
/// This used to carry a local `normalize_date` handling only `DD.MM.YYYY`,
/// which made the harness score a *less* normalized value than production
/// emits: `synthpass_core::normalize::extraction` runs on every Tier-2 result
/// in both pipeline paths, and the harness did not run it at all. Measured
/// over the 72-fixture corpus, 64% of date mismatches were the correct date in
/// a format the local helper could not read — the harness was reporting model
/// failures that the shipped product does not have. The fix is at the call
/// site (normalize the extraction, exactly as the pipeline does); what remains
/// here is only the trivial case/whitespace fold.
fn normalize(s: &str) -> String {
    s.trim().to_uppercase()
}

fn fields_match(a: Option<&str>, b: Option<&str>, _field: CoreField) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => normalize(a) == normalize(b),
        (None, None) => true,
        _ => false,
    }
}

/// Running match tally, per field and overall.
#[derive(Default)]
struct Tally {
    documents: usize,
    matched: usize,
    total: usize,
    per_field: std::collections::BTreeMap<&'static str, (usize, usize)>,
}

impl Tally {
    fn record(&mut self, field: CoreField, ok: bool) {
        self.total += 1;
        self.matched += ok as usize;
        let entry = self.per_field.entry(field.as_str()).or_default();
        entry.0 += ok as usize;
        entry.1 += 1;
    }

    fn rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.matched as f64 / self.total as f64
    }

    /// Fails when the match rate drops below `floor`.
    ///
    /// An **empty** set is skipped rather than failed. A rate of zero over zero
    /// fields is not a regression — it means the fixtures aren't on disk, which
    /// is a checkout problem and deserves to say so instead of masquerading as a
    /// model that got worse. (Reported rather than silent: a set that quietly
    /// stops contributing is how a measurement rots.)
    fn assert_floor(&self, label: &str, floor: f64) {
        if self.total == 0 {
            println!(
                "\n{label} fixtures: none found — floor not checked. \
                 Regenerate with `cargo run -p synthpass-bench --release \
                 --example ground_truth_candidates`."
            );
            return;
        }
        assert!(
            self.rate() >= floor,
            "{label}-fixture accuracy regressed: {}/{} ({:.1}%) below the {:.0}% floor",
            self.matched,
            self.total,
            self.rate() * 100.0,
            floor * 100.0
        );
    }

    fn print(&self, label: &str) {
        println!(
            "\n{label}: {} document(s), {}/{} fields ({:.1}%)",
            self.documents,
            self.matched,
            self.total,
            self.rate() * 100.0
        );
        for (field, (ok, n)) in &self.per_field {
            println!(
                "  {field:<16} {ok:>3}/{n:<3} ({:.0}%)",
                if *n == 0 {
                    0.0
                } else {
                    *ok as f64 / *n as f64 * 100.0
                }
            );
        }
    }
}

// ─── MRZ holdout (knowledge/VIZ_TIER2_DESIGN.md §5.2) ────────────────────
//
// Every fixture here has a checksum-valid MRZ — that is where its ground
// truth comes from. But Tier 2 only runs when Tier 1 *fails*, so this corpus
// is by construction the set of documents Tier 2 never sees in production.
// That is fine for a regression check and wrong for predicting escalation
// accuracy, which is what the VIZ work needs.
//
// The holdout closes it using the same fixtures: feed the model the OCR text
// with the MRZ-shaped lines removed, and score against the ground truth those
// lines produced. The input becomes the escalation case; the truth stays
// checksum-proven because it was derived before the holdout; and the proven
// fields are all printed in the visual zone in human-readable form, so they
// remain recoverable in principle.
//
// Implemented as a transform on the markdown string rather than §5.2's
// "flag on the generator" plus a variant fixture set. Two reasons: the
// generator needs the source images, which live on the `samples-data` branch
// and are not present in a normal checkout; and a transform is
// byte-deterministic where a re-OCR pass explicitly is not, so the holdout
// input is reproducible from what is already committed.

/// The ICAO 9303 MRZ character set, spelled out locally.
///
/// `synthpass-imageprep` exports the canonical copy, but `synthpass-llm` does
/// not depend on it and adding a dependency to the whole crate for one test
/// constant is the wrong trade. Drift is not a real risk: this set is fixed by
/// the standard.
const HOLDOUT_MRZ_CHARSET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789<";

/// Minimum non-whitespace length before a line can be stripped on shape alone.
/// TD1's lines are 30 characters and OCR truncates, so this sits below that
/// with margin while staying clear of ordinary visual-zone labels.
const HOLDOUT_MIN_LEN: usize = 25;

/// Minimum fraction of a line's characters that must be in the MRZ charset for
/// the shape half of the predicate. Deliberately applied **without**
/// upper-casing first: an MRZ is uppercase-only, so lowercase is evidence
/// *against* a line being MRZ. Normalising case here would let ordinary prose
/// like `Passaport/Passport/Passeport` score as MRZ and strip visual-zone text
/// the measurement depends on.
const HOLDOUT_MIN_CHARSET_FRACTION: f64 = 0.85;

/// Minimum similarity to a ground-truth MRZ line for the similarity half.
///
/// Measured over all 72 fixtures (2970 lines): the shape half alone strips 400
/// lines but **misses 17** that this half catches — short, badly-corrupted
/// fragments such as `KOVACEVIC<AZRA<MARINA<<<` (0.89) and
/// `MANOLECORINA<IOANA<<<<` (0.85), which leak surname and given names
/// outright. Leaving those in would hand the model the answer and inflate the
/// holdout score, which is the one failure mode that silently invalidates the
/// whole measurement.
const HOLDOUT_MIN_SIMILARITY: f64 = 0.60;

/// `SYNTHPASS_PARITY_HOLDOUT=1` runs the parity test in holdout mode.
fn holdout_enabled() -> bool {
    std::env::var("SYNTHPASS_PARITY_HOLDOUT").is_ok_and(|v| v.trim() == "1")
}

/// The shape half of the strip predicate: long enough, and overwhelmingly
/// MRZ-charset.
fn looks_like_mrz_shape(line: &str) -> bool {
    let compact: Vec<char> = line.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.len() < HOLDOUT_MIN_LEN {
        return false;
    }
    let hits = compact
        .iter()
        .filter(|c| HOLDOUT_MRZ_CHARSET.contains(**c))
        .count();
    hits as f64 / compact.len() as f64 >= HOLDOUT_MIN_CHARSET_FRACTION
}

/// Longest-common-subsequence similarity, `2*LCS / (len(a) + len(b))`.
///
/// Hand-rolled rather than pulled in: lines are at most a few dozen characters
/// against a 44-character MRZ, so the quadratic form is free, and
/// `synthpass-llm` gains no dependency for a test helper. Whitespace is
/// dropped first because OCR sprays spaces through the filler runs.
fn similarity(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().filter(|c| !c.is_whitespace()).collect();
    let b: Vec<char> = b.chars().filter(|c| !c.is_whitespace()).collect();
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let mut prev = vec![0usize; b.len() + 1];
    let mut cur = vec![0usize; b.len() + 1];
    for ca in &a {
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = if ca == cb {
                prev[j] + 1
            } else {
                cur[j].max(prev[j + 1])
            };
        }
        std::mem::swap(&mut prev, &mut cur);
        cur.iter_mut().for_each(|v| *v = 0);
    }
    2.0 * prev[b.len()] as f64 / (a.len() + b.len()) as f64
}

/// Remove every line that is MRZ-shaped **or** recognisably a corruption of
/// this fixture's known MRZ. Returns the surviving text and how many lines
/// went.
///
/// The union is deliberately biased toward stripping. The two errors are not
/// symmetric: a leaked MRZ fragment hands the model the answer and inflates
/// the score, which is fatal to what the number means, whereas an
/// over-stripped visual-zone line merely removes context and makes the result
/// conservative. Spot-checked over the corpus, the lines the shape half takes
/// that the similarity half does not are mostly heavily-corrupted MRZ reads
/// (`P<CANMARTIN<SARAKKKKKKKKKKK<K<K<KK<K<`), with some genuine visual-zone
/// text (`ENDORSEMENTS AND LIMITATIONS`) — and crucially none of that text
/// carries the checksum-proven fields, so losing it costs context, not
/// answers.
fn strip_mrz_lines(markdown: &str, mrz_line: Option<&str>) -> (String, usize) {
    let truth: Vec<&str> = mrz_line
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let mut removed = 0usize;
    let kept: Vec<&str> = markdown
        .lines()
        .filter(|line| {
            let t = line.trim();
            if t.is_empty() {
                return true;
            }
            let strip = looks_like_mrz_shape(t)
                || truth
                    .iter()
                    .any(|m| similarity(t, m) >= HOLDOUT_MIN_SIMILARITY);
            if strip {
                removed += 1;
            }
            !strip
        })
        .collect();
    (kept.join("\n"), removed)
}

#[test]
fn holdout_strips_mrz_lines_and_keeps_visual_zone_text() {
    let truth = "PPCANMARTIN<<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<\n\
                 P001756ZA5CAN9505047F3303034<<<<<<<<<<<<<<06";
    let md = "Passport / Passeport\n\
              PPCANMARTIN?<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<\n\
              POO1756ZA\n\
              PPCANMARTIN<SARAHCKKKKKKKKKKKKK\n\
              P001756ZA5CAN9505047F3303034<<<<<<<<<<06\n\
              Date of birth / Date de naissance";
    let (out, removed) = strip_mrz_lines(md, Some(truth));
    assert_eq!(removed, 3, "three MRZ-derived lines should go");
    assert!(
        out.contains("POO1756ZA"),
        "the human-readable visual-zone document number is the signal being \
         measured and must survive: {out}"
    );
    assert!(out.contains("Date of birth / Date de naissance"));
    assert!(out.contains("Passport / Passeport"));
    assert!(
        !out.contains("CAN9505047F3303034"),
        "no MRZ payload may leak: {out}"
    );
}

#[test]
fn holdout_catches_short_fragments_the_shape_test_misses() {
    // Real leak from the corpus: 24 chars, so under HOLDOUT_MIN_LEN, but it
    // spells out surname and given names.
    let fragment = "KOVACEVIC<AZRA<MARINA<<<";
    assert!(
        !looks_like_mrz_shape(fragment),
        "precondition: the shape half does not catch this"
    );
    let truth = "IDBIHKOVACEVIC<<AZRA<MARINA<<<<<<<<<<<<<<<<<<";
    let (out, removed) = strip_mrz_lines(fragment, Some(truth));
    assert_eq!(removed, 1, "the similarity half must catch it");
    assert!(out.trim().is_empty());
}

#[test]
fn holdout_does_not_strip_lowercase_prose() {
    // Uppercasing before the charset test would let this score as MRZ.
    let line = "Passaport/Passport/Passeport";
    assert!(!looks_like_mrz_shape(line));
    let (out, removed) = strip_mrz_lines(line, Some("PPCANMARTIN<<SARAH<<<<<<<<<<"));
    assert_eq!(removed, 0);
    assert_eq!(out.trim(), line);
}

/// Pre-flight for the holdout run: every fixture on disk must actually lose
/// its MRZ, and must keep something for the model to read.
///
/// This is cheap (no model, milliseconds) and it guards the expensive test
/// against its own worst failure mode. A fixture whose MRZ survives the strip
/// is not a holdout case at all — the model is handed the answer and its score
/// inflates the total silently. Catching that here costs nothing; catching it
/// after a ~31-minute run costs the run.
#[test]
fn holdout_strips_every_fixture_on_disk() {
    let fixtures = fixtures();
    if fixtures.is_empty() {
        println!("no fixtures on disk — skipping holdout pre-flight");
        return;
    }

    let mut leaks = Vec::new();
    let mut emptied = Vec::new();
    let (mut total_removed, mut total_kept) = (0usize, 0usize);

    for fixture in &fixtures {
        let Ok(markdown) = std::fs::read_to_string(&fixture.markdown) else {
            continue;
        };
        let truth: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&fixture.truth).expect("json reads"))
                .expect("fixture is valid JSON");
        let mrz = truth.get("mrz_line").and_then(|v| v.as_str());

        let (kept, removed) = strip_mrz_lines(&markdown, mrz);
        total_removed += removed;
        total_kept += kept.lines().filter(|l| !l.trim().is_empty()).count();

        if removed == 0 {
            leaks.push(fixture.stem.clone());
        }
        if kept.trim().is_empty() {
            emptied.push(fixture.stem.clone());
        }
    }

    println!(
        "holdout pre-flight: {} fixtures, {total_removed} line(s) stripped, \
         {total_kept} line(s) kept for the model",
        fixtures.len()
    );

    assert!(
        leaks.is_empty(),
        "these fixtures kept their MRZ and would inflate a holdout run: {leaks:?}"
    );
    // An emptied fixture is the opposite failure: nothing survives for the
    // model to read, so it scores zero for a reason that has nothing to do
    // with visual-zone recovery.
    assert!(
        emptied.is_empty(),
        "these fixtures were stripped to nothing, leaving no visual zone to \
         measure: {emptied:?}"
    );
}

#[test]
fn similarity_is_bounded_and_reflexive() {
    assert!((similarity("ABC<<<", "ABC<<<") - 1.0).abs() < 1e-9);
    assert_eq!(similarity("", "ABC"), 0.0);
    assert!(similarity("ABCDEF", "ZZZZZZ") < 0.2);
}

#[test]
#[ignore]
fn native_llm_field_accuracy_over_sample_set() {
    let model_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../qwen2.5-1.5b-instruct-q4_k_m.gguf");
    assert!(
        model_path.exists(),
        "model not found at {} — download it first",
        model_path.display()
    );
    let llm = NativeLlm::load(&model_path, 2048).expect("model loads");

    let fixtures = fixtures();
    assert!(
        !fixtures.is_empty(),
        "no fixtures found — samples/ocr_fixtures/ is tracked on main, and \
         `cargo run -p synthpass-bench --release --example ground_truth_candidates` \
         regenerates it"
    );

    let holdout = holdout_enabled();
    let mut holdout_removed = 0usize;
    let mut holdout_leaks = 0usize;
    if holdout {
        println!(
            "\nMRZ HOLDOUT MODE (SYNTHPASS_PARITY_HOLDOUT=1): MRZ-derived lines \
             are stripped from every fixture's OCR text before the model sees \
             it. Scores are NOT comparable to the standard baseline — see \
             knowledge/VIZ_TIER2_DESIGN.md §5.2."
        );
    }

    let mut reviewed = Tally::default();
    let mut derived = Tally::default();
    let mut legacy = Tally::default();

    // Progress accounting. This run takes tens of minutes and is routinely
    // interrupted; without a counter and an ETA the per-fixture output tells
    // you *what* it was doing but not *how far in*, so a killed run can't be
    // told apart from a stalled one, and there is no way to judge whether
    // waiting is worthwhile. Cheap to print, and the difference between
    // "died at fixture 3" and "died at 49 of 72".
    let run_started = Instant::now();
    let total_fixtures = fixtures.len();

    for (index, fixture) in fixtures.iter().enumerate() {
        let fixture_started = Instant::now();
        let markdown = std::fs::read_to_string(&fixture.markdown).expect("markdown reads");
        // Ground-truth fixtures predate `extraction_method` being required;
        // backfill it the same way `repair::parse_extraction` does for model
        // output, so this harness doesn't need its own fixture format.
        let mut expected_value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&fixture.truth).expect("json reads"))
                .expect("fixture is valid JSON");
        expected_value
            .as_object_mut()
            .expect("fixture is a JSON object")
            .entry("extraction_method")
            .or_insert_with(|| "mrz-deterministic".into());
        let mut expected: Extraction =
            serde_json::from_value(expected_value).expect("fixture parses as Extraction");
        // Normalized on both sides, so a fixture written in a slightly
        // different-but-equivalent form cannot score as a model error. The
        // normalizer is idempotent (`normalize::date_is_idempotent` and
        // friends), so this is a no-op on a fixture already in canonical form
        // — which every generated one is.
        synthpass_core::normalize::extraction(&mut expected);

        // Holdout: remove the MRZ the ground truth came from, so the model is
        // scored on what it can recover from the visual zone alone.
        let markdown = if holdout {
            let (stripped, removed) = strip_mrz_lines(&markdown, expected.mrz_line.as_deref());
            if removed == 0 {
                // Not a warning to skim past: zero removals means this
                // fixture's MRZ is still in the model's input, so its score is
                // measuring the wrong thing and silently inflating the total.
                println!(
                    "  HOLDOUT LEAK {}: no MRZ-derived lines removed",
                    fixture.stem
                );
                holdout_leaks += 1;
            }
            holdout_removed += removed;
            stripped
        } else {
            markdown
        };

        let mut actual = llm.extract(&markdown, None).expect("extraction succeeds");
        // Both pipeline entry points call this on every Tier-2 result before
        // anything downstream sees it (`synthpass-pipeline/src/lib.rs`), so a
        // harness that skipped it would be scoring a value the product never
        // emits. Measuring anything other than what ships is the one way this
        // number can be confidently wrong.
        synthpass_core::normalize::extraction(&mut actual);

        // Field lookup goes through the v2 lift so this file holds no third
        // copy of the `CoreField` -> struct-field mapping.
        let expected = ExtractionV2::from(&expected);
        let actual = ExtractionV2::from(&actual);

        let tally = if fixture.reviewed {
            &mut reviewed
        } else {
            &mut derived
        };
        tally.documents += 1;

        // ETA from the mean so far rather than the last document: per-fixture
        // cost varies several-fold with OCR text length, so a trailing
        // estimate would swing wildly and read as noise.
        let done = index + 1;
        let elapsed = run_started.elapsed();
        let eta = if done > 0 && done < total_fixtures {
            let mean = elapsed.as_secs_f64() / done as f64;
            format!(", eta {:.0}m", mean * (total_fixtures - done) as f64 / 60.0)
        } else {
            String::new()
        };
        println!(
            "--- [{done}/{total_fixtures}] {} ({}) {:.1?}, {:.0}m elapsed{eta} ---",
            fixture.stem,
            if fixture.reviewed {
                "reviewed"
            } else {
                "derived"
            },
            fixture_started.elapsed(),
            elapsed.as_secs_f64() / 60.0,
        );
        for field in fixture.scored_fields() {
            let exp = expected.fields.get(field);
            let act = actual.fields.get(field);
            let ok = fields_match(exp, act, field);
            tally.record(field, ok);
            if fixture.reviewed && LEGACY_SCORED.contains(&field) {
                legacy.record(field, ok);
            }
            println!(
                "  {:<16} expected={:?} actual={:?} {}",
                field.as_str(),
                exp,
                act,
                if ok { "OK" } else { "MISMATCH" }
            );
        }
    }
    legacy.documents = reviewed.documents;

    reviewed.print("reviewed fixtures (all nine prompt fields)");
    derived.print("derived fixtures (check-digited fields only)");
    println!(
        "\nlegacy seven-field rate over reviewed fixtures: {}/{} ({:.1}%)",
        legacy.matched,
        legacy.total,
        legacy.rate() * 100.0
    );
    // The Atlas §8 acceptance criterion, reported rather than asserted: with
    // grammar-constrained decoding on, model output should already be a valid
    // JSON object, so nothing should reach the repair path. Printing it here
    // makes "the grammar is earning its keep" a measurement instead of a
    // belief — and makes it visible when it *isn't*.
    println!(
        "repair fallbacks: {} (grammar {})",
        synthpass_llm::repair::repair_fallbacks(),
        if std::env::var("SYNTHPASS_LLM_GRAMMAR").as_deref() == Ok("0") {
            "disabled"
        } else {
            "enabled"
        }
    );

    // Two floors, because the two sets ask different questions and pooling them
    // would let a large easy set hide a regression in a small hard one.
    //
    // Both sit far below the measured baseline (52.5% / 52.5%, recorded in
    // `knowledge/benchmarks/parity-mrz-holdout-2026-09-04.md`), and that
    // distance is deliberate. These catch a *broken* prompt or a repair-JSON
    // bug — failures that take the rate to near zero — not drift.
    // `knowledge/technical_debt.md` reached the same conclusion from the other
    // direction: "a single pass/fail at a 25% floor would not have caught
    // anything here either. Recording the rate over time is the more valuable
    // half."
    //
    // The floor was lowered 0.25 -> 0.15 when the scored set widened from seven
    // fields to nine, against a then-measured 26.5%. That measurement was an
    // artifact of this harness not normalizing (see the module docs); the real
    // rate was 48.1% all along, so the headroom the lower floor was buying was
    // never as thin as it looked. Left at 0.15 regardless: nothing here argues
    // for a tighter gate, and a floor that tracks the current number upward
    // stops being a floor and becomes a ratchet.
    if holdout {
        println!(
            "\nholdout: {holdout_removed} MRZ-derived line(s) stripped across \
             {} fixture(s); {holdout_leaks} fixture(s) had none removed",
            fixtures.len()
        );
        assert_eq!(
            holdout_leaks, 0,
            "{holdout_leaks} fixture(s) kept their MRZ in the model's input — \
             those documents are not holdout cases and their scores inflate the \
             totals, so the run does not measure what it claims to"
        );
        // Deliberately no accuracy floor here. The floors below are a
        // regression gate for the standard corpus; a holdout run is a
        // *measurement* of a harder task, and a low number is the finding
        // rather than a failure. Asserting one would either be vacuous or
        // convert a legitimate result into a red build.
        println!(
            "holdout: accuracy floors not applied — this is a measurement, \
             not a regression gate."
        );
        return;
    }

    reviewed.assert_floor("reviewed", 0.15);
    derived.assert_floor("derived", 0.15);
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
