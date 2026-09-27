//! Pinned source URLs and SHA-256 hashes for the two `.rten` model files this
//! crate needs, plus (behind the `download` cargo feature) the code that
//! actually fetches and verifies them.
//!
//! **The extraction path never calls anything in this module that touches
//! the network** (issue #491) — the only way these files reach disk is the
//! explicit `synthpass fetch-models` command (`synthpass-cli`). This module
//! exists so that command, and only that command, has one place to read the
//! pinned values from, in both the default build (which can only print them —
//! `reqwest` isn't even in the dependency graph, see this crate's `Cargo.toml`)
//! and the `download`-feature build (which also fetches).
//!
//! Before this module took its current shape, `ensure_models` ran lazily
//! inside the OCR call itself (`synthpass-pipeline`'s `ocr.rs`) whenever
//! `SYNTHPASS_OCR_AUTO_DOWNLOAD` wasn't exactly `0` — the first extraction on
//! a fresh install made a network call by default, which contradicted
//! `knowledge/project_principles.md` #5 ("no runtime downloads on the
//! extraction path") and the README's "zero cloud calls" claim. See
//! `changelog.d/491.changed.md`.

use std::path::Path;
// `PathBuf` is only named outside `cfg(test)`/`cfg(feature = "download")` code
// (see `fetch_and_verify` and the test module below), so it's imported at
// each use site instead of here — a blanket `use` would warn as unused on
// the plain (non-test, non-`download`) `cargo check` of this crate's `lib`
// target.

pub const DETECTION_FILENAME: &str = "text-detection.rten";
pub const RECOGNITION_FILENAME: &str = "text-recognition.rten";

const DETECTION_URL: &str = "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten";
const RECOGNITION_URL: &str =
    "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten";

/// One model file `synthpass fetch-models` knows how to obtain: its filename
/// (resolved against `SYNTHPASS_OCR_MODEL_DIR`), its pinned source URL, and
/// the function that resolves its expected SHA-256 (the built-in constant, or
/// the `SYNTHPASS_OCR_*_SHA256` env override — see `verify.rs`). Carries no
/// `reqwest` type, so it's available unconditionally: the default build uses
/// it to print instructions, the `download`-feature build also uses it to
/// fetch.
pub struct ModelSpec {
    pub filename: &'static str,
    pub url: &'static str,
    pub expected_sha256: fn() -> String,
}

/// The two `.rten` files the pure-Rust OCR engine needs, in load order
/// (detection, then recognition).
pub fn model_specs() -> [ModelSpec; 2] {
    [
        ModelSpec {
            filename: DETECTION_FILENAME,
            url: DETECTION_URL,
            expected_sha256: crate::verify::expected_detection_sha256,
        },
        ModelSpec {
            filename: RECOGNITION_FILENAME,
            url: RECOGNITION_URL,
            expected_sha256: crate::verify::expected_recognition_sha256,
        },
    ]
}

/// Write `bytes` to a sibling temp file, check them against `expected_sha256`,
/// and only then rename into `path`.
///
/// Writing to a temp file first means a crash or interrupted download never
/// leaves a truncated file that looks "present" on the next run. Checking
/// *before* the rename covers the matching case: the load path in
/// `synthpass-pipeline` verifies again and is the real security boundary, but
/// a bad download that had already taken the final filename would poison the
/// cache until someone deleted it by hand. Discarding the temp file keeps a
/// transient bad fetch retryable.
///
/// Exposed with `pub(crate)` visibility so `fetch_and_verify` (the only
/// caller today) and this module's own tests can both reach it without a
/// `reqwest` dependency of their own — this function itself never touches the
/// network. Only genuinely dead when neither this module's tests nor the
/// `download` feature are compiled in (a plain, non-test `cargo check`/`build`
/// of this crate's `lib` target with default features) — allowed rather than
/// warned in that one configuration.
#[cfg_attr(not(feature = "download"), allow(dead_code))]
pub(crate) fn commit_verified(
    path: &Path,
    bytes: &[u8],
    source: &str,
    expected_sha256: impl FnOnce() -> String,
) -> Result<(), String> {
    let tmp_path = path.with_extension("part");
    std::fs::write(&tmp_path, bytes)
        .map_err(|e| format!("failed to write {}: {e}", tmp_path.display()))?;

    if !crate::verify::skip_verify() {
        let expected = expected_sha256();
        let actual = synthpass_core::audit::sha256_hex(bytes);
        if actual != expected {
            std::fs::remove_file(&tmp_path).ok();
            return Err(format!(
                "downloaded {source} but its SHA-256 does not match the known-good hash \
                 (expected {expected}, got {actual}) — discarded the partial file, \
                 so re-running will retry the download"
            ));
        }
    }

    std::fs::rename(&tmp_path, path)
        .map_err(|e| format!("failed to finalize {}: {e}", path.display()))?;
    Ok(())
}

/// Whether [`fetch_and_verify`] downloaded new bytes or found a file already
/// on disk (and re-verified it rather than trusting its presence blindly) —
/// mirrors ci.yml's own "download only if missing, verify either way" fetch
/// step, so a second `synthpass fetch-models` run is cheap and still catches
/// a corrupted or tampered cached file.
#[cfg(feature = "download")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchOutcome {
    AlreadyPresentAndVerified,
    Downloaded,
}

/// Ensures `spec` is present, SHA-256-verified, at `dest_dir`: downloads it if
/// missing, or re-verifies it if already there — the only code in this crate
/// that makes a network call, and only present when the `download` cargo
/// feature is on (see this crate's `Cargo.toml`: a default build has no
/// `reqwest` in its dependency graph at all). Called exclusively from
/// `synthpass fetch-models`; nothing on the extraction path calls this.
#[cfg(feature = "download")]
pub fn fetch_and_verify(
    spec: &ModelSpec,
    dest_dir: &Path,
) -> Result<(std::path::PathBuf, FetchOutcome), String> {
    let path = dest_dir.join(spec.filename);
    if path.exists() {
        if !crate::verify::skip_verify() {
            let expected = (spec.expected_sha256)();
            if let Err(e) = synthpass_core::audit::verify_file_sha256(&path, &expected) {
                // A pre-staged file that fails verification is deleted, not
                // left in place — the same "mismatch → delete → fail" rule a
                // freshly downloaded file gets in `commit_verified` below, so
                // re-running `synthpass fetch-models` is always the fix for a
                // corrupted or tampered model file, never a manual `rm`.
                std::fs::remove_file(&path).ok();
                return Err(format!(
                    "{} was present at {} but failed verification ({e}) — deleted it; \
                     re-run `synthpass fetch-models` to fetch a clean copy",
                    spec.filename,
                    path.display()
                ));
            }
        }
        return Ok((path, FetchOutcome::AlreadyPresentAndVerified));
    }

    let bytes = reqwest::blocking::get(spec.url)
        .and_then(|resp| resp.error_for_status())
        .map_err(|e| format!("failed to fetch {}: {e}", spec.url))?
        .bytes()
        .map_err(|e| format!("failed to read response body from {}: {e}", spec.url))?;
    commit_verified(&path, &bytes, spec.url, spec.expected_sha256)?;
    Ok((path, FetchOutcome::Downloaded))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "synthpass-ocr-download-{tag}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A download whose bytes don't match must leave *nothing* behind — neither
    /// the final file (which would poison the cache, since a real fetch never
    /// overwrites a path that already exists) nor the `.part` scratch file.
    /// No network access: `commit_verified` only ever touches bytes already in
    /// memory, so this exercises the exact verification logic
    /// `fetch_and_verify` and `synthpass fetch-models` rely on without needing
    /// the `download` feature or a real download.
    #[test]
    fn bad_bytes_are_discarded_not_committed() {
        let dir = tmp_dir("bad");
        let path = dir.join(DETECTION_FILENAME);

        let err = commit_verified(&path, b"not a real rten file", "test://detection", || {
            crate::verify::KNOWN_GOOD_SHA256_DETECTION.to_string()
        })
        .expect_err("hash mismatch should be rejected");

        assert!(err.contains("does not match the known-good hash"), "{err}");
        assert!(!path.exists(), "bad download must not take the final name");
        assert!(
            !path.with_extension("part").exists(),
            "temp file must be cleaned up"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Matching bytes are renamed into place and readable.
    #[test]
    fn good_bytes_are_committed() {
        let dir = tmp_dir("good");
        let path = dir.join(RECOGNITION_FILENAME);
        let payload = b"pretend model bytes";
        let expected = synthpass_core::audit::sha256_hex(payload);

        commit_verified(&path, payload, "test://recognition", || expected)
            .expect("matching hash should commit");

        assert_eq!(std::fs::read(&path).unwrap(), payload);
        assert!(!path.with_extension("part").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// `model_specs()` is what both build modes read the pinned URLs/hashes
    /// from — pin its shape so a future edit can't silently drop a field the
    /// CLI's `fetch-models --help`-equivalent output depends on.
    #[test]
    fn model_specs_names_both_files_with_https_urls() {
        let specs = model_specs();
        assert_eq!(specs[0].filename, DETECTION_FILENAME);
        assert_eq!(specs[1].filename, RECOGNITION_FILENAME);
        for spec in &specs {
            assert!(spec.url.starts_with("https://"), "{}", spec.url);
            assert!(!(spec.expected_sha256)().is_empty());
        }
    }

    /// A pre-staged file whose bytes don't match the expected hash must be
    /// deleted, not left behind poisoning every future run — no network
    /// access needed: `fetch_and_verify` checks `path.exists()` before ever
    /// reaching the `reqwest` call, so this never fetches anything.
    #[cfg(feature = "download")]
    #[test]
    fn existing_file_that_fails_verification_is_deleted_not_left_behind() {
        let _env = crate::env_lock();
        let dir = tmp_dir("existing-mismatch");
        let path = dir.join(DETECTION_FILENAME);
        std::fs::write(&path, b"not the real model").unwrap();

        // SAFETY: serialized by `env_lock` above — same pattern
        // `verify.rs`'s own `respects_env_override` test uses.
        unsafe { std::env::set_var("SYNTHPASS_OCR_DETECTION_SHA256", "0".repeat(64)) };
        let spec = ModelSpec {
            filename: DETECTION_FILENAME,
            url: "https://example.invalid/unused",
            expected_sha256: crate::verify::expected_detection_sha256,
        };
        let err = fetch_and_verify(&spec, &dir).expect_err("mismatched pre-staged file must fail");
        unsafe { std::env::remove_var("SYNTHPASS_OCR_DETECTION_SHA256") };

        assert!(err.contains("failed verification"), "{err}");
        assert!(
            !path.exists(),
            "a mismatched pre-staged file must be deleted, not left in place"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A pre-staged file whose bytes DO match is accepted with no network
    /// call at all: `fetch_and_verify` returns `AlreadyPresentAndVerified`,
    /// not `Downloaded`, so a second `synthpass fetch-models` run is cheap.
    #[cfg(feature = "download")]
    #[test]
    fn existing_file_that_matches_is_accepted_without_downloading() {
        let _env = crate::env_lock();
        let dir = tmp_dir("existing-match");
        let path = dir.join(RECOGNITION_FILENAME);
        let payload = b"pretend model bytes";
        std::fs::write(&path, payload).unwrap();
        let expected = synthpass_core::audit::sha256_hex(payload);

        // SAFETY: serialized by `env_lock` above.
        unsafe { std::env::set_var("SYNTHPASS_OCR_RECOGNITION_SHA256", &expected) };
        let spec = ModelSpec {
            filename: RECOGNITION_FILENAME,
            url: "https://example.invalid/unused",
            expected_sha256: crate::verify::expected_recognition_sha256,
        };
        let (found, outcome) =
            fetch_and_verify(&spec, &dir).expect("matching pre-staged file must be accepted");
        unsafe { std::env::remove_var("SYNTHPASS_OCR_RECOGNITION_SHA256") };

        assert_eq!(found, path);
        assert_eq!(outcome, FetchOutcome::AlreadyPresentAndVerified);
        std::fs::remove_dir_all(&dir).ok();
    }
}
