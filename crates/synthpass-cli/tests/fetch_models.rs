//! Black-box coverage of `synthpass fetch-models` (issue #491) in the
//! *default* build, i.e. without the `download` cargo feature — the shape
//! `cargo test --workspace` actually exercises. This is the only place a
//! `.rten` OCR model file may be fetched at all; the extraction path never
//! downloads.
//!
//! A `--features download` build's actual-fetch behaviour needs network
//! access, so it isn't covered here — see `crates/synthpass-ocr/src/download.rs`'s
//! `fetch_and_verify` unit tests for the verification logic that path shares
//! with this one, exercised without any network call.

use std::process::Command;

#[test]
fn fetch_models_prints_urls_and_hashes_and_exits_ok_without_the_download_feature() {
    let dir = std::env::temp_dir().join(format!(
        "synthpass-cli-fetch-models-default-{}",
        std::process::id()
    ));

    let output = Command::new(env!("CARGO_BIN_EXE_synthpass"))
        .arg("fetch-models")
        .env("SYNTHPASS_OCR_MODEL_DIR", &dir)
        .output()
        .expect("run `synthpass fetch-models`");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        output.status.code(),
        Some(0),
        "a default (non-`download`) build must still succeed — it only prints \
         instructions, got: {output:?}"
    );
    assert!(
        stdout.contains("text-detection.rten") && stdout.contains("text-recognition.rten"),
        "expected both model filenames, got:\n{stdout}"
    );
    assert!(
        stdout.contains("https://ocrs-models.s3-accelerate.amazonaws.com/"),
        "expected the pinned source URL, got:\n{stdout}"
    );
    // The known-good SHA-256 hashes from crates/synthpass-ocr/src/known_good_hashes.rs
    // — printed literally so an operator can `sha256sum -c` a manual download.
    assert!(
        stdout.contains("f15cfb56bd02c4bf478a20343986504a1f01e1665c2b3a0ad66340f054b1b5ca"),
        "expected the detection model's known-good sha256, got:\n{stdout}"
    );
    assert!(
        stdout.contains("e484866d4cce403175bd8d00b128feb08ab42e208de30e42cd9889d8f1735a6e"),
        "expected the recognition model's known-good sha256, got:\n{stdout}"
    );
    assert!(
        stdout.contains(dir.to_str().unwrap()),
        "expected the target directory (SYNTHPASS_OCR_MODEL_DIR) to be named, got:\n{stdout}"
    );
    // A default build must never actually create the directory or fetch
    // anything — it only prints instructions.
    assert!(
        !dir.exists(),
        "a default build must not touch the filesystem beyond printing instructions"
    );

    std::fs::remove_dir_all(&dir).ok();
}
