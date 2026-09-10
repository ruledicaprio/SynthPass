# ADR-0008 chunk 1 — the OCR-stack gap, attributed

**2026-09-10.** The close-out of [`ADR-0008`](../decisions/ADR-0008-mrz-detection-track.md)'s first
chunk: a controlled measurement of why the browser's tesseract.js path out-reads the native
`ocrs`/`rten` path on the same documents. This document answers
[`Tesseract_OCR_studies.md`](../research/Tesseract_OCR_studies.md)'s nine numbered deliverables
directly, drawing on the four measurements that got here:

| # | measurement | writeup |
| --: | :-- | :-- |
| 1A | denominator correction — the target was 94 documents that can't be read | [`denominator-correction-2026-09-09.md`](denominator-correction-2026-09-09.md) |
| 1B | both OCR arms measured the same day — gap +5.6 pp, 17 named documents | [`ocr-stack-gap-2026-09-09.md`](ocr-stack-gap-2026-09-09.md) |
| 1C(a) | pass ordering — ruled out | [`ocr-order-band-first-2026-09-10.md`](ocr-order-band-first-2026-09-10.md) |
| 1C(b) | the gap is a text-*detection* gap — ~11 documents, `ocrs` returns noise | [`ocr-gap-is-detection-2026-09-10.md`](ocr-gap-is-detection-2026-09-10.md) |
| 1C(c) | the detection failures are a **wrong-orientation** failure | [`ocr-crop-recognizer-2026-09-10.md`](ocr-crop-recognizer-2026-09-10.md) |

## The one-paragraph answer

Native's `choose_rotation` rotates the page 90° or 270° before OCR on exactly the documents where
`ocrs` detects too little text for its rotation score to be meaningful — 11 of 11 of the
detection-failure set. Every downstream crop then presents the MRZ vertically, so neither `ocrs`
nor tesseract's OCR-B model can read it. The browser reads these same documents because
`web/scan.js` applies rotations only as *late retry passes* and never lets an orientation guess
rewrite its input — a design it arrived at by removing the upfront probe native still has. **The
gap is native page-orientation handling, not the recognizer, not the band search, not the
retry-pass ordering, and only secondarily scale.** The fix is pure-Rust and adds no dependency.

---

## The nine deliverables

### 1. Why the browser could plausibly out-read native on MRZ detection

Four candidate mechanisms, from `Tesseract_OCR_studies.md`: an OCR-B-specialised recognizer, more
aggressive Leptonica preprocessing (scale + binarisation), a segmentation/localisation edge, and a
larger retry budget. Measured, in order of how much each explains:

- **Orientation retry (the answer).** The browser tries 0°/90°/270°/180° as retry passes; native
  makes one upfront `choose_rotation` decision and commits. On low-signal scans that decision is a
  coin flip, and a wrong 90° turn takes the document out entirely — cell (c), 11/11.
- **Scale (secondary).** The browser's first pass, `plain_band`, both crops *and* upscales the MRZ
  band; native's *general* pass does not upscale at all. This is why the browser also recovers
  sub-300px documents (`Monaco_ID` is 254×162) that stay lost even once oriented right — cell (c).
- **OCR-B recognizer (not it).** Cell (a) measured `plain_band`-first natively: it recovers zero
  documents on its own merit. Cell (c) ran OCR-B over native's own crops: it fails identically to
  `ocrs` when the crop is sideways, and reads the MRZ off the *originals* at 0° just as a general
  recognizer would. OCR-B is a real advantage on marginal character shapes (`Belgium`,
  `Afghanistan` — O/0 confusables, cell b) but it is not what drives the detection gap.
- **Retry-pass ordering (not it).** Cell (a): moving `plain_band` to the front changes +1/−0 hits
  against a control that shows reordering trades documents.

### 2. Tesseract traits relevant to SynthPass

- **Orientation as retry, not as an upfront rewrite.** `web/scan.js` already implements this; the
  finding is that the *native* pipeline should too. This is the single highest-value trait.
- **Scale before recognition.** `plain_band` upscales; the native general pass should as well.
- **Character-set constraint.** Already shared — `MRZ_CHARSET` is single-sourced and applied both
  sides (`synthpass_imageprep::MRZ_CHARSET`).
- **Multi-variant retry with a deterministic scorer.** Already native's design (`recognize_detailed`
  runs up to 11 passes, ICAO check digits decide).

### 3. Traits **not** to copy

- The C++/Leptonica dependency, Tesseract's page-layout machinery, generic dictionaries and
  language models — `VISION.md` and `ADR-0008` both forbid a new dependency, and nothing measured
  here needs one.
- **PSM tuning.** Confirmed by grep: neither side sets a page-segmentation mode. The browser runs
  default PSM 3 even on a cropped band. The research note's "PSM 7/13 localisation advantage"
  hypothesis is closed — there is no PSM advantage because there is no PSM setting.
- Swapping `ocrs`/`rten` for another engine. Cell (c) shows the recognizer is not the gap.

### 4. Variables that had to be controlled, and how

| confounder | how it was held constant |
| :-- | :-- |
| recognizer model | cell (c) ran OCR-B over `ocrs`'s exact crops (`SYNTHPASS_OCR_DUMP_VARIANTS`) |
| preprocessing path | `synthpass-imageprep` compiles to wasm; the browser runs the identical Rust `mrz_variants`/`plain_band` |
| retry/variant budget | cell (a)'s `SYNTHPASS_OCR_ORDER`, same binary; `DEFAULT_MAX_PASSES` proven to reach every variant |
| corpus | both arms measured the same day on the same `samples/` tree (1B); denominator fixed first (1A) |
| character set | `MRZ_CHARSET` single-sourced, applied to both recognizers |
| orientation | **the uncontrolled variable that turned out to be the answer** — isolated in cell (c) via the `SYNTHPASS_OCR_VERBOSE` rotation log |

### 5. The experiment matrix that distinguished the four advantages

Three same-binary A/B cells, each an env-var toggle in one build against a control var, cheapest
first:

- **(a) ordering** — `SYNTHPASS_OCR_ORDER=default|band-first|control`. Rules out retry ordering.
- **(b) localization vs recognition** — `--dump-ocr` widened to `no_mrz_found`, dumping
  `mrz_band_score` + raw OCR text per miss. Splits "band found, misread" from "no text detected".
- **(c) recognizer held constant** — `SYNTHPASS_OCR_DUMP_VARIANTS` writes `ocrs`'s exact crops;
  `tests/web/recognize-crops.mjs` runs OCR-B over them. Isolates recognizer from crop, and the
  crop from orientation.

Cell (b) collapsed the research note's five-level factorial to this because levels 1–3 (same
pixels / same crop / same preprocessing) were already satisfied by the shared wasm preprocessing.

### 6. Per-document instrumentation schema

Delivered across the three cells: `document_id`, native OCR char count and full text, `mrz_band`
bbox + `mrz_band_score`, the rotation `choose_rotation` applied, each preprocessed variant image
(PNG), tesseract's text + confidence per variant per orientation, `mrz::find_and_parse` verdict,
checksum verdict, final `MissReason`. The JSONL dumps stay under `artifacts/` (gitignored) because
they carry specimen OCR text.

### 7. Same-binary A/B harness

`provider-bench --real-specimens --mrz-only` is the native arm; `tests/web/run-corpus.mjs` the
browser arm; both read the same `samples/` corpus. Every 1C toggle is an environment variable read
once in `recognize_detailed` (`ocr_order()`, `dump_variants_dir()`, mirroring the existing
`trailing_texture_mode()`), so `default` is provably byte-identical to the shipped path and a
control var (`SYNTHPASS_OCR_ORDER=control`, `SYNTHPASS_OCR_TEXTURE=control`) guards every delta.
Never a rebuild-based before/after.

### 8. What to reproduce natively in Rust — no Tesseract, no dependency

In priority order, all changes to `recognize_detailed`'s existing structure:

1. **Gate `choose_rotation`.** Do not rotate when the detected-text signal is below a confidence
   floor — the same "is there enough here to trust a decision" test the detection-failure documents
   fail. This alone should recover most of the 11.
2. **Move 90°/270° into the retry chain.** Native already has `rotate_image` and rotates for the
   0°/180° tie-break; extend it to try the other two orientations as *late* band-crop variants when
   the chain is about to exhaust on `no_mrz_found`. This is `web/scan.js`'s exact design.
3. **Upscale before the general detection pass.** `plain_band` upscales; the general pass does not.
   Recovers the sub-300px documents.

Each is measurable same-binary A/B against the 11 named documents. Building them is the **next
chunk**, not this one — `ADR-0008` mandates the measurement before the construction, and this is
the measurement.

### 9. Is the browser's number using OCR-B/MRZ-specific traineddata? — **CONFIRMED, yes**

`web/scan.js:54-62`: `Tesseract.createWorker('mrz', 1, …)` loads `web/tessdata/mrz.traineddata`
(1.4 MB, BSD-3-Clause © DoubangoTelecom, see `tessdata/LICENSE`), OEM 1 (LSTM only),
`tessedit_char_whitelist` set to `synthpass_imageprep::MRZ_CHARSET`. `eng` 4.0.0_best_int is the
fallback model for non-MRZ passes only. This is **not UNKNOWN** — the research note's "smoking gun"
is real, but cell (c) shows the OCR-B specialisation is not what drives the detection gap.

---

## Errata against earlier writeups

`ocr-stack-gap-2026-09-09.md` (1B) has a short correction note appended for two claims this chunk's
reconnaissance overturned: *"`plain_band` is never called natively"* (it runs, as the
second-to-last pass) and the implication that a PSM difference could explain a localisation
advantage (neither side sets one). `ocr-gap-is-detection-2026-09-10.md` (cell b) predicted §8
"scale and binarization" as the lever; cell (c) found the lever is more specific — page
orientation, with scale secondary. Those writeups are dated records and are left as written; this
document is the reconciled result.
