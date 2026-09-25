//! `synthpass-bench` — dev/CI corpus runner for M4 (Regression &
//! Benchmarking). Not a subcommand of the shipped `synthpass` CLI: this is a
//! vendor-side measurement tool, same separation `synthpass-license-issuer`
//! already demonstrates for a non-user-facing binary living in its own
//! crate.
//!
//! Generates a fixed, deterministic corpus from seeds `seed..seed+count`,
//! runs each through [`synthpass_bench::check_document`], and writes a
//! generated JSON report — the report is never hand-edited, only produced by
//! this binary, so it stays an honest reflection of the last real run.
//!
//! ```text
//! synthpass-bench [--count N] [--seed N] [--profile NAME] [--document-type TYPE]
//!                 [--out PATH] [--min-hit-rate F] [--dump-ocr] [--escalation-report]
//!   --count N            number of documents to check (default: 100)
//!   --seed N             base seed; document i uses seed N+i (default: 0)
//!   --profile NAME       clean|mobile|scanner|worn|border-kiosk|all (default: clean)
//!                        "all" round-robins the five profiles across the corpus
//!   --document-type TYPE td1|td2|td3|mrva|mrvb — the ICAO 9303 MRZ format to
//!                        *generate* (default: td3); a run is always a single format, so
//!                        the per-format Tier-1 hit rate is one run per type
//!   --out PATH           report JSON path (default: artifacts/bench-report.json)
//!   --min-hit-rate F     exit non-zero if the measured hit rate is below F
//!                        (e.g. 0.35); unset means "measure and report only"
//!   --dump-ocr           print every document's raw OCR text (one printed
//!                        line per detected line) before the summary — a
//!                        small-`--count` diagnostic, not for a full run
//!   --escalation-report  Tier-2 escalation rate under the default routing
//!                        policy vs. Chunk 7's composite-only opt-in, and
//!                        whether the newly-accepted documents are correct
//! ```

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use synthpass_bench::{check_document, generate_corpus, miss_kind, ProfileChoice};
use synthpass_gen::DocumentType;
use synthpass_ocr::NativeOcr;

struct Args {
    count: u64,
    seed: u64,
    profile: ProfileChoice,
    out: String,
    min_hit_rate: Option<f64>,
    /// The ICAO 9303 MRZ format to generate — `--document-type`, never
    /// `--format`: `provider-bench --format` already means `SpecimenClass`
    /// (which `samples/` directory to read), a different axis entirely (see
    /// `synthpass_bench::SpecimenClass`'s doc comment).
    document_type: DocumentType,
    /// Print the raw OCR text — one printed line per line `mrz::find_and_parse`
    /// saw — for every document in the run, before the hit/miss summary.
    ///
    /// Added for the M6 TD1 root-cause diagnosis: TD1's hit rate was 26.7%
    /// against TD2/TD3's 76-80%, and nothing in this binary's output showed
    /// *what OCR actually returned* — only the parsed-or-not result. A wrong
    /// MRZ-row assignment (the actual TD1 bug: the name line comes back as
    /// the watermark or a duplicate of line 1) is invisible in a hit/miss
    /// count and in per-field CER alike; it is only visible in the raw text.
    /// Meant for a small `--count` (3-10) — this prints unconditionally for
    /// every document, so it is a diagnostic run, not something to leave on
    /// for a 100-document corpus.
    dump_ocr: bool,
    /// Print the Tier-2 escalation rate under the shipped default routing
    /// policy *and* under Chunk 7's `accept_composite_only_failure` opt-in,
    /// plus whether the documents that opt-in would newly accept are actually
    /// correct.
    ///
    /// Pure post-processing of the `SeedResult`s this run already produced —
    /// it consults no `RoutingPolicy`, changes no measured behaviour, and both
    /// arms come from the **same run of the same binary**, which is what
    /// `knowledge/benchmarks/README.md`'s same-binary A/B rule requires.
    escalation_report: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            count: 100,
            seed: 0,
            profile: ProfileChoice::Clean,
            out: "artifacts/bench-report.json".to_string(),
            min_hit_rate: None,
            document_type: DocumentType::TD3,
            dump_ocr: false,
            escalation_report: false,
        }
    }
}

fn usage() {
    eprintln!(
        "Usage: synthpass-bench [--count N] [--seed N] [--profile NAME] [--document-type TYPE] \
         [--out PATH] [--min-hit-rate F] [--dump-ocr] [--escalation-report]"
    );
    eprintln!("  --count N            number of documents to check (default: 100)");
    eprintln!("  --seed N             base seed; document i uses seed N+i (default: 0)");
    eprintln!("  --profile NAME       clean|mobile|scanner|worn|border-kiosk|all (default: clean)");
    eprintln!(
        "  --document-type TYPE td1|td2|td3|mrva|mrvb — the ICAO 9303 MRZ format to *generate* \
         (default: td3). Not the same axis as provider-bench's --format, which scopes which \
         real samples/ directory to read."
    );
    eprintln!("  --out PATH           report JSON path (default: artifacts/bench-report.json)");
    eprintln!("  --min-hit-rate F     exit non-zero if the measured hit rate is below F");
    eprintln!(
        "  --dump-ocr           print every document's raw OCR text (one printed line per \
         detected line) before the summary — meant for a small --count diagnostic run"
    );
    eprintln!(
        "  --escalation-report  print the Tier-2 escalation rate under the default routing \
         policy and under Chunk 7's accept_composite_only_failure opt-in, plus whether the \
         documents that opt-in would newly accept are correct"
    );
}

/// Hand-rolled flag parser, consistent with `synthpass-cli`'s style (no
/// clap, no new arg-parsing dependency).
fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut parsed = Args::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--count" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--count requires a value".to_string())?;
                parsed.count = v
                    .parse::<u64>()
                    .map_err(|_| format!("--count: not a valid number: {v}"))?;
                i += 2;
            }
            "--seed" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--seed requires a value".to_string())?;
                parsed.seed = v
                    .parse::<u64>()
                    .map_err(|_| format!("--seed: not a valid number: {v}"))?;
                i += 2;
            }
            "--profile" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--profile requires a value".to_string())?;
                parsed.profile = ProfileChoice::parse(v)?;
                i += 2;
            }
            "--document-type" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--document-type requires a value".to_string())?;
                parsed.document_type = DocumentType::parse(v)?;
                i += 2;
            }
            "--out" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--out requires a value".to_string())?;
                parsed.out = v.clone();
                i += 2;
            }
            "--min-hit-rate" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--min-hit-rate requires a value".to_string())?;
                parsed.min_hit_rate = Some(
                    v.parse::<f64>()
                        .map_err(|_| format!("--min-hit-rate: not a valid number: {v}"))?,
                );
                i += 2;
            }
            "--dump-ocr" => {
                parsed.dump_ocr = true;
                i += 1;
            }
            "--escalation-report" => {
                parsed.escalation_report = true;
                i += 1;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(parsed)
}

#[derive(Serialize)]
struct SeedResult {
    seed: u64,
    profile: &'static str,
    hit: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    /// Coarse miss class, separate from `reason`'s human-readable text so
    /// misses can be counted by kind without parsing prose.
    #[serde(skip_serializing_if = "Option::is_none")]
    miss_kind: Option<&'static str>,
    /// Observed state of every check digit on every parsed MRZ, keyed by
    /// `mrz::Field::as_str()` names (`"document_number"`,
    /// `"date_of_birth"`, `"date_of_expiry"`, `"personal_number"`,
    /// `"composite"`). Values are `true` (verified), `false` (failed), or
    /// `null` (the layout does not print that digit); absent only if no MRZ
    /// parsed. This turns the largest miss bucket into an actionable breakdown
    /// without discarding successful evidence — see
    /// `knowledge/MRZ_SEQUENCE_COMPLETENESS.md` chunk 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    check_states: Option<BTreeMap<&'static str, Option<bool>>>,
    elapsed_ms: u128,
    /// Per-field character error rates, keyed by field name. Reported for
    /// every document that produced a parseable MRZ *and* for those that did
    /// not (as a total loss), so a mean over this is not biased by dropping
    /// the worst documents.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fields: Vec<FieldReport>,
    /// `true` iff `synthpass_core::fusion::check_line1_integrity` flagged
    /// this read — reported independently of `hit`, since the checksum a
    /// hit proves never covered `document_type`/`issuing_country`/`surname`/
    /// `given_names` in the first place (see `knowledge/ROADMAP.md`'s per-field
    /// CER note). `false` when no MRZ was read at all.
    line1_flagged: bool,
    /// `synthpass_bench::HitResult::names_exact` passthrough — `true` iff
    /// `surname` and `given_names` both matched ground truth exactly.
    /// `false` when no MRZ parsed, same as `line1_flagged`. See `Report`'s
    /// `strict_hit_rate` for the aggregate this feeds.
    names_exact: bool,
    /// `synthpass_bench::HitResult::name_error` passthrough, as its stable
    /// `NameError::as_str()` string — `None` both when `names_exact` is
    /// `true` and when no MRZ parsed (distinguish via `names_exact`).
    #[serde(skip_serializing_if = "Option::is_none")]
    name_error: Option<&'static str>,
    /// `true` iff `hit` and at least one of the 12 ICAO-scored fields (see
    /// `synthpass_bench`'s `COMPARED_FIELDS`; excludes the diagnostic
    /// `mrz_lines` row) differs from the generator's ground truth — issue
    /// #453's "wrong accept": `hit` proves only a checksum-consistent zone
    /// whose document number matches truth. `document_type`,
    /// `issuing_country`, both names, `nationality` and `sex` carry no check
    /// digit at all, and even a check-digited field can still be wrong — a
    /// check digit is consistency, not proof, so two compensating errors or a
    /// damaged-pass candidate that happens to validate can both pass it.
    /// Report-only: never redefines `hit`/`hit_rate`, and no gate reads this
    /// field. `false` when `hit` is `false` — there is no "accept" to judge.
    wrong_accept: bool,
    /// Which of the 12 scored fields differed from truth, in `COMPARED_FIELDS`
    /// order, when `wrong_accept` is `true`. Empty (and omitted from JSON)
    /// otherwise, including on every non-hit.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    wrong_fields: Vec<&'static str>,
}

#[derive(Serialize)]
struct FieldReport {
    field: &'static str,
    cer: f64,
    /// Only carried on an imperfect read — a report full of identical
    /// expected/got pairs is noise, and these are synthetic values so there
    /// is no PII concern in recording the ones that differ.
    #[serde(skip_serializing_if = "Option::is_none")]
    expected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    got: Option<String>,
}

/// One row of the "mean character error rate by field" table, annotated
/// with the physical MRZ line the field lives on for this run's format —
/// the JSON counterpart of the stdout table, so the line split can be
/// tracked across runs rather than only read once from a terminal. `line`
/// comes from `synthpass_bench::provider_bench::mrz_field_line`, never a
/// second copy of the ICAO field-layout tables that function already reads.
#[derive(Debug, Clone, Serialize)]
struct FieldLineCer {
    field: &'static str,
    mean_cer: f64,
    /// `None` for `mrz_lines` (a whole-zone aggregate, never a field with a
    /// span) and for any field with no span in this format's layout — never
    /// a fabricated line for either case.
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}

/// Mean of the per-field mean CERs assigned to each physical MRZ line,
/// unweighted across fields (every field counts equally regardless of how
/// many characters it spans, matching the per-field table's own units).
/// Rows with no resolved `line` are excluded rather than folded into a
/// bucket they were never measured against.
fn mean_cer_by_line(rows: &[FieldLineCer]) -> BTreeMap<usize, f64> {
    let mut sums: BTreeMap<usize, (f64, usize)> = BTreeMap::new();
    for row in rows {
        if let Some(line) = row.line {
            let entry = sums.entry(line).or_insert((0.0, 0));
            entry.0 += row.mean_cer;
            entry.1 += 1;
        }
    }
    sums.into_iter()
        .map(|(line, (sum, n))| (line, sum / n as f64))
        .collect()
}

#[derive(Serialize)]
struct Report {
    timestamp_unix: u64,
    profile: &'static str,
    /// The ICAO 9303 MRZ format every document in this run was generated as
    /// (`--document-type`). A single run is always one format — `hit_rate`
    /// is already that format's Tier-1 hit rate; run once per format to get
    /// the per-format comparison (see `knowledge/benchmarks/README.md`).
    document_type: &'static str,
    count: u64,
    seed_start: u64,
    hits: u64,
    hit_rate: f64,
    /// Tier-1 hits that *also* read both name fields exactly right (`hit &&
    /// names_exact`). **Not** a redefinition of `hits`/`hit_rate` — those
    /// stay exactly what the M4 CI gate (`--min-hit-rate`) and every
    /// existing measurement were calibrated against: checksum-valid
    /// document-number match, nothing about names. `strict_hits` answers a
    /// different question — what a user actually wants from an identity
    /// read — that no ICAO check digit and no existing metric covers. See
    /// `knowledge/benchmarks/README.md`.
    strict_hits: u64,
    /// `strict_hits / count`, the same denominator `hit_rate` uses. Every
    /// synthetic document carries ground truth for both name fields by
    /// construction (`Labels`), so "name-scorable documents" and `count` are
    /// the same population here — `provider-bench`'s `strict_tier1_hit_rate`
    /// divides by the narrower "name-scorable documents in the scored
    /// denominator" population instead, because a real specimen can lack a
    /// name label. See `knowledge/benchmarks/README.md`'s "Strict name hit
    /// rate" entry for why the two harnesses need different denominators to
    /// mean the same thing.
    strict_hit_rate: f64,
    /// `strict_hits / hits` — of the Tier-1 hits specifically (not of every
    /// document), how many also read both names exactly. Distinct from
    /// `strict_hit_rate`: a document that missed Tier-1 for an unrelated
    /// reason (checksum failure, no MRZ found) never enters this ratio's
    /// denominator, so it isolates the name-read question from detection
    /// accuracy. `0.0` when `hits` is `0` — there is no hit population to
    /// divide by, not a measured "every hit had a wrong name".
    names_exact_among_hits: f64,
    /// Tier-1 hits where at least one of the 12 scored fields
    /// (`synthpass_bench::COMPARED_FIELDS`) differs from the generator's
    /// ground truth — issue #453's "wrong accept". `hit` proves only a
    /// checksum-consistent zone whose document number matches truth.
    /// `document_type`, `issuing_country`, both names, `nationality` and
    /// `sex` carry no check digit at all, and even a check-digited field can
    /// still be wrong — a check digit is consistency, not proof, so two
    /// compensating errors or a damaged-pass candidate that happens to
    /// validate can both pass it. **Report-only**: never redefines `hit`,
    /// `hit_rate`, or the `--min-hit-rate` gate — see
    /// `knowledge/benchmarks/README.md`.
    wrong_accepts: u64,
    /// `wrong_accepts / hits`, the same denominator `names_exact_among_hits`
    /// uses — of the Tier-1 hits specifically, how many are wrong on at
    /// least one scored field. `0.0` when `hits` is `0`, not a fabricated
    /// "every hit wrong".
    wrong_accept_rate: f64,
    /// Mean CER per field (worst first), each annotated with the physical
    /// MRZ line it lives on for `document_type` — the JSON form of the
    /// stdout "mean character error rate by field" table. Empty when no
    /// document produced any field outcome at all.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    mean_cer_by_field: Vec<FieldLineCer>,
    /// [`mean_cer_by_line`] over `mean_cer_by_field` — the headline this
    /// table exists to surface: e.g. on a TD3 run every line-1 field's mean
    /// CER can dwarf every line-2 field's, and that split is invisible in a
    /// table sorted by magnitude alone. Keyed by 1-based physical MRZ line;
    /// excludes `mean_cer_by_field` rows with no resolved line.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    mean_cer_by_line: BTreeMap<usize, f64>,
    results: Vec<SeedResult>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match parse_args(&args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ {e}");
            usage();
            std::process::exit(1);
        }
    };

    let root = repo_root();
    let ocr = NativeOcr::load(
        &root.join("text-detection.rten"),
        &root.join("text-recognition.rten"),
    )
    .expect("failed to load OCR models — run from the repo root");

    let corpus = generate_corpus(
        parsed.profile,
        parsed.seed,
        parsed.count,
        parsed.document_type,
    );

    // Deliberately sequential: `NativeOcr::recognize` budgets its MRZ-retry
    // passes against wall-clock time (see synthpass-ocr's `max_duration`).
    // Running checks concurrently oversubscribes the CPU against rten's own
    // internal inference threads, inflating each pass's wall-clock time and
    // causing the retry budget to cut passes short — this was observed to
    // drop the measured hit rate by ~20 points versus running one at a time,
    // which is a resource-contention artifact, not a real accuracy signal.
    let results: Vec<SeedResult> = corpus
        .into_iter()
        .map(|doc| {
            let result = check_document(&ocr, &doc.image, &doc.labels);
            if parsed.dump_ocr {
                println!("--- seed {} raw OCR lines ---", doc.seed);
                match &result.raw_text {
                    Some(text) if !text.is_empty() => {
                        for (i, line) in text.lines().enumerate() {
                            println!("  [{i}] {line:?}");
                        }
                    }
                    Some(_) => println!("  (OCR returned no text)"),
                    None => println!("  (OCR failed: {:?})", result.reason),
                }
            }
            let line1_flagged = matches!(
                result.line1_integrity,
                Some(synthpass_core::fusion::Verdict::NeedsReview { .. })
            );
            let check_states = result.check_states.clone();
            let names_exact = result.names_exact;
            let name_error = result.name_error.map(synthpass_bench::NameError::as_str);
            let wrong_fields = wrong_scored_fields(&result.fields);
            let wrong_accept = result.hit && !wrong_fields.is_empty();
            SeedResult {
                seed: doc.seed,
                profile: doc.profile.as_str(),
                hit: result.hit,
                miss_kind: result.reason.as_ref().map(miss_kind),
                check_states,
                reason: result.reason.map(|r| r.to_string()),
                elapsed_ms: result.elapsed.as_millis(),
                line1_flagged,
                names_exact,
                name_error,
                wrong_accept,
                wrong_fields,
                fields: result
                    .fields
                    .into_iter()
                    .map(|f| {
                        let imperfect = f.cer > 0.0;
                        FieldReport {
                            field: f.field,
                            cer: f.cer,
                            expected: imperfect.then_some(f.expected),
                            got: if imperfect { f.got } else { None },
                        }
                    })
                    .collect(),
            }
        })
        .collect();

    let hits = results.iter().filter(|r| r.hit).count() as u64;
    let hit_rate = hits as f64 / parsed.count.max(1) as f64;

    for r in &results {
        if r.hit {
            println!("seed {} [{}]: HIT ({} ms)", r.seed, r.profile, r.elapsed_ms);
        } else {
            println!(
                "seed {} [{}]: MISS ({} ms) - {}",
                r.seed,
                r.profile,
                r.elapsed_ms,
                r.reason.as_deref().unwrap_or("unknown")
            );
        }
    }
    println!(
        "\n{hits}/{} = {:.1}% (profile: {}, document-type: {})",
        parsed.count,
        hit_rate * 100.0,
        parsed.profile.as_str(),
        parsed.document_type.as_str()
    );

    // Miss classes. A hit rate alone cannot distinguish "OCR found no MRZ"
    // from "OCR read the MRZ and one character was wrong" — those are
    // different problems with different fixes.
    let mut kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
    for r in results.iter().filter_map(|r| r.miss_kind) {
        *kinds.entry(r).or_default() += 1;
    }
    if !kinds.is_empty() {
        println!("\nmisses by kind:");
        for (kind, n) in &kinds {
            println!("  {n:>4}  {kind}");
        }
    }

    // Sub-breakdown of the checksum_failed bucket specifically — it has been
    // the single largest real-specimen miss kind, and was previously one
    // undifferentiated count. A document can fail more than one check digit
    // at once, so this tallies occurrences, not documents; it will not sum
    // to `kinds["checksum_failed"]`.
    let mut failing_field_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for r in &results {
        if let Some(check_states) = &r.check_states {
            for (field, state) in check_states {
                if *state == Some(false) {
                    *failing_field_counts.entry(field).or_default() += 1;
                }
            }
        }
    }
    if !failing_field_counts.is_empty() {
        println!("\nchecksum_failed, by failing field:");
        for (field, n) in &failing_field_counts {
            println!("  {n:>4}  {field}");
        }
    }

    // The headline this measurement exists for: the checksum a `hit` proves
    // never covered document_type/issuing_country/surname/given_names, so a
    // passing Tier-1 gate and a structurally wrong line 1 can both be true of
    // the same document at once. This reports how often that actually
    // happens, rather than leaving it as the one hand-counted number in
    // knowledge/ROADMAP.md's per-field CER note.
    if hits > 0 {
        let flagged_hits = results.iter().filter(|r| r.hit && r.line1_flagged).count();
        println!(
            "\nof {hits} Tier-1 hits, {flagged_hits} ({:.1}%) still have a line-1 integrity finding",
            flagged_hits as f64 / hits as f64 * 100.0
        );
    }

    // Strict hit rate: of the documents that were already a Tier-1 hit, how
    // many also read both name fields exactly right. Measured 2026-09-16
    // (seed 0, 100 clean synthetic documents/format) at roughly half across every
    // format — see `knowledge/benchmarks/README.md`. `hit_rate` above stays
    // the M4 CI gate's number unchanged; this is a second, additive metric,
    // never a redefinition of it.
    let strict_hits = results.iter().filter(|r| r.hit && r.names_exact).count() as u64;
    let strict_hit_rate = strict_hits as f64 / parsed.count.max(1) as f64;
    let names_exact_among_hits = strict_hits as f64 / hits.max(1) as f64;

    // Wrong accepts (issue #453): a Tier-1 hit whose checksum and document
    // number match truth but at least one of the other 11 scored fields does
    // not. `wrong_accept` is computed per-document above, from the same
    // per-field CER comparison `fields`/`strict_hits` already use — see
    // `wrong_scored_fields`. Report-only: does not affect `hit`, `hit_rate`,
    // or `--min-hit-rate`.
    let wrong_accepts = results.iter().filter(|r| r.wrong_accept).count() as u64;
    let wrong_accept_rate = wrong_accepts as f64 / hits.max(1) as f64;

    if hits > 0 {
        println!(
            "\nof {hits} Tier-1 hits, {strict_hits} ({:.1}%) read both names exactly — strict \
             hit rate {strict_hits}/{} = {:.1}%",
            names_exact_among_hits * 100.0,
            parsed.count,
            strict_hit_rate * 100.0
        );

        let mut name_error_kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
        for r in results.iter().filter(|r| r.hit) {
            if let Some(kind) = r.name_error {
                *name_error_kinds.entry(kind).or_default() += 1;
            }
        }
        if !name_error_kinds.is_empty() {
            println!("\nname errors among Tier-1 hits, by kind:");
            for (kind, n) in &name_error_kinds {
                println!("  {n:>4}  {kind}");
            }
        }

        // Wrong accepts (issue #453): of the Tier-1 hits, how many are wrong
        // on at least one of the 12 ICAO-scored fields. Every synthetic
        // document carries exact ground truth, so this is the same check
        // digit blind spot the m4-gate-440 finding measured by hand
        // (knowledge/benchmarks/m4-gate-440-wrong-reads-refused-2026-09-25.md)
        // — report-only, never gated and never a redefinition of `hit`.
        println!(
            "\nof {hits} Tier-1 hits, {wrong_accepts} ({:.1}%) are wrong on at least one of the \
             12 scored fields — report-only (issue #453), not gated",
            wrong_accept_rate * 100.0
        );
    }

    // Mean CER per field, over every document — including those that never
    // produced an MRZ, which count as a total loss. This is the number that
    // says *where* the accuracy goes, rather than only how much of it.
    //
    // Each row is also annotated with the physical MRZ line the field lives
    // on for this run's format: on a TD3 run every line-1 field's mean CER
    // has been observed to dwarf every line-2 field's, with no overlap at
    // all, and that split is invisible in a table sorted by magnitude alone
    // unless the reader already knows the format's layout by heart.
    // `mrz_field_line` is format-aware (TD1 assigns different fields to line
    // 1 than TD2/TD3 do), and shares `provider_bench`'s own ICAO field-layout
    // tables rather than restating them.
    let mut totals: BTreeMap<&'static str, (f64, usize)> = BTreeMap::new();
    for f in results.iter().flat_map(|r| &r.fields) {
        let entry = totals.entry(f.field).or_insert((0.0, 0));
        entry.0 += f.cer;
        entry.1 += 1;
    }
    let format = parsed.document_type.as_str();
    let mut mean_cer_by_field: Vec<FieldLineCer> = totals
        .iter()
        .map(|(field, (sum, n))| FieldLineCer {
            field,
            mean_cer: sum / *n as f64,
            line: synthpass_bench::provider_bench::mrz_field_line(format, field),
        })
        .collect();
    mean_cer_by_field.sort_by(|a, b| {
        b.mean_cer
            .partial_cmp(&a.mean_cer)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mean_cer_by_line_map = mean_cer_by_line(&mean_cer_by_field);
    if !mean_cer_by_field.is_empty() {
        println!("\nmean character error rate by field (worst first):");
        for row in &mean_cer_by_field {
            let line_label = row.line.map_or_else(
                || "no single line".to_string(),
                |line| format!("line {line}"),
            );
            println!(
                "  {:>7.2}%  {:<20}{line_label}",
                row.mean_cer * 100.0,
                row.field
            );
        }

        if !mean_cer_by_line_map.is_empty() {
            let mut parts: Vec<String> = mean_cer_by_line_map
                .iter()
                .map(|(line, mean)| format!("line {line} mean {:.2}%", mean * 100.0))
                .collect();
            if mean_cer_by_line_map.len() >= 2 {
                let max = mean_cer_by_line_map
                    .values()
                    .cloned()
                    .fold(f64::MIN, f64::max);
                let min = mean_cer_by_line_map
                    .values()
                    .cloned()
                    .fold(f64::MAX, f64::min);
                if min > 0.0 {
                    parts.push(format!("ratio {:.1}x", max / min));
                }
            }
            println!("  {}", parts.join("   "));
        }
    }

    if parsed.escalation_report {
        print_escalation_report(&results, parsed.document_type.as_str());
    }

    let report = Report {
        timestamp_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        profile: parsed.profile.as_str(),
        document_type: parsed.document_type.as_str(),
        count: parsed.count,
        seed_start: parsed.seed,
        hits,
        hit_rate,
        strict_hits,
        strict_hit_rate,
        names_exact_among_hits,
        wrong_accepts,
        wrong_accept_rate,
        mean_cer_by_field,
        mean_cer_by_line: mean_cer_by_line_map,
        results,
    };
    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    if let Some(parent) = std::path::Path::new(&parsed.out).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).expect("create report directory");
        }
    }
    std::fs::write(&parsed.out, json).expect("write report");
    println!("report written to {}", parsed.out);

    if let Some(min) = parsed.min_hit_rate {
        if hit_rate < min {
            eprintln!(
                "❌ hit rate {:.1}% is below the required minimum {:.1}%",
                hit_rate * 100.0,
                min * 100.0
            );
            std::process::exit(1);
        }
    }
}

/// The four fields whose own ICAO check digit exists and passed in a
/// composite-only failure — exactly the set
/// `synthpass_core::v2::FieldConfidence::mrz_checksum_scope_partial` promotes
/// to `CHECKSUM_PARTIAL` (0.95). Accepting such a record makes a claim about
/// each of these, so each is what correctness has to be judged on.
///
/// Line-1 fields (`document_type`, `issuing_country`, `surname`,
/// `given_names`) are deliberately excluded: no check digit covers them in
/// **any** ICAO format, so they are no better and no worse off than under an
/// ordinary Tier-1 accept. They are reported separately, for context, and are
/// not part of the pass/fail judgement.
const CHECK_DIGITED_FIELDS: [&str; 4] = [
    "document_number",
    "date_of_birth",
    "date_of_expiry",
    "personal_number",
];

/// Chunk 7's decision data: what the `accept_composite_only_failure` opt-in
/// would change, and whether the change is safe.
///
/// Derived entirely from `results` — no `RoutingPolicy` is consulted and
/// nothing is re-measured. That is sound because
/// `synthpass_die::routing`'s `default_policy_is_exactly_the_v1_2_0_predicate`
/// proves exhaustively that under the shipped default,
/// `Decision::Escalate` ⇔ `!(mrz_found && mrz_checksums_valid)` — which in this
/// harness's vocabulary is exactly `ocr_error` + `no_mrz_found` +
/// `checksum_failed`.
///
/// `document_number_mismatch` is **not** an escalation: those documents' check
/// digits verified, so the router accepts them. They are wrong *and* accepted,
/// which is the separate compound-cancellation blind spot
/// (`knowledge/ROADMAP.md`'s check-digit blind-spot note), not something this
/// opt-in touches either way.
/// Under `RoutingPolicy::default()`, `Decision::Escalate` ⇔
/// `!(mrz_found && mrz_checksums_valid)`. In this harness's vocabulary that is
/// exactly these three miss kinds.
///
/// `document_number_mismatch` is deliberately **absent**: those documents' check
/// digits verified, so the router accepts them. They are wrong *and* accepted —
/// the separate compound-cancellation blind spot, which this opt-in neither
/// helps nor worsens.
/// The 12 ICAO-scored fields (`synthpass_bench::COMPARED_FIELDS`'s names)
/// that differ from ground truth on this read, in scoring order. Excludes the
/// diagnostic `mrz_lines` row `compare_fields` appends — that row is the raw
/// zone, not one of the 12 fields the schema reports.
///
/// Issue #453: `hit` proves only a checksum-consistent zone whose document
/// number matches truth. `document_type`, `issuing_country`, either name
/// field, `nationality` and `sex` carry no check digit at all, and even a
/// check-digited field can still be wrong — a check digit is consistency,
/// not proof, so two compensating errors or a damaged-pass candidate that
/// happens to validate can both pass it. A `hit` can be a wrong read on any
/// of the 12 scored fields.
fn wrong_scored_fields(fields: &[synthpass_bench::FieldOutcome]) -> Vec<&'static str> {
    fields
        .iter()
        .filter(|f| f.field != "mrz_lines" && f.cer > 0.0)
        .map(|f| f.field)
        .collect()
}

fn escalates_by_default(r: &SeedResult) -> bool {
    matches!(
        r.miss_kind,
        Some("ocr_error") | Some("no_mrz_found") | Some("checksum_failed")
    )
}

/// The narrow case `mrz::Checks::only_composite_failed` names: the composite is
/// the sole failing digit.
fn is_composite_only(r: &SeedResult) -> bool {
    r.miss_kind == Some("checksum_failed")
        && r.check_states.as_ref().is_some_and(|check_states| {
            check_states
                .values()
                .filter(|state| **state == Some(false))
                .count()
                == 1
                && check_states.get("composite") == Some(&Some(false))
        })
}

/// Everything the Chunk 7 decision needs, separated from its formatting so the
/// arithmetic the decision rests on can be unit-tested.
#[derive(Debug, PartialEq)]
struct EscalationSummary {
    total: usize,
    escalate_off: usize,
    escalate_on: usize,
    newly_accepted: usize,
    /// Newly-accepted documents carrying at least one wrong *check-digited*
    /// field — the ones an accept would make a false claim about.
    wrong_docs: usize,
    wrong_by_field: BTreeMap<&'static str, usize>,
    /// Field-instances with CER > 0 among fields no check digit covers.
    /// Context only, never disqualifying.
    line1_imperfect: usize,
}

fn summarize_escalation(results: &[SeedResult]) -> EscalationSummary {
    let escalate_off = results.iter().filter(|r| escalates_by_default(r)).count();
    let newly: Vec<&SeedResult> = results.iter().filter(|r| is_composite_only(r)).collect();

    let mut wrong_docs = 0usize;
    let mut wrong_by_field: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut line1_imperfect = 0usize;
    for r in &newly {
        let mut any_wrong = false;
        for f in &r.fields {
            if f.cer <= 0.0 {
                continue;
            }
            if CHECK_DIGITED_FIELDS.contains(&f.field) {
                *wrong_by_field.entry(f.field).or_default() += 1;
                any_wrong = true;
            } else {
                line1_imperfect += 1;
            }
        }
        if any_wrong {
            wrong_docs += 1;
        }
    }

    EscalationSummary {
        total: results.len(),
        escalate_off,
        escalate_on: escalate_off - newly.len(),
        newly_accepted: newly.len(),
        wrong_docs,
        wrong_by_field,
        line1_imperfect,
    }
}

fn print_escalation_report(results: &[SeedResult], document_type: &str) {
    let total = results.len();
    if total == 0 {
        return;
    }
    let s = summarize_escalation(results);
    let newly_accepted: Vec<&SeedResult> =
        results.iter().filter(|r| is_composite_only(r)).collect();
    let (escalate_off, escalate_on) = (s.escalate_off, s.escalate_on);

    let pct = |n: usize| n as f64 / total as f64 * 100.0;
    println!("\nescalation (Chunk 7 A/B, document-type: {document_type}):");
    println!(
        "  escalation rate (v1.2.0 default):   {:>6.1}%  ({escalate_off}/{total})",
        pct(escalate_off)
    );
    println!(
        "  escalation rate (+composite-only):  {:>6.1}%  ({escalate_on}/{total})",
        pct(escalate_on)
    );
    println!(
        "    delta:                            {:>+6.2}pp ({} documents)",
        pct(escalate_on) - pct(escalate_off),
        newly_accepted.len()
    );

    if newly_accepted.is_empty() {
        println!(
            "\n  no composite-only failures in this corpus — the opt-in would change nothing.\n  \
             (Expected for MRV-A/MRV-B: ICAO 9303 Part 7 defines no composite check digit for\n  \
             either format, so `only_composite_failed` can never be true there.)"
        );
        return;
    }

    let (wrong_docs, wrong_by_field, line1_imperfect) =
        (s.wrong_docs, &s.wrong_by_field, s.line1_imperfect);
    let n = newly_accepted.len();
    println!("\n  composite-only accepted set (n={n}):");
    for field in CHECK_DIGITED_FIELDS {
        let wrong = wrong_by_field.get(field).copied().unwrap_or(0);
        println!("    {:<20} correct: {:>3}/{n}", field, n - wrong);
    }
    println!("    {:<20} {line1_imperfect} field-instances (no check digit covers these; not disqualifying)", "line-1 CER > 0:");

    if wrong_docs == 0 {
        println!("\n  VERDICT: no wrong check-digited field in the accepted set.");
    } else {
        println!(
            "\n  VERDICT: {wrong_docs}/{n} accepted documents carry a WRONG check-digited field."
        );
        println!(
            "  Accepting these would surface a proven-wrong value at CHECKSUM_PARTIAL (0.95)\n  \
             with Decision::Accept. Offending documents:"
        );
        for r in &newly_accepted {
            for f in &r.fields {
                if f.cer > 0.0 && CHECK_DIGITED_FIELDS.contains(&f.field) {
                    println!(
                        "    seed {} [{}] {}: expected {:?} got {:?}",
                        r.seed,
                        r.profile,
                        f.field,
                        f.expected.as_deref().unwrap_or("?"),
                        f.got.as_deref().unwrap_or("?")
                    );
                }
            }
        }
    }
}

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/synthpass-bench is two levels below the repo root")
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(
        miss: Option<&'static str>,
        failing: &[&'static str],
        fields: &[(&'static str, f64)],
    ) -> SeedResult {
        SeedResult {
            seed: 0,
            profile: "clean",
            hit: miss.is_none(),
            reason: None,
            miss_kind: miss,
            check_states: match miss {
                Some("no_mrz_found" | "ocr_error") => None,
                _ => Some(
                    [
                        "document_number",
                        "date_of_birth",
                        "date_of_expiry",
                        "personal_number",
                        "composite",
                    ]
                    .into_iter()
                    .map(|field| (field, Some(!failing.contains(&field))))
                    .collect(),
                ),
            },
            elapsed_ms: 0,
            fields: fields
                .iter()
                .map(|(field, cer)| FieldReport {
                    field,
                    cer: *cer,
                    expected: None,
                    got: None,
                })
                .collect(),
            line1_flagged: false,
            names_exact: false,
            name_error: None,
            wrong_accept: false,
            wrong_fields: Vec::new(),
        }
    }

    /// The escalation set is exactly what `RoutingPolicy::default()` escalates
    /// on. The load-bearing case is `document_number_mismatch`: its check digits
    /// *verified*, so the router accepts it — counting it as an escalation would
    /// inflate both arms and understate the opt-in's delta.
    #[test]
    fn escalation_set_excludes_checksum_valid_misses() {
        let results = vec![
            doc(None, &[], &[]),                               // hit -> accept
            doc(Some("document_number_mismatch"), &[], &[]),   // accept, wrongly
            doc(Some("no_mrz_found"), &[], &[]),               // escalate
            doc(Some("ocr_error"), &[], &[]),                  // escalate
            doc(Some("checksum_failed"), &["composite"], &[]), // escalate
        ];
        let s = summarize_escalation(&results);
        assert_eq!(s.total, 5);
        assert_eq!(s.escalate_off, 3);
    }

    /// The opt-in fires only on an *exactly* composite-only failure — never on a
    /// multi-field failure that merely includes the composite.
    #[test]
    fn only_an_exactly_composite_only_failure_is_newly_accepted() {
        let results = vec![
            doc(Some("checksum_failed"), &["composite"], &[]),
            doc(
                Some("checksum_failed"),
                &["date_of_birth", "composite"],
                &[],
            ),
            doc(Some("checksum_failed"), &["document_number"], &[]),
            doc(Some("no_mrz_found"), &[], &[]),
        ];
        let s = summarize_escalation(&results);
        assert_eq!(s.escalate_off, 4);
        assert_eq!(s.newly_accepted, 1);
        assert_eq!(
            s.escalate_on, 3,
            "only the composite-only document stops escalating"
        );
    }

    /// Correctness is judged on the four fields an accept makes a claim about.
    /// A line-1 field with CER > 0 is recorded but must never mark a document
    /// wrong — no check digit covers those in any ICAO format, so they are no
    /// worse off than under an ordinary Tier-1 accept.
    #[test]
    fn only_check_digited_fields_can_condemn_a_document() {
        let results = vec![
            doc(
                Some("checksum_failed"),
                &["composite"],
                &[("surname", 0.5), ("given_names", 0.2)],
            ),
            doc(
                Some("checksum_failed"),
                &["composite"],
                &[("personal_number", 0.1)],
            ),
            doc(
                Some("checksum_failed"),
                &["composite"],
                &[("document_number", 0.0)],
            ),
        ];
        let s = summarize_escalation(&results);
        assert_eq!(s.newly_accepted, 3);
        assert_eq!(
            s.wrong_docs, 1,
            "only the personal_number document is condemned"
        );
        assert_eq!(s.wrong_by_field.get("personal_number"), Some(&1));
        assert_eq!(s.line1_imperfect, 2);
        assert!(!s.wrong_by_field.contains_key("surname"));
    }

    /// A corpus where the opt-in changes nothing must report a zero delta rather
    /// than dividing by an empty set — the MRV-A/MRV-B case, where ICAO 9303
    /// Part 7 defines no composite check digit at all.
    #[test]
    fn a_corpus_with_no_composite_only_failures_has_no_delta() {
        let results = vec![doc(None, &[], &[]), doc(Some("no_mrz_found"), &[], &[])];
        let s = summarize_escalation(&results);
        assert_eq!(s.newly_accepted, 0);
        assert_eq!(s.escalate_on, s.escalate_off);
        assert_eq!(s.wrong_docs, 0);
    }

    fn field_outcome(
        field: &'static str,
        expected: &str,
        got: &str,
    ) -> synthpass_bench::FieldOutcome {
        synthpass_bench::FieldOutcome {
            field,
            expected: expected.to_string(),
            got: Some(got.to_string()),
            cer: synthpass_bench::cer(expected, got),
        }
    }

    /// A hit whose every scored field matches truth is not a wrong accept —
    /// the ordinary, unremarkable case issue #453 does not touch.
    #[test]
    fn all_fields_equal_is_not_a_wrong_accept() {
        let fields = vec![
            field_outcome("document_type", "P", "P"),
            field_outcome("surname", "SMITH", "SMITH"),
            field_outcome(
                "mrz_lines",
                "P<GBRSMITH<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<",
                "P<GBRSMITH<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<",
            ),
        ];
        assert!(wrong_scored_fields(&fields).is_empty());
    }

    /// One scored field diverging from truth is exactly issue #453's "wrong
    /// accept" — the checksum and document-number match `hit` requires never
    /// covered `surname`.
    #[test]
    fn one_differing_scored_field_is_a_wrong_accept() {
        let fields = vec![
            field_outcome("document_type", "P", "P"),
            field_outcome("surname", "SMITH", "ASTELLANO"),
        ];
        assert_eq!(wrong_scored_fields(&fields), vec!["surname"]);
    }

    /// `mrz_lines` is the diagnostic raw-zone row `compare_fields` appends,
    /// not one of the 12 ICAO-scored fields — a difference there alone must
    /// not mark a document a wrong accept.
    #[test]
    fn mrz_lines_divergence_alone_is_not_a_wrong_accept() {
        let fields = vec![
            field_outcome("document_type", "P", "P"),
            field_outcome("mrz_lines", "P<GBR...", "P<GBR<.."),
        ];
        assert!(wrong_scored_fields(&fields).is_empty());
    }

    /// A non-hit is never a wrong accept, regardless of how wrong its fields
    /// are — `wrong_accept` judges an *accepted* read, and there is nothing
    /// accepted here.
    #[test]
    fn a_non_hit_is_never_a_wrong_accept() {
        let fields = vec![field_outcome("surname", "SMITH", "ASTELLANO")];
        assert!(!wrong_scored_fields(&fields).is_empty());
        let hit = false;
        assert!(!(hit && !wrong_scored_fields(&fields).is_empty()));
    }

    fn field_line_cer(field: &'static str, mean_cer: f64, line: Option<usize>) -> FieldLineCer {
        FieldLineCer {
            field,
            mean_cer,
            line,
        }
    }

    /// The core arithmetic behind the "line 1 mean / line 2 mean / ratio"
    /// footer: an unweighted average of the per-field means assigned to each
    /// line, and a row with no resolved line contributes to no line's mean —
    /// it must neither vanish silently nor pull a bucket it was never placed
    /// in.
    #[test]
    fn mean_cer_by_line_averages_only_fields_that_resolve_a_line() {
        let rows = vec![
            field_line_cer("a", 0.40, Some(1)),
            field_line_cer("b", 0.20, Some(1)),
            field_line_cer("c", 0.10, Some(2)),
            field_line_cer("d", 1.00, None),
        ];
        let by_line = mean_cer_by_line(&rows);
        assert_eq!(
            by_line.len(),
            2,
            "the unresolved row must not open a third bucket"
        );
        assert!((by_line[&1] - 0.30).abs() < 1e-9);
        assert!((by_line[&2] - 0.10).abs() < 1e-9);
    }

    /// A run where every field resolves to the same line reports one bucket,
    /// not a phantom second one — the ratio and the per-line footer both
    /// depend on `by_line`'s length matching the lines actually observed.
    #[test]
    fn mean_cer_by_line_is_empty_when_no_row_resolves_a_line() {
        let rows = vec![field_line_cer("mrz_lines", 0.5, None)];
        assert!(mean_cer_by_line(&rows).is_empty());
    }

    /// TD3 puts `document_type`/`issuing_country`/`surname`/`given_names` on
    /// line 1 and the rest on line 2; TD1 puts the combined name field on
    /// line 3 instead. `mrz_field_line` (shared with `provider_bench`, not
    /// duplicated here) must reflect that per-format difference rather than
    /// a single fixed mapping.
    #[test]
    fn td1_and_td3_place_the_name_fields_on_different_lines() {
        assert_eq!(
            synthpass_bench::provider_bench::mrz_field_line("TD3", "surname"),
            Some(1)
        );
        assert_eq!(
            synthpass_bench::provider_bench::mrz_field_line("TD1", "surname"),
            Some(3)
        );
    }
}
