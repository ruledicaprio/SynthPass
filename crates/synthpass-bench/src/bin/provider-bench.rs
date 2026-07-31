//! `provider-bench` — M7 multi-provider benchmark. Runs every registered
//! [`FieldReader`](synthpass_die::FieldReader) (today: the deterministic
//! `MrzReader` and the LLM's `LlmFieldReader`) against the same synthetic
//! corpus and reports, per provider: accuracy, speed, JSON validity, an
//! unsupported-assertion rate, and resident memory.
//!
//! Requires the shipped GGUF present at the repo root (same precondition as
//! the existing `#[ignore]`d `native_llm_e2e`/`parity` tests in
//! `synthpass-llm`) — the LLM provider has to actually run to be measured.
//!
//! ```text
//! provider-bench [--count N] [--seed N] [--profile NAME] [--out PATH] [--measure-memory]
//!   --count N          number of documents to check (default: 20)
//!   --seed N           base seed; document i uses seed N+i (default: 0)
//!   --profile NAME     clean|mobile|scanner|worn|border-kiosk|damaged|all (default: clean)
//!   --out PATH         report JSON path (default: provider-bench-report.json)
//!   --measure-memory   sample process RSS around each provider's loop
//!                      (requires the `measure-memory` feature; see its doc
//!                      comment in Cargo.toml for why this is coarse)
//! ```

use serde::Serialize;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use synthpass_bench::provider_bench::{run_provider_bench, ProviderReport};
use synthpass_bench::{generate_corpus, ProfileChoice};
use synthpass_ocr::NativeOcr;
use synthpass_pipeline::{InferBackend, NativeInferer, OcrEngine, Pipeline, RustOcrEngine};

struct Args {
    count: u64,
    seed: u64,
    profile: ProfileChoice,
    out: String,
    measure_memory: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            count: 20,
            seed: 0,
            profile: ProfileChoice::Clean,
            out: "provider-bench-report.json".to_string(),
            measure_memory: false,
        }
    }
}

fn usage() {
    eprintln!(
        "Usage: provider-bench [--count N] [--seed N] [--profile NAME] [--out PATH] [--measure-memory]"
    );
    eprintln!("  --count N          number of documents to check (default: 20)");
    eprintln!("  --seed N           base seed; document i uses seed N+i (default: 0)");
    eprintln!(
        "  --profile NAME     clean|mobile|scanner|worn|border-kiosk|damaged|all (default: clean)"
    );
    eprintln!("  --out PATH         report JSON path (default: provider-bench-report.json)");
    eprintln!("  --measure-memory   sample process RSS around each provider's loop (coarse)");
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
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(parsed)
}

#[derive(Serialize)]
struct CapabilityReport {
    deterministic: bool,
    cost: &'static str,
}

#[derive(Serialize)]
struct AccuracyReport {
    field_match_rate: f64,
    mean_cer: f64,
    per_field_cer: Vec<PerFieldCer>,
}

#[derive(Serialize)]
struct PerFieldCer {
    field: &'static str,
    mean_cer: f64,
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
    unsupported_assertion_rate: f64,
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
                cost: r.capability.cost,
            },
            accuracy: AccuracyReport {
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
            unsupported_assertion_rate: r.unsupported_assertion_rate,
            declared_resident_bytes: r.declared_resident_bytes,
            measured_rss_delta_bytes: r.measured_rss_delta_bytes,
        }
    }
}

#[derive(Serialize)]
struct Report {
    timestamp_unix: u64,
    profile: &'static str,
    count: u64,
    seed_start: u64,
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

    let corpus = generate_corpus(parsed.profile, parsed.seed, parsed.count);
    let reports =
        run_provider_bench(pipeline.catalog(), &ocr, &corpus, parsed.measure_memory).await;

    for r in &reports {
        println!(
            "{}: {} docs, field match {:.1}%, mean CER {:.3}, mean {} ms, unsupported-assertion \
             rate {:.1}%",
            r.provider_id,
            r.documents,
            r.accuracy.field_match_rate * 100.0,
            r.accuracy.mean_cer,
            r.speed.mean.as_millis(),
            r.unsupported_assertion_rate * 100.0,
        );
    }

    let report = Report {
        timestamp_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        profile: parsed.profile.as_str(),
        count: parsed.count,
        seed_start: parsed.seed,
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
