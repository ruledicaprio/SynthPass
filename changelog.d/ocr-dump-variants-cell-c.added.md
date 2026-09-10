- **`SYNTHPASS_OCR_DUMP_VARIANTS=<dir>`** — a diagnostic env var (unset by default,
  a no-op then) that writes every preprocessed image `synthpass-ocr` feeds the
  recognizer — the general full-page pass and each retry variant — to `<dir>` as a
  PNG. Added for ADR-0008 chunk 1C cell (c): it lets a different recognizer be run
  offline over the exact bytes `ocrs` saw. New `cargo run -p synthpass-ocr --example
  dump_variants` drives it over a chosen document list, and `tests/web/recognize-crops.{html,mjs}`
  runs the vendored tesseract.js OCR-B model over the dumped crops. The measurement:
  [`ocr-crop-recognizer-2026-09-10.md`](knowledge/benchmarks/ocr-crop-recognizer-2026-09-10.md).
