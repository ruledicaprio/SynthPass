- **The browser MRZ validator is measured now.** `tests/web/run-corpus.mjs` runs the real assembled
  demo site in headless Chromium over every specimen the corpus manifest marks as carrying an MRZ,
  and reports the checksum-valid hit rate. Until now **no measurement of the web OCR path existed at
  all** — no corpus run, no CI, no baseline — so every change to `web/scan.js` could only be
  justified by "seems better". The browser is a separate OCR stack from the native pipeline
  (tesseract.js with an OCR-B model, not `ocrs`/`rten`), so native benchmark numbers never said
  anything about it.

  The harness loads its page from inside the deployed build and imports the same `scan.js`, the same
  WASM parser and the same SHA-256-pinned tesseract.js runtime the demo imports. It reports the same
  documents' native result alongside — read from the manifest's `mrz.observed.checksums_valid` — so
  the web number has something to mean something against. Misses are split into near misses (MRZ
  found, check digits failed) and no-MRZ-found, and field accuracy is scored only on checksum-valid
  reads, reusing `parity.rs`'s reviewed-vs-derived fixture split. An empty candidate set reports
  "not measured", never 0%.

  Runs on demand and nightly via `.github/workflows/web-ocr.yml`, not as a pull-request gate — a
  full sweep takes 20-30 minutes and needs the orphan `samples-data` branch.

- **First result: the browser is ahead of the native pipeline, not behind it.** Over all 190
  MRZ-bearing specimens, tesseract.js with the OCR-B-trained model reads **122 (64.2 %)** against
  `ocrs`/`rten`'s recorded **113 (59.5 %)** — nine more documents, with 103 read by both. The native
  figure is the full path (`corpus_manifest.rs` calls `NativeOcr::recognize`, a thin wrapper over
  `recognize_detailed`, so rotation detection and every retry variant are included), so this is not
  a weakened comparison. Recorded in `knowledge/WEB_OCR_BASELINE.md`, along with the 10 documents
  the browser loses — eight of which are near misses rather than blank failures, and one of which
  is the corpus's only `_rotated` specimen, `scan.js` having no orientation handling at all.

  This changes a standing architecture assumption: `knowledge/ARCHITECTURE.md`'s "one OCR engine
  everywhere" end state — `ocrs`/`rten` in the browser, "deleting tesseract.js entirely" — was
  gated on latency alone. On this corpus that swap would cost accuracy, so it is now gated on
  accuracy too.

- **`scripts/build-site.sh` is now the single definition of how the demo site is assembled**, used by
  both the Pages deploy workflow and the measurement harness. A harness that built the site its own
  way would measure something other than what deploys.
