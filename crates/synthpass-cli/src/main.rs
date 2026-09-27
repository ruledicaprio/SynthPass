//! synthpass (CLI) — command-line front-end for the SynthPass extraction pipeline.
//!
//! Run from the repository root so the in-process model files resolve:
//!
//! ```powershell
//! cargo run -p synthpass-cli -- samples/passports/Canada_Passport_Specimen_PP_CAN_2023_mrz_wide.jpg
//! ```
//!
//! ## The output contract (issue #493)
//!
//! stdout is human-readable by default; it becomes a script-parseable contract only under an
//! explicit `--json`. In **every** mode — default or `--json` — progress and diagnostics
//! (the config echo, "Processing local file...", "OCR successful!", batch's per-run summary
//! line, every line carrying an emoji) go to stderr, never stdout; only extraction *content*
//! is ever printed to stdout.
//!
//! - **Default:** unchanged from before this issue. `synthpass <image>` prints the legacy
//!   pretty-printed v1 `extracted` JSON to stdout for a Tier-1 (deterministic MRZ) result (Tier-2
//!   results print nothing to stdout, matching prior behavior); `synthpass batch <dir|glob>`
//!   prints one pretty-printed per-document record (including a failed document's error) per line.
//! - **`--json`:** `synthpass <image> --json` prints exactly one line of compact JSON — the v2
//!   [`ExtractionV2`](synthpass_core::v2::ExtractionV2) object (`PipelineResult::extracted_v2`) —
//!   to stdout when extraction produced a usable result, and nothing to stdout otherwise (the
//!   error still goes to stderr). `synthpass batch <dir|glob> --json` prints the same shape as
//!   [JSON Lines](https://jsonlines.org/): one compact `ExtractionV2` object per line, one line
//!   per successfully extracted document, in the same deterministic input order the non-`--json`
//!   output uses — a document that failed extraction writes no stdout line, only a stderr error.
//! - Exit codes are unaffected by `--json` (`knowledge/ARCHITECTURE.md` §12), and the `<input>.json`
//!   sidecar file extraction writes to disk is unchanged either way — `--json` only changes what
//!   this process prints to its own stdout.

use serde_json::json;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use synthpass_pipeline::Pipeline;

mod export;
mod generate;

/// The CLI's exit-code convention (issue #492) — every command maps its
/// outcome onto one of these four buckets instead of the ad hoc mix of
/// "print an error to stderr and exit 0 anyway" this file used to have.
/// `knowledge/ARCHITECTURE.md` §12 "Exit codes" documents the same table for
/// readers outside the source; keep both in sync.
///
/// Kept as a small typed enum rather than raw `i32`/`ExitCode` values
/// everywhere so a command's outcome is chosen once, close to the code that
/// knows *why* it failed, and converted to a process exit status exactly
/// once, in `main`. No `std::process::exit` call exists anywhere in this
/// crate — every path returns its `Exit` up through `Result` to `main`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Exit {
    /// 0 — success.
    Ok,
    /// 1 — a runtime or extraction failure: a pipeline error, a document that
    /// failed inside `batch` (even when others in the same batch succeeded),
    /// a `decrypt` failure or missing input file, or a `generate`/`export`
    /// run failure that happens after its arguments parsed fine.
    Failure,
    /// 2 — a usage error: an unknown option, a missing, bad or surplus
    /// argument, or a `generate`/`export` argument error.
    Usage,
    /// 3 — a license refusal on the extraction path (single-document or
    /// `batch`): no license file, or an invalid, expired or
    /// fingerprint-mismatched one. `verify-license` also reports a missing
    /// license *file* under this code, not `Failure` — the command's whole
    /// purpose is to check license validity, and "no license to check" is
    /// itself a refusal. A license that verifies fine but lacks a specific
    /// feature (`batch`/`export`) is **not** this code any more (issue
    /// #494): those uses are metered (one stderr warning) rather than
    /// refused, in line with `knowledge/BRANDING.md` §5.
    License,
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> Self {
        match exit {
            Exit::Ok => ExitCode::SUCCESS,
            Exit::Failure => ExitCode::from(1),
            Exit::Usage => ExitCode::from(2),
            Exit::License => ExitCode::from(3),
        }
    }
}

/// Interior width of the banner box (character count between the two `│`
/// border columns). Wide enough for the longest centered line (the tagline).
const BOX_WIDTH: usize = 76;

/// Centers `text` in a field of `width` chars, padding with spaces on both
/// sides. Truncates instead of panicking if `text` is already too long, so a
/// future edit that overruns `BOX_WIDTH` degrades gracefully rather than
/// crashing `--help`.
fn center(text: &str, width: usize) -> String {
    let len = text.chars().count();
    if len >= width {
        return text.chars().take(width).collect();
    }
    let pad = width - len;
    let left = pad / 2;
    let right = pad - left;
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
}

fn box_line(text: &str) -> String {
    format!("│{}│", center(text, BOX_WIDTH))
}

/// The word the banner draws in block letters.
const WORDMARK: &str = "SYNTHPASS";

/// One 5×5 block-letter glyph per character of [`WORDMARK`]. `None` for a
/// character with no glyph, so a future wordmark edit fails a test instead of
/// drawing a gap.
fn glyph(c: char) -> Option<[&'static str; 5]> {
    Some(match c {
        'S' => [" ████", "█    ", " ███ ", "    █", "████ "],
        'Y' => ["█   █", " █ █ ", "  █  ", "  █  ", "  █  "],
        'N' => ["█   █", "██  █", "█ █ █", "█  ██", "█   █"],
        'T' => ["█████", "  █  ", "  █  ", "  █  ", "  █  "],
        'H' => ["█   █", "█   █", "█████", "█   █", "█   █"],
        'P' => ["████ ", "█   █", "████ ", "█    ", "█    "],
        'A' => [" ███ ", "█   █", "█████", "█   █", "█   █"],
        _ => return None,
    })
}

/// Static ASCII banner in the style of a boxed CLI splash screen — printed
/// once for `--help`/no-args, never on the extraction path.
fn banner() -> String {
    let glyphs: Vec<[&str; 5]> = WORDMARK.chars().filter_map(glyph).collect();

    let mut out = String::new();
    out.push_str(&format!("┌{}┐\n", "─".repeat(BOX_WIDTH)));
    out.push_str(&format!("{}\n", box_line("")));
    out.push_str(&format!("{}\n", box_line("[ SYNTHPASS ]")));
    out.push_str(&format!("{}\n", box_line(&"-".repeat(50))));
    out.push_str(&format!("{}\n", box_line("")));
    for row in 0..5 {
        let line = glyphs.iter().map(|g| g[row]).collect::<Vec<_>>().join("  ");
        out.push_str(&format!("{}\n", box_line(&line)));
    }
    out.push_str(&format!("{}\n", box_line("")));
    out.push_str(&format!(
        "{}\n",
        box_line("Offline ICAO 9303 ID extraction — zero cloud calls, air-gapped by design")
    ));
    out.push_str(&format!("{}\n", box_line("")));
    out.push_str(&format!(
        "{}\n",
        box_line("[ MRZ TIER-1 ]  [ LLM TIER-2 ]  [ ED25519 LICENSE ]  [ AIR-GAPPED ]")
    ));
    out.push_str(&format!("{}\n", box_line("")));
    out.push_str(&format!("└{}┘", "─".repeat(BOX_WIDTH)));
    out
}

fn print_usage() {
    println!("{}", banner());
    println!();
    println!(
        "synthpass v{}  |  github.com/ruledicaprio/SynthPass",
        env!("CARGO_PKG_VERSION")
    );
    println!("{}", "-".repeat(BOX_WIDTH + 2));
    println!();
    println!("Commands");
    println!("  synthpass <path_to_image> [--json] extract (needs a license — see below)");
    println!(
        "  synthpass batch <dir|glob> [--json] extract every image in a directory or matching a glob"
    );
    println!("                                     (needs a license, same as single-document extraction; a license lacking the");
    println!("                                     'batch' feature still runs, metered with a warning — emits one JSON per input + a summary)");
    println!("                                     --json: print the v2 extraction object as compact JSON to stdout instead of the default");
    println!("                                     human-readable output (JSON Lines, one per document, for batch); all progress/diagnostic");
    println!("                                     lines go to stderr either way — see crates/synthpass-cli/README.md");
    println!("  synthpass decrypt <file.json.enc>  decrypt (needs SYNTHPASS_KEY)");
    println!("  synthpass doctor                   preflight: OCR/inferer/license, config sanity");
    println!("  synthpass fetch-models             stage the OCR models: prints URL + SHA-256 to fetch manually, or");
    println!("                                     downloads + verifies them into SYNTHPASS_OCR_MODEL_DIR (`download` feature build)");
    println!(
        "  synthpass fingerprint              print this machine's fingerprint (send to your vendor)"
    );
    println!("  synthpass verify-license [path]    verify a license file (default: SYNTHPASS_LICENSE_PATH or ./license.synthpass)");
    println!("  synthpass generate [--count N] [--seed N] [--profile NAME] [--document-type TYPE] [--out-dir DIR]");
    println!("                                     generate synthetic td1|td2|td3|mrva|mrvb document images + label JSON (no license required)");
    println!("  synthpass export --format jsonl|hf [--count N] [--seed N] [--document-type TYPE] [--pack-pages N] --out-dir DIR");
    println!("                                     export a synthetic corpus as a training dataset (no license required to run; a license");
    println!("                                     lacking the 'export' feature is metered with a warning — see knowledge/EXPORTS.md)");
    println!("  synthpass --help, -h               show this message");
    println!("  synthpass --version, -V            show the version");
    println!();
    println!("No license yet? Run `synthpass fingerprint` and contact your vendor, or set");
    println!("SYNTHPASS_LICENSE_SKIP=1 to bypass the gate for local development.");
}

/// Converts `env::args_os()` (or any `OsString` iterator) into `Vec<String>`,
/// naming the first non-UTF-8 argument in an error instead of the panic
/// `env::args()` gives on one — a Windows path pasted from a non-UTF-8
/// codepage, or a non-UTF-8 locale on Unix, can produce exactly that.
fn args_to_strings(args: impl Iterator<Item = std::ffi::OsString>) -> Result<Vec<String>, String> {
    args.enumerate()
        .map(|(i, a)| {
            a.into_string()
                .map_err(|_| format!("argument {i} is not valid UTF-8"))
        })
        .collect()
}

/// Thin wrapper: `run` does all the work and returns the typed [`Exit`];
/// `main` is the single place that (a) prints the message for an error that
/// bubbled up via `?` instead of an explicit branch in `run`, and (b)
/// converts the outcome to the [`ExitCode`] the process actually exits with.
#[tokio::main]
async fn main() -> ExitCode {
    run()
        .await
        .unwrap_or_else(|e| {
            eprintln!("❌ {e}");
            Exit::Failure
        })
        .into()
}

async fn run() -> Result<Exit, Box<dyn std::error::Error>> {
    let args: Vec<String> = match args_to_strings(env::args_os()) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("❌ {e}");
            return Ok(Exit::Usage);
        }
    };
    if args.len() < 2 {
        print_usage();
        return Ok(Exit::Ok);
    }

    match args[1].as_str() {
        "--help" | "-h" => {
            print_usage();
            return Ok(Exit::Ok);
        }
        "--version" | "-V" => {
            println!("synthpass {}", env!("CARGO_PKG_VERSION"));
            return Ok(Exit::Ok);
        }
        // `synthpass batch <dir|glob>` — extract every matching image, one JSON
        // object per input plus a summary (M5 job queue). Stays synchronous
        // end to end (submit + wait) — only synthpass-serve exposes async job
        // endpoints; see synthpass_pipeline::jobs's module doc.
        "batch" => {
            return batch_command(
                args.get(2).map(String::as_str),
                args.get(3..).unwrap_or_default(),
            )
            .await
        }
        // `synthpass decrypt <file>` — decrypt an AES-256-GCM payload to stdout.
        "decrypt" => return decrypt_command(args.get(2).map(String::as_str)),
        // `synthpass generate` — synthetic passport image + label-JSON factory (M3).
        // No real PII is ever produced, so this bypasses `check_license` entirely,
        // same as `fingerprint`/`verify-license` below.
        "generate" => return generate::generate_command(&args[2..]),
        // `synthpass export` — synthetic-corpus → training-dataset exporter
        // (M6 expansion track, ADR-0007). Export produces synthetic,
        // PII-free data — it is not extraction, so it never refuses on
        // license grounds. The license `export` feature (bulk generation is
        // a capacity surface, BRANDING §5) is metered instead: a missing or
        // invalid license, or one that lacks `export`, prints one warning
        // and export runs anyway (issue #494).
        "export" => {
            warn_unentitled_feature(synthpass_license::FEATURE_EXPORT);
            return export::export_command(&args[2..]);
        }
        // `synthpass doctor` — preflight checks before running the pipeline for real.
        "doctor" => return doctor_command().await,
        // `synthpass fetch-models` — the only place a `.rten` OCR model file
        // is ever fetched (issue #491): the extraction path never downloads.
        // No license required — same reasoning as `fingerprint`/`generate`
        // below, this is setup, not extraction. Runs on the blocking pool: a
        // `download` build fetches with `reqwest::blocking`, which panics when
        // called on an async worker thread.
        "fetch-models" => {
            return tokio::task::spawn_blocking(|| {
                fetch_models_command().map_err(|e| e.to_string())
            })
            .await?
            .map_err(Into::into)
        }
        // `synthpass fingerprint` / `synthpass verify-license` — diagnostic/recovery
        // commands that must work WITHOUT a valid license (you need
        // `fingerprint` to obtain one in the first place), so neither is gated
        // by `check_license` below.
        "fingerprint" => {
            println!("{}", synthpass_license::machine_fingerprint());
            return Ok(Exit::Ok);
        }
        "verify-license" => return verify_license_command(args.get(2).map(String::as_str)),
        _ => {}
    }

    // Anything else is either a file path to extract or a typo'd flag/command
    // — an unknown `-`-prefixed arg is almost never a real filename, so give
    // a targeted error instead of a confusing "File not found: --hlep".
    if args[1].starts_with('-') {
        eprintln!("❌ Unknown option: {}", args[1]);
        eprintln!("   Run `synthpass --help` for usage.");
        return Ok(Exit::Usage);
    }

    let (json_mode, surplus) = split_json_flag(&args[2..]);
    if let Err(e) = reject_surplus_args(&surplus) {
        eprintln!("❌ {e}");
        return Ok(Exit::Usage);
    }

    let input = Path::new(&args[1]);
    if !input.exists() {
        eprintln!("❌ Error: File not found at {}", input.display());
        return Ok(Exit::Failure);
    }

    // Extraction is the one path that actually needs a valid license.
    if let Err(e) = check_license() {
        eprintln!("❌ {e}");
        eprintln!("   run `synthpass fingerprint` and contact your vendor for a license, or set SYNTHPASS_LICENSE_SKIP=1 for local development");
        return Ok(Exit::License);
    }

    let pipeline = Pipeline::from_env();
    // Config echo and progress lines are diagnostics, not content — stderr in
    // every mode, `--json` or not (issue #493).
    eprintln!(
        "⚙️  [Rust] config: ocr={}, inferer={}, license={}",
        pipeline.ocr_engine(),
        pipeline.infer_describe(),
        if env::var("SYNTHPASS_LICENSE_SKIP").as_deref() == Ok("1") {
            "skipped".to_string()
        } else {
            env::var("SYNTHPASS_LICENSE_PATH").unwrap_or_else(|_| DEFAULT_LICENSE_PATH.into())
        }
    );
    print_non_default_ocr_knobs(&pipeline);
    eprintln!(
        "🔄 [Rust] Processing local file: {} (ocr: {})...",
        input.display(),
        pipeline.ocr_engine()
    );

    match pipeline.process_document(input).await {
        Ok(result) => {
            eprintln!("✅ [Rust] OCR successful!");
            eprintln!("💾 [Rust] Saved Markdown to: {}", result.md_path.display());
            match result.method {
                synthpass_pipeline::Method::MrzDeterministic => {
                    eprintln!("🔐 [Rust] ICAO 9303 checksums valid — deterministic MRZ extraction (LLM skipped)");
                    print_line1_integrity(&result);
                    // `--json` prints the v2 object instead, below, once
                    // `print_completion` has decided the document actually
                    // succeeded — this legacy pretty-print is default mode's
                    // stdout content only (issue #493: additive, not a
                    // replacement of today's default-mode output).
                    if !json_mode {
                        if let Some(extracted) = &result.extracted {
                            println!("{}", serde_json::to_string_pretty(extracted)?);
                        }
                    }
                }
                synthpass_pipeline::Method::Llm => {
                    match &result.mrz {
                        Some(m) => eprintln!(
                            "⚠️ [Rust] MRZ found but checksums failed ({:?}) — falling back to LLM",
                            m.checks
                        ),
                        None => eprintln!("ℹ️ [Rust] No MRZ found — using LLM extraction"),
                    }
                    print_line1_integrity(&result);
                }
            }
            // A document that never actually produced usable output — a
            // Tier-2 fallback that itself failed, or a persist failure on
            // either tier — must not exit 0 just because `process_document`
            // itself didn't return `Err`: see `print_completion`'s doc for
            // exactly which cases `completion_message` treats that way. The
            // same condition gates `--json`'s stdout line (issue #493): a
            // document that failed to actually produce usable output prints
            // nothing to stdout, only the diagnostics `print_completion`
            // already sent to stderr.
            let succeeded = print_completion(&result);
            if json_mode && succeeded {
                if let Some(v2) = &result.extracted_v2 {
                    print_json_line(v2)?;
                }
            }
            if succeeded {
                Ok(Exit::Ok)
            } else {
                Ok(Exit::Failure)
            }
        }
        Err(e) => {
            eprintln!("❌ [Rust] {e}");
            Ok(Exit::Failure)
        }
    }
}

/// What to report to mark a document's extraction as finished, or `None`
/// when there's nothing truthful to say there. `None` covers two
/// cases: a Tier-2 failure (`llm_error`, already reported separately by
/// [`print_completion`]) and a persist failure — `Pipeline::process_document`
/// leaves `sidecar_stdout` holding its own `"warning: could not persist
/// output: ..."` message when `write_outputs` fails, on *either* tier, and in
/// that case `json_path` still holds its pre-write default (`<input>.json`,
/// a file that was never created). Printing "saved to: <path>" over that
/// would claim a save that never happened.
fn completion_message(result: &synthpass_pipeline::PipelineResult) -> Option<String> {
    if result.llm_error.is_some() {
        return None;
    }
    if result
        .sidecar_stdout
        .starts_with(synthpass_pipeline::PERSIST_FAILURE_PREFIX)
    {
        return None;
    }
    Some(format!(
        "🎉 [Rust] Pipeline completed via {}! JSON saved to: {}",
        result.method.as_str(),
        result.json_path.display()
    ))
}

/// Prints a document's post-extraction status — entirely to stderr, in every
/// mode (issue #493: this is diagnostics, never stdout content): any
/// diagnostic note from [`PipelineResult::sidecar_stdout`] (previously
/// printed to stdout, and only on the Tier-2 branch — a persist failure on
/// the Tier-1 branch used to go unreported), then either the "saved to" line
/// or the LLM failure warning, per [`completion_message`].
///
/// Returns whether the "saved to" line was printed — i.e. whether
/// [`completion_message`] found anything truthful to report — so the caller
/// can tell a document that never actually produced usable output (a failed
/// Tier-2 fallback, or a persist failure on either tier) apart from a real
/// success, both for the exit code (issue #492) and for whether `--json`
/// prints anything to stdout for this document (issue #493).
///
/// [`PipelineResult::sidecar_stdout`]: synthpass_pipeline::PipelineResult::sidecar_stdout
fn print_completion(result: &synthpass_pipeline::PipelineResult) -> bool {
    if !result.sidecar_stdout.is_empty() {
        eprint!("{}", result.sidecar_stdout);
        if !result.sidecar_stdout.ends_with('\n') {
            eprintln!();
        }
    }
    match completion_message(result) {
        Some(msg) => {
            eprintln!("{msg}");
            true
        }
        None => {
            if let Some(e) = &result.llm_error {
                eprintln!("⚠️ [Rust] LLM extraction failed: {e}");
            }
            false
        }
    }
}

/// Surface a `NeedsReview` line-1 integrity verdict in the terminal, not only in
/// the JSON. Stderr, not stdout, in every mode (issue #493) — this is a
/// diagnostic warning, not extraction content.
///
/// The check digits cover `document_number`, the two dates and
/// `personal_number` — not `document_type`, `issuing_country`, `nationality` or
/// the names. Printing only "checksums valid" over a record whose own integrity
/// verdict flags one of those reads would be an unearned all-clear.
///
/// Called from both extraction paths. The Tier-2 (LLM) path fills
/// `line1_integrity` from the same deterministic read as Tier 1, so a suspect
/// line 1 on an escalated document used to be warned about nowhere.
fn print_line1_integrity(result: &synthpass_pipeline::PipelineResult) {
    let Some(synthpass_core::fusion::Verdict::NeedsReview { reasons }) = result
        .extracted_v2
        .as_ref()
        .and_then(|v2| v2.line1_integrity.as_ref())
    else {
        return;
    };
    eprintln!(
        "⚠️ [Rust] {} line-1 field(s) carry no check digit and look wrong — review before trusting:",
        reasons.len()
    );
    for reason in reasons {
        eprintln!("[Rust]   • {reason:?}");
    }
}

/// Prints the run's non-default `SYNTHPASS_OCR_*` configuration to stderr —
/// one sorted, space-separated, copy-pasteable-into-a-shell `NAME=value`
/// line — or nothing at all when every knob is at its default (issue #495).
/// Shared by `synthpass <file>` (once per document, right after the config
/// echo) and `synthpass batch` (once, before any document is processed):
/// both read the same [`Pipeline::ocr_config_overrides`], so they can never
/// disagree on what "non-default" means.
///
/// `BTreeMap`'s iteration order is already the env-var-name sort order this
/// needs, so no separate sort is required.
fn print_non_default_ocr_knobs(pipeline: &Pipeline) {
    let overrides = pipeline.ocr_config_overrides();
    if overrides.is_empty() {
        return;
    }
    let knobs = overrides
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(" ");
    eprintln!("⚙️  [Rust] non-default OCR knobs: {knobs}");
}

/// Splits an (optionally present) `--json` flag out of `args`, returning
/// whether it was found and the remaining arguments with it removed — so the
/// existing surplus-argument check (`reject_surplus_args`) still catches any
/// other unexpected token. Shared by the single-document and `batch` argument
/// handling (issue #493) rather than hand-rolled twice. `--json` may appear
/// anywhere in `args` (not only immediately after the positional argument);
/// a repeated `--json` is accepted, same as a repeated flag elsewhere in this
/// CLI (see `export`'s `parse_args`).
fn split_json_flag(args: &[String]) -> (bool, Vec<String>) {
    let mut json = false;
    let mut rest = Vec::with_capacity(args.len());
    for arg in args {
        if arg == "--json" {
            json = true;
        } else {
            rest.push(arg.clone());
        }
    }
    (json, rest)
}

/// The `ExtractionV2` a document contributes to `batch --json`'s stdout, if
/// any (issue #493): `None` for a genuine batch failure (`Failed`/`Pending`)
/// or for a `Done` result whose Tier-2 fallback itself failed (`llm_error`
/// set, so `extracted_v2` is `None`) — either way, that document writes no
/// stdout line, only a stderr error, so a script reading stdout as JSON
/// Lines never sees a non-JSON or partial line. Factored out of
/// `batch_command`'s loop so this selection can be pinned by a test without
/// running the pipeline for real.
fn batch_json_value(
    status: &synthpass_pipeline::DocumentStatus,
) -> Option<&synthpass_core::v2::ExtractionV2> {
    match status {
        synthpass_pipeline::DocumentStatus::Done(result) => result.extracted_v2.as_ref(),
        synthpass_pipeline::DocumentStatus::Failed(_)
        | synthpass_pipeline::DocumentStatus::Pending => None,
    }
}

/// Compact (single-line) JSON for `value` — the one framing both the
/// single-document and `batch --json` (JSON Lines) paths share (issue #493),
/// factored out from [`print_json_line`] so a test can pin the exact bytes
/// without capturing this process's real stdout.
fn json_line(value: &synthpass_core::v2::ExtractionV2) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

/// Prints [`json_line`]'s output to stdout, terminated by `\n` (`println!`
/// already appends it) — so a caller parsing stdout line-by-line never has
/// to special-case which command produced a given line.
fn print_json_line(value: &synthpass_core::v2::ExtractionV2) -> Result<(), serde_json::Error> {
    println!("{}", json_line(value)?);
    Ok(())
}

/// Default path for the license file when `SYNTHPASS_LICENSE_PATH` is unset.
const DEFAULT_LICENSE_PATH: &str = "license.synthpass";

/// Gate for the extraction path only (see call site in `main`) — `decrypt`,
/// `doctor`, `fingerprint`, and `verify-license` all stay usable without a
/// valid license. `SYNTHPASS_LICENSE_SKIP=1` bypasses this for local development,
/// mirroring `SYNTHPASS_MODEL_SKIP_VERIFY`.
fn check_license() -> Result<(), String> {
    if env::var("SYNTHPASS_LICENSE_SKIP").as_deref() == Ok("1") {
        return Ok(());
    }
    let path = env::var("SYNTHPASS_LICENSE_PATH").unwrap_or_else(|_| DEFAULT_LICENSE_PATH.into());
    synthpass_license::load_and_check(Path::new(&path))
        .map(|_| ())
        .map_err(|e| format!("license check failed ({path}): {e}"))
}

/// The reason a use of `feature` should be metered rather than refused
/// (issue #494; `knowledge/BRANDING.md` §5: features are metered, never
/// gated — only the extraction path's license *validity* is, via
/// [`check_license`]): `None` when the feature is granted, or when
/// `SYNTHPASS_LICENSE_SKIP=1` opts out of licensing altogether and every
/// feature is unlocked. Otherwise names the feature and the reason: the
/// license is missing/invalid, or it verified fine but doesn't list
/// `feature`.
fn unentitled_feature_reason(feature: &str) -> Option<String> {
    if env::var("SYNTHPASS_LICENSE_SKIP").as_deref() == Ok("1") {
        return None;
    }
    let path = env::var("SYNTHPASS_LICENSE_PATH").unwrap_or_else(|_| DEFAULT_LICENSE_PATH.into());
    match synthpass_license::load_and_check(Path::new(&path)) {
        Ok(status) => match synthpass_license::check_feature(&status.payload, feature) {
            Ok(()) => None,
            Err(_) => Some(format!("the license at {path} does not include it")),
        },
        Err(e) => Some(format!("no valid license at {path} ({e})")),
    }
}

/// Prints the one stderr warning a metered-but-unentitled use of `feature`
/// gets (issue #494) — the command runs regardless. Never touches stdout,
/// so a caller parsing a command's stdout (#493) sees no difference.
fn warn_unentitled_feature(feature: &str) {
    if let Some(reason) = unentitled_feature_reason(feature) {
        eprintln!(
            "⚠️  '{feature}' feature: {reason}. This use is metered, not refused; run \
             `synthpass fingerprint` and contact your vendor for a license with the \
             '{feature}' feature"
        );
    }
}

/// Rejects extra positional arguments a shell may have added by expanding an
/// unquoted glob before synthpass ever saw it — `synthpass batch *.jpg`
/// becomes `batch a.jpg b.jpg c.jpg` once the shell expands `*.jpg`, and
/// `synthpass a.jpg b.jpg` is two files where one path was expected. Both
/// used to silently read only the first argument and drop the rest, which
/// hides most of the input instead of erroring; this names the surplus and
/// suggests the actual fix.
fn reject_surplus_args(surplus: &[String]) -> Result<(), String> {
    if surplus.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "unexpected extra argument(s): {} — if this is an unquoted glob, quote it \
             (e.g. \"*.jpg\") so the shell doesn't expand it before synthpass sees it",
            surplus.join(", ")
        ))
    }
}

/// Minimal shell-style glob matcher: `*` matches any run of characters
/// (including none), `?` matches exactly one *character* (not byte — see
/// below). No bracket/brace/double-star support — this exists only so
/// `synthpass batch` can accept a pattern like `samples/passports/*.jpg`
/// without pulling in a `glob` crate dependency for it.
///
/// Iterative two-pointer match (the classic wildcard-matching algorithm),
/// not the tempting recursive one: a naive `(Some(b'*'), _) =>
/// helper(&p[1..], n) || (!n.is_empty() && helper(p, &n[1..]))` recursion is
/// exponential in the number of `*`s against an adversarial input (e.g.
/// `*a*a*a*a...` against a name with no trailing `a`), which turns a long
/// pattern into an effective hang. This is `O(pattern.len() *
/// name.len())` worst case. Matching over `chars()` rather than bytes also
/// makes `?` consume one Unicode scalar, not one UTF-8 byte — the old
/// byte-oriented version could match half of a multi-byte character.
fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    let (mut pi, mut ni) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut resume = 0usize;

    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            resume = ni;
            pi += 1;
        } else if let Some(star_pi) = star {
            pi = star_pi + 1;
            resume += 1;
            ni = resume;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Recursively walks `dir`, collecting every file for which `keep` returns
/// `true`. Used so `collect_batch_inputs` can find images nested in
/// subdirectories (e.g. `samples/passports/`, `samples/id_cards/`) rather
/// than only those directly inside the given directory.
///
/// Checks `entry.file_type()` (which reports the entry's own type without
/// following a symlink) rather than `path.is_dir()` (which resolves through
/// a symlink via `fs::metadata`) — a directory symlink that points back at
/// an ancestor would otherwise recurse forever. A symlink to a *file* still
/// gets through the `path.is_file()` check below, which is the one place
/// this intentionally still resolves the symlink.
fn walk_dir_files(dir: &Path, keep: &impl Fn(&Path) -> bool, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            walk_dir_files(&path, keep, out);
        } else if path.is_file() && keep(&path) {
            out.push(path);
        }
    }
}

/// Resolves `synthpass batch <arg>`'s argument into a sorted list of image
/// files: every image found recursively under `arg` (including
/// subdirectories) if it's a directory, or every file in `arg`'s parent
/// directory matching `arg`'s filename as a glob pattern otherwise (e.g.
/// `samples/passports/*.jpg`). Sorted so batch output order is deterministic
/// across runs (directory iteration order is not guaranteed by any
/// platform).
fn collect_batch_inputs(arg: &str) -> Result<Vec<PathBuf>, String> {
    let path = Path::new(arg);
    if path.is_dir() {
        let mut files = Vec::new();
        walk_dir_files(
            path,
            &|p| synthpass_pipeline::is_supported_image(p),
            &mut files,
        );
        files.sort();
        return Ok(files);
    }

    let (dir, pattern) = match path.parent().filter(|p| !p.as_os_str().is_empty()) {
        Some(parent) => (
            parent.to_path_buf(),
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string(),
        ),
        None => (PathBuf::from("."), arg.to_string()),
    };
    if !dir.is_dir() {
        return Err(format!("{arg}: not a file, directory, or resolvable glob"));
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("could not read directory {}: {e}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| {
            p.is_file()
                && synthpass_pipeline::is_supported_image(p)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|name| glob_match(&pattern, name))
        })
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("no files matched {arg}"));
    }
    Ok(files)
}

/// `synthpass batch <dir|glob> [--json]` — extract every matching image.
/// Submits the whole batch as one job via `Pipeline::submit` (exercising the
/// same job abstraction `synthpass-serve`'s async endpoints use) and
/// immediately `.wait()`s on it, so from the operator's point of view this
/// behaves like a simple loop over `synthpass <path>` — one record per input,
/// in the same order they were collected, plus a summary line (stderr, like
/// every other progress line — issue #493). Without `--json`, that record is
/// the pretty-printed JSON this command has always printed, one per document
/// including a failed one's error. With `--json`, it is [JSON
/// Lines](https://jsonlines.org/): one compact `ExtractionV2` object per
/// stdout line, one line per document that actually produced one — a failed
/// document (or a `Done` one whose Tier-2 fallback itself failed) writes no
/// stdout line at all, only a stderr error.
async fn batch_command(
    arg: Option<&str>,
    rest: &[String],
) -> Result<Exit, Box<dyn std::error::Error>> {
    let Some(arg) = arg else {
        eprintln!("Usage: synthpass batch <dir|glob> [--json]");
        return Ok(Exit::Usage);
    };

    let (json_mode, surplus) = split_json_flag(rest);
    if let Err(e) = reject_surplus_args(&surplus) {
        eprintln!("❌ {e}");
        return Ok(Exit::Usage);
    }

    // Batch is extraction, so it needs the same valid license
    // single-document extraction does (ARCHITECTURE §6) — this is the one
    // refusal (exit 3) left in `batch`. A valid license that simply lacks
    // the `batch` feature is metered, not refused (issue #494): see the
    // `warn_unentitled_feature` call below, after this check has passed.
    if let Err(e) = check_license() {
        eprintln!("❌ {e}");
        eprintln!(
            "   run `synthpass fingerprint` and contact your vendor for a license, or set \
             SYNTHPASS_LICENSE_SKIP=1 for local development"
        );
        return Ok(Exit::License);
    }
    warn_unentitled_feature(synthpass_license::FEATURE_BATCH);

    // Neither branch here is a usage error: `arg` itself may be perfectly
    // well-formed (an existing directory with nothing in it, or a glob whose
    // syntax is fine but that matches nothing right now) — this is the same
    // "target doesn't resolve to anything" runtime condition a missing input
    // file is for `decrypt`/single-document extraction, so it gets the same
    // code (1), not the usage code (2).
    let inputs = match collect_batch_inputs(arg) {
        Ok(inputs) if inputs.is_empty() => {
            eprintln!("❌ no image files found at {arg}");
            return Ok(Exit::Failure);
        }
        Ok(inputs) => inputs,
        Err(e) => {
            eprintln!("❌ {e}");
            return Ok(Exit::Failure);
        }
    };

    eprintln!(
        "🔄 [Rust] submitting {} document(s) from {arg} for batch extraction...",
        inputs.len()
    );

    let pipeline = Pipeline::from_env();
    print_non_default_ocr_knobs(&pipeline);
    let handle = pipeline.submit(inputs.clone());
    let status = handle.wait().await;

    let mut tier1 = 0usize;
    let mut tier2 = 0usize;
    let mut failed = 0usize;

    for (input, entry) in inputs.iter().zip(handle.documents().iter()) {
        // `record` and the tier1/tier2/failed counters are exactly what this
        // loop computed before `--json` existed — issue #493 only changes
        // *which* of `record`'s renderings reaches stdout, never these
        // counts or the default-mode (pretty-printed record) output.
        let record = match &entry.status {
            synthpass_pipeline::DocumentStatus::Done(result) => {
                match result.method {
                    synthpass_pipeline::Method::MrzDeterministic => tier1 += 1,
                    synthpass_pipeline::Method::Llm => tier2 += 1,
                }
                json!({
                    "input": input.display().to_string(),
                    "method": result.method.as_str(),
                    "extracted": result.extracted,
                    "error": result.llm_error,
                })
            }
            synthpass_pipeline::DocumentStatus::Failed(e) => {
                failed += 1;
                json!({ "input": input.display().to_string(), "error": e })
            }
            // Unreachable once `wait()` has returned (every document is
            // populated by then — see `JobHandle::wait`'s doc), but degrade
            // to a clearly-labeled record rather than panicking if that
            // invariant is ever violated.
            synthpass_pipeline::DocumentStatus::Pending => {
                failed += 1;
                json!({
                    "input": input.display().to_string(),
                    "error": "job ended without a result for this document",
                })
            }
        };

        if json_mode {
            match batch_json_value(&entry.status) {
                Some(v2) => print_json_line(v2)?,
                None => {
                    let reason = record
                        .get("error")
                        .and_then(|e| e.as_str())
                        .unwrap_or("extraction failed");
                    eprintln!("❌ [Rust] {}: {reason}", input.display());
                }
            }
        } else {
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
    }

    eprintln!(
        "🎉 [Rust] batch complete ({}): {tier1} via Tier 1, {tier2} via Tier 2, {failed} failed",
        status.as_str()
    );

    // Any failed document makes the whole batch exit 1, even when the rest
    // succeeded — the summary line above already says how many; the exit
    // code is what a script actually branches on (issue #492).
    if failed > 0 {
        Ok(Exit::Failure)
    } else {
        Ok(Exit::Ok)
    }
}

/// `synthpass doctor`'s license block: required unless `SYNTHPASS_LICENSE_SKIP=1`
/// (mirrors the `SYNTHPASS_KEY`/`SYNTHPASS_AUDIT_LOG` blocks' shape below, but this
/// one toggles `ok` since — unlike those two — a missing/invalid license
/// blocks the extraction path entirely, not just an optional feature).
fn check_license_doctor(ok: &mut bool) {
    if env::var("SYNTHPASS_LICENSE_SKIP").as_deref() == Ok("1") {
        println!("✅ License: skipped (SYNTHPASS_LICENSE_SKIP=1)");
        return;
    }
    let path = env::var("SYNTHPASS_LICENSE_PATH").unwrap_or_else(|_| DEFAULT_LICENSE_PATH.into());
    match synthpass_license::load_and_check(Path::new(&path)) {
        Ok(status) => {
            let days_left = status.days_until_expiry(synthpass_license::current_unix());
            if days_left < 30 {
                println!(
                    "⚠️  License ({path}): {} — expires in {days_left} days",
                    status.payload.tier
                );
            } else {
                println!(
                    "✅ License ({path}): {} — expires in {days_left} days",
                    status.payload.tier
                );
            }
        }
        Err(e) => {
            println!("❌ License ({path}): {e}");
            if matches!(e, synthpass_license::LicenseError::Io(_)) {
                println!(
                    "   Tip: no license yet? Run `synthpass fingerprint` to get one from your \
                     vendor, or set SYNTHPASS_LICENSE_SKIP=1 for local development."
                );
            }
            *ok = false;
        }
    }
}

/// `synthpass verify-license [path]` — verify a license file and print its
/// status. `path` overrides `SYNTHPASS_LICENSE_PATH`/the default. Exits with
/// the license-refusal code (3), not the generic failure code (1), on *any*
/// failure to verify — including a missing license *file* (a
/// `synthpass_license::LicenseError::Io`): this command's whole job is to
/// report validity, so "no license to check" is itself a refusal, the same
/// as an invalid or expired one, not a separate runtime error.
fn verify_license_command(path: Option<&str>) -> Result<Exit, Box<dyn std::error::Error>> {
    let path = path.map(String::from).unwrap_or_else(|| {
        env::var("SYNTHPASS_LICENSE_PATH").unwrap_or_else(|_| DEFAULT_LICENSE_PATH.into())
    });

    match synthpass_license::load_and_check(Path::new(&path)) {
        Ok(status) => {
            let days_left = status.days_until_expiry(synthpass_license::current_unix());
            println!("✅ License valid ({path})");
            println!("   id: {}", status.payload.license_id);
            println!("   customer: {}", status.payload.customer);
            println!("   tier: {}", status.payload.tier);
            println!(
                "   bound to: {}",
                if status.payload.hw_fingerprint.is_empty() {
                    "(unbound — any machine)".to_string()
                } else {
                    status.payload.hw_fingerprint
                }
            );
            println!("   days until expiry: {days_left}");
            Ok(Exit::Ok)
        }
        Err(e) => {
            eprintln!("❌ License invalid ({path}): {e}");
            Ok(Exit::License)
        }
    }
}

/// `synthpass decrypt <file.json.enc>` — decrypt an AES-256-GCM payload (written when
/// `SYNTHPASS_KEY` is set) to stdout, using the same `SYNTHPASS_KEY`.
///
/// Exit codes: no `SYNTHPASS_KEY` set, or one that isn't a valid base64
/// 32-byte key, is a usage/config error (2) — nothing about the input file
/// has been touched yet. A missing input file, and a decrypt failure once a
/// well-formed key is in hand (wrong key, corrupt/truncated ciphertext), are
/// both a runtime failure (1).
fn decrypt_command(file: Option<&str>) -> Result<Exit, Box<dyn std::error::Error>> {
    let Some(file) = file else {
        eprintln!("Usage: synthpass decrypt <file.json.enc>   (reads key from SYNTHPASS_KEY)");
        return Ok(Exit::Usage);
    };
    let key = match env::var("SYNTHPASS_KEY") {
        Ok(s) => match synthpass_core::crypt::key_from_base64(&s) {
            Ok(key) => key,
            Err(e) => {
                eprintln!("❌ {e}");
                return Ok(Exit::Usage);
            }
        },
        Err(_) => {
            eprintln!("❌ set SYNTHPASS_KEY (base64-encoded 32-byte AES-256 key)");
            return Ok(Exit::Usage);
        }
    };
    let data = std::fs::read(file)?;
    match synthpass_core::crypt::decrypt(&key, &data) {
        Ok(plain) => {
            std::io::stdout().write_all(&plain)?;
            Ok(Exit::Ok)
        }
        Err(e) => {
            eprintln!("❌ decrypt failed: {e}");
            Ok(Exit::Failure)
        }
    }
}

/// `synthpass fetch-models` — the only place a `.rten` OCR model file is ever
/// fetched (issue #491: the extraction path never downloads). No license
/// required — same reasoning as `fingerprint`/`generate`: this is setup, not
/// extraction.
///
/// Default build (no `download` cargo feature, so `reqwest` isn't even linked
/// in — see `crates/synthpass-ocr/Cargo.toml`): prints each model's pinned
/// URL and expected SHA-256 plus the target directory, so an operator can
/// fetch and verify manually (e.g. `curl` + `sha256sum`).
///
/// `download`-feature build: actually downloads into that directory and
/// verifies each file's SHA-256 — skipping the network call and just
/// re-verifying a file that's already there, mirroring ci.yml's own "download
/// only if missing, verify either way" fetch step. On a mismatch (freshly
/// downloaded or already staged) the bad file is deleted and the command
/// fails (see `synthpass_ocr::download::fetch_and_verify`).
#[cfg(feature = "ocr-native-rust")]
fn fetch_models_command() -> Result<Exit, Box<dyn std::error::Error>> {
    let model_dir = env::var("SYNTHPASS_OCR_MODEL_DIR").unwrap_or_else(|_| ".".into());
    let dir = Path::new(&model_dir);
    println!(
        "OCR model directory (SYNTHPASS_OCR_MODEL_DIR): {}",
        dir.display()
    );
    fetch_models_for_dir(dir)
}

#[cfg(all(feature = "ocr-native-rust", feature = "download"))]
fn fetch_models_for_dir(dir: &Path) -> Result<Exit, Box<dyn std::error::Error>> {
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("❌ could not create {}: {e}", dir.display());
        return Ok(Exit::Failure);
    }
    let mut ok = true;
    for spec in synthpass_ocr::download::model_specs() {
        match synthpass_ocr::download::fetch_and_verify(&spec, dir) {
            Ok((path, synthpass_ocr::download::FetchOutcome::Downloaded)) => println!(
                "✅ {}: downloaded and sha256-verified at {}",
                spec.filename,
                path.display()
            ),
            Ok((path, synthpass_ocr::download::FetchOutcome::AlreadyPresentAndVerified)) => {
                println!(
                    "✅ {}: already present and sha256-verified at {}",
                    spec.filename,
                    path.display()
                )
            }
            Err(e) => {
                eprintln!("❌ {}: {e}", spec.filename);
                ok = false;
            }
        }
    }
    Ok(if ok { Exit::Ok } else { Exit::Failure })
}

#[cfg(all(feature = "ocr-native-rust", not(feature = "download")))]
fn fetch_models_for_dir(dir: &Path) -> Result<Exit, Box<dyn std::error::Error>> {
    println!(
        "This build cannot download models — the `download` cargo feature is off, so `reqwest` \
         isn't even linked in (principle 5: no runtime downloads on the extraction path, \
         enforced at compile time here)."
    );
    println!();
    println!("Fetch each file yourself and verify its SHA-256, e.g.:");
    println!();
    for spec in synthpass_ocr::download::model_specs() {
        let target = dir.join(spec.filename);
        println!("  curl -fL -o {} {}", target.display(), spec.url);
        println!(
            "  echo \"{}  {}\" | sha256sum -c -",
            (spec.expected_sha256)(),
            target.display()
        );
        println!();
    }
    Ok(Exit::Ok)
}

#[cfg(not(feature = "ocr-native-rust"))]
fn fetch_models_command() -> Result<Exit, Box<dyn std::error::Error>> {
    eprintln!("❌ this build lacks the `ocr-native-rust` feature — no OCR models to fetch");
    Ok(Exit::Failure)
}

/// `synthpass doctor` — preflight checks: OCR/inferer reachability + config sanity.
/// OCR reachability is required for the pipeline to run at all (a failure
/// there is [`Exit::Failure`] — the same code any other runtime failure gets,
/// since `doctor`'s job is to predict whether extraction would work, not to
/// distinguish which subsystem failed by exit code).
///
/// Tier 2 (the GGUF inferer) is optional (issue #496): a failed Tier-2 check
/// fails `doctor` only when `SYNTHPASS_MODEL_PATH` is set. See
/// [`tier2_failure`].
///
/// `SYNTHPASS_KEY`/`SYNTHPASS_AUDIT_LOG` checks are advisory since those
/// features are optional and never flip `ok`.
async fn doctor_command() -> Result<Exit, Box<dyn std::error::Error>> {
    let mut ok = true;

    // `SYNTHPASS_OCR_ENGINE` no longer selects anything: `synthpass_pipeline::ocr::engine_from_env`
    // treats any value other than `rust` (including the retired Tesseract-based
    // `native` engine) as `rust` and only warns. Doctor used to special-case
    // `native` and skip the model check entirely on the theory that it needed
    // no local files — but the pipeline runs the Rust OCR engine regardless of
    // this var, so that branch was checking a configuration nothing can
    // actually select, while silently skipping the check that matters for the
    // engine that always runs. Warn instead, and always check the real engine.
    let ocr_engine = env::var("SYNTHPASS_OCR_ENGINE").unwrap_or_else(|_| "rust".into());
    if ocr_engine != "rust" {
        println!(
            "⚠️  SYNTHPASS_OCR_ENGINE={ocr_engine} is set but ignored — the pipeline always uses \
             the pure-Rust OCR engine (the Tesseract-based `native` engine was retired in \
             v1.2.0); checking that engine below"
        );
    }
    check_rust_ocr_models(&mut ok);

    let pipeline = Pipeline::from_env();
    let infer_desc = pipeline.infer_describe();
    let model_path_explicit = env::var("SYNTHPASS_MODEL_PATH").is_ok();
    match pipeline.infer_health().await {
        Ok(status) => println!("✅ Tier-2 inferer ({infer_desc}): {status}"),
        Err(e) => match tier2_failure(model_path_explicit) {
            Tier2Failure::Fail => {
                println!("❌ Tier-2 inferer ({infer_desc}) NOT healthy: {e}");
                ok = false;
            }
            Tier2Failure::Advisory => println!(
                "⚠️  Tier-2 inferer ({infer_desc}) not available: {e}. Tier 1 still works; set \
                 SYNTHPASS_MODEL_PATH to a GGUF file to enable Tier 2"
            ),
        },
    }

    check_license_doctor(&mut ok);

    if let Ok(key) = env::var("SYNTHPASS_KEY") {
        match synthpass_core::crypt::key_from_base64(&key) {
            Ok(_) => println!("✅ SYNTHPASS_KEY is a valid base64 32-byte key"),
            Err(e) => println!(
                "⚠️  SYNTHPASS_KEY is set but invalid ({e}) — encryption will be silently disabled"
            ),
        }
    }

    if let Ok(log_path) = env::var("SYNTHPASS_AUDIT_LOG") {
        let parent_ok = Path::new(&log_path)
            .parent()
            .map(|p| p.as_os_str().is_empty() || p.exists())
            .unwrap_or(true);
        if parent_ok {
            println!("✅ SYNTHPASS_AUDIT_LOG parent directory exists ({log_path})");
        } else {
            println!(
                "⚠️  SYNTHPASS_AUDIT_LOG parent directory does not exist ({log_path}) — audit records will silently fail to write"
            );
        }
    }

    if ok {
        Ok(Exit::Ok)
    } else {
        // Each failed check already printed its own `❌ ...` line above —
        // that's the diagnostic. Nothing further is added on stderr here;
        // the non-zero exit code is what a script or `doctor`'s own caller
        // actually branches on.
        Ok(Exit::Failure)
    }
}

/// How [`doctor_command`] reports a failed Tier-2 health check (issue #496).
/// A missing and an unloadable GGUF are treated alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tier2Failure {
    /// `SYNTHPASS_MODEL_PATH` unset: a Tier-1-only install is supported, so
    /// `doctor` prints a `⚠️` line and its exit code is unaffected.
    Advisory,
    /// `SYNTHPASS_MODEL_PATH` set: the user asked for that model, so a missing
    /// or unloadable one fails `doctor`.
    Fail,
}

fn tier2_failure(model_path_explicit: bool) -> Tier2Failure {
    if model_path_explicit {
        Tier2Failure::Fail
    } else {
        Tier2Failure::Advisory
    }
}

/// Models are baked into the binary at compile time (`ocr-embedded` feature,
/// musl release builds) — nothing on disk to check, but that is not the same
/// as nothing that can fail: a baked-in `.rten` file can still be in a format
/// this build's `rten` can no longer parse (see `rten`'s own format
/// deprecation), and that is exactly the failure a preflight command exists
/// to catch before the user hits it mid-extraction. So this constructs the
/// real engine from the embedded bytes rather than just asserting they are
/// present — loading is cheap (well under a second); a full recognition pass
/// is not (the `native_ocr_e2e` test takes ~268s), so this stops at "does it
/// load", which is what a preflight can afford and what a format
/// incompatibility actually breaks.
#[cfg(all(feature = "ocr-native-rust", feature = "ocr-embedded"))]
fn check_rust_ocr_models(ok: &mut bool) {
    match synthpass_ocr::NativeOcr::load_embedded() {
        Ok(_) => {
            println!("✅ OCR (rust) detection+recognition models embedded in binary and load OK")
        }
        Err(e) => {
            println!("❌ OCR (rust) embedded models present but failed to load: {e}");
            *ok = false;
        }
    }
}

/// Checks the default `rust` OCR engine's two `.rten` weight files: present
/// under `SYNTHPASS_OCR_MODEL_DIR` (default `.`), sha256-verified, and —
/// unlike a byte check — actually loadable by this build's `rten`.
///
/// The sha256 check alone is not enough: it proves the bytes on disk are the
/// known-good file for that filename, not that the current binary can do
/// anything with them. A `.rten` file whose *format* this `rten` version can
/// no longer parse still has the correct, unmodified bytes, so the hash
/// matches and this used to print a bare `✅` over a model the pipeline could
/// not actually use. Constructing [`synthpass_ocr::NativeOcr`] from the files
/// is what catches that: it deserializes both weight files and builds the
/// `ocrs` engines around them, which is fast (well under a second) — unlike
/// an actual recognition pass, which the `native_ocr_e2e` test measures at
/// ~268s and which a preflight command cannot afford to run.
#[cfg(all(feature = "ocr-native-rust", not(feature = "ocr-embedded")))]
type OcrModelVerifyFn = fn(&Path) -> Result<(), synthpass_ocr::verify::VerifyError>;

#[cfg(all(feature = "ocr-native-rust", not(feature = "ocr-embedded")))]
fn check_rust_ocr_models(ok: &mut bool) {
    let model_dir = env::var("SYNTHPASS_OCR_MODEL_DIR").unwrap_or_else(|_| ".".into());
    let dir = Path::new(&model_dir);
    let skip = synthpass_ocr::verify::skip_verify();
    let detection_path = dir.join(synthpass_ocr::download::DETECTION_FILENAME);
    let recognition_path = dir.join(synthpass_ocr::download::RECOGNITION_FILENAME);
    let checks: [(&str, &PathBuf, OcrModelVerifyFn); 2] = [
        (
            "detection",
            &detection_path,
            synthpass_ocr::verify::verify_detection_model,
        ),
        (
            "recognition",
            &recognition_path,
            synthpass_ocr::verify::verify_recognition_model,
        ),
    ];
    let mut both_present = true;
    for (label, path, verify_fn) in checks {
        if !path.exists() {
            println!(
                "❌ OCR (rust) {label} model missing at {} — run `synthpass fetch-models` to \
                 stage it",
                path.display()
            );
            *ok = false;
            both_present = false;
        } else if skip {
            println!(
                "✅ OCR (rust) {label} model present at {} (sha256 verification skipped)",
                path.display()
            );
        } else {
            match verify_fn(path) {
                Ok(()) => println!(
                    "✅ OCR (rust) {label} model present and sha256-verified at {}",
                    path.display()
                ),
                Err(e) => {
                    println!("❌ OCR (rust) {label} model: {e}");
                    *ok = false;
                }
            }
        }
    }

    // Only attempt a load when both files are at least present — a missing
    // file already failed above, and there is nothing useful to load. This
    // runs regardless of a hash mismatch or `skip_verify`, so it is the only
    // check standing between "present" and "usable" when verification is
    // disabled (`SYNTHPASS_OCR_MODEL_SKIP_VERIFY=1`).
    if both_present {
        match synthpass_ocr::NativeOcr::load(&detection_path, &recognition_path) {
            Ok(_) => println!("✅ OCR (rust) models load OK (detection+recognition engines built)"),
            Err(e) => {
                println!("❌ OCR (rust) models present but failed to load: {e}");
                *ok = false;
            }
        }
    }
}

#[cfg(not(feature = "ocr-native-rust"))]
fn check_rust_ocr_models(ok: &mut bool) {
    println!("❌ OCR engine 'rust' selected but this build lacks the `ocr-native-rust` feature");
    *ok = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_wordmark_character_has_a_glyph() {
        for c in WORDMARK.chars() {
            assert!(glyph(c).is_some(), "no banner glyph for {c:?}");
        }
    }

    #[test]
    fn every_banner_line_fills_the_box_exactly() {
        for line in banner().lines() {
            assert_eq!(line.chars().count(), BOX_WIDTH + 2, "misaligned: {line:?}");
        }
    }

    #[test]
    fn banner_draws_the_wordmark_not_the_old_name() {
        // Row 0 of the block letters, rebuilt from the glyph table: the
        // banner must contain it, so it spells WORDMARK and nothing else.
        let row0 = WORDMARK
            .chars()
            .filter_map(glyph)
            .map(|g| g[0])
            .collect::<Vec<_>>()
            .join("  ");
        assert!(banner().contains(&row0));
        assert_eq!(WORDMARK, "SYNTHPASS");
    }

    fn sample_result(
        sidecar_stdout: &str,
        llm_error: Option<&str>,
    ) -> synthpass_pipeline::PipelineResult {
        synthpass_pipeline::PipelineResult {
            markdown: String::new(),
            md_path: PathBuf::from("doc.md"),
            json_path: PathBuf::from("doc.json"),
            extracted: None,
            extracted_v2: None,
            llm_error: llm_error.map(String::from),
            sidecar_stdout: sidecar_stdout.to_string(),
            mrz: None,
            method: synthpass_pipeline::Method::MrzDeterministic,
        }
    }

    #[test]
    fn a_failed_tier2_check_is_advisory_without_an_explicit_model_path() {
        assert_eq!(tier2_failure(false), Tier2Failure::Advisory);
    }

    #[test]
    fn a_failed_tier2_check_fails_doctor_when_the_model_path_is_set() {
        assert_eq!(tier2_failure(true), Tier2Failure::Fail);
    }

    #[test]
    fn completion_message_suppressed_on_persist_failure() {
        let result = sample_result("warning: could not persist output: disk full", None);
        assert_eq!(
            completion_message(&result),
            None,
            "a persist failure must not print a \"saved to\" line"
        );
    }

    #[test]
    fn completion_message_present_on_clean_tier1_success() {
        let result = sample_result("", None);
        let msg = completion_message(&result).expect("no persist failure, no llm_error");
        assert!(msg.contains("saved to"));
        assert!(msg.contains("doc.json"));
    }

    #[test]
    fn completion_message_absent_on_llm_error() {
        let result = sample_result("", Some("model unavailable"));
        assert_eq!(completion_message(&result), None);
    }

    #[test]
    fn reject_surplus_args_accepts_empty() {
        assert!(reject_surplus_args(&[]).is_ok());
    }

    #[test]
    fn reject_surplus_args_names_the_extras() {
        let surplus = vec!["b.jpg".to_string(), "c.jpg".to_string()];
        let err = reject_surplus_args(&surplus).unwrap_err();
        assert!(err.contains("unexpected"));
        assert!(err.contains("b.jpg"));
        assert!(err.contains("c.jpg"));
        assert!(err.contains("quote"));
    }

    // ── issue #493: the `--json` output contract ──────────────────────

    #[test]
    fn split_json_flag_finds_the_flag_anywhere_and_leaves_the_rest() {
        let (json, rest) = split_json_flag(&["a.jpg".to_string(), "--json".to_string()]);
        assert!(json);
        assert_eq!(rest, vec!["a.jpg".to_string()]);

        let (json, rest) = split_json_flag(&["--json".to_string(), "a.jpg".to_string()]);
        assert!(json);
        assert_eq!(rest, vec!["a.jpg".to_string()]);
    }

    #[test]
    fn split_json_flag_absent_leaves_args_untouched() {
        let (json, rest) = split_json_flag(&["a.jpg".to_string(), "b.jpg".to_string()]);
        assert!(!json);
        assert_eq!(rest, vec!["a.jpg".to_string(), "b.jpg".to_string()]);
    }

    #[test]
    fn split_json_flag_repeated_is_still_just_json_mode() {
        let (json, rest) = split_json_flag(&["--json".to_string(), "--json".to_string()]);
        assert!(json);
        assert!(rest.is_empty());
    }

    /// A deterministic `ExtractionV2` fixture — every field that would carry
    /// real PII is a fixed placeholder, never a sample document. Used only to
    /// pin the wire shape of `--json`'s stdout line, not to test extraction
    /// itself (that needs real OCR/MRZ models — out of scope for a fast,
    /// network-free unit test; see `tests/json_contract.rs`'s module doc).
    fn sample_v2(
        extraction_method: &str,
        document_number: &str,
    ) -> synthpass_core::v2::ExtractionV2 {
        let mut v2 = synthpass_core::v2::ExtractionV2::default();
        v2.extraction_method = extraction_method.to_string();
        v2.fields.document_number = Some(document_number.to_string());
        v2
    }

    /// Pins `--json`'s stdout line to an exact byte sequence (issue #493):
    /// one line, compact (no embedded newlines/indentation), and it parses
    /// back to the same `ExtractionV2`. A real end-to-end extraction isn't
    /// exercised here — see this module's doc for why — so this test's job
    /// is narrower but load-bearing: prove `json_line`/`print_json_line`
    /// serialize `ExtractionV2` as one compact JSON line and nothing else,
    /// which is the part `main`'s own logic controls.
    #[test]
    fn json_line_is_one_compact_line_that_round_trips() {
        let v2 = sample_v2("mrz-deterministic", "L898902C3");
        let line = json_line(&v2).expect("ExtractionV2 always serializes");

        // The golden itself: byte-for-byte, so a change to field order, a
        // renamed key, or a newly-`Some`/non-empty default (which would
        // silently start serializing an `mrz`/`validity`/`trace`/`barcodes`
        // key this fixture omits) fails here even though it wouldn't fail
        // the round-trip check below.
        assert_eq!(
            line,
            r#"{"schema_version":2,"document":{"kind":"other"},"fields":{"document_type":null,"issuing_country":null,"document_number":"L898902C3","surname":null,"given_names":null,"nationality":null,"date_of_birth":null,"sex":null,"date_of_expiry":null,"personal_number":null,"optional_data_1":null,"optional_data_2":null},"confidence":{"document_type":0.0,"issuing_country":0.0,"document_number":0.0,"surname":0.0,"given_names":0.0,"nationality":0.0,"date_of_birth":0.0,"sex":0.0,"date_of_expiry":0.0,"personal_number":0.0,"optional_data_1":0.0,"optional_data_2":0.0},"provenance":{"kind":"mrz_checksum"},"extraction_method":"mrz-deterministic"}"#
        );
        assert_eq!(
            line.lines().count(),
            1,
            "must be exactly one line, got: {line:?}"
        );
        assert!(
            !line.contains("  ") && !line.contains('\n'),
            "must be compact JSON, not pretty-printed: {line:?}"
        );

        let parsed: synthpass_core::v2::ExtractionV2 =
            serde_json::from_str(&line).expect("must parse back as ExtractionV2");
        assert_eq!(parsed, v2, "must round-trip losslessly");

        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        let obj = value.as_object().expect("top level is a JSON object");
        for key in [
            "schema_version",
            "document",
            "fields",
            "confidence",
            "provenance",
            "extraction_method",
        ] {
            assert!(
                obj.contains_key(key),
                "missing top-level key {key:?}: {line}"
            );
        }
        assert_eq!(value["schema_version"], 2);
        assert_eq!(value["extraction_method"], "mrz-deterministic");
        assert_eq!(value["fields"]["document_number"], "L898902C3");
    }

    fn sample_result_with_v2(
        v2: synthpass_core::v2::ExtractionV2,
        method: synthpass_pipeline::Method,
    ) -> synthpass_pipeline::PipelineResult {
        synthpass_pipeline::PipelineResult {
            markdown: String::new(),
            md_path: PathBuf::from("doc.md"),
            json_path: PathBuf::from("doc.json"),
            extracted: None,
            extracted_v2: Some(v2),
            llm_error: None,
            sidecar_stdout: String::new(),
            mrz: None,
            method,
        }
    }

    #[test]
    fn batch_json_value_is_some_only_for_a_document_that_actually_extracted() {
        let done_ok = synthpass_pipeline::DocumentStatus::Done(Box::new(sample_result_with_v2(
            sample_v2("mrz-deterministic", "A"),
            synthpass_pipeline::Method::MrzDeterministic,
        )));
        assert!(batch_json_value(&done_ok).is_some());

        // `Done` but Tier-2 itself failed: `extracted_v2` is `None`, exactly
        // like a real `llm_error` result — no stdout line, only stderr.
        let mut done_failed =
            sample_result_with_v2(sample_v2("llm", "B"), synthpass_pipeline::Method::Llm);
        done_failed.extracted_v2 = None;
        done_failed.llm_error = Some("model unavailable".to_string());
        let done_failed = synthpass_pipeline::DocumentStatus::Done(Box::new(done_failed));
        assert!(batch_json_value(&done_failed).is_none());

        let failed = synthpass_pipeline::DocumentStatus::Failed("ocr error".to_string());
        assert!(batch_json_value(&failed).is_none());

        let pending = synthpass_pipeline::DocumentStatus::Pending;
        assert!(batch_json_value(&pending).is_none());
    }

    /// The batch-`--json` golden: N documents, all of which actually
    /// extracted, give exactly N JSON Lines in the same order as the inputs —
    /// pinned against the real per-document selection logic
    /// (`batch_json_value`) rather than re-implemented in the test, so a
    /// change to that logic is what this test would actually catch.
    #[test]
    fn batch_json_all_successful_documents_yield_one_line_each_in_order() {
        let statuses = [
            synthpass_pipeline::DocumentStatus::Done(Box::new(sample_result_with_v2(
                sample_v2("mrz-deterministic", "DOC-A"),
                synthpass_pipeline::Method::MrzDeterministic,
            ))),
            synthpass_pipeline::DocumentStatus::Done(Box::new(sample_result_with_v2(
                sample_v2("mrz-deterministic", "DOC-B"),
                synthpass_pipeline::Method::MrzDeterministic,
            ))),
            synthpass_pipeline::DocumentStatus::Done(Box::new(sample_result_with_v2(
                sample_v2("llm", "DOC-C"),
                synthpass_pipeline::Method::Llm,
            ))),
        ];

        let lines: Vec<String> = statuses
            .iter()
            .map(|status| {
                let v2 = batch_json_value(status).expect("every status here extracted");
                json_line(v2).expect("ExtractionV2 always serializes")
            })
            .collect();

        assert_eq!(lines.len(), statuses.len());
        let document_numbers: Vec<String> = lines
            .iter()
            .map(|line| {
                let value: serde_json::Value = serde_json::from_str(line).unwrap();
                value["fields"]["document_number"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(document_numbers, vec!["DOC-A", "DOC-B", "DOC-C"]);
    }

    /// A batch mixing a genuine failure among successes: the failed document
    /// contributes no line, and the successful ones keep their input order
    /// around it (issue #493's "a document that fails extraction writes no
    /// stdout line").
    #[test]
    fn batch_json_a_failed_document_contributes_no_line_but_does_not_shift_the_others() {
        let statuses = [
            synthpass_pipeline::DocumentStatus::Done(Box::new(sample_result_with_v2(
                sample_v2("mrz-deterministic", "DOC-A"),
                synthpass_pipeline::Method::MrzDeterministic,
            ))),
            synthpass_pipeline::DocumentStatus::Failed("ocr error".to_string()),
            synthpass_pipeline::DocumentStatus::Done(Box::new(sample_result_with_v2(
                sample_v2("mrz-deterministic", "DOC-C"),
                synthpass_pipeline::Method::MrzDeterministic,
            ))),
        ];

        let lines: Vec<String> = statuses
            .iter()
            .filter_map(|status| batch_json_value(status))
            .map(|v2| json_line(v2).expect("ExtractionV2 always serializes"))
            .collect();

        assert_eq!(
            lines.len(),
            2,
            "the failed document must not appear: {lines:?}"
        );
        let value_a: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        let value_c: serde_json::Value = serde_json::from_str(&lines[1]).unwrap();
        assert_eq!(value_a["fields"]["document_number"], "DOC-A");
        assert_eq!(value_c["fields"]["document_number"], "DOC-C");
    }

    #[test]
    fn args_to_strings_converts_valid_utf8() {
        let args = vec![
            std::ffi::OsString::from("synthpass"),
            std::ffi::OsString::from("batch"),
        ];
        let result = args_to_strings(args.into_iter()).expect("all valid UTF-8");
        assert_eq!(result, vec!["synthpass".to_string(), "batch".to_string()]);
    }

    #[cfg(unix)]
    #[test]
    fn args_to_strings_reports_non_utf8_instead_of_panicking() {
        use std::os::unix::ffi::OsStringExt;
        let args = vec![
            std::ffi::OsString::from("synthpass"),
            std::ffi::OsString::from_vec(vec![0xFF, 0xFE]),
        ];
        let err = args_to_strings(args.into_iter()).unwrap_err();
        assert!(err.contains("argument 1"));
        assert!(err.contains("not valid UTF-8"));
    }

    /// Table test for [`glob_match`]: `*`/`?` semantics, a non-ASCII case
    /// proving `?` consumes one *character* rather than one UTF-8 byte, and a
    /// many-star pattern proving the iterative matcher doesn't blow up the
    /// way the old recursive one did.
    #[test]
    fn glob_match_table() {
        let cases: &[(&str, &str, bool)] = &[
            ("*.jpg", "photo.jpg", true),
            ("*.jpg", "photo.png", false),
            ("a?c", "abc", true),
            ("a?c", "ac", false),
            ("a?c", "abbc", false),
            ("*", "anything", true),
            ("*", "", true),
            ("", "", true),
            ("", "x", false),
            ("photo??.jpg", "photo42.jpg", true),
            ("photo??.jpg", "photo4.jpg", false),
            // `?` must match one *character*, not one UTF-8 byte: 'é' below
            // is a single Unicode scalar encoded as 2 bytes.
            ("a?c", "aéc", true),
            ("*.jpg", "sub/photo.jpg", true), // no path-segment semantics: `*` crosses `/`
        ];
        for (pattern, name, expected) in cases {
            assert_eq!(
                glob_match(pattern, name),
                *expected,
                "glob_match({pattern:?}, {name:?})"
            );
        }
    }

    /// A pattern with many stars used to be exponential in the old recursive
    /// matcher against a name with no match near the end; this just needs to
    /// return (quickly) rather than hang.
    #[test]
    fn glob_match_many_stars_does_not_blow_up() {
        let pattern = "*a".repeat(30) + "b";
        let name = "a".repeat(40); // never matches (no trailing 'b')
        assert!(!glob_match(&pattern, &name));
    }

    #[test]
    fn collect_batch_inputs_glob_branch_filters_non_images() {
        let dir = std::env::temp_dir().join(format!(
            "synthpass_cli_glob_filter_test_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.jpg"), b"x").unwrap();
        std::fs::write(dir.join("a.json"), b"{}").unwrap();
        std::fs::write(dir.join("a.md"), b"# x").unwrap();

        let pattern = dir.join("*").to_string_lossy().to_string();
        let files = collect_batch_inputs(&pattern).expect("should match at least one file");

        assert_eq!(files.len(), 1, "expected only the image file: {files:?}");
        assert_eq!(files[0].file_name().unwrap(), "a.jpg");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn walk_dir_files_does_not_follow_a_directory_symlink_loop() {
        use std::os::unix::fs::symlink;
        let dir = std::env::temp_dir().join(format!(
            "synthpass_cli_symlink_loop_test_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.jpg"), b"x").unwrap();
        // Symlink back to the directory itself — a real ancestor loop, since
        // walking into it would recurse into `dir` again forever.
        symlink(&dir, dir.join("loop")).unwrap();

        let mut out = Vec::new();
        walk_dir_files(
            &dir,
            &|p| synthpass_pipeline::is_supported_image(p),
            &mut out,
        );

        assert_eq!(out.len(), 1, "expected only the real file, got: {out:?}");
        assert_eq!(out[0].file_name().unwrap(), "a.jpg");

        std::fs::remove_dir_all(&dir).ok();
    }
}
