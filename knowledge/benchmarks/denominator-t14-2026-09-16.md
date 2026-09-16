# Closing the denominator before the next accuracy chunk — 2026-09-16

**Date:** 2026-09-16 · **MAIN:** `73231fa` · **DATA:** not stated · **Evidence:** unlabelled (pre-standard) · **Status:** current

Ground-truth work only. **No count in `real-specimen-mrz-baseline.json` moves on
this document**, no pipeline code changed, and no benchmark arm was run. Its
purpose is to make the *next* measurement attributable: the `checksum_failed`
bucket held three documents with no ground truth at all, so nothing
distinguished "our OCR misread it" from "the specimen prints a zone no correct
read could validate".

Baseline this starts from: measured 2026-09-15 at CI `7fff390`, 145 / 159
scored = 91.2%, `checksum_failed` 10, `no_mrz_found` 4.

Two findings below (§4, §5) establish reclassifications that need a rename on
the `samples-data` branch to take effect. Those renames and their re-bless are
held for a follow-up so they do not land inside another cohort's re-bless.

## 1. The three unlabelled `checksum_failed` books — transcribed

Added 2026-09-14 by cohorts c03/c07/c09 with no `ground_truth_stem` and no
`expected_document_number`. `CORPUS_COVERAGE.md` attributed all three to "the
single-character-misread pattern", which was plausible and unmeasured.

| Specimen | Source px | Printed zone |
| :-- | :-- | :-- |
| `Germany_Passport_Specimen_P0_D00_2024_mrz` | 665 × 472 | conforming, all 5 check digits validate |
| `Hong_Kong_Passport_Specimen_P0_HKG_2007_mrz` | 267 × 181 | conforming, all 5 validate |
| `Hong_Kong_Passport_Specimen_P0_HKG_2019_mrz` | 265 × 181 | conforming, all 5 validate |

Verified twice and independently: by hand arithmetic over the ICAO 7-3-1
weights, and by the repo's own parser — `cargo test -p synthpass-bench --test
ocr_fixtures` asserts `mrz::find_and_parse(mrz_line).valid()` agrees with each
fixture's recorded `mrz_checksums_valid: true`.

**This refutes the open hypothesis.** `benchmarks/README.md` recorded that "the
published 91.2% is a floor that may be up to three documents pessimistic",
because a non-conforming printed zone would have moved these three
off-denominator as `checksum_failed_specimen`. None of the three is
non-conforming. All three print a zone a correct read would validate, so all
three stay in the denominator as genuine read failures. **91.2% is not
pessimistic; it is the rate.**

The three fixtures therefore change no bucket count — a specimen with no fixture
is already classified `specimen_nonconforming: false`. What they add is ground
truth, which is what lets the next arm attribute a flip to a mechanism.

**Provenance of the paired `.md` sidecars**, which §6 makes worth stating
precisely. All three are real OCR output from `synthpass` on the specimen, on
an unoptimised build:

- **Hong Kong 2007 and 2019** — default `SYNTHPASS_OCR_MAX_SECONDS`. Both
  sidecars contain their zone, so the ladder reached the winning variant within
  the budget; nothing was truncated and nothing needed re-running.
- **Germany 2024** — captured with the budget raised. At the default it stops
  early on this build and the sidecar contains **no zone at all**, which is not
  what CI sees: the baseline classifies this document `checksum_failed`, and
  that requires a parseable zone, which requires reaching variant 10.

Each variant's output is deterministic given the variant; only *which* variants
run depends on the clock. So the raised-budget capture is the faithful artefact
here and the truncated one is an artefact of this machine.

## 2. Germany `P0_D00_2024` is stored sideways, and misses by one character

The image is rotated 90°: the MRZ runs vertically down the left edge of the
stored raster. Recovery depends entirely on the two quarter-turn variants at
positions 12 and 13 — the *last* two rungs of the retry ladder — which
`RotateMode::Default` documents as the accepted cost of retiring the upfront
orientation vote (`crates/synthpass-ocr/src/lib.rs:1265`).

Given enough budget to reach the tail (§6), the winning read is:

```
got   C76311T472D<<8308126F34050192405<<<<<<<<<10   (43 chars)
truth CZ6311T472D<<8308126F34050192405<<<<<<<<<<10  (44 chars)
```

One substantive error — `Z` read as `7` in the document number — plus one
dropped `<` from the filler run. `document_number` and `composite` fail;
`date_of_birth`, `date_of_expiry` and `personal_number` all pass. So
`CORPUS_COVERAGE.md`'s "single-character-misread pattern" attribution is now
**measured rather than assumed**, and it was right.

## 3. Character-level attribution for HKG 2007

Against the verified transcription:

| Line | Errors | Dominant confusion |
| :-- | --: | :-- |
| 1 | 29 / 44 | `<` → `E` (15), `<` → `C` (11), `<` → `R` (1) — **26 of 29 are filler runs** |
| 2 | 15 / 44 | `0` → `O` (8), `0` → `D` (4) — **12 of 15 are zero-vs-letter** |

Both classes are already named in the corpus: the filler collapse is the same
mechanism behind the 108 `document_code`/`issuing_state` disagreements in the
TCR1 re-OCR, and `0`/`O` is the confusable pair `mrz` 0.7.1 added a repair for.
Neither is reachable by that repair here — `damaged_pass` applies one
substitution per line, and this needs twelve.

Note what is *not* wrong: the date and expiry digit runs (`8008080`, `170205`)
read correctly. The errors concentrate in the zero-heavy placeholder document
number `K00000000`, where OCR-B `0` and `O` are least separable and no ICAO
field-type constraint distinguishes them — both are legal MRZ characters in
that position, so the MRZ-charset beam search cannot break the tie either.

The Hong Kong pair is also the corpus's resolution floor: at 265 px wide a
44-character line gives roughly 6 px per glyph before any upscale.

## 4. San Marino 2017 back prints no MRZ (rename pending)

`San_Marino_ID_Specimen_2017_back_mrz.jpg` (613 × 388) prints **three lines of
nothing but `<` fillers**: TD1's exact shape, 30 uniform glyph runs per line,
and no character in any check-digit position. It is a blank facsimile —
established here by inspection rather than inherited from the earlier modelled
read.

When the rename lands: `no_mrz_found` 4 → 3, `no_mrz_expected` 51 → 52,
`scored` 159 → 158, HIT unchanged at 145. **The detection residual is three
documents, not four** — France ID 2020 back, Italy CIE 2022 back, Moldova
`PA_MDA_2014`.

## 5. Two filenames still inherit their classification from OCR (fix pending)

`corpus_manifest` derives `mrz.present` as
`claims.mrz_present.unwrap_or(observed.found)`, so a file carrying neither `mrz`
nor `no_mrz` in its name records *what OCR returned on that pass* — in the very
field the benchmark's denominator is built from.

The example's own module documentation already states the invariant this
violates: "There is deliberately no OCR fallback: a name that claims nothing
records nothing." That is true of `document_code` and `issuing_state`. It is
not true of `present`.

Two files remain in that hole, both Bosnian driving licences, and both are now
verified MRZ-less by inspection rather than by the accident the earlier review
named ("right only because `ocrs` returns nothing on a GIF"):

- `..._Specimen_face.gif` — the portrait side. Labelled fields, no zone.
- `..._Specimen_front.gif` — despite the name, the **back**: the category table,
  carrying a 1D barcode (`4d. 1234567890123`), not an MRZ.

Also recorded: both Bosnian filenames have the wrong side token — the file
called `face` is the front, the one called `front` is the back. A wrong side
token is exactly what produced the Sweden misclassification in PR #277.

## 6. The retry budget is wall-clock, so the bucket depends on the machine

Found while generating the fixtures above, and **the most consequential finding
here**. `recognize_detailed`'s retry loop breaks on
`overall_started.elapsed() >= max_duration`
(`crates/synthpass-ocr/src/lib.rs:574`) with `DEFAULT_MAX_SECONDS = 52`.
Measured on the same binary and the same image:

| Budget | Variants reached | Outcome for Germany `P0_D00_2024` |
| :-- | :-- | :-- |
| 52 s (default) | exhausted early | **`no_mrz_found`** |
| 3000 s | all 13; variant 10 wins | **`checksum_failed`** (one character out) |

Per-variant cost on that run: variant 7 178 s, variant 8 227 s, variant 10
306 s. The run is an unoptimised build, which is the point — nothing about the
*input* changed, only how fast the machine got through it, and the document
moved between two different regression buckets.

`RotateMode::Default` already documents the trade ("a tightened
`DEFAULT_MAX_PASSES` override or an exhausted `DEFAULT_MAX_SECONDS` budget
loses sideways pages… a caller trimming either budget for latency is making
that trade, and should know it"). What is not documented is that **no caller
has to trim anything**: a slower machine, a loaded CI runner or a larger scan
takes the trade silently, and the variants lost first are the tail — positions
12 and 13, where orientation recovery lives.

This sits against `project_principles.md` #1 and the README's "given the same
seed and parameters, output is byte-identical". A wall-clock cutoff on the
extraction path is not a deterministic parameter. Scoped as its own defect and
its own fix; not addressed here.

## 7. Nineteen fixtures are not ICAO-exact

Measured across all 62 tracked `samples/ocr_fixtures/*.json` carrying an
`mrz_line`: **19 have at least one line whose length is not its format's
width.** The distribution is not random — 15 of the 19 are the same slip, one
`<` too many on line 1 of a TD3:

| Shape | Count | Fixtures |
| :-- | --: | :-- |
| `[45, 44]` | 15 | Angola 2026, Azerbaijan 2022, Bangladesh 2023, Djibouti 2017 & 2019, Dominican Republic 2020, India 2022 & 2023, Indonesia 2024, Nepal 2019, Nigeria 2022, Pakistan 2024, Somalia 2023, Somaliland 2023, Uzbekistan 2013 |
| `[45, 45]` | 2 | Argentina 2026, Argentina 2026 blur |
| `[43, 44]` | 1 | Canada 2023 |
| `[44, 45]` | 1 | Mauritania 2010 |

`mrz::find_and_parse` tolerates the slip, which is why no test caught it and
why no published number is wrong because of it. But these are the labels every
`zone_mismatch` figure is measured against, so a correction shifts previously
published character counts. **Recorded, not fixed**: the repair is mechanical
(pad or trim line 1 to the format width) and belongs in its own PR, where the
changed character counts are the point of the change rather than a side effect
of a denominator fix.

## What this leaves

Every one of the 10 `checksum_failed` documents now carries a hand-transcribed
printed zone, so every one can be attributed character-by-character rather than
argued about. Three follow-ups are named and scoped above: the `samples-data`
renames with their re-bless (§4, §5), the wall-clock budget defect (§6), and
the fixture length census (§7).
