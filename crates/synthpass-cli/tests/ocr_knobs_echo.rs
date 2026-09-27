//! Black-box coverage of issue #495: a non-default `SYNTHPASS_OCR_*` knob
//! must be visible in the CLI's own output, not only inside `trace` — printed
//! once on stderr, sorted, space-separated `NAME=value`, and never on stdout
//! in any mode.
//!
//! Same trick as `json_contract.rs`: `SYNTHPASS_LICENSE_SKIP=1` and an empty
//! `SYNTHPASS_OCR_MODEL_DIR` make the OCR stage fail fast and
//! deterministically (issue #491: the extraction path never downloads a
//! model, so a missing one fails at once) — no real `.rten` model is needed,
//! and the run is fast enough to leave unmarked in `cargo test --workspace`.
//! The line this test pins is printed *before* that OCR failure (right after
//! the config echo / at the start of `batch`), so it appears regardless.
//!
//! Every test here starts from `env_remove`-ing all nine knobs the issue
//! names, on the spawned command only — never `std::env::set_var` on this
//! test process itself, which would race every other test in the binary.

use std::path::PathBuf;
use std::process::{Command, Output};

/// The exact nine `SYNTHPASS_OCR_*` knobs issue #495 records — the seven
/// `synthpass_ocr::OcrArms` measurement arms plus the retry pass/time budget.
/// Not on this list, deliberately: `VERBOSE`, `DUMP_VARIANTS`, `THREADS`,
/// `ENGINE`, `AUTO_DOWNLOAD`, `MODEL_DIR`, `*_SHA256`, `MODEL_SKIP_VERIFY` —
/// logging, diagnostics, concurrency, engine selection, a retired variable,
/// and model location and integrity, none of them in the nine #495 names.
const ALL_KNOBS: &[&str] = &[
    "SYNTHPASS_OCR_TEXTURE",
    "SYNTHPASS_OCR_ORDER",
    "SYNTHPASS_OCR_ROTATE",
    "SYNTHPASS_OCR_SKEW",
    "SYNTHPASS_OCR_CHARGRID",
    "SYNTHPASS_OCR_STOP",
    "SYNTHPASS_OCR_CONFIRM_PASSES",
    "SYNTHPASS_OCR_MAX_PASSES",
    "SYNTHPASS_OCR_MAX_SECONDS",
];

/// Removes the directory even if an assertion panics mid-test.
struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// An empty, real directory with no `.rten` files in it — see
/// `json_contract.rs`'s identically-named helper for why this makes the OCR
/// stage fail fast rather than fetching or running real inference.
fn empty_model_dir(label: &str) -> (PathBuf, TempDirGuard) {
    let dir = std::env::temp_dir().join(format!(
        "synthpass-cli-test-ocr-knobs-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create empty model dir");
    let guard = TempDirGuard(dir.clone());
    (dir, guard)
}

/// A command with every one of the nine knobs removed from its environment
/// (regardless of what this test *process*'s own environment carries) and
/// the license/model-dir setup every test here needs.
fn base_cmd(model_dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_synthpass"));
    cmd.env("SYNTHPASS_LICENSE_SKIP", "1")
        .env("SYNTHPASS_OCR_MODEL_DIR", model_dir);
    for knob in ALL_KNOBS {
        cmd.env_remove(knob);
    }
    cmd
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn dummy_input(label: &str) -> PathBuf {
    let input = std::env::temp_dir().join(format!(
        "synthpass-cli-test-ocr-knobs-input-{label}-{}.jpg",
        std::process::id()
    ));
    std::fs::write(&input, b"not a real image").expect("write dummy input");
    input
}

#[test]
fn single_document_prints_nothing_when_every_knob_is_default() {
    let (model_dir, _guard) = empty_model_dir("single-default-knobs");
    let input = dummy_input("single-default-knobs");

    let output = base_cmd(&model_dir)
        .arg(input.to_str().unwrap())
        .output()
        .expect("run `synthpass <file>`");
    std::fs::remove_file(&input).ok();

    let stderr = stderr_of(&output);
    assert!(
        !stderr.contains("non-default OCR knobs"),
        "no knob was set, so the line must be absent entirely, got: {stderr}"
    );
    assert!(
        !stdout_of(&output).contains("non-default OCR knobs"),
        "the line must never reach stdout"
    );
}

#[test]
fn single_document_echoes_one_non_default_knob_on_stderr_only() {
    let (model_dir, _guard) = empty_model_dir("single-one-knob");
    let input = dummy_input("single-one-knob");

    let output = base_cmd(&model_dir)
        .env("SYNTHPASS_OCR_MAX_PASSES", "1")
        .arg(input.to_str().unwrap())
        .output()
        .expect("run `synthpass <file>`");
    std::fs::remove_file(&input).ok();

    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("⚙️  [Rust] non-default OCR knobs: SYNTHPASS_OCR_MAX_PASSES=1"),
        "expected the exact copy-pasteable line, got: {stderr}"
    );
    assert!(
        !stdout_of(&output).contains("non-default OCR knobs"),
        "the line must never reach stdout, even when a knob is non-default"
    );
}

#[test]
fn single_document_sorts_multiple_non_default_knobs_by_env_var_name() {
    let (model_dir, _guard) = empty_model_dir("single-two-knobs");
    let input = dummy_input("single-two-knobs");

    // Set in reverse alphabetical order on the command line to prove the
    // printed line is sorted, not "insertion order" or "argument order".
    let output = base_cmd(&model_dir)
        .env("SYNTHPASS_OCR_STOP", "clean")
        .env("SYNTHPASS_OCR_MAX_PASSES", "1")
        .arg(input.to_str().unwrap())
        .output()
        .expect("run `synthpass <file>`");
    std::fs::remove_file(&input).ok();

    let stderr = stderr_of(&output);
    assert!(
        stderr.contains(
            "⚙️  [Rust] non-default OCR knobs: SYNTHPASS_OCR_MAX_PASSES=1 SYNTHPASS_OCR_STOP=clean"
        ),
        "expected both knobs, sorted by env var name, got: {stderr}"
    );
}

#[test]
fn batch_prints_nothing_when_every_knob_is_default() {
    let (dir, _guard) = empty_model_dir("batch-default-knobs");
    std::fs::write(dir.join("doc.jpg"), b"not a real image").expect("write dummy batch input");

    let output = base_cmd(&dir)
        .args(["batch", dir.to_str().unwrap()])
        .output()
        .expect("run `synthpass batch <dir>`");

    let stderr = stderr_of(&output);
    assert!(
        !stderr.contains("non-default OCR knobs"),
        "no knob was set, so the line must be absent entirely, got: {stderr}"
    );
    assert!(!stdout_of(&output).contains("non-default OCR knobs"));
}

#[test]
fn batch_echoes_one_non_default_knob_once_on_stderr_only() {
    let (dir, _guard) = empty_model_dir("batch-one-knob");
    std::fs::write(dir.join("doc.jpg"), b"not a real image").expect("write dummy batch input");

    let output = base_cmd(&dir)
        .env("SYNTHPASS_OCR_STOP", "clean")
        .args(["batch", dir.to_str().unwrap()])
        .output()
        .expect("run `synthpass batch <dir>`");

    let stderr = stderr_of(&output);
    assert_eq!(
        stderr.matches("non-default OCR knobs").count(),
        1,
        "batch prints the line once, at the start, not per document: {stderr}"
    );
    assert!(
        stderr.contains("⚙️  [Rust] non-default OCR knobs: SYNTHPASS_OCR_STOP=clean"),
        "expected the exact copy-pasteable line, got: {stderr}"
    );
    assert!(!stdout_of(&output).contains("non-default OCR knobs"));
}
