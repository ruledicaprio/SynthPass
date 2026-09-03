- **A retry pass that removes the security printing underneath the MRZ.** Passports and ID cards
  print guilloche, rosette and microprint under the machine-readable zone, and two helpers in
  `synthpass-imageprep` already named that pattern — `contrast_stretched` calls itself "robust to
  the washed-out look of guilloche-patterned document backgrounds", `text_bands` filters
  "guilloche-pattern noise". Both address what the pattern does to the histogram. Neither can
  touch it in space: a percentile stretch is a point operation, and `local_threshold` models the
  background with a box mean an order of magnitude coarser than a strand of microprint. So the
  strands survived into the binarized image and merged with the strokes sitting on top of them.

  `preprocess::texture_variants` adds a median filter ahead of the adaptive threshold, with its
  radius derived from the band's own measured text-row height rather than hardcoded, and clamped
  so that a mismeasured band cannot produce a kernel wide enough to erase a glyph stroke. The two
  compose rather than duplicate: the median removes high-frequency structure before thresholding,
  adaptive thresholding removes low-frequency illumination.

  A median rather than the sharper morphological closing, deliberately. Closing is the right
  polarity for thin dark strands, but it is only selective while the kernel sits between the
  strand width and the stroke width, and mis-sized by a single pixel it erases stroke terminals
  and thins `1` into nothing — manufacturing the very confusion class this targets. A median
  fails safe: texture wider than the kernel leaves the output close to the input, making the pass
  redundant rather than destructive.

  It runs last, after every existing variant, so the ICAO check digits decide whether it helped
  and no specimen that already validates can regress — the same additive-only contract the module
  upholds throughout. `MAX_RETRY_VARIANTS` moves 8 to 9 accordingly; without that bump the retry
  loop's `passes_run >= max_passes` break would silently truncate the new pass on exactly the
  documents it exists for. Zero new dependencies, and the crate still builds for
  `wasm32-unknown-unknown`.

  Gated by `SYNTHPASS_OCR_TEXTURE` (`on` / `off` / `control`), **default off**, so the stage can
  be A/B-measured from one binary. The `control` arm is a placebo — `plain_band`, one extra
  trailing pass with real budget and text cost but already measured as inert on `ocrs` — because
  appending any ninth pass changes more than the pixels, and a rebuild-based before/after has
  hidden a real regression on this project before.

  Measured on the Tier-1 fixture corpus and **null there: 18/20 in all three arms, zero false
  positives across 42 negative controls, zero per-document flips**. That corpus cannot test the
  idea, and the writeup says so plainly: this stage targets `checksum_failed` misses, and the
  corpus's only two misses are `no MRZ found in text` — the other failure mode. The run did close
  two open questions with data: the pass demonstrably executes (it costs +114s of wall-clock, which
  a truncated variant could not), and `SYNTHPASS_OCR_MAX_SECONDS` did not need raising (the single
  document at the 45s ceiling is a negative control that was already over it). See
  `knowledge/benchmarks/texture-suppression-ab-2026-09-03.md`.
