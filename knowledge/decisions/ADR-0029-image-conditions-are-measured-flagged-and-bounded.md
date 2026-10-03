# ADR-0029 — Image conditions are measured on the normalised band, reported as levels, and remedied only within bounds

**Status:** Proposed
**Date:** 2026-10-01

## Context

Every image condition is handled today by one blind retry ladder of up to 14 passes, and the check
digits pick the read:

- blur and focus;
- contrast and illumination;
- background texture and security print;
- glare;
- skew and perspective;
- resolution;
- compression and noise.

The ladder works, but nothing in the output names the condition, so a miss explains nothing. On the
France ID card 2020 back the ladder ran 13 passes in about 50 s and read nothing (Observed, n = 1,
#563).

The engine gives no help (Observed, `ocrs` 0.13.1 and `ocrs-models` source):

- its preprocessing is a greyscale conversion and a bias, with no contrast normalisation, no
  binarisation and no denoising;
- its models were trained with colour jitter, affine and perspective augmentation, but no blur,
  noise or JPEG augmentation;
- its detection constants are in pass-image pixels.

**Pixel thresholds do not transfer across photo sizes.** That is why the published blur and
threshold constants fail on photographs ten times larger than a scan.
[ADR-0028](ADR-0028-scale-follows-the-measured-mrz-pitch.md) (#642, Proposed) has SynthPass do every
resample itself, at a scale chosen from the zone's measured pitch. On that normalised band
a threshold can be stated in the print's own units, which ICAO and ISO fix:

- the pitch, 2.54 mm;
- the capital height;
- the stroke, 0.35 mm.

**ADR-0026 already does this for one condition.** For occlusion it has:

- raw features in `synthpass-imageprep`;
- one classifier with a named threshold per kind;
- default off;
- promotion by stated criteria.

This ADR generalises that pattern.

**The same France document is also the warning** (Observed, n = 1, in a local experiment that is
not tracked; its numbers are seeds for sweeps, never thresholds):

- A Gaussian blur of 0.43–0.64 stroke widths made the zone read.
- But 5 of its 6 checksum-valid reads were wrong, all in cells no check digit covers: TD1 line 3,
  and filler cells read as letters of the filler's own residue class.

A remedy that makes a zone validate can make its unchecked cells worse.

The owner decided the eight questions of the 2026-09-30 analysis on 2026-10-01, all as recommended.
They are Decisions 6 to 12 below.

## Decision

1. **One framework for every condition, in five steps:**

   | Step | What | Lives in |
   | --- | --- | --- |
   | F0. Normalise | ADR-0028's band: found, measured, cropped from the original, deskewed, resampled once to a known pitch, otherwise untreated | `synthpass-imageprep`, `synthpass-ocr` |
   | F1. Measure | pure, wasm-clean, environment-free raw features in physical units, with integer sums where possible (as #583 did) | `synthpass-imageprep` |
   | F2. Classify | one pure function per kind: features and named thresholds in, `Ok`, `Degraded` or `Unreadable` out. The thresholds are passed in; the named constants and their sweeps live in `synthpass-ocr` | `synthpass-imageprep` (function), `synthpass-ocr` (constants) |
   | F3. Act | each `Degraded` kind maps to at most one remedy pass, sized in physical units and labelled in `PassTransform`. The blind ladder stays as the fallback | `synthpass-ocr` |
   | F4. Accept and report | the check digits decide, unchanged. ADR-0026 applies occlusion. The levels go to the trace | `mrz`, `synthpass-die`, `synthpass-core` |

2. **Physical units.** Every threshold is stated in pitch, capital-height or stroke units on the
   normalised band, never in image pixels. Two kinds are the exception: JPEG blocks and sensor
   noise live in source pixels, and are measured there.

3. **The order is binding.**
   - Flags are measured on the untreated normalised band. A remedied pixel never feeds a
     measurement.
   - Occlusion is measured first. A low-pass remedy can make a blurred redaction read as plausible
     letters, so a cell measured as occluded stays occluded whatever a later pass reads.
   - Flags are per pass. A zone stitched from several passes carries no per-cell flag (#583).
   - A flag never overrides a checksum-valid read. The one exception is ADR-0026's covered checked
     cell.

4. **The catalogue.** The kinds, what each measures, and what each may do:

   | Kind | Measured as | Bounded remedy | `Unreadable` when |
   | --- | --- | --- | --- |
   | Resolution | the source pitch, from ADR-0028's measurement | normalise (ADR-0028) | the pitch is below a swept floor |
   | Blur and focus | the 10–90% edge spread across stroke profiles, in stroke widths. Its direction separates motion from defocus | none restores it | the total spread exceeds a swept bound, per glyph class |
   | Background texture | high-frequency energy in the gaps between glyphs and above the band, and the chroma of the pixels that are not ink | the K channel, 1 − max(R, G, B), which keeps black ink and drops pale coloured print (Part 3 §4.11); a Gaussian sized in stroke widths; the existing median | never on its own |
   | Contrast and illumination | the print-contrast ratio from band percentiles, and the background ratio across the band (ISO 1831 §5.4.3) | the existing stretch; a local threshold whose window is in pitch units | contrast on check-digit cells is below a swept floor |
   | Glare | the fraction of saturated, colourless pixels per cell | none: the information is gone | Decision 10 |
   | Occlusion | ADR-0026 | none: never reconstructed | ADR-0026 |
   | Skew and perspective | a line fit through glyph centres; the pitch ratio of the two halves | the existing deskew; later, a rectification from the grid's corners | perspective exceeds a swept bound |
   | Compression and noise | the 8 × 8 block-edge energy; the noise in flat background over stroke contrast, in source pixels | the existing median | rarely; mostly explanatory |

   Binarisation stays out of the remedy list until a sweep shows it helps. Deblurring and
   super-resolution are rejected (see Alternatives).

5. **Default off until measured.** The framework runs behind `SYNTHPASS_OCR_FLAGS` = `off`, `on` or
   `control`, report-only first. Every threshold is named and swept per kind
   (`knowledge/benchmarks/README.md`, "Naming a threshold").

   **Promotion to on by default requires all of the following:**
   - zero false `Unreadable` and zero false occlusion on every clean synthetic profile;
   - zero covered-field leaks on the redaction profiles;
   - scale invariance: the same document at three scales yields the same levels;
   - a report-only real arm with zero outcome changes, and every document it flags `Unreadable`
     named in a dated note;
   - for each remedy arm: zero hit→miss, and names scored against the reviewed fixtures, not
     against `valid()`.

6. **A new framework ADR** (decided 2026-10-01, d1). ADR-0026 is its first instance and stays as
   written. The two together are the vocabulary.

7. **Levels on the wire** (d2). `ExtractionTrace` gains `image_condition: [{kind, level, scope}]`
   beside `mrz_occlusion`.
   - `level` is `degraded` or `unreadable`. A kind at `ok` is omitted.
   - `scope` names what was measured: the band, one line, or a span of cells.
   - Only enumerated values are allowed, never a float (principle 2), and the keys are locked by
     `crates/synthpass-core/tests/schema_keys.rs`.
   - Raw feature values stay in the local pass trace (ADR-0024), which never leaves the machine.

8. **"Not readable" in the trace now; a baseline bucket later** (d3). When the evidence is
   destroyed, the band's level is `unreadable`, and it is never reconstructed. Counting it as its
   own outcome in the real-specimen baseline is a later decision of its own, which is ADR-0015's
   open question.

9. **Tier 2 is barred from MRZ fields when the band is `Unreadable`** (d4), as ADR-0026 Decision 6
   bars it from a covered field.

10. **Glare is occlusion** (d6): it is reported under ADR-0026's vocabulary as kind `fill`. A glare
    on a checked cell refuses the zone after a hit, by ADR-0026 Decision 2. Before a hit, it
    explains `no_mrz_found`.

11. **Unchecked cells after a remedy** (d5). The candidate rule is:
    - keep a remedied pass's checked fields;
    - on unchecked cells, prefer the least-processed validating variant;
    - accept a remedied value there only when a less-processed variant agrees cell by cell, or when
      the line-1 selector (#574) chooses it.

    The rule is adopted only after a measurement shows it moves no field from correct to wrong.
    Until then, a remedy arm stays an arm.

12. **Later steps.**
    - Mosaic and textured redaction come after occlusion v1's promotion (d7).
    - Targeted remedies may run before the blind ladder, but only on gate evidence (d8): zero
      hit→miss and every mover named, as ADR-0008's chunk 2 did for rotation.

## Alternatives rejected

- **The blind ladder only, as today.** It is kept as the fallback. As the only mechanism it costs
  13 passes on a document it cannot read, explains no miss, and runs on pixel constants that break
  with size.
- **Gate and refuse, as capture SDKs do.** Their remedy is to take another photo, and an offline
  single-image reader cannot ask for one. Refusing on a flag would also override a valid read, which
  principle 1 forbids.
- **A learned quality model.** It produces one opaque score. Training is a permanent non-goal
  ([VISION.md](../VISION.md)), and the score would look like calibrated confidence (principle 2).
- **Deblurring or super-resolution.** They invent glyphs, and an invented glyph can validate: the
  wrong-but-valid read this ADR guards against.
- **Widen ADR-0026 instead.** That would turn an accepted decision about one condition into a
  framework for nine (decided against on 2026-10-01).

## Consequences

- **Positive.** A miss can say why. Thresholds stop depending on photo size. One targeted remedy is
  cheaper than most of a 14-pass ladder, and the browser runs the same measurements through
  `mrz-wasm`.
- **Negative.** Nothing here works before ADR-0028's normalised band exists, so this ADR depends on
  it.
- **Negative.** Each kind needs synthetic profiles in physical units, with a truth label per cell,
  before its thresholds can be swept. The generator's blur, noise and occlusion are in pixels
  today.
- **Negative.** A remedy can produce a wrong-but-valid read. Decision 11's measurement and
  fixture-scored names bound that risk; nothing removes it.
- **Neutral.** No default changes until a promotion. No baseline count moves until the separate
  bucket decision.

**Build order.** Each step is its own PR, and none changes default behaviour before a promotion
under Decision 5:

0. This ADR.
1. #565's occlusion PRs, under ADR-0026's own go/no-go.
2. `imageprep::quality` raw features, unwired: resolution, contrast, edge spread, glare, texture,
   blockiness. Tested on constructed images, with a scale-invariance test and a wasm32 build.
3. Generator degradations in millimetres, a security-print layer, and per-cell truth labels,
   sharing ADR-0028's synthetic scale axis.
4. `SYNTHPASS_OCR_FLAGS`, report-only, with the per-kind threshold sweep.
5. The K-channel remedy arm.
6. The stroke-unit Gaussian and pitch-unit local-threshold arms.
7. `ExtractionTrace.image_condition`, with the `unreadable` level (Decisions 7 and 8).
8. The Tier-2 bar for an `unreadable` band (Decision 9).
9. Targeted remedies before the blind ladder, on gate evidence (Decision 12).

**What would reverse it.** Either of two outcomes:

- the synthetic sweeps find no thresholds that keep false `Unreadable` and false occlusion at zero
  on clean profiles while the levels stay invariant across scale;
- the remedy arms cannot move a real document without moving an unchecked cell from correct to
  wrong.
