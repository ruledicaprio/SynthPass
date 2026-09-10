# Cell (c): the detection failures are a wrong-orientation failure

**2026-09-10.** [`ADR-0008`](../decisions/ADR-0008-mrz-detection-track.md) chunk 1C, cell (c).
Cell (a) ([`ocr-order-band-first-2026-09-10.md`](ocr-order-band-first-2026-09-10.md)) ruled out
pass ordering. Cell (b) ([`ocr-gap-is-detection-2026-09-10.md`](ocr-gap-is-detection-2026-09-10.md))
localised the gap to native text *detection*: on ~11 clean passports `ocrs` returns a few dozen
stray characters for the whole page. This cell holds the recognizer as the variable — it runs the
browser's tesseract.js OCR-B model over the **exact preprocessed crops `ocrs` was fed** — and finds
why those crops are unreadable: **native rotated every one of them 90°.**

## Method

- New `SYNTHPASS_OCR_DUMP_VARIANTS=<dir>` env var makes `synthpass-ocr` write every preprocessed
  image it hands the recognizer (the general full-page pass + each retry variant) to disk as a PNG.
  `cargo run -p synthpass-ocr --example dump_variants` drives it over a 14-document set: the 11
  detection failures from cell (b) + 3 recognition cases (`France_ID_2020_back`,
  `Netherlands_Driving_License`, `Italy_ID_2022_back`) as a control.
- `tests/web/recognize-crops.{html,mjs}` runs the *verbatim* `web/scan.js` OCR-B worker config
  (`createWorker('mrz', 1, …)`, OEM 1, `MRZ_CHARSET` whitelist, no PSM) over the dumped PNGs in
  real headless Chromium — recognizer only, no second preprocessing, no `eng` fallback. `--best`
  additionally OCRs each image at all four orientations and keeps the best.
- `SYNTHPASS_OCR_MAX_SECONDS=90` so every retry variant runs on a miss. Corpus: local `samples/`.

## Every one of the 11 detection failures was auto-rotated

The `SYNTHPASS_OCR_VERBOSE` pass log records the rotation `choose_rotation` applied before the
first pass:

| document | native OCR chars (cell b) | rotation applied | original |
| :-- | --: | :-- | :-- |
| `Monaco_ID_XXXX_back` | 12 | **270°** | upright |
| `Argentina_P0_ARG_2026_mrz_blur` | 14 | **90°** | upright |
| `Vietnam_P0_VNM_2023` | 22 | **270°** | upright |
| `Canada_PP_CAN_2023` | 29 | **90°** | upright |
| `Finland_P0_FIN_2023` | 33 | **270°** | upright |
| `India_P0_IND_2024` | 35 | **90°** | upright |
| `Russian_Federation_P0_RUS_2014` | 49 | **90°** | upright |
| `Portugal_PX_PRT_2017` | 51 | **90°** | upright |
| `Finland_P0_FIN_2007` | 54 | **270°** | upright |
| `Kuwait_P0_KWT_2023` | 57 | **270°** | upright |
| `Oman_P0_OMN_2004` | 61 | **270°** | upright |

**11 of 11.** The 3 recognition-control documents were **not** rotated. The correlation with cell
(b)'s split is exact. (`Argentina_P0_ARG_2026_mrz_blur` is a deliberately blurred variant *and* a
Bucket-A fake-zone specimen, so it is the weakest member of the set — the other ten are clean
scans whose only defect is the turn.)

These are not sideways photographs. `Canada_PP_CAN_2023` is a 389×256 scan of an upright passport
bio page with a horizontal MRZ (`P123456AA0CAN9008010F3301144<<<<<<<<<<<<<<<06`). Native turned it
90°, so `ocrs`'s detector — and every downstream `mrz_variants` / `plain_band` crop, all derived
from the rotated image — saw the MRZ as a vertical column of glyphs.

### Why `choose_rotation` misfires here

`choose_rotation` (`synthpass-ocr/src/lib.rs`) scores each candidate rotation by the mean
width÷height ratio of the text lines `ocrs` detects and picks a 90°/270° turn if it beats upright
by `ROTATION_MARGIN` (1.2×). On these small, low-contrast specimen scans `ocrs` detects almost no
text on *any* orientation, so the score is computed from a handful of noise boxes and a wrong
rotation clears the margin by chance. The detection failure and the orientation failure are the
same failure: too little real text detected to decide anything.

`web/scan.js` **had this exact bug and removed it**. Its comment, verbatim (lines 187–192):

> These run only after EVERY upright attempt has failed, so a document that reads today cannot be
> broken by them. That ordering is not a stylistic choice: an earlier version of this probed the
> page orientation up front and rotated before scanning, and the probe was wrong often enough to
> cost 9 upright documents (125 -> 116). Detection that can be wrong must not be allowed to rewrite
> the input; the ICAO check digits decide instead.

The browser now applies rotations only as *late retry passes*, never as an upfront commitment.
Native's `choose_rotation` is the upfront probe the browser threw away.

## Tesseract OCR-B over native's crops — as native produced them

Garbage on all 14. Best crop per document: 12–806 characters at confidence 3–41, none
MRZ-parseable — `Oman`'s best reads `PI0 H / 0 5181 / I W EF / I 3 G SI8 B / …`, the MRZ glyphs
split one per line because the text is sideways. Rotating native's *processed* `mrz_variants`
crops back through all four orientations (`--best`) does not recover them either: those variants
are contrast-stretched, binarised blind crops of an already-rotated page, and a low-resolution
binarisation is lossy. **A specialised OCR-B recognizer cannot read native's crop — not because of
the pixels, because of the rotation (and, downstream of it, the wrong crop region).**

## Tesseract OCR-B over the *original* images — best of four orientations

Every one lands at **0° — no rotation** — and OCR-B reads the MRZ straight off the full page on 4
of 6 sampled documents, with no band crop at all:

| document | best rotation | line 2 read (truth in parens) |
| :-- | :-: | :-- |
| `Canada_PP_CAN_2023` | 0° | `F123656AA00AN9008010F3301144<<<…06` (`P123456AA0CAN9008010F3301144<<<…06`) |
| `India_P0_IND_2024` | 0° | `5F003369<21ND9407015F34090281065269546124<78` (`SP003369<2IND…`) |
| `Kuwait_P0_KWT_2023` | 0° | `P068299555KWT0405247M2801038304052400244<<<6` — **exact** |
| `Portugal_PX_PRT_2017` | 0° | `P<PRTGARCAO<DE<MAGALHAES<<INES<<…` clean line 1; line 2 mostly clean |
| `Monaco_ID_XXXX_back` | — | noise — the original is 254×162; this one also needs the upscale |
| `Oman_P0_OMN_2004` | — | noise at full page — needs the band crop to isolate the MRZ from the VIZ |

And the browser's own corpus sweep (`node tests/web/run-corpus.mjs`, the harness behind
[`WEB_OCR_BASELINE.md`](../WEB_OCR_BASELINE.md)) reads **Canada, Oman and Monaco** —
`web_checksum_valid: true`, `winning_pass: "Reading MRZ band (OCR-B model)"`, **`passes: 2`**: its
first OCR attempt, `plain_band` + OCR-B, at the original orientation. No rotation. Monaco and Oman
come in because `plain_band` crops *and upscales* the band — so scale is a secondary lever behind
orientation, not a primary one.

## What this answers for 1D

The research note ([`Tesseract_OCR_studies.md`](../research/Tesseract_OCR_studies.md)) framed the
choice as *localization advantage vs recognizer advantage*, with §8 nominating scale and
binarisation. The measurement says it is none of those and simpler: **native commits to one
page orientation up front and gets it wrong on exactly the documents where `ocrs` detects too
little text to choose**, then crops and recognises a sideways page. The recognizer is not the
variable — OCR-B fails on native's rotated crop just as `ocrs` does, and reads the MRZ off the
same documents once they are upright.

The lever is pure-Rust and adds no dependency:

1. **Gate `choose_rotation`** — do not rotate when the detected-text signal is below a confidence
   floor (the same "is there enough here to trust" test cell (b) shows these documents fail).
2. **Move 90°/270° into the retry chain** — native already rotates for the 0°/180° tie-break
   (`rotate_image`, `should_flip_180`); extend that to try the other two orientations as *late*
   band-crop variants when the chain is about to exhaust on `no_mrz_found`, exactly as `scan.js`
   does.
3. A modest upscale before detection (`plain_band` already upscales; the *general* pass does not)
   would additionally recover the sub-300px documents like Monaco.

All three are small, testable changes to `recognize_detailed`'s existing structure, measurable
same-binary A/B against these 11.

## Reproducing

```console
$ cargo build --release -p synthpass-ocr --example dump_variants
$ SYNTHPASS_OCR_DUMP_VARIANTS=artifacts/cell-c/crops SYNTHPASS_OCR_MAX_SECONDS=90 \
    SYNTHPASS_OCR_VERBOSE=1 ./target/release/examples/dump_variants <14 image names>
$ cd tests/web && npm ci && npx playwright install chromium
$ node recognize-crops.mjs --crops ../../artifacts/cell-c/crops \
    --out ../../artifacts/cell-c/tesseract-on-native-crops.jsonl
$ node recognize-crops.mjs --crops ../../artifacts/cell-c/crops --only __variant00 --best \
    --out ../../artifacts/cell-c/tesseract-best-orientation.jsonl
$ # and over the untouched originals:
$ node recognize-crops.mjs --crops <dir of original images> --best \
    --out ../../artifacts/cell-c/tesseract-originals-best.jsonl
```

The JSONL carries specimen OCR text and stays under `artifacts/` (gitignored); it never enters a
committed report.
