//! Regression tests for `synthpass doctor`'s OCR checks — see the fix's
//! changelog entry (`doctor-proves-model-loads.fixed.md`) for the bug this
//! guards against.
//!
//! Before this fix, `doctor`'s sha256 check only proved the bytes on disk
//! were the known-good file for that filename, never that this build's
//! `rten` could actually parse them — a `.rten` file in a format `rten` no
//! longer supports keeps its original bytes (and hash) but cannot be loaded.
//! These tests spawn the built `synthpass` binary (patterned on
//! `license_roundtrip.rs`) against a garbage file whose sha256 is forced to
//! match via the `SYNTHPASS_OCR_*_SHA256` overrides `verify.rs` already
//! supports — a "hash-correct but unloadable" model constructed the same way
//! the verifier's own tests do, not a fabricated failure.
//!
//! No real `.rten` model is required to run these — the workspace's real
//! models are gitignored and not present in a fresh CI checkout, so every
//! case here is self-contained.

use std::path::PathBuf;
use std::process::Command;

/// Removes the directory even if an assertion panics mid-test.
struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run_doctor(model_dir: &std::path::Path, extra_env: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_synthpass"));
    cmd.arg("doctor")
        .env("SYNTHPASS_LICENSE_SKIP", "1")
        .env("SYNTHPASS_OCR_AUTO_DOWNLOAD", "0")
        .env("SYNTHPASS_OCR_MODEL_DIR", model_dir);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.output().expect("run `synthpass doctor`")
}

/// A model file whose sha256 the verifier accepts (forced via the env
/// override, same mechanism `verify.rs`'s own `respects_env_override` test
/// uses) but whose contents are not a real `.rten` file, so `rten` must
/// reject them at load time. This is the case the sha256-only check could
/// never catch.
#[test]
fn doctor_fails_on_hash_correct_but_unloadable_model() {
    let dir = std::env::temp_dir().join(format!(
        "synthpass-cli-doctor-unloadable-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create temp model dir");
    let _guard = TempDirGuard(dir.clone());

    let garbage = b"not a real rten model file, just garbage bytes for the doctor test\n";
    std::fs::write(dir.join("text-detection.rten"), garbage).expect("write detection stub");
    std::fs::write(dir.join("text-recognition.rten"), garbage).expect("write recognition stub");
    let hash = synthpass_core::audit::sha256_hex(garbage);

    let output = run_doctor(
        &dir,
        &[
            ("SYNTHPASS_OCR_DETECTION_SHA256", &hash),
            ("SYNTHPASS_OCR_RECOGNITION_SHA256", &hash),
        ],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("sha256-verified"),
        "expected the hash check to pass (that's the point — matching bytes, wrong format), got:\n{stdout}"
    );
    assert!(
        stdout.contains("failed to load"),
        "expected the load check to report the format failure, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("load OK"),
        "a model that fails to load must never also report load OK, got:\n{stdout}"
    );
    assert!(
        !output.status.success(),
        "doctor must exit non-zero when the OCR model cannot load, got:\n{stdout}"
    );
}

/// `SYNTHPASS_OCR_ENGINE=native` no longer exists as a selectable engine (the
/// Tesseract-based `native` engine was retired in v1.2.0); the pipeline
/// always runs the Rust engine regardless of this variable. `doctor` must
/// warn rather than silently skip the model check it used to skip for this
/// value.
#[test]
fn doctor_warns_and_still_checks_ocr_models_when_engine_var_is_native() {
    let dir = std::env::temp_dir().join(format!(
        "synthpass-cli-doctor-native-engine-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create empty model dir");
    let _guard = TempDirGuard(dir.clone());

    let output = run_doctor(&dir, &[("SYNTHPASS_OCR_ENGINE", "native")]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SYNTHPASS_OCR_ENGINE=native is set but ignored"),
        "expected a warning that the variable no longer selects an engine, got:\n{stdout}"
    );
    assert!(
        stdout.contains("OCR (rust)"),
        "expected the real (rust) OCR engine to still be checked, got:\n{stdout}"
    );
    assert!(
        !output.status.success(),
        "the model dir is empty, so doctor must still fail overall, got:\n{stdout}"
    );
}
