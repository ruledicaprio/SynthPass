- **`synthpass doctor` now proves the OCR models actually load, not just that their bytes are
  present.** Previously all three OCR checks could pass over a model this build's `rten` cannot
  use: the sha256 check only proved the on-disk bytes were the known-good file for that
  filename, and the `ocr-embedded` (musl release) and `SYNTHPASS_OCR_ENGINE=native` paths never
  touched the `ok` flag at all, so neither could fail. `doctor` now constructs the real OCR
  engine from the model files (disk or embedded) and reports a load failure as `❌`; this is a
  format/deserialization check, not a full recognition pass, so it stays fast. Also,
  `SYNTHPASS_OCR_ENGINE` no longer skips the model check for `native`: the pipeline retired the
  Tesseract-based native engine in v1.2.0 and always runs the Rust OCR engine regardless of this
  variable, so `doctor` now warns that the value is ignored and checks the engine that actually
  runs.
