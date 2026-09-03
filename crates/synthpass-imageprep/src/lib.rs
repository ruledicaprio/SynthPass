//! Deterministic MRZ image preprocessing and layout geometry — the part of
//! the OCR pipeline that has nothing to do with which OCR engine runs.
//!
//! # Why this is its own crate
//!
//! Two OCR engines read SynthPass documents: `ocrs`/`rten` natively, and
//! tesseract.js in the browser demo. That split is real and stays — they are
//! different recognizers with different strengths (see
//! `knowledge/WEB_OCR_BASELINE.md`; on the specimen corpus the browser's
//! OCR-B-trained model currently reads *more* documents than the native one).
//!
//! What was **not** real was the second copy of everything around the
//! recognizer. `web/scan.js` used to be a hand-written JavaScript port of
//! [`preprocess`] — the same band crop, the same percentile contrast stretch,
//! the same Otsu threshold, written twice and drifting apart constant by
//! constant. Every gap between the browser and the native pipeline traced back
//! to a place the port had fallen behind.
//!
//! So the preprocessing moved here, to a crate with one dependency and no
//! engine, that compiles both natively and to `wasm32-unknown-unknown`. The
//! browser now runs *this* code through `mrz-wasm`, and the JavaScript port is
//! gone.
//!
//! # The wasm32 constraint
//!
//! Everything in this crate must build for `wasm32-unknown-unknown`. That
//! rules out the filesystem, threads, the clock, and any dependency that is
//! not pure Rust. It is why `image` is taken without default features (no
//! codecs — callers hand over decoded pixels) and why file-format sniffing
//! here ([`sniff`]) works on a byte slice rather than a path.
//!
//! # Determinism
//!
//! Every helper is a pure function of its input pixels. No randomness, no
//! ambient state, no I/O. Correctness of any retry built on them is proven or
//! rejected by the ICAO check digits upstream, never assumed — which is what
//! lets the retry passes be additive: a variant can only ever *add* a
//! checksum-valid read, never break one that already worked.

pub mod geometry;
pub mod preprocess;
pub mod sniff;

pub use geometry::{BBox, OcrLine, OcrPage};

/// The ICAO 9303 machine-readable-zone character set: A-Z, 0-9 and the filler
/// `<`, and nothing else.
///
/// Single-sourced here because it is needed at three layers that used to each
/// keep their own copy — the native engine's recognition constraint
/// (`synthpass_ocr::NativeOcr`'s `allowed_chars`), the line scoring in
/// [`geometry::mrz_line_score`], and the browser demo's tesseract
/// `tessedit_char_whitelist`, which reaches JavaScript through `mrz-wasm`.
pub const MRZ_CHARSET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789<";
