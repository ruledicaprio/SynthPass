# ADR-0028 — SynthPass owns every resample; scale follows the measured MRZ pitch

**Status:** Proposed
**Date:** 2026-10-01

> **Nothing here is built.** This ADR states a policy and the order in which it is built. The
> Consequences say what changes as each step lands. No default output changes before step 6.

## Context

### Three doors, three scales

The same photo reaches recognition at a different scale depending on how it enters:

- **CLI and library: full decoded resolution.** Nothing on the native path scales down an image
  that recognition reads. `scale_by` in `crates/synthpass-imageprep/src/preprocess.rs` returns a
  copy for any factor at or below 1, and band variants are only ever upscaled, to `BAND_MIN_WIDTH`.
- **Browser demo: at most 1600 px on the long side.** `web/scan.js` draws the image onto a canvas
  of that size before anything else runs, so the browser chooses the resampling filter.
- **Serve upload page: at most 2000 px, then JPEG.** `crates/synthpass-serve/src/index.html`
  downscales an image to 2000 px on its long side and re-encodes it as JPEG at quality 0.85 before
  upload. Only an image already within 2000 px and under 2 MB passes through untouched. No
  document recorded this until now. It is lossy, it happens before any measurement, and this ADR
  records it as **known drift** (open question 2).

None of the three scales was measured.

### What the engine does with the scale it is given

Observed in `ocrs` 0.13.1 and `rten` 0.26.0 (References):

1. **Detection runs on a fixed 800 × 600 input (height × width).** The exported graph admits no
   other size.
2. **The fit is per axis, not aspect-preserving.** A short axis is padded and a long one resized,
   so a band on a page that is not 4:3 reaches the detector with its aspect changed.
3. **The resize is bilinear, two taps per axis, with no antialiasing.** At a large decimation most
   source rows and columns contribute nothing.
4. **Detection post-processing constants are in pass-image pixels** and are not configurable: the
   threshold, the expansion distance, the minimum area, the polygon tolerance and the layout
   overlaps. They act at whatever scale the page arrives at.
5. **Recognition resizes every line to a 64 px height, without antialiasing,** from the
   full-resolution pass image. The recognition model was trained on lines resized with
   antialiasing.
6. **The public API already allows full control without a fork.** A caller can pass inputs of the
   exact detector size, pass its own line rectangles to `recognize_text`, and read the raw map
   from `detect_text_pixels`. The upstream issues for downscaling and antialiasing are open, with
   no fix proposed.

So on a multi-megapixel photo the zone reaches the detector squashed and aliased, and it is
post-processed by constants set for another scale. SynthPass's own pixel constants (the median
cap, the local-threshold window, the row window) assume a band near `BAND_MIN_WIDTH`.

### What the corpus can and cannot show

Real specimens are mostly small and cropped to the document, and phone framing is absent. The
one scored multi-megapixel document, France ID 2020 back, is in the
[post-M6 residual](../benchmarks/README.md#post-m6-residual). **The real corpus cannot test this
policy.** A synthetic scale axis whose oracle is the true pitch can, under ROADMAP's rule that a
known transform is the oracle
([Generator variety](../ROADMAP.md#generator-variety--the-axes-and-what-each-one-is-for)).

Scale has been raised before and left open:

- [ADR-0008](ADR-0008-mrz-detection-track.md)'s 2026-09-10 amendment made scale a secondary lever.
- Its 2026-09-12 amendment did not build the general-pass upscale.
- The band-squeeze hypothesis for France was left untested
  ([`orientation-fix-2026-09-12.md`](../benchmarks/orientation-fix-2026-09-12.md)).
- [ADR-0011](ADR-0011-split-m6-packaging-into-m8.md) named that hypothesis as the first thing that
  would change its decision.

### The zone is its own ruler

Doc 9303 fixes the MRZ character pitch at 2.54 mm, so every other dimension can be stated in
pitches (p):

| Quantity | mm | p | Source |
| --- | --- | --- | --- |
| Character pitch | 2.54 | 1 | Doc 9303 Part 3 §4.4; ISO 1073-2 §3.8; ECMA-11 §3.8 |
| Digit height, size I | 2.66 | 1.05 | ISO 1073-2 §4.1 (Table 1) |
| Nominal stroke width | 0.35 | 0.138 | ISO 1073-2 §11.4.1 |
| Minimum character spacing | 2.30 | 0.906 | ISO 1831 §6.7.2 |
| Nominal line pitch, TD3 and MRV | 6.35 | 2.50 | Doc 9303 Part 4 §3; Part 7 |
| Nominal line pitch, TD1 | 4.23 | 1.665 | Doc 9303 Part 5 §4.2.1 |
| Line pitch, TD2 | not transcribed | — | measured first ([ADR-0015](ADR-0015-geometric-mrz-band-location.md), 2026-09-28) |
| Maximum print skew | — | 3° | Doc 9303 Part 3 §4.11; ISO 1831 §6.4 |

The tracked distillation is [`line-and-pitch.md`](../ocrb/line-and-pitch.md) and
[`glyph-dimensions.md`](../ocrb/glyph-dimensions.md). Where this table and those pages disagree,
those pages win. Conforming prints vary around the nominal line pitch within a band that is
different for each format (ADR-0015's 2026-09-28 amendment).

Two further observations shape how the ruler is read:

- **Line width divided by the character count is a precise ruler. Glyph box height is not, and
  width alone does not reveal the character count.** This was observed on the corpus's recorded
  line boxes during the design for [#563](https://github.com/ruledicaprio/SynthPass/issues/563).
  Build step 2's trace-replay report publishes the counts as a dated note.
- **The obvious anchor is the weakest cell.**
  - On clean synthetic TD3 renders, `P` reads as itself. The `<` after it is read as `S` in 96 of
    100 ([glyph atlas](../benchmarks/glyph-atlas-p-filler-2026-09-30.md)).
  - `P<` is also being phased out: Doc 9303 Part 4 §4.4 requires a second letter on every passport
    issued from 2028.
  - Anchoring on a literal `P<` would therefore fail on the cell OCR misreads most, and on a
    growing share of passports.

## Decision

1. **SynthPass chooses the scale of every image an engine reads.** The engine's own fixed-size
   resizes are left with a factor near 1:
   - detection receives an input of exactly its fixed size, so `ocrs` neither pads nor resizes;
   - recognition receives line rectangles whose height falls inside a swept range around its fixed
     line height.

   No engine resample decides the scale.
2. **Scale follows the measured pitch of the zone, not the size of the image.**

   | Stage | What it does | Where |
   | --- | --- | --- |
   | 0. Decode | EXIF orientation, lossless (exists) | `synthpass-ocr` |
   | A. Locate | Build an overview: the whole page, isotropic and antialiased, letterboxed to the detector's exact input size, and detect on it at 1:1. If there is no candidate band and the overview's glyphs fall below a floor, go one level finer with overlapping tiles. At most two levels | canvas: `synthpass-imageprep`; detection: `synthpass-ocr` |
   | B. Measure | For each candidate band: the anchor class, hypotheses for the character count N and the line count, and the pitch cues of item 5 | geometry: a new `synthpass_imageprep::pitch` module; the anchor check from `mrz`: `synthpass-ocr` |
   | C. Normalise | Crop the band from the original decoded image in pitch units, deskew it with the existing estimators, and change its scale once, antialiased, to a fixed pitch | `synthpass-imageprep` |
   | D. Read | Detect on a band canvas of the detector's exact size at a detection pitch. Map the line rectangles onto a recognition image at a recognition pitch, and call `recognize_text` with them | `synthpass-ocr` |
   | E. Decide | `mrz::find_and_parse`, and the check digits decide, as today. The parse fixes N. If it differs from Stage B's hypothesis, Stage C runs once more with the parsed N | `mrz`, `synthpass-ocr` |

3. **The parser settles N.** The anchor class gives a family: `P` → 44; `V` → 44 or 36; `A`, `C`
   or `I` → 30 or 36. The line count and the line spacing narrow it further. Each surviving N
   yields a pitch, and the first checksum-valid parse fixes N.
4. **The anchor is a class of document codes over cells 0 and 1. It ranks and never gates.** The
   classes are defined in [ADR-0021](ADR-0021-fixed-grid-mrz-strips.md)'s 2026-10-01 amendment to
   item 7b.
5. **A pitch is used only when independent cues agree.**
   - **Glyph centres:** a least-squares fit over glyph positions (`chargrid::fit_grid` does this
     today). It is robust to short filler tails.
   - **Line width divided by N:** taken on the most complete line.
   - **Line spacing divided by the format's nominal line pitch.** Its tolerance is the format's
     conforming spacing band from ADR-0015's 2026-09-28 amendment, not the agreement tolerance.
     That band is narrow for TD1 and wide for TD3.
   - **Box height:** a cross-check only.

   If the cues disagree, the page is not normalised. It falls through to today's chain, and the
   disagreement is recorded in the pass trace. How it is reported beyond the trace is a separate
   decision. Two more signals rank candidates and never gate:
   - the number of fillers across the band;
   - where the page edge is in frame, the reference-edge position of ADR-0021 item 7a.
6. **One scale change per buffer, taken from the original pixels.** Stage C scales the original
   decoded image once and never rescales a buffer that was already resampled. The filter is
   pinned to one named `image` crate filter, because resampling libraries disagree with each other
   (References). The existing rule of at most one continuous rotation per buffer (`rotate_rgb` in
   `preprocess.rs`) is unchanged.
7. **Every constant is derived, swept, measured or pinned.** This ADR fixes no value.
   - **Derived** from the standards above: pitch, glyph and line geometry; the pitch tolerance
     floor (ISO 1831's minimum spacing) and ceiling (printing-zone width divided by N); crop
     margins from the zone geometry, the first-character tolerance and the 3° skew allowance. Each
     is a named `const` whose doc comment cites its section and shows the arithmetic.
   - **Swept:** the detection pitch, the recognition pitch, the identity window (open question 3),
     the cue-agreement tolerance, the overview floor before tiling, and the letterbox fill. Each is
     named under [Naming a threshold](../benchmarks/README.md#naming-a-threshold).
   - **Measured:** the ratio of box height to pitch, used as a cross-check only.
   - **Pinned:** the resampler.
8. **Boundaries.**
   - `synthpass-imageprep` stays `wasm32`-clean, reads no environment variable and gains no
     dependency. Every size it targets (detector input, pitches) is an argument its caller
     supplies. No engine's size is a constant there.
   - The arm lives in `synthpass-ocr` only: `SYNTHPASS_OCR_SCALE=off|on|control`, added to
     `OcrArms`, so a baseline refuses to run with it away from its default.
   - No engine name enters `synthpass-die` or `synthpass-core`. Per-pass scale observations go to
     the benchmark pass trace only
     ([ADR-0024](ADR-0024-per-document-benchmark-archive.md), amendment 1).
   - Nothing in this ADR accepts a read. The check digits decide.
9. **Rollout: default off and trailing, with one named exception.**
   - **Through step 5,** the normalised path is a trailing tier behind the arm, off by default.
   - **The exception to "additive and trailing" (owner's decision, 2026-09-30).** From step 6 the
     normalised path may run *ahead* of the proven chain, and only on pages outside the identity
     window. It needs zero hit→miss at the real-specimen gate, with the per-document diff read
     field by field (`names_exact`, `name_error`, `check_states`, `mrz_format`,
     `retry_variant_id`), and every document that moves named in a dated note.
   - **Precedent.** ADR-0008's 2026-09-12 amendment changed the front of the default path on gate
     evidence when it retired the upfront orientation vote.
   - **What the exception keeps.** The proven chain is not removed or reordered. It runs,
     unchanged, after any normalised attempt that does not validate. Retries stay additive: Tier-2
     input only gains candidate lines. A wrong location or a wrong pitch costs passes and time,
     never the chain.
   - **What it risks.** A normalised read that is wrong in unchecked cells but checksum-valid
     would stop the loop before the proven chain runs. That is why the gate reads fields, not
     hits.
   - **Why the scope is limited.**
     [`WEB_OCR_BASELINE.md`](../WEB_OCR_BASELINE.md#2026-09-03-d--rotation) records the lesson that
     a guess which can be wrong must not rewrite the input. The exception is therefore limited to
     out-of-window pages, where the proven chain already reads at a scale the engine was not
     calibrated for.
   - This ADR grants the exception, not the promotion. Step 6 still needs its own gate evidence.
10. **Browser (owner's decision, 2026-09-30).** The shared normaliser, exposed through `mrz-wasm`,
    replaces the browser's 1600 px canvas cap (step 7). The browser's own engine supplies Stage A's
    candidate lines. Stages B and C are the same code natively and in the browser.
11. **Memory (owner's decision, 2026-09-30).** The overview is built right after decode, so no
    full-resolution variant is held longer than Stage C needs it. A megapixel limit on decode is
    named only if memory measurements demand one.
12. **Measurement.**
    - **A synthetic scale axis, oracle the true pitch:** source pitch, framing, canvas size,
      background, perspective, and capture blur in millimetres, across all five formats. Synthetic
      results can only veto (ADR-0021, default-on rule 4).
    - **A/B discipline:** both arms run on one runner with the budget pinned
      ([ADR-0027](ADR-0027-ci-runners-measure-public-benchmark-arms.md), decisions 3 and 4). No
      CI timing is a result (decision 6).
    - **Real corpus:** the pitch estimators are first scored by replaying the recorded pass trace,
      with no OCR run.
    - **Scoring:** against fixtures and synthetic truth, never against `valid()` alone. A
      checksum-valid read can still be wrong in cells no check digit covers: TD1 line 3, every name
      field, and the substitutions `mrz::Blindspot` bounds.

## Alternatives

- **A. Status quo: full resolution into `ocrs`.** Rejected. Every large photo gets the per-axis
  fit, the aliasing and the unscaled constants. The pass budget is spent on large pages without
  reading them.
- **B. Cap the long side, as the browser (1600 px) and the serve page (2000 px) do.** One line of
  code, with a precedent in two doors. Rejected as the policy, and interim at most:
  - neither cap was measured;
  - a cap leaves the per-axis fit untouched;
  - it fixes the image size, not the glyph size, so a document filling a small part of a large
    frame still lands at the wrong pitch;
  - in the browser the canvas filter is the browser's choice, not ours.
- **C. Normalise by the measured pitch.** Chosen. It uses a physical ruler and makes thresholds
  independent of scale. It needs no fork, no new dependency and no new model. It costs a locate
  step and more code in `synthpass-imageprep`.
- **D. Tile at native resolution.** It needs no scale guess. Rejected as the default: the number of
  detector runs grows with the photo's area, and recognition still aliases. It is kept as Stage A's
  one finer level.
- **E. Fork `ocrs`, or export a variable-size detector.** This would fix the fit and the constants
  at the source. Not needed: the public API already gives the control this ADR uses. A fork is a
  standing maintenance cost, and a new model artifact needs its own provenance review.
- **F. Swap the detector for one with dynamic input.** Out of scope. It brings a new model, a
  licence and a provenance review, and ADR-0011 requires an ADR of its own for an engine swap.
- **Rejected inside C:**
  - *A literal `P<` anchor:* see Context; ADR-0021's 2026-10-01 amendment replaces it with classes.
  - *N from the line width alone:* width does not reveal N, so the parser settles it.
  - *Pitch from glyph box height:* too variable, so it is a cross-check only.
  - *Rescaling an already-upscaled band variant:* interpolation compounds, so Stage C crops from
    the original.
  - *Keeping the engine's scale and fixing only its constants:* they are not configurable in the
    pinned version, which leads back to E.

## Consequences

**As each step lands** (Build order below):

- **Step 0, this ADR:** no behaviour change. The serve page's downscale and re-encode are on
  record as drift.
- **Step 1:** the pass trace records per-pass scale, and retry variants are built lazily. Outputs
  are identical. The pass file's schema changes, so the replay reader changes with it (ADR-0024,
  amendment 3).
- **Steps 2 and 3:** pure, unwired functions in `synthpass-imageprep`, tested on constructed images
  and built for `wasm32`. No output changes.
- **Step 4:** new generator profiles. The synthetic ledger moves on those profiles only.
- **Step 5:** the arm exists, off by default. Default output does not change.
- **Step 6:** out-of-window pages are read normalised first. Default output changes on those pages
  only, and only after the gate.
- **Step 7:** the browser drops its cap, so browser output changes. Measured in `web-ocr.yml`.
- **Step 8:** later arms, each default off.

**Positive**

- One scale policy for every door and every photo size.
- Thresholds stated in pitches, so they hold at any resolution.
- Fixed-size engine inputs, so a large photo costs less per pass than it does now.
- No fork, no new dependency and no new model. Native and browser share the measuring code.

**Negative**

- A locate step before reading, which is more code and a new way to be wrong (a wrong pitch). It
  is bounded by cue agreement and by falling through to today's chain.
- An isotropic fit would shrink glyphs on small scans that read well today. That is the identity
  window's reason to exist (open question 3).
- Floating-point resampling can differ between native and browser. Pin the filter, compare both in
  `web-ocr.yml`, and never mix CI and local arms.
- Tiles can duplicate or split lines. Merge them in one frame, and test on synthetic pages.
- The claim rests on synthetic evidence. The real corpus can only veto.
- A leading normalised pass spends budget before the proven chain runs. The gate measures whether
  that costs a document.
- The next `rten` bump may shift results. The glyph atlas is re-run as a fingerprint.

**Explicitly not licensed by this decision:**

- a change to any gate, baseline, tolerance or denominator;
- a new dependency, a new model or a fork of `ocrs`;
- a sixth `mrz::Format`;
- resampling pages inside the identity window (open question 3);
- step 6 without its gate evidence.

**What would reverse it.** Either of these:

- The synthetic sweep finds no detection and recognition pitch at which normalised reads match or
  beat full-resolution reads without more wrong-but-valid reads.
- The trace replay shows the pitch cues do not agree on conforming zones often enough to be used.

## Build order

Each step is its own PR. Nothing changes default behaviour before step 6.

| Step | Content | Behaviour change | How it is measured |
| --- | --- | --- | --- |
| 0 | This ADR. The serve page's 2000 px downscale and JPEG re-encode recorded as drift | none | review |
| 1 | The pass trace records per-pass scale: detector factors per axis, aspect change, recognition factor, measured pitch. Retry variants built lazily | none | replay identical; `bench_ab_diff --expect-identical` |
| 2 | `synthpass_imageprep::pitch`: the estimators and the agreement test, and the named geometry constants | none (unwired) | unit tests; trace-replay report on the recorded zones, published as a dated note |
| 3 | `synthpass-imageprep`: the overview canvas, the pitch-unit crop, and the scale change to a target pitch | none (unwired) | tests on constructed images; `wasm32` build |
| 4 | Generator: scale, framing, background and blur in millimetres, each labelled with the true pitch | the synthetic ledger moves (new profiles only) | M4 ledger re-installed from CI |
| 5 | `SYNTHPASS_OCR_SCALE=off\|on\|control`, as a trailing tier (Stages A–E) | none by default | synthetic sweep of the swept constants; a real arm |
| 6 | Promotion: the normalised path leads, only outside the identity window (decision 9) | yes | zero hit→miss, read field by field; every mover named |
| 7 | Browser: `mrz-wasm` exposes the normaliser, and `web/scan.js` drops its 1600 px cap | browser only | `web-ocr.yml` |
| 8 | Later arms: line rectangles from the grid alone; scale-aware post-processing on `detect_text_pixels` | none by default | same-binary A/B |

## Open questions, decided

The owner decided all three on 2026-10-01, each as the design analysis recommended.

1. **The one-line driving-licence MRZ: (a), it stays beyond Doc 9303.** ISO/IEC 18013-3 Amd 1
   §8.3.2.5 defines a one-line zone. It lies outside Doc 9303, and ROADMAP places licences under
   "Beyond ICAO 9303". Stage C's crop geometry keeps room for a one-line shape. A sixth
   `mrz::Format` would need an ADR of its own, because it widens scope beyond Doc 9303.
2. **The serve page's 2000 px downscale and JPEG re-encode: (a), removed once the server
   normalises.** Step 0 records them as drift until then. The removal is a PR of its own after
   step 6.
3. **An identity window: yes.** Pages inside the window are never resampled, so today's population
   of small images is untouched. Decision 9 already limits the promotion to pages outside it. The
   window's bounds are swept (decision 7).

## References

| Source | Location | Used for |
| --- | --- | --- |
| ocrs 0.13.1 `detection.rs` | https://github.com/robertknight/ocrs/blob/e5f1c5205326804637368422fe420aa1b9769676/ocrs/src/detection.rs | per-axis fit; pixel constants |
| ocrs 0.13.1 `recognition.rs` | https://github.com/robertknight/ocrs/blob/e5f1c5205326804637368422fe420aa1b9769676/ocrs/src/recognition.rs | line resize to 64 px |
| ocrs 0.13.1 `layout_analysis.rs` | https://github.com/robertknight/ocrs/blob/e5f1c5205326804637368422fe420aa1b9769676/ocrs/src/layout_analysis.rs | layout constants |
| ocrs 0.13.1 `lib.rs` | https://github.com/robertknight/ocrs/blob/e5f1c5205326804637368422fe420aa1b9769676/ocrs/src/lib.rs | control without a fork |
| Shipped `text-detection.rten` | https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten | fixed detector input |
| rten 0.26.0 `ops/resize.rs` | https://github.com/robertknight/rten/blob/14cbf4620476f402237bb29f731a244d3b6bed4d/src/ops/resize.rs | bilinear, no antialias |
| rten 0.26.0 `onnx_registry.rs` | https://github.com/robertknight/rten/blob/14cbf4620476f402237bb29f731a244d3b6bed4d/src/op_registry/onnx_registry.rs | resize operator mapping |
| rten issue #15; ocrs issues #15 and #274 | https://github.com/robertknight/rten/issues/15, https://github.com/robertknight/ocrs/issues/15, https://github.com/robertknight/ocrs/issues/274 | upstream status |
| ocrs-models `train_detection.py`, `datasets/hiertext.py` | https://github.com/robertknight/ocrs-models/blob/068934f1725959b734edef025b13c46ddf784326/ocrs_models/train_detection.py, https://github.com/robertknight/ocrs-models/blob/068934f1725959b734edef025b13c46ddf784326/ocrs_models/datasets/hiertext.py | training sizes; antialiased line resize |
| image crate `imageops/sample.rs` (v0.25.10) | https://github.com/image-rs/image/blob/v0.25.10/src/imageops/sample.rs | antialiased resize available without a new dependency |
| Parmar, Zhang, Zhu, CVPR 2022 | https://arxiv.org/abs/2104.11222 | pin the resampler |
| HierText | https://arxiv.org/abs/2203.15143 | training image sizes |
| ICAO Doc 9303 | https://www.icao.int/publications/pages/publication.aspx?docnum=9303; transcription in [`../docs9303/`](../docs9303/README.md) | pitch, geometry, document codes |
| ISO/IEC 18013-3:2009/Amd 1:2012 | https://cdn.standards.iteh.ai/samples/57823/63b5d7c5a489460986c1383f3aeb797c/ISO-IEC-18013-3-2009-Amd-1-2012.pdf | one-line licence MRZ (open question 1) |
| ECMA-11, 3rd edition | https://www.ecma-international.org/wp-content/uploads/ECMA-11_3rd_edition_march_1976.pdf | pitch, stroke, size I |
| ISO 1831 / ISO 1073-2 | https://www.iso.org/standard/6480.html; distilled in [`../ocrb/`](../ocrb/README.md) | spacing, skew, glyph size |
| Hartl, Arth, Schmalstieg, VISAPP 2015 | https://doi.org/10.5220/0005294700790087 | prior art: grid fit, then rectify |
| PassportEye `mrz/image.py` | https://github.com/konstantint/PassportEye/blob/c584943d49b5942c289f9ff019c157dee728057c/passporteye/mrz/image.py | prior art: coarse to fine |
| Gayer, Ershova, Arlazarov, IJDAR 2023 | https://doi.org/10.1007/s10032-023-00435-w | prior art: re-detection in a region of interest |
| Richardson et al., arXiv 1907.12122 | https://arxiv.org/abs/1907.12122 | prior art: predict the scale, then detect at a canonical scale |
| Smith, Tesseract overview, ICDAR 2007 | https://tesseract-ocr.github.io/docs/tesseracticdar2007.pdf | fixed-pitch detection |
| SAHI, arXiv 2202.06934 | https://arxiv.org/abs/2202.06934 | tiling cost and practice |
| PaddleOCR 3.x OCR pipeline | https://www.paddleocr.ai/latest/en/version3.x/pipeline_usage/OCR.html | no one runs a detector at native 12–50 MP |
