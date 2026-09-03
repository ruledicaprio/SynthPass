# Web OCR baseline

Measured hit rate of the browser MRZ validator
(<https://ruledicaprio.github.io/SynthPass/>) over the specimen corpus.

Harness: [`tests/web/`](../tests/web/README.md). Re-run with
`node tests/web/run-corpus.mjs --out report.json`.

---

## 2026-09-03 — first measurement

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
