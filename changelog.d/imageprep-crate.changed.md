- **The browser demo runs the real preprocessing now, not a JavaScript port of it.**
  `web/scan.js` used to reimplement `preprocess.rs` by hand — the same band crop, the same
  percentile contrast stretch, the same Otsu threshold, written twice and drifting constant by
  constant (the port capped upscaling at 3× where the Rust capped at 5×, and it had no deskew, no
  row-density band search, and no local threshold at all). Every gap between the browser and the
  native pipeline traced back to somewhere the port had fallen behind.

  `preprocess.rs` and `geometry.rs` now live in a new crate, **`synthpass-imageprep`** — one
  dependency (`image`, no default features), no OCR engine, and a hard constraint that it keeps
  compiling for `wasm32-unknown-unknown`. `mrz-wasm` exposes it, and the demo calls it. The
  JavaScript port is deleted.

  `synthpass-ocr` re-exports `preprocess`, `geometry`, `BBox`, `OcrLine`, `OcrPage` and
  `MRZ_CHARSET` at their original paths, so the move is invisible to every existing caller.

- **Measured effect on the demo, over all 190 MRZ-bearing specimens:** 122 → **125** checksum-valid
  reads (64.2 % → 65.8 %), and documents where no MRZ was found at all dropped 11 → **6**. Two of
  the four newly-read documents were previously native-only wins, including the corpus's only
  rotated specimen, which the deskewed band variant recovered.

  Reported in full because it is not a clean win: one document regressed
  (`Egypt_Passport_Specimen_P0_EGY_2017`), two others kept a valid read but lost **line-1** fields —
  which carry no check digits, so a checksum-valid MRZ can still misread them — and median latency
  rose 51 % (1.5 s → 2.3 s) as the variant list grew from six passes to seven. All three regressions
  point at the band upscale's resampling filter (`Lanczos3` vs the canvas's bilinear), now
  single-sourced and measurable. See `knowledge/WEB_OCR_BASELINE.md`.

- **Three hand-maintained duplicates removed.** The MRZ charset had a copy in `scan.js` and another
  in `visual_zone_survey.rs`; the MRZ-band score threshold had a copy in that survey too, carrying a
  comment that it was "kept in lockstep deliberately". Lockstep is the compiler's job now. The
  magic-byte table behind the "this file is really a PDF/HEIC" diagnostic is shared between the
  native decoder and the browser instead of existing twice.

- **`mrz-wasm` needs `--enable-nontrapping-float-to-int`.** Every `as` cast from a float to an
  integer lowers to `i32.trunc_sat_*`, and the preprocessing is full of them, so `wasm-opt`'s
  feature allowlist rejected the release build until it was named. The WASM payload grows
  164 KB → 211 KB.
