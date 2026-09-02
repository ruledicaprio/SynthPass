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
//! Measured baseline, 2026-09-02, `qwen2.5-1.5b-instruct-q4_k_m`, ~31 min:
//! **reviewed 43/162 (26.5%) over 18 documents, derived 47/162 (29.0%) over
//! 54.** The two sets agree closely on the one field they both score
//! (`document_number`: 50% reviewed, 54% derived), which is the check that the
//! generated half behaves like the hand-verified half rather than being easier.
//! Per-field numbers and the full record are in `knowledge/ROADMAP.md`.
//!
//! Two of the nine fields are weak for a reason worth reading before treating
//! them as model failure: `document_type` scores 6% because the model answers
//! `"PASSPORT"` where the MRZ says `"P"`, and `issuing_country` 22% because it
//! answers `"CROATIA"` where the MRZ says `"HRV"`. Those are format
//! disagreements, not wrong answers, and they are exactly what the narrow-ask
//! work in `knowledge/VIZ_TIER2_DESIGN.md` §2.3 addresses — the prompt never
//! says which form it wants. The harness deliberately does **not** normalise
//! them away: doing so would raise the number without changing what the
//! pipeline actually receives.
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

fn normalize(s: &str) -> String {
    s.trim().to_uppercase()
}

/// `DD.MM.YYYY` -> `YYYY-MM-DD`, the model's favorite date rendering vs. the
/// fixtures' ISO form. Falls back to the input unchanged if it isn't that shape.
fn normalize_date(s: &str) -> String {
    let parts: Vec<&str> = s.trim().split('.').collect();
    if let [d, m, y] = parts[..] {
        if d.len() <= 2 && m.len() <= 2 && y.len() == 4 {
            return format!("{y}-{m:0>2}-{d:0>2}");
        }
    }
    normalize(s)
}

fn fields_match(a: Option<&str>, b: Option<&str>, field: CoreField) -> bool {
    let is_date = matches!(field, CoreField::DateOfBirth | CoreField::DateOfExpiry);
    match (a, b) {
        (Some(a), Some(b)) if is_date => normalize_date(a) == normalize_date(b),
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

    let mut reviewed = Tally::default();
    let mut derived = Tally::default();
    let mut legacy = Tally::default();

    for fixture in &fixtures {
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
        let expected: Extraction =
            serde_json::from_value(expected_value).expect("fixture parses as Extraction");
        let actual = llm.extract(&markdown, None).expect("extraction succeeds");

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
        println!(
            "--- {} ({}) ---",
            fixture.stem,
            if fixture.reviewed {
                "reviewed"
            } else {
                "derived"
            }
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
    // Both sit far below the measured baseline (26.5% / 29.0%, recorded in
    // `knowledge/ROADMAP.md`), and that distance is deliberate. These catch a
    // *broken* prompt or a repair-JSON bug — failures that take the rate to
    // near zero — not drift. `knowledge/technical_debt.md` reached the same
    // conclusion from the other direction: "a single pass/fail at a 25% floor
    // would not have caught anything here either. Recording the rate over time
    // is the more valuable half."
    //
    // The old floor was 0.25 against a then-measured 35.7% over seven fields.
    // Keeping 0.25 while widening the scored set to nine would have *tightened*
    // the gate by accident — 26.5% leaves it barely two fields of headroom on a
    // 162-field denominator, which is a tripwire, not a floor.
    reviewed.assert_floor("reviewed", 0.15);
    derived.assert_floor("derived", 0.15);
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
