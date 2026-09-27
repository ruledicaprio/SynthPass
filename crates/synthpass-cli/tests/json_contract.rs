//! Black-box coverage of the CLI's output contract (issue #493): stdout is
//! human-readable by default and becomes a script-parseable contract only
//! under `--json`, and — in *every* mode — progress and diagnostics (the
//! config echo, "Processing local file...", "OCR successful!", batch's
//! per-run summary line, every emoji line) go to stderr, never stdout.
//!
//! These tests spawn the built `synthpass` binary (patterned on
//! `exit_codes.rs`/`license_roundtrip.rs`), with `SYNTHPASS_LICENSE_SKIP=1`
//! so the license gate never enters into it, and — critically — an *empty*
//! `SYNTHPASS_OCR_MODEL_DIR` with `SYNTHPASS_OCR_AUTO_DOWNLOAD=0`, the same
//! trick `license_roundtrip.rs` uses to get a fast, deterministic, network-free
//! failure out of the OCR stage: no real `.rten` model is required (this
//! workspace's real models are gitignored and not present in a fresh CI
//! checkout), and — unlike a real recognition pass, which the ignored
//! `native_ocr_e2e`/`rust_ocr_smoke` tests measure at minutes in debug mode —
//! this fails in milliseconds, so it is safe to run as part of a plain
//! `cargo test --workspace`, unmarked `#[ignore]`.
//!
//! That means these tests exercise the *stream split* (item 3 of #497's
//! golden-stdout-tests ask) end to end, but not a real extraction's stdout
//! *content* — a document that fails this fast produces no successful
//! extraction to print in the first place. The content side (item 1: a
//! single `--json` line is exactly the v2 object; item 2: `batch --json`
//! emits one JSONL line per successfully extracted document, in order) is
//! pinned instead against a constructed `ExtractionV2`/`DocumentStatus` in
//! `src/main.rs`'s own unit tests (`json_line_is_one_compact_line_that_round_trips`,
//! `batch_json_all_successful_documents_yield_one_line_each_in_order`,
//! `batch_json_a_failed_document_contributes_no_line_but_does_not_shift_the_others`)
//! — a real end-to-end extraction needs the real OCR/MRZ models this
//! environment (and a fresh CI checkout) may not have staged, and would run
//! debug-mode `rten` inference on a par with the ignored e2e tests' multi-minute
//! runtime, which a test that isn't `#[ignore]`d cannot afford.

use std::path::PathBuf;
use std::process::{Command, Output};

/// Removes the directory even if an assertion panics mid-test.
struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// An empty, real directory with no `.rten` files in it — `SYNTHPASS_OCR_MODEL_DIR`
/// pointed here plus `SYNTHPASS_OCR_AUTO_DOWNLOAD=0` makes the OCR stage fail
/// fast (missing-file, not a network fetch or a real inference pass) and
/// deterministically, the same way `license_roundtrip.rs` gets a downstream
/// failure without staging real models.
fn empty_model_dir(label: &str) -> (PathBuf, TempDirGuard) {
    let dir = std::env::temp_dir().join(format!(
        "synthpass-cli-test-json-contract-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create empty model dir");
    let guard = TempDirGuard(dir.clone());
    (dir, guard)
}

fn base_cmd(model_dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_synthpass"));
    cmd.env("SYNTHPASS_LICENSE_SKIP", "1")
        .env("SYNTHPASS_OCR_AUTO_DOWNLOAD", "0")
        .env("SYNTHPASS_OCR_MODEL_DIR", model_dir);
    cmd
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

// ── single-document extraction ──────────────────────────────────────────

#[test]
fn single_document_default_mode_writes_nothing_to_stdout_on_a_fast_ocr_failure() {
    let (model_dir, _guard) = empty_model_dir("single-default");
    let input = std::env::temp_dir().join(format!(
        "synthpass-cli-test-json-contract-input-{}.jpg",
        std::process::id()
    ));
    std::fs::write(&input, b"not a real image").expect("write dummy input");

    let output = base_cmd(&model_dir)
        .arg(input.to_str().unwrap())
        .output()
        .expect("run `synthpass <file>`");
    std::fs::remove_file(&input).ok();

    assert_eq!(
        stdout_of(&output),
        "",
        "a document that never reached a usable result must print nothing to stdout, got: {output:?}"
    );
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("⚙️") && stderr.contains("config:"),
        "the config echo must be on stderr, got: {stderr}"
    );
    assert!(
        stderr.contains("🔄") && stderr.contains("Processing local file"),
        "the processing progress line must be on stderr, got: {stderr}"
    );
}

#[test]
fn single_document_json_mode_writes_nothing_to_stdout_on_a_fast_ocr_failure() {
    let (model_dir, _guard) = empty_model_dir("single-json");
    let input = std::env::temp_dir().join(format!(
        "synthpass-cli-test-json-contract-input-json-{}.jpg",
        std::process::id()
    ));
    std::fs::write(&input, b"not a real image").expect("write dummy input");

    let output = base_cmd(&model_dir)
        .args([input.to_str().unwrap(), "--json"])
        .output()
        .expect("run `synthpass <file> --json`");
    std::fs::remove_file(&input).ok();

    assert_eq!(
        stdout_of(&output),
        "",
        "--json must print nothing to stdout for a document that failed extraction, got: {output:?}"
    );
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("⚙️") && stderr.contains("🔄"),
        "progress lines must still reach stderr under --json, got: {stderr}"
    );
}

#[test]
fn a_trailing_json_flag_does_not_get_treated_as_a_surplus_argument() {
    // Same fast-failure setup, just proving `--json` itself is consumed and
    // never reported as an "unexpected extra argument" (issue #493 sits on
    // top of #503's surplus-argument handling, not around it).
    let (model_dir, _guard) = empty_model_dir("single-json-flag-parse");
    let input = std::env::temp_dir().join(format!(
        "synthpass-cli-test-json-contract-flag-parse-{}.jpg",
        std::process::id()
    ));
    std::fs::write(&input, b"not a real image").expect("write dummy input");

    let output = base_cmd(&model_dir)
        .args([input.to_str().unwrap(), "--json"])
        .output()
        .expect("run `synthpass <file> --json`");
    std::fs::remove_file(&input).ok();

    let stderr = stderr_of(&output);
    assert!(
        !stderr.contains("unexpected extra argument"),
        "--json must not be rejected as a surplus argument, got: {stderr}"
    );
}

#[test]
fn a_genuine_surplus_argument_alongside_json_is_still_a_usage_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_synthpass"))
        .args(["a.jpg", "--json", "b.jpg"])
        .output()
        .expect("run `synthpass a.jpg --json b.jpg`");

    assert_eq!(
        output.status.code(),
        Some(2),
        "a real surplus argument must still be a usage error even alongside --json, got: {output:?}"
    );
    assert!(
        stderr_of(&output).contains("unexpected extra argument"),
        "expected the surplus-argument message, got: {}",
        stderr_of(&output)
    );
}

// ── batch extraction ────────────────────────────────────────────────────

/// A directory containing one image whose *name* (not contents —
/// `is_supported_image` only checks the extension) `collect_batch_inputs`
/// will pick up, so `batch` has exactly one document to process — which then
/// fails fast at the OCR stage the same way the single-document tests above
/// do.
fn dir_with_one_dummy_image(label: &str) -> (PathBuf, TempDirGuard) {
    let (dir, guard) = empty_model_dir(label);
    std::fs::write(dir.join("doc.jpg"), b"not a real image").expect("write dummy batch input");
    (dir, guard)
}

#[test]
fn batch_default_mode_progress_and_summary_are_on_stderr_not_stdout() {
    let (dir, _guard) = dir_with_one_dummy_image("batch-default");

    let output = base_cmd(&dir)
        .args(["batch", dir.to_str().unwrap()])
        .output()
        .expect("run `synthpass batch <dir>`");

    let stdout = stdout_of(&output);
    assert!(
        !stdout.contains('🔄') && !stdout.contains('🎉'),
        "no progress/summary emoji line may reach stdout in default mode either, got: {stdout}"
    );

    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("🔄") && stderr.contains("submitting"),
        "the submission progress line must be on stderr, got: {stderr}"
    );
    assert!(
        stderr.contains("🎉") && stderr.contains("batch complete"),
        "the summary line must be on stderr, got: {stderr}"
    );
}

#[test]
fn batch_json_mode_writes_no_stdout_line_for_a_failed_document() {
    let (dir, _guard) = dir_with_one_dummy_image("batch-json");

    let output = base_cmd(&dir)
        .args(["batch", dir.to_str().unwrap(), "--json"])
        .output()
        .expect("run `synthpass batch <dir> --json`");

    assert_eq!(
        stdout_of(&output),
        "",
        "the one document in this batch failed OCR, so --json must print no JSONL line for it, got: {output:?}"
    );

    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("🔄") && stderr.contains("🎉"),
        "progress and summary lines must still reach stderr under --json, got: {stderr}"
    );
    assert!(
        stderr.contains("❌"),
        "the failed document's error must be reported on stderr under --json, got: {stderr}"
    );
}

#[test]
fn batch_surplus_argument_alongside_json_is_still_a_usage_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_synthpass"))
        .args(["batch", "some-dir", "--json", "extra"])
        .output()
        .expect("run `synthpass batch some-dir --json extra`");

    assert_eq!(
        output.status.code(),
        Some(2),
        "a real surplus argument must still be a usage error even alongside --json, got: {output:?}"
    );
    assert!(
        stderr_of(&output).contains("unexpected extra argument"),
        "expected the surplus-argument message, got: {}",
        stderr_of(&output)
    );
}

// ── `--json` does not leak into other subcommands ───────────────────────

/// `export` has its own hand-rolled flag parser (`export::parse_args`) with
/// no `--json` flag of its own — `--json` is single-document/`batch`-only, so
/// passing it to `export` must fail the same way any other unrecognized
/// export flag does, not be silently accepted or misinterpreted (issue #493's
/// "does `--json` conflict with another subcommand's flags" question).
#[test]
fn json_flag_is_not_recognized_by_export() {
    let output = Command::new(env!("CARGO_BIN_EXE_synthpass"))
        .args(["export", "--format", "jsonl", "--out-dir", "out", "--json"])
        .output()
        .expect("run `synthpass export ... --json`");

    assert_eq!(
        output.status.code(),
        Some(2),
        "export has no --json flag, so this must be its usual usage error, got: {output:?}"
    );
    assert!(
        stderr_of(&output).contains("unknown argument: --json"),
        "expected export's own unknown-argument message, got: {}",
        stderr_of(&output)
    );
}
