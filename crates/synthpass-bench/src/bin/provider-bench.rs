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
//! provider-bench [--count N] [--seed N] [--profile NAME] [--out PATH]
//!                [--measure-memory] [--real-specimens]
//!   --count N          number of documents to check (default: 20)
//!   --seed N           base seed; document i uses seed N+i (default: 0)
//!   --profile NAME     clean|mobile|scanner|worn|border-kiosk|damaged|all (default: clean)
//!   --out PATH         report JSON path (default: provider-bench-report.json)
//!   --measure-memory   sample process RSS around each provider's loop
//!                      (requires the `measure-memory` feature; see its doc
//!                      comment in Cargo.toml for why this is coarse)
//!   --real-specimens   run over samples/ instead of the synthetic corpus;
//!                      ground truth is optional per specimen, and
//!                      --count/--seed/--profile are ignored
//! ```

use serde::Serialize;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use synthpass_bench::provider_bench::{
    run_provider_bench, run_provider_bench_real, AssertionBucket, ProviderReport,
    UnsupportedAssertion,
};
use synthpass_bench::{generate_corpus, load_real_specimens, ProfileChoice};
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
}

impl Default for Args {
    fn default() -> Self {
        Self {
            count: 20,
            seed: 0,
            profile: ProfileChoice::Clean,
            out: "provider-bench-report.json".to_string(),
            measure_memory: false,
            real_specimens: false,
            limit: None,
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
    eprintln!("  --out PATH         report JSON path (default: provider-bench-report.json)");
    eprintln!("  --measure-memory   sample process RSS around each provider's loop (coarse)");
    eprintln!(
        "  --real-specimens   run over samples/ instead of the synthetic corpus (ground truth \
         optional per specimen; --count/--seed/--profile are ignored)"
    );
    eprintln!(
        "  --limit N          with --real-specimens: run N specimens spread evenly across the \
         corpus (deterministic; default: all)"
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
            other => return Err(format!("unknown argument: {other}")),
        }
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
struct AssertionBucketReport {
    rate: f64,
    assertions_total: usize,
    documents: usize,
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

    let root = repo_root();
    let ocr = NativeOcr::load(
        &root.join("text-detection.rten"),
        &root.join("text-recognition.rten"),
    )
    .expect("failed to load OCR models — run from the repo root");

    // A second OCR engine handle purely to satisfy `Pipeline::new`'s
    // constructor — never actually invoked. Every document in this harness
    // is OCR'd once via `ocr` above and shared across every reader; the
    // pipeline itself is only a vehicle for reaching its registered
    // `ProviderCatalog` (`LlmFieldReader` is `pub(crate)` there).
    let pipeline_ocr: Box<dyn OcrEngine> = Box::new(RustOcrEngine::new(&root, false));
    let infer: Box<dyn InferBackend> = Box::new(NativeInferer::new(model_path(), n_ctx()));
    let pipeline = Pipeline::new(pipeline_ocr, infer);

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
        let reports =
            run_provider_bench_real(pipeline.catalog(), &ocr, &specimens, parsed.measure_memory)
                .await;
        (
            reports,
            "real-specimens",
            None,
            specimens.len() as u64,
            None,
        )
    } else {
        let corpus = generate_corpus(parsed.profile, parsed.seed, parsed.count);
        let reports =
            run_provider_bench(pipeline.catalog(), &ocr, &corpus, parsed.measure_memory).await;
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
                let half = |label: &str, b: &Option<AssertionBucket>| match b {
                    Some(b) => format!(", {label} {:.1}% ({} docs)", b.rate * 100.0, b.documents),
                    None => String::new(),
                };
                format!(
                    "{:.1}%{}{}",
                    overall.rate * 100.0,
                    half("with-MRZ", with_mrz_anchor),
                    half("no-MRZ", without_mrz_anchor),
                )
            }
            UnsupportedAssertion::NotApplicable { reason } => format!("n/a ({reason})"),
        };
        println!(
            "{}: {} docs ({} labelled), field match {field_match}, mean CER {mean_cer}, mean {} \
             ms, unsupported-assertion rate {unsupported}",
            r.provider_id,
            r.documents,
            r.accuracy.labelled_documents,
            r.speed.mean.as_millis(),
        );
    }

    let report = Report {
        timestamp_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        source,
        profile,
        count,
        seed_start,
        providers: reports.into_iter().map(ProviderRow::from).collect(),
    };
    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    std::fs::write(&parsed.out, json).expect("write report");
    println!("report written to {}", parsed.out);
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
}
