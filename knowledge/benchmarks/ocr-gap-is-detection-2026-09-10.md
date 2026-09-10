# Cell (b): the browser/native OCR gap is a text-*detection* gap

**2026-09-10.** [`ADR-0008`](../decisions/ADR-0008-mrz-detection-track.md) chunk 1C, cell (b).
Cell (a) ([`ocr-order-band-first-2026-09-10.md`](ocr-order-band-first-2026-09-10.md)) ruled out
pass ordering. This cell asks the research note's central question — *does the browser localize the
band better, or recognize characters better?* — and the answer is **neither**: on the documents
that drive the gap, native `ocrs`/`rten` never produces enough text to try.

## Method

`provider-bench --real-specimens --mrz-only --dump-ocr` (the gate widened to `no_mrz_found` in
this branch) writes, per in-denominator miss: `OcrPage::mrz_band_score` (the content-scored MRZ
band, `None` when no group of OCR'd lines scored MRZ-shaped) and the full accumulated OCR text the
recognizer produced across the general pass and every retry variant. 25 in-denominator misses:
18 `no_mrz_found` + 7 `checksum_failed`.

## The 18 `no_mrz_found` split three ways

### 11 — OCR **detection** failure: near-empty text for the whole page

| document | total OCR chars | band |
| :-- | --: | :-- |
| `Monaco_ID_XXXX_back` | 8 | none |
| `Argentina_P0_ARG_2026_mrz_blur` | 9 | none |
| `Vietnam_P0_VNM_2023` | 14 | none |
| `Canada_PP_CAN_2023` | 17 | none |
| `Finland_P0_FIN_2023` | 22 | none |
| `India_P0_IND_2024` | 21 | none |
| `Russian_Federation_P0_RUS_2014` | 33 | none |
| `Portugal_PX_PRT_2017` | 33 | none |
| `Finland_P0_FIN_2007` | 39 | none |
| `Kuwait_P0_KWT_2023` | 38 | none |
| `Oman_P0_OMN_2004` | 40 | none |

For all eleven, the *entire page* — general full-page pass plus every preprocessed retry variant —
OCR's to a few dozen stray characters: `"N\nunde\nWE\ns\nG\ni\nP\n?\nI\n3\nP\n1\nR"` is the
complete output for the Canada passport. The MRZ is not misread. It is not read. `ocrs`'s text
**detection** stage returns almost nothing on these images, so there is nothing for band
localization or character recognition to work with.

These are not damaged documents — the user hand-read every MRZ zone from the same images without
difficulty (`P123456AA0CAN9008010F3301144<<<06` for Canada). And they are **exactly the documents
the browser reads**: 8 of these 11 are named in
[`ocr-stack-gap-2026-09-09.md`](ocr-stack-gap-2026-09-09.md)'s list of native detection failures
that tesseract.js recovers. Tesseract extracts a usable MRZ from the same pixels `ocrs` sees as
noise.

### 3 — recognition: band found, MRZ text present, not assembled

| document | total OCR chars | band | what's there |
| :-- | --: | :-- | :-- |
| `France_ID_2020_back` | 10 305 | 0.36 | line 2 (`9007138F3002119FRA…`) and line 3 (`MARTIN<MAELYS…`) read with errors; line 1 lost; the MRZ is drowned in ~10 kB of French legal boilerplate the blind bottom-crop drags in |
| `Netherlands_Driving_License` | 2 086 | 0.77 | DL 1-line format (`D1NLD…`); the band scorer is TD-shaped |
| `Italy_ID_2022_back` | 906 | 0.68 | line 2 read **exactly** (`6412308F2212304ITA<<<<<<<<<<<0`), line 3 nearly so, line 1 (`C<ITACA00000AA4`) garbled to `KACAOOODOAAK` — heavy O/0/A confusion |

### 4 — specimen artifacts / correct refusal (bucket A/D, not detector-movable)

`Egypt_P0_EGY_2012` (MRZ fully X-redacted, just not filename-tagged `_redacted_mrz`),
`Moldova_PA_MDA_2014` (2×58 novelty specimen, `…ALIENSWITHEXTRAORDINARYSKILLS…`),
`Argentina_P0_ARG_2021_mrz_child` (child-passport `ZZZ` layout), `Sweden_ID_2027` (genuinely no
MRZ — a correct refusal). Same reclassification shape as
[the Malaysia specimen](denominator-correction-2026-09-09.md).

## The 7 `checksum_failed` are all recognition

Every one has a healthy band (0.32–0.90, except `Russia_P0_RUS_2019` which parsed off a blind
crop) and a parsed 2- or 3-line MRZ with one wrong character. `Afghanistan` and `Belgium` are
O/0 confusables (`…UT0…` for `…UTO…`); `Belgium` is the document cell (a)'s `band-first` recovered.
Two are specimen artifacts: `Croatia_ID_2021` (all-zeros printed zone) belongs in
`checksum_failed_specimen`; `Russia_P0_RUS_2019` is the one document missed by *both* stacks.

## Tally

| bucket | count | the lever |
| :-- | --: | :-- |
| **OCR detection failure** | **~11** | `ocrs`/`rten`'s detection model, or the preprocessing feeding it |
| recognition (wrong character / drowned zone) | ~8 | recognizer character accuracy; O/0 confusables; dense-page crop |
| specimen artifact / correct refusal | ~6 | denominator reclassification (bucket A/D) |

The genuine detector-movable target is **~19**, not 25 — and **detection is the majority of it.**

## What this answers

The research note ([`Tesseract_OCR_studies.md`](../research/Tesseract_OCR_studies.md)) framed the
choice as *localization advantage vs recognizer advantage*. The measurement says the dominant
factor is upstream of both: **`ocrs` is not detecting text on ~11 clean passport images at all**,
while tesseract's pipeline (Leptonica binarization + scaling + its own detector) extracts a
readable MRZ from the same bytes. Band localization, OCR-B character accuracy, PSM, and retry
budget are all downstream of a detector that is not firing.

The note's §8 — *"don't underestimate scale and binarization … Tesseract isn't better at
recognizing MRZ; its input is effectively 2× upscaled and better binarized"* — is the most likely
explanation, and the one with a pure-Rust fix that adds no dependency.

## Next

- **Cell (c)** — dump the exact preprocessed variant images `ocrs` is fed for these 11 and run
  tesseract's recognizer over those same bytes. If tesseract *also* fails on native's crops, the
  gap is entirely in native's detection/preprocessing (not the recognizer), and the fix is a
  Rust-native preprocessing change. If tesseract succeeds on native's crops, the recognizer
  contributes too.
- **Bucket A** — a third denominator refinement (Egypt, Moldova, Croatia, Argentina ×2): same
  shape as the Malaysia specimen, its own small PR.

## Reproducing

```console
$ cargo build --release -p synthpass-bench --bin provider-bench   # on this branch
$ SYNTHPASS_OCR_MAX_SECONDS=90 ./target/release/provider-bench \
    --real-specimens --mrz-only --progress --dump-ocr --out artifacts/dump-run.json
$ # rows land in artifacts/provider-bench-miss-ocr-dump.jsonl
```

The JSONL carries specimen OCR text and stays under `artifacts/` (gitignored); it never enters a
committed report.
