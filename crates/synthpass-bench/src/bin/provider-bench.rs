//! `provider-bench` — M7 multi-provider benchmark. Runs every registered
//! [`FieldReader`](synthpass_die::FieldReader) (today: the deterministic
//! `MrzReader` and the LLM's `LlmFieldReader`) against the same corpus and
//! reports, per provider: accuracy, speed, JSON validity, an
//! unsupported-assertion rate, and resident memory.
//!
//! Two corpus sources: the synthetic one from `synthpass-gen` (default), and
//! the real specimens in `samples/` (`--real-specimens`). The latter is the
//! only one that can contain a document with no MRZ, which is the population
//! the unsupported-assertion split below exists to measure separately.
//!
//! Requires the shipped GGUF present at the repo root (same precondition as
//! the existing `#[ignore]`d `native_llm_e2e`/`parity` tests in
//! `synthpass-llm`) — the LLM provider has to actually run to be measured.
//!
//! ```text
//! provider-bench [--count N] [--seed N] [--profile NAME] [--document-type TYPE] [--out PATH]
//!                [--measure-memory] [--real-specimens] [--limit N] [--mrz-only]
//!                [--format NAME] [--verbose] [--dump-ocr]
//!                [--write-baseline PATH] [--assert-baseline PATH]
//!   --count N          number of documents to check (default: 20)
//!   --seed N           base seed; document i uses seed N+i (default: 0)
//!   --profile NAME     clean|mobile|scanner|worn|border-kiosk|damaged|all (default: clean)
//!   --document-type TYPE  td1|td2|td3|mrva|mrvb — the ICAO 9303 MRZ format to *generate* for the
//!                      synthetic corpus (default: td3). Not the same axis as --format
//!                      below (which scopes which real samples/ directory --real-specimens
//!                      reads) — rejected together with --real-specimens, the same way
//!                      --format is rejected without it.
//!   --out PATH         report JSON path (default: artifacts/provider-bench-report.json)
//!   --measure-memory   sample process RSS around each provider's loop
//!                      (requires the `measure-memory` feature; see its doc
//!                      comment in Cargo.toml for why this is coarse)
//!   --real-specimens   run over samples/ instead of the synthetic corpus;
//!                      ground truth is optional per specimen, and
//!                      --count/--seed/--profile/--document-type are ignored
//!   --limit N          with --real-specimens: run N specimens spread evenly
//!                      across the (possibly --format-scoped) corpus
//!   --format NAME      with --real-specimens: restrict to one document
//!                      class — passport|id_card|driving_license (default:
//!                      all classes), applied before --limit
//!   --verbose, -v      print the per-document breakdown behind each
//!                      provider's aggregates
//!   --dump-ocr         with --real-specimens: for every checksum_failed
//!                      miss, print the full pre-parse OCR text, the MRZ band
//!                      score, and the recovered MRZ zone + failing check
//!                      digit(s), and append a row per miss to
//!                      <out-dir>/provider-bench-checksum-failed-dump.jsonl.
//!                      The real-specimen equivalent of synthpass-bench's own
//!                      --dump-ocr, scoped to this one miss kind since a
//!                      real-specimen run is much larger than a synthetic
//!                      diagnostic one
//!   --progress         force the per-document stderr progress log on even
//!                      when stderr is redirected. It is already on by
//!                      default whenever stderr is a terminal, so this flag
//!                      is only needed to keep it when piping to a file
//!   --mrz-only         register only the deterministic `mrz` reader, skipping
//!                      the Tier-2 LLM provider (and its ~1 GB GGUF, never
//!                      loaded). Turns a full real-specimen run from hours into
//!                      the ~1-minute deterministic pass — what the per-PR
//!                      `real-specimen-gate.yml` CI job runs. Valid over either
//!                      corpus source.
//!   --write-baseline PATH
//!                      after the run, write the `mrz` provider's Tier-1
//!                      snapshot (HIT count + miss-kind histogram + denominator)
//!                      as JSON to PATH and exit 0. How
//!                      `knowledge/benchmarks/real-specimen-mrz-baseline.json`
//!                      is (re)generated — always on CI, never hand-edited.
//!   --assert-baseline PATH
//!                      compare the `mrz` provider's Tier-1 snapshot against the
//!                      committed baseline at PATH and exit non-zero on a
//!                      regression: HIT count dropped, or any miss bucket
//!                      (`checksum_failed`, `no_mrz_found`, …) grew. A missing
//!                      PATH is written and passes (first-run bootstrap). The
//!                      per-PR no-regression gate.
//! ```

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use synthpass_bench::provider_bench::{
    run_provider_bench, run_provider_bench_real, AssertionBucket, ProviderReport, Tier1HitRate,
    UnsupportedAssertion,
};
use synthpass_bench::{
    generate_corpus, load_real_specimens, miss_kind, MissReason, ProfileChoice, SpecimenClass,
};
use synthpass_die::{MrzReader, ProviderCatalog};
use synthpass_gen::DocumentType;
use synthpass_ocr::NativeOcr;
use synthpass_pipeline::{InferBackend, NativeInferer, OcrEngine, Pipeline, RustOcrEngine};

struct Args {
    count: u64,
    seed: u64,
    profile: ProfileChoice,
    out: String,
    measure_memory: bool,
    /// Run over real specimens in `samples/` instead of the synthetic
    /// corpus — see `synthpass_bench::load_real_specimens`. `--count`/
    /// `--seed`/`--profile` are synthetic-corpus-only and ignored in this
    /// mode: a real specimen has no seed to start from and no capture
    /// profile to apply, it is what it is.
    real_specimens: bool,
    /// Cap on how many real specimens to run (`--real-specimens` only).
    /// `None` runs the whole corpus.
    limit: Option<usize>,
    /// Print the per-document breakdown behind each provider's aggregates.
    verbose: bool,
    /// With `real_specimens`: for every `checksum_failed` miss, dump the full
    /// pre-parse OCR text, the MRZ band score, and the recovered MRZ zone +
    /// failing check digit(s) to stdout, and a row per miss to
    /// `<out-dir>/provider-bench-checksum-failed-dump.jsonl`. A row for a
    /// labelled specimen also carries the hand-transcribed true zone and the
    /// character-mismatch count against it (`zone_mismatch`: `0` → the printed
    /// zone was read faithfully and its own check digits failed; `> 0` → OCR
    /// introduced the error). See
    /// `synthpass_bench::provider_bench::run_prepped`'s doc for why this is
    /// scoped to that one miss kind rather than every document the way
    /// `synthpass-bench --dump-ocr` is.
    dump_ocr: bool,
    /// Force the per-document progress log on even when stderr is not a
    /// terminal. Progress is *already* on by default for an interactive run
    /// (see `show_progress` in `main`) — a full real-specimen pass takes over
    /// an hour and being silent for it is the complaint this exists to fix.
    /// The flag only matters when stderr is redirected, where the default is
    /// off so a captured log or a CI step stays clean.
    progress: bool,
    /// Restrict `--real-specimens` to one `samples/` document class (e.g.
    /// `passport`) via `synthpass_bench::classify_specimen`. `--real-specimens`
    /// only, applied before `--limit` so a stride subsamples the already-scoped
    /// population, not the whole corpus.
    format: Option<SpecimenClass>,
    /// The ICAO 9303 MRZ format to *generate* for the synthetic corpus.
    /// `None` means "unspecified" (defaults to TD3 where used) — kept as an
    /// `Option`, not a bare `DocumentType` defaulting to `TD3`, so
    /// `parse_args` can tell "explicitly TD3" apart from "never mentioned"
    /// the same way `format: Option<SpecimenClass>` does, and reject
    /// `--document-type` together with `--real-specimens` accordingly.
    /// Deliberately named `--document-type`, not `--format`: `--format`
    /// above already means `SpecimenClass` (which real `samples/` directory
    /// to read).
    document_type: Option<DocumentType>,
    /// Register only the deterministic `mrz` reader — skip the Tier-2 LLM
    /// provider entirely. The LLM pass dominates a real-specimen run (~19-37s
    /// per document, hours for the whole corpus) while the deterministic
    /// reader is µs-ms per document, and the LLM's GGUF (never loaded here) is
    /// a ~1 GB download. The per-PR `real-specimen-gate.yml` CI job runs with
    /// this on; it is also useful for any quick local Tier-1 measurement.
    mrz_only: bool,
    /// Write the `mrz` provider's Tier-1 snapshot to this path as JSON and
    /// exit 0 — the regeneration path for the committed real-specimen
    /// baseline. CI-only by convention (`--assert-baseline`'s doc explains
    /// why local and CI numbers differ).
    write_baseline: Option<String>,
    /// Compare the `mrz` provider's Tier-1 snapshot against the committed
    /// baseline at this path; exit non-zero on a regression (HIT count down,
    /// or a miss bucket up, beyond the baseline's `tolerance`). A path that
    /// does not exist yet is written and passes — the first-run bootstrap.
    assert_baseline: Option<String>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            count: 20,
            seed: 0,
            profile: ProfileChoice::Clean,
            out: "artifacts/provider-bench-report.json".to_string(),
            measure_memory: false,
            real_specimens: false,
            limit: None,
            verbose: false,
            dump_ocr: false,
            progress: false,
            format: None,
            document_type: None,
            mrz_only: false,
            write_baseline: None,
            assert_baseline: None,
        }
    }
}

fn usage() {
    eprintln!(
        "Usage: provider-bench [--count N] [--seed N] [--profile NAME] [--out PATH] \
         [--measure-memory] [--real-specimens]"
    );
    eprintln!("  --count N          number of documents to check (default: 20)");
    eprintln!("  --seed N           base seed; document i uses seed N+i (default: 0)");
    eprintln!(
        "  --profile NAME     clean|mobile|scanner|worn|border-kiosk|damaged|all (default: clean)"
    );
    eprintln!(
        "  --out PATH         report JSON path (default: artifacts/provider-bench-report.json)"
    );
    eprintln!("  --measure-memory   sample process RSS around each provider's loop (coarse)");
    eprintln!(
        "  --real-specimens   run over samples/ instead of the synthetic corpus (ground truth \
         optional per specimen; --count/--seed/--profile are ignored)"
    );
    eprintln!(
        "  --limit N          with --real-specimens: run N specimens spread evenly across the \
         corpus (deterministic; default: all)"
    );
    eprintln!(
        "  --format NAME      with --real-specimens: restrict to one document class — \
         passport|id_card|driving_license (default: all classes)"
    );
    eprintln!(
        "  --document-type TYPE  td1|td2|td3|mrva|mrvb — the ICAO 9303 MRZ format to *generate* \
         for the synthetic corpus (default: td3). Not the same axis as --format, which scopes \
         which real samples/ directory --real-specimens reads; rejected together with \
         --real-specimens the same way --format is rejected without it."
    );
    eprintln!(
        "  --dump-ocr         with --real-specimens: for every checksum_failed miss, print the \
         full pre-parse OCR text + MRZ band score + recovered MRZ zone + failing check digit(s), \
         and append a row per miss to <out-dir>/provider-bench-checksum-failed-dump.jsonl"
    );
    eprintln!(
        "  --progress         force the per-document stderr progress log on when stderr is \
         redirected (already on by default when stderr is a terminal)"
    );
    eprintln!(
        "  --mrz-only         register only the deterministic mrz reader, skipping the Tier-2 \
         LLM provider and its GGUF (the per-PR real-specimen CI gate runs this way)"
    );
    eprintln!(
        "  --write-baseline PATH  write the mrz provider's Tier-1 snapshot (HIT count + \
         miss-kind histogram) to PATH as JSON and exit"
    );
    eprintln!(
        "  --assert-baseline PATH  compare the mrz provider's Tier-1 snapshot against the \
         committed baseline at PATH; exit non-zero on a regression (missing PATH is written \
         and passes)"
    );
}

/// Hand-rolled flag parser, consistent with `synthpass-bench`'s own binary
/// (no clap, no new arg-parsing dependency).
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
            "--out" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--out requires a value".to_string())?;
                parsed.out = v.clone();
                i += 2;
            }
            "--measure-memory" => {
                parsed.measure_memory = true;
                i += 1;
            }
            "--real-specimens" => {
                parsed.real_specimens = true;
                i += 1;
            }
            "--verbose" | "-v" => {
                parsed.verbose = true;
                i += 1;
            }
            "--dump-ocr" => {
                parsed.dump_ocr = true;
                i += 1;
            }
            // Deliberately not gated on --real-specimens the way --dump-ocr
            // is: the synthetic corpus is slow enough to want progress too,
            // and a flag that only forces on an already-default behaviour has
            // no invalid combination to reject.
            "--progress" => {
                parsed.progress = true;
                i += 1;
            }
            "--limit" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--limit requires a value".to_string())?;
                let n = v
                    .parse::<usize>()
                    .map_err(|_| format!("--limit: not a valid number: {v}"))?;
                if n == 0 {
                    return Err("--limit must be at least 1".to_string());
                }
                parsed.limit = Some(n);
                i += 2;
            }
            "--format" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--format requires a value".to_string())?;
                parsed.format = Some(SpecimenClass::parse(v)?);
                i += 2;
            }
            "--document-type" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--document-type requires a value".to_string())?;
                parsed.document_type = Some(DocumentType::parse(v)?);
                i += 2;
            }
            "--mrz-only" => {
                parsed.mrz_only = true;
                i += 1;
            }
            "--write-baseline" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--write-baseline requires a path".to_string())?;
                parsed.write_baseline = Some(v.clone());
                i += 2;
            }
            "--assert-baseline" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--assert-baseline requires a path".to_string())?;
                parsed.assert_baseline = Some(v.clone());
                i += 2;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if parsed.format.is_some() && !parsed.real_specimens {
        return Err("--format is only valid together with --real-specimens".to_string());
    }
    if parsed.dump_ocr && !parsed.real_specimens {
        return Err("--dump-ocr is only valid together with --real-specimens".to_string());
    }
    if parsed.document_type.is_some() && parsed.real_specimens {
        return Err(
            "--document-type generates the synthetic corpus and is not valid together with \
             --real-specimens (use --format to scope which real samples/ directory is read)"
                .to_string(),
        );
    }
    if parsed.write_baseline.is_some() && parsed.assert_baseline.is_some() {
        return Err(
            "--write-baseline and --assert-baseline are mutually exclusive: one regenerates the \
             baseline, the other checks against it"
                .to_string(),
        );
    }
    Ok(parsed)
}

/// Take `n` specimens spread evenly across `all`, preserving order.
///
/// Deliberately a stride, not `all.truncate(n)`. `load_real_specimens` returns
/// paths sorted, and `samples/` is organised by document class
/// (`driving_licenses/`, `id_cards/`, `misc/`, `ocr_fixtures/`, `passports/`),
/// so taking the first `n` would fill the whole subset from the alphabetically
/// earliest directories and include no passports at all — a subset that
/// silently answers a different question than the corpus it claims to sample.
/// An even stride keeps the class mix roughly proportional.
///
/// Deterministic by construction: same `n`, same corpus, same subset, every
/// run. A random sample would be a better estimator in principle and a worse
/// benchmark in practice — two runs that disagree because they drew different
/// documents cannot be compared, which is the same reasoning
/// `ProviderCatalog`'s insertion-ordered consultation already applies to
/// provider order.
fn subsample<T>(all: Vec<T>, n: usize) -> Vec<T> {
    let total = all.len();
    if n >= total {
        return all;
    }
    // `i * total / n` for i in 0..n — integer arithmetic, so no float rounding
    // decides membership, and the first element is always included.
    let mut keep: Vec<Option<T>> = all.into_iter().map(Some).collect();
    (0..n)
        .map(|i| i * total / n)
        .filter_map(|idx| keep[idx].take())
        .collect()
}

#[derive(Serialize)]
struct CapabilityReport {
    deterministic: bool,
    vision: bool,
    cost: &'static str,
}

/// `field_match_rate`/`mean_cer`/each `PerFieldCer::mean_cer` are `null` in
/// the JSON report exactly when `labelled_documents == 0` (whole report) or
/// no labelled document had ground truth for that specific field (per-field)
/// — see `synthpass_bench::provider_bench::AccuracyStats`'s doc for why that
/// is reported as absent rather than a fabricated `0.0`.
#[derive(Serialize)]
struct AccuracyReport {
    labelled_documents: usize,
    field_match_rate: Option<f64>,
    mean_cer: Option<f64>,
    per_field_cer: Vec<PerFieldCer>,
}

#[derive(Serialize)]
struct PerFieldCer {
    field: &'static str,
    mean_cer: Option<f64>,
}

/// Mirrors `synthpass_bench::provider_bench::UnsupportedAssertion` for JSON:
/// `serde`'s internally-tagged enum representation, so a report reader sees
/// either `{"status": "computed", "overall": {...}, ...}` or
/// `{"status": "not_applicable", "reason": "..."}` — never a bare number
/// that would look the same whether it was measured or skipped.
///
/// `with_mrz_anchor`/`without_mrz_anchor` are `null` when the corpus had no
/// document of that kind — always the case for `without_mrz_anchor` on the
/// synthetic corpus, since every generated document carries an MRZ.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum UnsupportedAssertionReport {
    Computed {
        overall: AssertionBucketReport,
        with_mrz_anchor: Option<AssertionBucketReport>,
        without_mrz_anchor: Option<AssertionBucketReport>,
    },
    NotApplicable {
        reason: &'static str,
    },
}

#[derive(Serialize)]
/// `rate` is `null` when `assertions_total == 0` — the provider answered
/// nothing over this subset, which is a different fact from a measured rate
/// of zero and must not serialize as one.
struct AssertionBucketReport {
    rate: Option<f64>,
    assertions_total: usize,
    documents: usize,
}

/// Per-document rows behind a provider's aggregates. Counts and field *names*
/// only — never an asserted value, which would put document content into a
/// report file (see `synthpass_bench::provider_bench::DocumentDetail`).
#[derive(Serialize)]
struct DocumentDetailReport {
    name: String,
    mrz_found: bool,
    /// The document's resolved ICAO 9303 MRZ format ("TD1"/"TD2"/"TD3"/
    /// "MRVA"/"MRVB"), `null` when none could be resolved — see
    /// `synthpass_bench::provider_bench::DocumentDetail::mrz_format`'s doc
    /// for the synthetic-vs-real-specimen resolution rules.
    #[serde(skip_serializing_if = "Option::is_none")]
    mrz_format: Option<&'static str>,
    read_ok: bool,
    mrz_checksums_valid: bool,
    /// Stable machine-readable miss class (`miss_kind`'s output) — `null` on
    /// a genuine Tier-1 hit. Never the free-text `MissReason` itself: some
    /// variants carry an inner detail string (a parse/provider error
    /// message), which is aggregate-report noise, not report content.
    #[serde(skip_serializing_if = "Option::is_none")]
    miss_reason: Option<&'static str>,
    /// Which check digit(s) failed, only present when `miss_reason` is
    /// `"checksum_failed"` or `"checksum_failed_specimen"` — `mrz::Field::as_str()`
    /// names. Field names only,
    /// same discipline as `unsupported_fields` above — see this struct's
    /// module doc. See `knowledge/MRZ_SEQUENCE_COMPLETENESS.md` chunk 1.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    failing_checks: Vec<&'static str>,
    assertions_total: usize,
    assertions_unsupported: usize,
    unsupported_fields: Vec<&'static str>,
}

impl From<AssertionBucket> for AssertionBucketReport {
    fn from(b: AssertionBucket) -> Self {
        Self {
            rate: b.rate,
            assertions_total: b.assertions_total,
            documents: b.documents,
        }
    }
}

impl From<UnsupportedAssertion> for UnsupportedAssertionReport {
    fn from(u: UnsupportedAssertion) -> Self {
        match u {
            UnsupportedAssertion::Computed {
                overall,
                with_mrz_anchor,
                without_mrz_anchor,
            } => Self::Computed {
                overall: overall.into(),
                with_mrz_anchor: with_mrz_anchor.map(Into::into),
                without_mrz_anchor: without_mrz_anchor.map(Into::into),
            },
            UnsupportedAssertion::NotApplicable { reason } => Self::NotApplicable { reason },
        }
    }
}

/// Mirrors `synthpass_bench::provider_bench::Tier1HitRate` for JSON: either
/// `{"status": "computed", "rate": ...}` or `{"status": "not_applicable",
/// "reason": "..."}` — never a bare number that would look identical whether
/// it was measured or skipped for a non-`capability.deterministic` provider.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Tier1HitRateReport {
    Computed { rate: f64 },
    NotApplicable { reason: &'static str },
}

impl From<Tier1HitRate> for Tier1HitRateReport {
    fn from(t: Tier1HitRate) -> Self {
        match t {
            Tier1HitRate::Computed(rate) => Self::Computed { rate },
            Tier1HitRate::NotApplicable { reason } => Self::NotApplicable { reason },
        }
    }
}

#[derive(Serialize)]
struct SpeedReport {
    mean_ms: u128,
    p50_ms: u128,
    p95_ms: u128,
}

#[derive(Serialize)]
struct JsonValidityReport {
    repair_fallbacks: u64,
    documents: usize,
    repair_fallback_rate: f64,
}

#[derive(Serialize)]
struct ProviderRow {
    provider_id: String,
    documents: usize,
    capability: CapabilityReport,
    accuracy: AccuracyReport,
    speed: SpeedReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    json_validity: Option<JsonValidityReport>,
    unsupported_assertion: UnsupportedAssertionReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    declared_resident_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    measured_rss_delta_bytes: Option<i64>,
    /// Always serialized, regardless of `--verbose` — the flag governs
    /// terminal noise, not what the report records. A run that took an hour
    /// should not have to be repeated because the per-document detail was
    /// only printed and never saved.
    documents_detail: Vec<DocumentDetailReport>,
    /// Genuine Tier-1 hit rate (MRZ found, checksums valid, document number
    /// matches when labelled) — see
    /// `synthpass_bench::provider_bench::ProviderReport::tier1_hit_rate`'s
    /// doc for why this is a different number from `read_ok`/nothing above.
    /// `NotApplicable` for a non-`capability.deterministic` provider — see
    /// `Tier1HitRate`'s doc.
    tier1_hit_rate: Tier1HitRateReport,
}

impl From<ProviderReport> for ProviderRow {
    fn from(r: ProviderReport) -> Self {
        Self {
            provider_id: r.provider_id,
            documents: r.documents,
            capability: CapabilityReport {
                deterministic: r.capability.deterministic,
                vision: r.capability.vision,
                cost: r.capability.cost,
            },
            accuracy: AccuracyReport {
                labelled_documents: r.accuracy.labelled_documents,
                field_match_rate: r.accuracy.field_match_rate,
                mean_cer: r.accuracy.mean_cer,
                per_field_cer: r
                    .accuracy
                    .per_field_cer
                    .into_iter()
                    .map(|(field, mean_cer)| PerFieldCer { field, mean_cer })
                    .collect(),
            },
            speed: SpeedReport {
                mean_ms: r.speed.mean.as_millis(),
                p50_ms: r.speed.p50.as_millis(),
                p95_ms: r.speed.p95.as_millis(),
            },
            json_validity: r.json_validity.map(|j| JsonValidityReport {
                repair_fallback_rate: if j.documents == 0 {
                    0.0
                } else {
                    j.repair_fallbacks as f64 / j.documents as f64
                },
                repair_fallbacks: j.repair_fallbacks,
                documents: j.documents,
            }),
            unsupported_assertion: r.unsupported_assertion.into(),
            declared_resident_bytes: r.declared_resident_bytes,
            measured_rss_delta_bytes: r.measured_rss_delta_bytes,
            documents_detail: r
                .documents_detail
                .into_iter()
                .map(|d| DocumentDetailReport {
                    name: d.name,
                    mrz_found: d.mrz_found,
                    mrz_format: d.mrz_format,
                    read_ok: d.read_ok,
                    mrz_checksums_valid: d.mrz_checksums_valid,
                    miss_reason: d.miss_reason.as_ref().map(miss_kind),
                    failing_checks: match &d.miss_reason {
                        Some(MissReason::ChecksumFailed { failing, .. }) => failing.clone(),
                        _ => Vec::new(),
                    },
                    assertions_total: d.assertions_total,
                    assertions_unsupported: d.assertions_unsupported,
                    unsupported_fields: d.unsupported_fields,
                })
                .collect(),
            tier1_hit_rate: r.tier1_hit_rate.into(),
        }
    }
}

#[derive(Serialize)]
struct Report {
    timestamp_unix: u64,
    /// "synthetic-corpus" or "real-specimens" — which of `run_provider_bench`/
    /// `run_provider_bench_real` produced `providers` below, since the two
    /// populate `profile`/`count`/`seed_start` differently (a real specimen
    /// has no capture profile and no seed).
    source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<&'static str>,
    /// The `--format` class this run was restricted to, `real-specimens`
    /// only — absent means every `samples/` class was included.
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'static str>,
    count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed_start: Option<u64>,
    providers: Vec<ProviderRow>,
}

/// `SYNTHPASS_MODEL_PATH`/default GGUF filename, mirroring
/// `synthpass-pipeline`'s private `infer::backend_from_env` — that function
/// isn't reachable from outside the crate, so this benchmark constructs
/// `NativeInferer` directly the same way `backend_from_env` does internally.
fn model_path() -> String {
    std::env::var("SYNTHPASS_MODEL_PATH")
        .unwrap_or_else(|_| "./qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string())
}

fn n_ctx() -> u32 {
    std::env::var("SYNTHPASS_MODEL_N_CTX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2048)
}

/// A one-reader catalog holding just the deterministic `mrz` provider — the
/// `--mrz-only` path. [`MrzReader::new`] takes no arguments and pulls in no
/// OCR/LLM dependency (it lives in `synthpass-die`), so this needs neither a
/// [`Pipeline`] nor a model file.
fn mrz_only_catalog() -> ProviderCatalog {
    ProviderCatalog::builder()
        .with_reader(std::sync::Arc::new(MrzReader::new()))
        .build()
        .expect("a single reader cannot collide on id")
}

/// The miss buckets whose growth is a Tier-1 regression. `redacted_mrz` and
/// `no_mrz_expected` are excluded on purpose — both sit outside the hit-rate
/// denominator (a redaction bar carries no readable zone; an ID-card front
/// carries no zone at all), so their counts moving is a corpus change, not a
/// parser regression. `checksum_failed_specimen` *is* included: a labelled
/// specimen whose printed zone the run no longer recovers exactly has regressed,
/// even though its printed check digits were always non-conforming.
/// `false_positive_mrz` is included and is the most serious of them: it means a
/// checksum-valid MRZ was returned for a document that has none.
const REGRESSION_BUCKETS: &[&str] = &[
    "checksum_failed",
    "checksum_failed_specimen",
    "no_mrz_found",
    "ocr_error",
    "document_number_mismatch",
    "false_positive_mrz",
];

const BASELINE_NOTE: &str = "Real-specimen Tier-1 no-regression baseline for the deterministic \
    `mrz` provider. Regenerate ONLY via CI: `gh workflow run real-specimen-gate.yml -f \
    mode=write-baseline`, download the artifact, commit it. Local numbers differ from CI \
    (OCR-inference float variance), so never hand-edit the counts. See \
    knowledge/benchmarks/README.md.";

/// The measured Tier-1 facts for the deterministic `mrz` provider over one run —
/// what [`--write-baseline`](Args::write_baseline) records (via
/// [`RealSpecimenBaseline`]) and [`--assert-baseline`](Args::assert_baseline)
/// checks against.
#[derive(Debug)]
struct RealSpecimenSnapshot {
    /// Documents that OCR'd successfully and reached the reader loop.
    documents: usize,
    /// Denominator of the Tier-1 hit rate: `documents` minus the two
    /// populations that cannot answer the question. `redacted_mrz` is a zone
    /// blacked out by whoever published the specimen; `no_mrz_expected` is a
    /// document that never had one (an ID-card front, a border pass, a
    /// driving-license face). Neither can prove or disprove detection accuracy,
    /// and until 2026-09-09 the second was not excluded — 42 correct refusals
    /// sat in this denominator as `no_mrz_found` failures, which put the
    /// headline rate 11.6 points low and roughly doubled the apparent size of
    /// the detection problem `ADR-0008` exists to attack.
    scored: usize,
    /// Documents that were a genuine Tier-1 hit (MRZ found, checksums valid,
    /// document number matches when labelled).
    tier1_hits: usize,
    /// Every miss kind that occurred, with its document count.
    by_miss_kind: BTreeMap<String, usize>,
}

impl RealSpecimenSnapshot {
    /// Build from the `"mrz"` provider's report. `None` when that provider is
    /// absent (a run that registered only the LLM — never the case under
    /// `--mrz-only`, and normally the full catalog has both).
    fn from_reports(reports: &[ProviderReport]) -> Option<Self> {
        let mrz = reports.iter().find(|r| r.provider_id == "mrz")?;
        let mut by_miss_kind: BTreeMap<String, usize> = BTreeMap::new();
        let mut tier1_hits = 0usize;
        for d in &mrz.documents_detail {
            match &d.miss_reason {
                None => tier1_hits += 1,
                Some(reason) => {
                    *by_miss_kind
                        .entry(miss_kind(reason).to_string())
                        .or_default() += 1
                }
            }
        }
        let documents = mrz.documents_detail.len();
        let off_denominator = ["redacted_mrz", "no_mrz_expected"]
            .iter()
            .filter_map(|k| by_miss_kind.get(*k))
            .sum::<usize>();
        Some(Self {
            documents,
            scored: documents - off_denominator,
            tier1_hits,
            by_miss_kind,
        })
    }

    fn redacted(&self) -> usize {
        self.by_miss_kind.get("redacted_mrz").copied().unwrap_or(0)
    }

    /// Documents scored out because they carry no MRZ to find.
    fn no_mrz_expected(&self) -> usize {
        self.by_miss_kind
            .get("no_mrz_expected")
            .copied()
            .unwrap_or(0)
    }
}

/// The committed baseline file
/// (`knowledge/benchmarks/real-specimen-mrz-baseline.json`): a
/// [`RealSpecimenSnapshot`]'s counts plus provenance and a `tolerance`.
#[derive(Debug, Serialize, Deserialize)]
struct RealSpecimenBaseline {
    /// What this file is and how to regenerate it. Ignored by the comparison.
    note: String,
    /// The commit the baseline run measured against (`GITHUB_SHA` on CI).
    measured_on_ci_sha: String,
    measured_date: String,
    /// `samples-data` branch HEAD at measurement time. The corpus is not
    /// pinned, so this is provenance only.
    samples_data_sha: String,
    documents: usize,
    scored: usize,
    tier1_hits: usize,
    /// Slack for run-to-run OCR-inference variance across CI runner hardware:
    /// subtracted from the HIT-count floor and added to every "may not grow"
    /// bucket bound. `0` today; raise it (a one-line reviewed change) only if
    /// runs prove flaky.
    tolerance: usize,
    by_miss_kind: BTreeMap<String, usize>,
}

fn baseline_from_snapshot(snap: &RealSpecimenSnapshot, ts_unix: u64) -> RealSpecimenBaseline {
    RealSpecimenBaseline {
        note: BASELINE_NOTE.to_string(),
        measured_on_ci_sha: env_or("GITHUB_SHA", git_head),
        measured_date: iso_date(ts_unix),
        samples_data_sha: env_or("SAMPLES_DATA_SHA", || "unknown".to_string()),
        documents: snap.documents,
        scored: snap.scored,
        tier1_hits: snap.tier1_hits,
        tolerance: 0,
        by_miss_kind: snap.by_miss_kind.clone(),
    }
}

/// Compare a fresh run against the committed baseline.
///
/// `Err` lists every regression — a HIT-count drop, or a [`REGRESSION_BUCKETS`]
/// bucket growing — beyond `baseline.tolerance`. `Ok(warnings)` is a clean run:
/// the `Vec` carries corpus-shape drift (`documents` or `redacted_mrz` moved),
/// which means the numbers legitimately changed and the author should
/// regenerate the baseline, but is not a parser regression to block a merge on.
fn check_baseline(
    actual: &RealSpecimenSnapshot,
    baseline: &RealSpecimenBaseline,
) -> Result<Vec<String>, Vec<String>> {
    let tol = baseline.tolerance;
    let mut failures = Vec::new();
    let mut warnings = Vec::new();

    if actual.tier1_hits + tol < baseline.tier1_hits {
        failures.push(format!(
            "Tier-1 HIT count regressed: {} -> {} (tolerance {tol})",
            baseline.tier1_hits, actual.tier1_hits
        ));
    }
    for &bucket in REGRESSION_BUCKETS {
        let was = baseline.by_miss_kind.get(bucket).copied().unwrap_or(0);
        let now = actual.by_miss_kind.get(bucket).copied().unwrap_or(0);
        if now > was + tol {
            failures.push(format!(
                "miss bucket `{bucket}` grew: {was} -> {now} (tolerance {tol})"
            ));
        }
    }

    if actual.documents != baseline.documents {
        warnings.push(format!(
            "corpus size changed: {} -> {} documents — regenerate the baseline in this PR",
            baseline.documents, actual.documents
        ));
    }
    // The denominator moving changes the published hit *rate* even when the HIT
    // count does not, and it was the one baseline field nothing compared. That
    // blind spot is not hypothetical: reclassifying the 42 MRZ-less specimens
    // as correct refusals moved `scored` 229 -> 187 and every regression check
    // above stayed silent, because no bucket grew and no hit was lost.
    if actual.scored != baseline.scored {
        warnings.push(format!(
            "hit-rate denominator changed: {} -> {} scored — the published rate moves even with \
             the same HIT count; regenerate the baseline in this PR",
            baseline.scored, actual.scored
        ));
    }
    // Both off-denominator populations drift the same way and warn the same
    // way: a count that moved means the corpus changed, not that anything
    // regressed. Reported per bucket rather than as one total so the message
    // names which population moved.
    for (bucket, now) in [
        ("redacted_mrz", actual.redacted()),
        ("no_mrz_expected", actual.no_mrz_expected()),
    ] {
        let was = baseline.by_miss_kind.get(bucket).copied().unwrap_or(0);
        if now != was {
            warnings.push(format!(
                "{bucket} count changed: {was} -> {now} (off-denominator) — regenerate the \
                 baseline if the corpus changed"
            ));
        }
    }

    if failures.is_empty() {
        Ok(warnings)
    } else {
        failures.extend(warnings);
        Err(failures)
    }
}

/// `--write-baseline` / `--assert-baseline`, run after the report JSON is on
/// disk. May [`std::process::exit`] non-zero on a regression.
fn run_baseline_step(parsed: &Args, snapshot: Option<RealSpecimenSnapshot>, ts_unix: u64) {
    let Some(snapshot) = snapshot else {
        eprintln!(
            "❌ --write-baseline/--assert-baseline need the `mrz` provider in the run — it was \
             not registered (check the catalog wiring)"
        );
        std::process::exit(1);
    };

    if let Some(path) = parsed.write_baseline.as_deref() {
        let baseline = baseline_from_snapshot(&snapshot, ts_unix);
        write_json_pretty(path, &baseline);
        println!(
            "baseline written to {path} (tier1_hits={}, scored={}, {} miss kind(s))",
            baseline.tier1_hits,
            baseline.scored,
            baseline.by_miss_kind.len(),
        );
        return;
    }

    let path = parsed
        .assert_baseline
        .as_deref()
        .expect("run_baseline_step is only reached with one of the two flags set");

    if !std::path::Path::new(path).exists() {
        let baseline = baseline_from_snapshot(&snapshot, ts_unix);
        write_json_pretty(path, &baseline);
        println!(
            "ℹ no baseline at {path} yet — wrote the current snapshot and passing. Review and \
             commit it (CI owns the committed value)."
        );
        return;
    }

    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read baseline {path}: {e}"));
    let baseline: RealSpecimenBaseline =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse baseline {path}: {e}"));

    print_baseline_table(&baseline, &snapshot);

    match check_baseline(&snapshot, &baseline) {
        Ok(warnings) => {
            for w in &warnings {
                eprintln!("⚠ {w}");
            }
            println!("✅ real-specimen Tier-1 (mrz): no regression vs baseline");
        }
        Err(problems) => {
            for p in &problems {
                eprintln!("❌ {p}");
            }
            eprintln!(
                "\nIf this change is intentional (parser improvement, corpus edit), regenerate \
                 the baseline in this PR — see knowledge/benchmarks/README.md."
            );
            std::process::exit(1);
        }
    }
}

fn print_baseline_table(baseline: &RealSpecimenBaseline, actual: &RealSpecimenSnapshot) {
    println!("\nreal-specimen Tier-1 baseline check (mrz provider):");
    let row = |label: &str, was: usize, now: usize| {
        println!(
            "  {label:<26} {was:>5} -> {now:<5} ({:+})",
            now as i64 - was as i64
        );
    };
    row("tier1_hits", baseline.tier1_hits, actual.tier1_hits);
    row("scored (denominator)", baseline.scored, actual.scored);
    row("documents", baseline.documents, actual.documents);
    let mut kinds: Vec<&str> = baseline
        .by_miss_kind
        .keys()
        .chain(actual.by_miss_kind.keys())
        .map(String::as_str)
        .collect();
    kinds.sort_unstable();
    kinds.dedup();
    for k in kinds {
        row(
            k,
            baseline.by_miss_kind.get(k).copied().unwrap_or(0),
            actual.by_miss_kind.get(k).copied().unwrap_or(0),
        );
    }
    println!(
        "  (baseline measured {} on {})",
        baseline.measured_date, baseline.measured_on_ci_sha
    );
}

fn write_json_pretty<T: Serialize>(path: &str, value: &T) {
    let json = serde_json::to_string_pretty(value).expect("serialize");
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).expect("create baseline directory");
        }
    }
    std::fs::write(path, format!("{json}\n")).unwrap_or_else(|e| panic!("write {path}: {e}"));
}

fn env_or(var: &str, fallback: impl FnOnce() -> String) -> String {
    std::env::var(var)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(fallback)
}

fn git_head() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// `YYYY-MM-DD` (UTC) from a Unix timestamp — a trimmed civil-from-days
/// (Howard Hinnant's algorithm). One date string in a report does not justify a
/// `chrono`/`time` dependency.
fn iso_date(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match parse_args(&args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ {e}");
            usage();
            std::process::exit(1);
        }
    };

    if parsed.measure_memory && !cfg!(feature = "measure-memory") {
        eprintln!(
            "❌ --measure-memory requires building with `--features measure-memory` \
             (declared_resident_bytes is still reported without it)"
        );
        std::process::exit(1);
    }

    // On by default for an interactive run, off when stderr is redirected —
    // a captured log or a CI step (`bench-charts.yml` runs this in a pwsh
    // step) stays exactly as clean as it is today, while a developer watching
    // a 1h37m real-specimen pass sees it advance. `--progress` forces it back
    // on for the redirected case.
    let show_progress = parsed.progress || std::io::stderr().is_terminal();

    let root = repo_root();
    let ocr = NativeOcr::load(
        &root.join("text-detection.rten"),
        &root.join("text-recognition.rten"),
    )
    .expect("failed to load OCR models — run from the repo root");

    // The full M7 catalog is `mrz` + the Tier-2 `LlmFieldReader`, and the only
    // way to reach the latter's registered instance is through a `Pipeline`
    // (`LlmFieldReader` is `pub(crate)` there). `--mrz-only` skips all of it —
    // no second OCR engine, no `NativeInferer`, no GGUF ever touched — and
    // builds a one-reader catalog directly instead, turning a real-specimen
    // run from hours into the ~1-minute deterministic pass the per-PR
    // `real-specimen-gate.yml` CI job needs.
    let pipeline;
    let deterministic_only;
    let catalog: &ProviderCatalog = if parsed.mrz_only {
        deterministic_only = mrz_only_catalog();
        &deterministic_only
    } else {
        // A second OCR engine handle purely to satisfy `Pipeline::new`'s
        // constructor — never actually invoked. Every document in this harness
        // is OCR'd once via `ocr` above and shared across every reader.
        let pipeline_ocr: Box<dyn OcrEngine> = Box::new(RustOcrEngine::new(&root, false));
        let infer: Box<dyn InferBackend> = Box::new(NativeInferer::new(model_path(), n_ctx()));
        pipeline = Pipeline::new(pipeline_ocr, infer);
        pipeline.catalog()
    };

    // Real specimens (ground truth optional, MRZ-less fronts included) vs.
    // the synthetic corpus (ground truth always present) — see
    // `synthpass_bench::provider_bench`'s top doc comment for why both
    // sources feed the exact same reader loop and reporting shape.
    let (reports, source, profile, count, seed_start) = if parsed.real_specimens {
        let mut specimens = load_real_specimens(&root.join("samples"));
        if specimens.is_empty() {
            eprintln!(
                "❌ no image files found under samples/ — is this being run from the repo root?"
            );
            std::process::exit(1);
        }
        if let Some(class) = parsed.format {
            specimens.retain(|s| s.class == class);
            if specimens.is_empty() {
                eprintln!(
                    "❌ no samples/ specimens classified as {} — nothing to run",
                    class.as_str()
                );
                std::process::exit(1);
            }
        }
        if let Some(n) = parsed.limit {
            specimens = subsample(specimens, n);
        }
        let labelled = specimens.iter().filter(|s| s.labels.is_some()).count();
        eprintln!(
            "loaded {} real specimens ({labelled} with samples/ocr_fixtures/ ground truth, {} \
             unlabelled)",
            specimens.len(),
            specimens.len() - labelled,
        );
        // `--dump-ocr` writes its JSONL next to the `--out` report; an --out
        // with no directory part means the current directory.
        let dump_dir = parsed.dump_ocr.then(|| {
            std::path::Path::new(&parsed.out)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(std::path::Path::to_path_buf)
                .unwrap_or_else(|| std::path::PathBuf::from("."))
        });
        let reports = run_provider_bench_real(
            catalog,
            &ocr,
            &specimens,
            parsed.measure_memory,
            dump_dir.as_deref(),
            show_progress,
        )
        .await;
        (
            reports,
            "real-specimens",
            None,
            specimens.len() as u64,
            None,
        )
    } else {
        let corpus = generate_corpus(
            parsed.profile,
            parsed.seed,
            parsed.count,
            parsed.document_type.unwrap_or(DocumentType::TD3),
        );
        let reports =
            run_provider_bench(catalog, &ocr, &corpus, parsed.measure_memory, show_progress).await;
        (
            reports,
            "synthetic-corpus",
            Some(parsed.profile.as_str()),
            parsed.count,
            Some(parsed.seed),
        )
    };

    for r in &reports {
        let field_match = r
            .accuracy
            .field_match_rate
            .map(|v| format!("{:.1}%", v * 100.0))
            .unwrap_or_else(|| "n/a".to_string());
        let mean_cer = r
            .accuracy
            .mean_cer
            .map(|v| format!("{v:.3}"))
            .unwrap_or_else(|| "n/a".to_string());
        let unsupported = match &r.unsupported_assertion {
            UnsupportedAssertion::Computed {
                overall,
                with_mrz_anchor,
                without_mrz_anchor,
            } => {
                // The split is the point of this line, not a detail: a single
                // blended figure hides that the anchored and unanchored
                // populations fail in different *kinds*, not just degrees.
                // Each half prints its own document count, because they are
                // never the same size and a bare rate would conceal that.
                //
                // "no claims" rather than "0.0%" when nothing was asserted.
                // The first real run made the difference matter: over the 22
                // documents with no MRZ, the deterministic reader asserted
                // nothing at all, and printing that as `0.0%` read as a
                // measured, perfect score in the same column where a genuine
                // 0.0 means "answered plenty, all of it supported". Declining
                // to answer is the correct behaviour there and deserves to be
                // legible as such, not disguised as a good grade.
                let rate = |b: &AssertionBucket| match b.rate {
                    Some(r) => format!("{:.1}%", r * 100.0),
                    None => "no claims".to_string(),
                };
                let half = |label: &str, b: &Option<AssertionBucket>| match b {
                    Some(b) => format!(", {label} {} ({} docs)", rate(b), b.documents),
                    None => String::new(),
                };
                format!(
                    "{}{}{}",
                    rate(overall),
                    half("with-MRZ", with_mrz_anchor),
                    half("no-MRZ", without_mrz_anchor),
                )
            }
            UnsupportedAssertion::NotApplicable { reason } => format!("n/a ({reason})"),
        };
        let tier1_hit_rate = match &r.tier1_hit_rate {
            Tier1HitRate::Computed(rate) => format!("{:.1}%", rate * 100.0),
            Tier1HitRate::NotApplicable { reason } => format!("n/a ({reason})"),
        };
        println!(
            "{}: {} docs ({} labelled), Tier-1 hit rate {tier1_hit_rate}, field match \
             {field_match}, mean CER {mean_cer}, mean {} ms, unsupported-assertion rate \
             {unsupported}",
            r.provider_id,
            r.documents,
            r.accuracy.labelled_documents,
            r.speed.mean.as_millis(),
        );

        // Per-format document counts — the M6 plan's "report the unparsed
        // population separately; a specimen with no MRZ is not a
        // TD-anything" applied to this harness: a distribution, keyed on the
        // same `mrz_format` the Tier-1 hit rate above and `miss_reason`
        // below share.
        let mut by_format: std::collections::BTreeMap<&str, usize> =
            std::collections::BTreeMap::new();
        for d in &r.documents_detail {
            *by_format
                .entry(d.mrz_format.unwrap_or("unresolved"))
                .or_default() += 1;
        }
        if !by_format.is_empty() {
            let counts: Vec<String> = by_format
                .iter()
                .map(|(fmt, n)| format!("{fmt}={n}"))
                .collect();
            println!("    by format: {}", counts.join(", "));
        }

        // Misses by kind — mirrors `synthpass-bench`'s own "misses by kind"
        // summary, so a real-specimen run answers "what kind of failure
        // dominates this track" from stdout, not just `--verbose` +
        // manual JSON inspection. Skipped entirely when there is nothing to
        // report, same as the format breakdown above.
        let mut by_miss_kind: std::collections::BTreeMap<&str, usize> =
            std::collections::BTreeMap::new();
        for d in &r.documents_detail {
            if let Some(reason) = &d.miss_reason {
                *by_miss_kind.entry(miss_kind(reason)).or_default() += 1;
            }
        }
        if !by_miss_kind.is_empty() {
            let counts: Vec<String> = by_miss_kind
                .iter()
                .map(|(kind, n)| format!("{kind}={n}"))
                .collect();
            println!("    misses by kind: {}", counts.join(", "));

            if let Some(n) = by_miss_kind.get("redacted_mrz") {
                println!(
                    "    (redacted_mrz: {n} specimen(s) with a physically redacted zone — \
                     excluded from the Tier-1 hit-rate denominator, not scored as a miss)"
                );
            }
            if let Some(n) = by_miss_kind.get("no_mrz_expected") {
                println!(
                    "    (no_mrz_expected: {n} specimen(s) that carry no MRZ at all and correctly \
                     yielded none — a correct refusal, excluded from the denominator)"
                );
            }
            // Loud on purpose, and phrased as a defect rather than a count: a
            // checksum-valid MRZ off a document that has none is either a
            // hallucinated record or a mislabelled corpus file, and both need
            // someone to look. It reads as a hit in every other summary.
            if let Some(n) = by_miss_kind.get("false_positive_mrz") {
                println!(
                    "    ⚠ FALSE POSITIVES: {n} specimen(s) tagged as carrying no MRZ returned a \
                     checksum-valid one. Either the read is invented or the file is mislabelled — \
                     see `--verbose` for which, and knowledge/benchmarks/README.md."
                );
            }

            // Sub-breakdown of checksum_failed specifically — see
            // `synthpass-bench.rs`'s identical summary and
            // `knowledge/MRZ_SEQUENCE_COMPLETENESS.md` chunk 1. Tallies
            // occurrences, not documents, so this need not sum to
            // `by_miss_kind["checksum_failed"]`.
            let mut by_failing_field: std::collections::BTreeMap<&str, usize> =
                std::collections::BTreeMap::new();
            for d in &r.documents_detail {
                if let Some(MissReason::ChecksumFailed { failing, .. }) = &d.miss_reason {
                    for field in failing {
                        *by_failing_field.entry(field).or_default() += 1;
                    }
                }
            }
            if !by_failing_field.is_empty() {
                let counts: Vec<String> = by_failing_field
                    .iter()
                    .map(|(field, n)| format!("{field}={n}"))
                    .collect();
                println!(
                    "    checksum_failed, by failing field: {}",
                    counts.join(", ")
                );
            }
        }

        if parsed.verbose {
            // Worst first. An aggregate tells you a provider made 61
            // unsupported assertions; this tells you which documents produced
            // them, which is the difference between a number and something
            // you can go and fix. Documents where nothing was asserted are
            // skipped — on the MRZ-less half that is most of the deterministic
            // reader's rows, and listing 22 empty lines would bury the ones
            // that matter.
            let mut rows: Vec<_> = r
                .documents_detail
                .iter()
                .filter(|d| !d.read_ok || d.assertions_total > 0)
                .collect();
            rows.sort_by(|a, b| {
                b.assertions_unsupported
                    .cmp(&a.assertions_unsupported)
                    .then_with(|| a.name.cmp(&b.name))
            });
            for d in rows {
                let anchor = if d.mrz_found { "mrz" } else { "no-mrz" };
                let format = d.mrz_format.unwrap_or("?");
                if !d.read_ok {
                    println!("    {:<6}  {:<5}  {}  READ FAILED", anchor, format, d.name);
                    continue;
                }
                let fields = if d.unsupported_fields.is_empty() {
                    String::new()
                } else {
                    format!("  [{}]", d.unsupported_fields.join(", "))
                };
                println!(
                    "    {:<6}  {:<5}  {:<52}  {}/{} unsupported{}",
                    anchor, format, d.name, d.assertions_unsupported, d.assertions_total, fields,
                );
            }
        }
    }

    // Captured before `reports` is consumed into the report below. `None` when
    // no baseline flag was passed, or `Some(None)` when one was but the `mrz`
    // provider is somehow absent from the run (handled in `run_baseline_step`).
    let baseline_snapshot = (parsed.write_baseline.is_some() || parsed.assert_baseline.is_some())
        .then(|| RealSpecimenSnapshot::from_reports(&reports));

    let timestamp_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let report = Report {
        timestamp_unix,
        source,
        profile,
        format: parsed.format.map(SpecimenClass::as_str),
        count,
        seed_start,
        providers: reports.into_iter().map(ProviderRow::from).collect(),
    };
    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    if let Some(parent) = std::path::Path::new(&parsed.out).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).expect("create report directory");
        }
    }
    std::fs::write(&parsed.out, json).expect("write report");
    println!("report written to {}", parsed.out);

    // Baseline write / assert — the per-PR real-specimen no-regression gate.
    // Runs last, after the report JSON is safely on disk, and may
    // `std::process::exit(1)` on a regression.
    if let Some(snapshot) = baseline_snapshot {
        run_baseline_step(&parsed, snapshot, timestamp_unix);
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

    #[test]
    fn subsample_returns_everything_when_the_limit_is_not_binding() {
        let all: Vec<u32> = (0..10).collect();
        assert_eq!(subsample(all.clone(), 10), all);
        assert_eq!(subsample(all.clone(), 99), all);
    }

    #[test]
    fn subsample_takes_a_spread_not_a_prefix() {
        // The property that matters: `samples/` is ordered by document class,
        // so a prefix would be all one class. A stride over 137 documents must
        // reach the end of the corpus, not stop a third of the way in.
        let all: Vec<u32> = (0..137).collect();
        let picked = subsample(all, 50);
        assert_eq!(picked.len(), 50);
        assert_eq!(picked[0], 0, "the first document is always included");
        assert!(
            *picked.last().expect("non-empty") > 100,
            "a stride must reach the far end of the corpus; got {:?}",
            picked.last()
        );
        let mut sorted = picked.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 50, "no duplicates");
        assert_eq!(sorted, picked, "corpus order is preserved");
    }

    #[test]
    fn subsample_is_deterministic() {
        // Two runs that disagree because they drew different documents cannot
        // be compared against each other, which is the whole point of a
        // benchmark subset.
        let all: Vec<u32> = (0..137).collect();
        assert_eq!(subsample(all.clone(), 50), subsample(all, 50));
    }

    #[test]
    fn subsample_handles_a_limit_of_one() {
        assert_eq!(subsample((0..137).collect::<Vec<u32>>(), 1), vec![0]);
    }

    #[test]
    fn format_parses_alongside_real_specimens() {
        let args: Vec<String> = ["--real-specimens", "--format", "passport"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let parsed = parse_args(&args).expect("valid combination");
        assert_eq!(parsed.format, Some(SpecimenClass::Passport));
        assert!(parsed.real_specimens);
    }

    #[test]
    fn format_without_real_specimens_is_rejected() {
        let args: Vec<String> = ["--format", "passport"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(
            parse_args(&args).is_err(),
            "--format only makes sense scoping --real-specimens"
        );
    }

    #[test]
    fn dump_ocr_parses_alongside_real_specimens() {
        let args: Vec<String> = ["--real-specimens", "--dump-ocr"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let parsed = parse_args(&args).expect("valid combination");
        assert!(parsed.dump_ocr);
    }

    #[test]
    fn dump_ocr_without_real_specimens_is_rejected() {
        let args: Vec<String> = ["--dump-ocr"].iter().map(|s| s.to_string()).collect();
        assert!(
            parse_args(&args).is_err(),
            "--dump-ocr only makes sense scoping --real-specimens"
        );
    }

    #[test]
    fn progress_parses_on_its_own() {
        // Unlike --dump-ocr, --progress carries no --real-specimens
        // precondition: it forces on a log that is already the default for an
        // interactive run, over either corpus source.
        let args: Vec<String> = ["--progress"].iter().map(|s| s.to_string()).collect();
        assert!(parse_args(&args).expect("--progress parses alone").progress);
    }

    #[test]
    fn progress_parses_alongside_real_specimens() {
        let args: Vec<String> = ["--real-specimens", "--progress"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(parse_args(&args).expect("--progress parses").progress);
    }

    #[test]
    fn progress_flag_defaults_off() {
        // The *flag* defaults off; the effective behaviour is
        // `parsed.progress || stderr().is_terminal()`, resolved in `main` so
        // that a redirected run stays silent unless asked.
        assert!(!Args::default().progress);
    }

    #[test]
    fn format_rejects_an_unknown_class() {
        let args: Vec<String> = ["--real-specimens", "--format", "visa"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn document_type_parses_for_the_synthetic_corpus() {
        let args: Vec<String> = ["--document-type", "td1"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let parsed = parse_args(&args).expect("valid on its own");
        assert_eq!(parsed.document_type, Some(DocumentType::TD1));
        assert!(!parsed.real_specimens);
    }

    #[test]
    fn document_type_with_real_specimens_is_rejected() {
        // `--document-type` generates the synthetic corpus; `--real-specimens`
        // reads samples/ instead. Combining them is the same category error
        // `--format` without `--real-specimens` already rejects, mirrored.
        let args: Vec<String> = ["--real-specimens", "--document-type", "td2"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn document_type_rejects_an_unknown_value() {
        let args: Vec<String> = ["--document-type", "td4"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(parse_args(&args).is_err());
    }

    // --- --mrz-only + the real-specimen no-regression gate ----------------

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn mrz_only_parses_alone_and_with_real_specimens() {
        assert!(parse_args(&argv(&["--mrz-only"])).expect("parses").mrz_only);
        let p = parse_args(&argv(&["--real-specimens", "--mrz-only"])).expect("parses");
        assert!(p.mrz_only && p.real_specimens);
    }

    #[test]
    fn mrz_only_catalog_holds_exactly_the_deterministic_reader() {
        let catalog = mrz_only_catalog();
        assert_eq!(catalog.readers().len(), 1);
        assert_eq!(catalog.readers()[0].id().as_str(), "mrz");
    }

    #[test]
    fn baseline_flags_parse_and_conflict() {
        let p = parse_args(&argv(&[
            "--real-specimens",
            "--mrz-only",
            "--assert-baseline",
            "x.json",
        ]))
        .expect("parses");
        assert_eq!(p.assert_baseline.as_deref(), Some("x.json"));
        assert!(parse_args(&argv(&[
            "--write-baseline",
            "a.json",
            "--assert-baseline",
            "b.json"
        ]))
        .is_err());
    }

    fn snap(hits: usize, kinds: &[(&str, usize)]) -> RealSpecimenSnapshot {
        let by_miss_kind: BTreeMap<String, usize> =
            kinds.iter().map(|(k, n)| ((*k).to_string(), *n)).collect();
        let misses: usize = kinds.iter().map(|(_, n)| n).sum();
        let documents = hits + misses;
        // Mirrors `RealSpecimenSnapshot::from_reports` — both off-denominator
        // populations, not just the redacted one, or these tests would exercise
        // a denominator the production path does not use.
        let off: usize = ["redacted_mrz", "no_mrz_expected"]
            .iter()
            .filter_map(|k| by_miss_kind.get(*k))
            .sum();
        RealSpecimenSnapshot {
            documents,
            scored: documents - off,
            tier1_hits: hits,
            by_miss_kind,
        }
    }

    fn baseline_with_tolerance(
        base: &RealSpecimenSnapshot,
        tolerance: usize,
    ) -> RealSpecimenBaseline {
        let mut b = baseline_from_snapshot(base, 1_757_030_400);
        b.tolerance = tolerance;
        b
    }

    #[test]
    fn baseline_identical_run_passes_clean() {
        let s = snap(
            120,
            &[
                ("checksum_failed", 24),
                ("no_mrz_found", 85),
                ("redacted_mrz", 9),
            ],
        );
        let b = baseline_with_tolerance(&s, 0);
        assert_eq!(check_baseline(&s, &b), Ok(vec![]));
    }

    #[test]
    fn baseline_fails_on_a_hit_count_drop() {
        let base = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        let b = baseline_with_tolerance(&base, 0);
        let regressed = snap(119, &[("checksum_failed", 25), ("no_mrz_found", 85)]);
        let err = check_baseline(&regressed, &b).expect_err("hits dropped");
        assert!(err.iter().any(|m| m.contains("HIT count regressed")));
    }

    #[test]
    fn baseline_fails_on_bucket_churn_that_nets_zero_on_hits() {
        // HITs flat at 120, but 3 documents moved checksum_failed -> no_mrz_found.
        let base = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        let b = baseline_with_tolerance(&base, 0);
        let churned = snap(120, &[("checksum_failed", 21), ("no_mrz_found", 88)]);
        let err = check_baseline(&churned, &b).expect_err("no_mrz_found grew");
        assert!(err.iter().any(|m| m.contains("`no_mrz_found` grew")));
    }

    #[test]
    fn baseline_passes_a_genuine_improvement() {
        let base = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        let b = baseline_with_tolerance(&base, 0);
        let better = snap(123, &[("checksum_failed", 21), ("no_mrz_found", 85)]);
        assert_eq!(check_baseline(&better, &b), Ok(vec![]));
    }

    #[test]
    fn baseline_tolerance_absorbs_small_drift_but_not_more() {
        let base = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        let b = baseline_with_tolerance(&base, 2);
        assert_eq!(
            check_baseline(
                &snap(118, &[("checksum_failed", 26), ("no_mrz_found", 85)]),
                &b
            ),
            Ok(vec![])
        );
        assert!(check_baseline(
            &snap(117, &[("checksum_failed", 27), ("no_mrz_found", 85)]),
            &b
        )
        .is_err());
    }

    #[test]
    fn baseline_corpus_growth_warns_but_does_not_fail() {
        let base = snap(
            120,
            &[
                ("checksum_failed", 24),
                ("no_mrz_found", 85),
                ("redacted_mrz", 9),
            ],
        );
        let b = baseline_with_tolerance(&base, 0);
        // two specimens added: one HIT, one redacted.
        let grown = snap(
            121,
            &[
                ("checksum_failed", 24),
                ("no_mrz_found", 85),
                ("redacted_mrz", 10),
            ],
        );
        let warnings = check_baseline(&grown, &b).expect("growth is not a regression");
        assert!(warnings.iter().any(|w| w.contains("corpus size")));
        assert!(warnings.iter().any(|w| w.contains("redacted_mrz")));
    }

    /// The exact shape of the 2026-09-09 reclassification: 42 documents move
    /// from `no_mrz_found` to `no_mrz_expected`, HITs are untouched, and no
    /// bucket grows — so every regression check stays silent while the
    /// published hit rate goes 52.0% -> 63.6%. Only the denominator warning
    /// notices, which is why it exists.
    #[test]
    fn baseline_warns_when_only_the_denominator_moved() {
        let base = snap(
            119,
            &[
                ("checksum_failed", 24),
                ("checksum_failed_specimen", 1),
                ("no_mrz_found", 85),
                ("redacted_mrz", 9),
            ],
        );
        let b = baseline_with_tolerance(&base, 0);
        assert_eq!(b.scored, 229, "the published denominator before the fix");

        let reclassified = snap(
            119,
            &[
                ("checksum_failed", 24),
                ("checksum_failed_specimen", 1),
                ("no_mrz_found", 43),
                ("no_mrz_expected", 42),
                ("redacted_mrz", 9),
            ],
        );
        assert_eq!(reclassified.scored, 187, "the corrected denominator");

        let warnings =
            check_baseline(&reclassified, &b).expect("a reclassification is not a regression");
        assert!(
            warnings.iter().any(|w| w.contains("denominator changed")),
            "the denominator moving must be reported: {warnings:?}"
        );
        assert!(warnings.iter().any(|w| w.contains("no_mrz_expected")));
    }

    /// A false positive is inside the denominator and inside the regression
    /// buckets: a checksum-valid MRZ returned for a document that carries none
    /// must block a merge, not warn.
    #[test]
    fn baseline_fails_when_false_positives_appear() {
        let base = snap(119, &[("no_mrz_found", 43), ("no_mrz_expected", 42)]);
        let b = baseline_with_tolerance(&base, 0);
        let hallucinating = snap(
            119,
            &[
                ("no_mrz_found", 43),
                ("no_mrz_expected", 41),
                ("false_positive_mrz", 1),
            ],
        );
        let err = check_baseline(&hallucinating, &b).expect_err("a false positive is a regression");
        assert!(err.iter().any(|m| m.contains("`false_positive_mrz` grew")));
    }

    #[test]
    fn baseline_json_round_trips() {
        let s = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        let b = baseline_with_tolerance(&s, 1);
        let text = serde_json::to_string_pretty(&b).expect("serialize");
        let back: RealSpecimenBaseline = serde_json::from_str(&text).expect("deserialize");
        assert_eq!(back.tier1_hits, b.tier1_hits);
        assert_eq!(back.by_miss_kind, b.by_miss_kind);
        assert_eq!(back.tolerance, 1);
    }

    #[test]
    fn iso_date_is_utc_civil_from_days() {
        assert_eq!(iso_date(0), "1970-01-01");
        assert_eq!(iso_date(1_757_030_400), "2025-09-05");
    }
}
