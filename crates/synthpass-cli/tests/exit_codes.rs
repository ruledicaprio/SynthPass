//! Black-box coverage of the CLI's exit-code convention (issue #492), across
//! commands not already covered by `decrypt_roundtrip.rs` / `license_roundtrip.rs`
//! / `doctor_ocr_models.rs`. See `knowledge/ARCHITECTURE.md` §12 "Exit codes"
//! for the table this pins down: 0 success, 1 runtime/extraction failure, 2
//! usage error, 3 license refusal.
//!
//! Every test spawns the built `synthpass` binary (`Command::env`/`env_remove`
//! per test) rather than mutating this process's own environment, so tests can
//! run concurrently without interfering with each other.

use std::path::PathBuf;
use std::process::Command;

/// Removes the directory even if an assertion panics mid-test.
struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn version_flag_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_synthpass"))
        .arg("--version")
        .output()
        .expect("run `synthpass --version`");

    assert_eq!(output.status.code(), Some(0), "got: {output:?}");
}

#[test]
fn unknown_option_is_a_usage_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_synthpass"))
        .arg("--totally-bogus-flag")
        .output()
        .expect("run `synthpass --totally-bogus-flag`");

    assert_eq!(
        output.status.code(),
        Some(2),
        "an unknown option is a usage error (2), got: {output:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Unknown option"),
        "expected an 'Unknown option' message, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn missing_input_file_is_a_runtime_failure() {
    let missing = std::env::temp_dir().join(format!(
        "synthpass-cli-test-exitcodes-missing-{}.jpg",
        std::process::id()
    ));

    let output = Command::new(env!("CARGO_BIN_EXE_synthpass"))
        .arg(missing.to_str().unwrap())
        .output()
        .expect("run `synthpass <missing file>`");

    assert_eq!(
        output.status.code(),
        Some(1),
        "a missing input file is a runtime failure (1), got: {output:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("File not found"),
        "expected a 'File not found' message, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn surplus_positional_arguments_are_a_usage_error() {
    // Neither path needs to exist — `reject_surplus_args` runs before the
    // file-existence check.
    let output = Command::new(env!("CARGO_BIN_EXE_synthpass"))
        .args(["a.jpg", "b.jpg"])
        .output()
        .expect("run `synthpass a.jpg b.jpg`");

    assert_eq!(
        output.status.code(),
        Some(2),
        "surplus positional arguments are a usage error (2), got: {output:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unexpected extra argument"),
        "expected the surplus-argument message, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// No license file, `SYNTHPASS_LICENSE_SKIP` explicitly unset (removed rather
/// than relying on it being absent from the ambient environment) — the
/// extraction path's license gate must refuse with exit 3, not fall through
/// to a generic runtime failure.
#[test]
fn extraction_without_a_license_is_a_license_refusal() {
    let input_path = std::env::temp_dir().join(format!(
        "synthpass-cli-test-exitcodes-input-{}.jpg",
        std::process::id()
    ));
    std::fs::write(&input_path, b"not a real image").expect("write dummy input");

    let missing_license = std::env::temp_dir().join(format!(
        "synthpass-cli-test-exitcodes-no-such-license-{}.mlis",
        std::process::id()
    ));

    let output = Command::new(env!("CARGO_BIN_EXE_synthpass"))
        .arg(input_path.to_str().unwrap())
        .env_remove("SYNTHPASS_LICENSE_SKIP")
        .env("SYNTHPASS_LICENSE_PATH", missing_license.to_str().unwrap())
        .output()
        .expect("run `synthpass <file>`");

    std::fs::remove_file(&input_path).ok();

    assert_eq!(
        output.status.code(),
        Some(3),
        "extraction with no license present must exit 3, got: {output:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("license"),
        "expected a license-related refusal, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generate_with_a_bad_profile_is_a_usage_error() {
    let out_dir = std::env::temp_dir().join(format!(
        "synthpass-cli-test-exitcodes-generate-{}",
        std::process::id()
    ));
    let _guard = TempDirGuard(out_dir.clone());

    let output = Command::new(env!("CARGO_BIN_EXE_synthpass"))
        .args([
            "generate",
            "--profile",
            "not-a-real-profile",
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .expect("run `synthpass generate --profile not-a-real-profile`");

    assert_eq!(
        output.status.code(),
        Some(2),
        "an unknown --profile value is a usage error (2), got: {output:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown profile"),
        "expected an 'unknown profile' message, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
