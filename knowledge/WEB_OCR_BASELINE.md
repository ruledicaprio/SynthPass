# Web OCR baseline

Measured hit rate of the browser MRZ validator
(<https://ruledicaprio.github.io/SynthPass/>) over the specimen corpus.

Harness: [`tests/web/`](../tests/web/README.md). Re-run with
`node tests/web/run-corpus.mjs --out report.json`.

---

## 2026-09-03 (c) — the band upscale filter, measured on both engines

(b) left one hypothesis outstanding: that its three regressions came from
`upscale_to_width` resampling with `Lanczos3` where the deleted JavaScript port
used the canvas's bilinear. Tested directly, one variable, on **both** OCR
engines.

**Determinism established first.** Two separate runs of the identical build
agreed on all 60 documents checked — 0 differing verdicts. Without that, a
rebuild-based A/B proves nothing.

| | `Lanczos3` | `Triangle` |
| :-- | --: | --: |
| **browser** (tesseract, OCR-B), 190 docs — checksum-valid | 125 (65.8 %) | 125 (65.8 %) |
| **browser** — reviewed line-1 fields | 143/153 | **150/153** |
| **browser** — median latency | 2 285 ms | **2 088 ms** |
| **native** (`ocrs`), `mrz_corpus` 20 docs | **18/20** | 17/20 |

### The engines want opposite filters

Not "one is better". The hit rate is identical in the browser and the two
filters win *different documents*: `Triangle` gains `Belarus P0_BLR_2006` and
`Egypt P0_EGY_2017`, and loses `Germany P0_D00_2018` and
`North_Macedonia P0_MKD_2020_mrz_rotated`. Documents where nothing MRZ-shaped
was found at all rose 6 → 12 under `Triangle`, while near misses fell 59 → 53.

The mechanism that explains all of it: **`Lanczos3` sharpens, which helps the
detector *find* MRZ-shaped text on a blurred or low-contrast scan, but it rings
on high-contrast edges — and MRZ glyph strokes are nothing but high-contrast
edges — which costs *character* accuracy.** `Triangle` is the reverse.

The sharpest single piece of evidence that this is real rather than corpus
noise: **`Cyprus_Passport_Specimen_P0_CYP_2010` is the one document native
*loses* under `Triangle`, and simultaneously the one where the browser
*recovers* line-1 fields under it.** Same image, same change, opposite
directions in two engines.

### Why the browser takes `Triangle` despite the tie

Because the tie is only on the checksum-valid rate, and **line 1 carries no
check digit in TD1, TD2 or TD3**. For `document_type`, `issuing_country`,
`surname` and `given_names` the recognizer is the only protection there is —
nothing downstream can catch a misread. `Triangle` is better exactly where the
standard offers no safety net (+7 fields, including four on
`United_Arab_Emirates_2011` that were wrong under `Lanczos3` *and* under the
old JS port), at no cost to the checksum-valid rate and ~9 % faster.

So `UpscaleFilter` is a parameter with **no `Default`**: `synthpass-ocr` passes
`Lanczos3`, `mrz-wasm` passes `Triangle`, and both say so at the call site. A
silent default is how the two engines' needs got conflated in the first place.

This is the second finding of the same shape — the first was `plain_band`, the
untreated pass tesseract needs and `ocrs` gains nothing from. The pattern is
worth stating once: **preprocessing is shared; ordering and tuning are
recognizer-dependent.**

### Caveat

The native arm is 20 documents, so one document is 5 %. It is a guardrail
against a large native regression, not a precise comparison. The browser arm
(190 documents) is the stronger evidence, and the two agree on direction.

### Not tested yet

The gains and losses are **disjoint documents**, and the retry chain is
additive by contract — so running *both* filters as separate variants should
capture all four rather than trading two for two. That needs its own
measurement and its own latency budget; it is not assumed here.

---

## 2026-09-03 (b) — preprocessing moved from the JS port to WASM

One variable: `web/scan.js`'s hand-written JavaScript preprocessing replaced by
`synthpass-imageprep` compiled to WASM. Same corpus, same harness, same
tesseract build, same machine.

| | JS port | WASM | |
| :-- | --: | --: | :-- |
| checksum-valid | 122 (64.2 %) | **125 (65.8 %)** | **+3** |
| near misses | 57 | 59 | +2 |
| found no MRZ | 11 | **6** | **−5** |
| reviewed-fixture fields | 147/153 | 143/153 | **−4** |
| median latency | 1 516 ms | 2 285 ms | **+51 %** |
| p90 latency | 10 271 ms | 13 590 ms | +32 % |

Gained 4 — `Afghanistan P0_AFG_2016`, `Germany P0_D00_2018`,
**`North_Macedonia P0_MKD_2020_mrz_rotated`**, `Sweden P0_SWE_2012_mrz_boxed`.
The last two were previously native-only wins, and the rotated one arrived via
the deskewed isolated-band variant, which the JS port never had.

Lost 1 — `Egypt P0_EGY_2017`, which used to win on the binarized pass and now
runs all seven and near-misses.

### The regressions have one likely cause, and it is not the port being gone

Two documents kept a checksum-valid read but lost **line-1** fields:
`Canada P0_CAN_2013` (`issuing_country`, `given_names`) and
`Cyprus P0_CYP_2010` (`issuing_country`, `surname`). Line 1 carries no check
digits in any of TD1/TD2/TD3, so a fully checksum-valid MRZ can still hold
misread characters there — which is exactly why those fields are scored only
against *reviewed* fixtures.

All three regressions point at resampling, the one thing that genuinely
changed under the band crop:

- **Filter.** The JS port upscaled with canvas `drawImage`
  (`imageSmoothingEnabled`, bilinear-ish). `upscale_to_width` uses
  **`Lanczos3`**, which is sharper and rings on high-contrast edges — exactly
  what MRZ glyph strokes are.
- **Cap.** The port capped upscale at **3×**, `preprocess::MAX_SCALE` at
  **5×**. Identical in practice for anything wider than ~533 px after the
  1600 px downscale, so this only bites on small source images.

Both are now single-sourced in Rust and measurable. **Next candidate change:**
A/B `Lanczos3` against `Triangle` for the band upscale. Do it as its own
measured change — not folded into anything else.

### Latency

+51 % on the median is the real cost. Two contributors: the variant list grew
from 6 attempts to 7, and `mrz_variants` computes the row-density band search
that the port did not have. Still well inside interactive range for the common
case — the median document finishes in ~2.3 s and 110 of 125 hits come on the
first pass, before any of the added work runs.

---

## 2026-09-03 (a) — first measurement

Real headless Chromium (Playwright 1.62.1, Chromium 151), the assembled
`_site/` build, tesseract.js 5.1.1 with the vendored OCR-B `mrz.traineddata`.
All 190 corpus rows with `mrz.present == true`.

| | checksum-valid | rate |
| :-- | --: | --: |
| **Browser** (tesseract.js + OCR-B) | **122 / 190** | **64.2 %** |
| Native (`ocrs`/`rten`), recorded in `samples/corpus.jsonl` | 113 / 190 | 59.5 % |

Head-to-head: **both 103**, browser only **19**, native only **10**.

Misses split: **57 near misses** (MRZ-shaped text found, check digits failed)
and **11 found no MRZ at all**. Per-document latency: median **1.5 s**, p90
**10.3 s**, full sweep 11.9 min.

Field accuracy on checksum-valid reads — reviewed fixtures **147/153 (96.1 %)**
over all nine fields, generated fixtures **149/150 (99.3 %)** over the three
ICAO proves. Exact `mrz_line` match against a fixture: 24.

### The browser is not behind the native pipeline

This inverts the assumption the harness was built to test. On this corpus the
browser reads **nine more documents** than `ocrs`/`rten` does, and the native
number is not a weakened comparison: `corpus_manifest.rs` calls
`NativeOcr::recognize`, a thin wrapper over `recognize_detailed`, so the
recorded native result is the full path — rotation detection, the pass/time
budget, and every `preprocess::mrz_variants` retry included. The manifest was
regenerated in #191, after the three mislabelled specimens were renamed, so all
190 files were readable by both engines.

The plausible cause is the model, not the pipeline: `mrz.traineddata` is
fine-tuned on OCR-B, the font MRZs are printed in, while `ocrs` runs a
general-purpose recognizer.

**This bears on [`ARCHITECTURE.md`](ARCHITECTURE.md) §"Beyond v1.0" item 3**,
which sets the end state as one OCR engine everywhere — `ocrs`/`rten` in the
browser, "deleting tesseract.js entirely". On this corpus that swap would cost
accuracy. Item 3 gates the spike on *latency*; this says it must be gated on
**accuracy** too, and that the current gap runs the other way.

### Where the browser loses (the 10 native-only documents)

`Angola PN_AGO_2022`, `China 2012` (×2), `Kazakhstan P0_KAZ_2004`,
`Malaysia …_redacted_mrz_blur`, `Netherlands P0_NLD_2014_mrz_blur`,
**`North_Macedonia P0_MKD_2020_mrz_rotated`**, `Russian_Federation ID 2013 back`,
`Sweden P0_SWE_2012_mrz_boxed`, `Switzerland ID 2003 back`.

Eight of the ten are near misses — the MRZ was found and misread, not missed.
Two (`China`) found nothing at all.

`…_mrz_rotated` is the named case for the rotation gap: `web/scan.js` has no
orientation handling of any kind, while native's `choose_rotation` probes all
four right angles. Two more are `_blur`, one `_boxed`.

### Where the browser wins (19 documents)

Dominated by ID-card backs — Belgium, Croatia, France, Monaco, Sweden,
Switzerland — i.e. TD1 three-line MRZs, plus passports from Canada, Czechia,
Finland (×2), India (×2), Kuwait, Oman, Portugal (×2), Romania, Russia,
Vietnam.

### The retry chain barely fires

Of 122 hits, **112 came from the very first pass** (plain bottom-band crop,
OCR-B model). Contrast stretch won 6, full-page 2, binarized 1, the generic
`eng` model 1.

Two readings, and they point the same way: the cheap path is doing nearly all
the work, and the five retry variants are recovering almost nothing. The 57
near misses are where the headroom is — those documents reach the parser with
MRZ-shaped text that fails its check digits, which is a *character accuracy*
problem, not a *band location* problem.

### Caveats

- The native column is read from the manifest, not re-measured in this run.
  It is one pass of one engine version on one machine.
- Corpus specimens skew toward clean scans of published specimen pages. Nothing
  here measures handheld phone photos, which is where the missing rotation
  handling would hurt most and where the corpus is silent.
