# ADR-0015 — `mrz-locate`: find the MRZ band by geometry, before recognition

**Status:** Proposed
**Date:** 2026-09-18

## Context

**Band detection in this repository is downstream of recognition.** `mrz_line_score`
(`crates/synthpass-imageprep/src/geometry.rs`) takes an `OcrLine`, whose fields are
`text: String`, `bbox` and `confidence`, and scores it by the density of `MRZ_CHARSET`
characters in that text, weighted by line length. `detect_mrz_band` picks the best-scoring
contiguous group. So the pipeline recognises every line on the page first, then asks which of
the resulting *strings* look like an MRZ.

That is circular. If recognition returns garbage over the band — because the print is degraded,
the background is cluttered, or a watermark crosses it — the score is low and the band is never
found. Nothing reports "there is clearly a fixed-pitch band here that I could not read"; the
document simply becomes `no_mrz_found`.

Three facts make this concrete rather than theoretical.

**1. The detection model's own output has never been consulted.** `ocrs::OcrEngine::detect_text_pixels`
is public and returns the raw `[H, W]` text-probability map, before word grouping, before line
finding, before recognition. A search of the workspace returns **zero call sites**. The pipeline
consumes `detect_words` and `find_text_lines` and discards the map they were derived from.

**2. France ID 2020 back is the failure in one document.** In `web-ocr.yml` run `35169813105`
(2026-09-17), over the same bytes: the native arm recorded `retry_stop: budget`,
`retry_budget_hit: true`, **72 227 ms**, no MRZ found; the browser arm (`tesseract.js`) returned a
checksum-valid read on **pass 3, a contrast stretch, in 1 305 ms**. It is the only budget-exhausting
document in the corpus. A later transcription confirmed the zone is a conforming TD1 whose four check
digits all validate. The band was always there and always readable — 55× the time bought nothing,
because more variants only help if one of them makes the *recogniser* succeed, which is the only
route by which this pipeline can conclude that a band exists.

**3. Recognition improvements cannot close the remaining detection misses.** The three detection
failures frozen in [ADR-0011](ADR-0011-split-m6-packaging-into-m8.md)'s 2026-09-17 amendment were
attributed on 2026-09-18 ([`detection-misses-2026-09-18.md`](../benchmarks/detection-misses-2026-09-18.md)):
France (conforming zone; a preprocessing gap because the native variant chain lacks the
contrast-stretch transform), Italy (conforming zone, FACSIMILE watermark crossing
the band), Moldova (no machine-readable zone exists at all — the printed lines are 57 characters,
longer than any ICAO format). A perfect classifier closes none of them. This is the complement to
[ADR-0014](ADR-0014-per-cell-ocrb-classification.md), which addresses the documents where the band
*is* found and misread.

[ADR-0008](ADR-0008-mrz-detection-track.md) recorded that the detection track's premise was spent
once `no_mrz_found` fell level with `checksum_failed`. This ADR proposes what replaces it.

### What an MRZ band looks like without reading it

The band is unusually well specified as a *shape*, before any character is known:

- two or three long horizontal bars of near-equal height, vertically adjacent at a known spacing;
- a fixed pitch — 30, 36 or 44 equal-width cells across the band;
- periodic low-ink gaps where filler runs sit, which every real MRZ contains and which give the
  pitch a strong, self-checking signature;
- a known aspect ratio per format, and a conventional position on the document.

Dense micro-text — France's background is the Declaration of the Rights of Man — produces a strong
text-probability response everywhere, but it is **not** fixed-pitch in 44 equal cells at an MRZ's
aspect. The discriminator is the geometry, not the presence of ink. That is precisely why a
recognition-free detector can succeed where recognise-then-score fails.

The repository already computes this periodicity: `synthpass_ocr::chargrid`'s `fit_grid`,
`grid_origin_from_ink` and `cell_ink` fit a pitch and phase from an ink profile. They are used only
*after* the crop, on a band that has already been found.

## Decision (proposed)

Prototype **`mrz-locate`**, a geometric band locator that consumes the detection probability map and
locates the band without recognising anything, behind a cargo feature and a three-arm environment
variable. Nothing enters the default path before the go/no-go below is met.

1. **Scope.** Locating and orienting the band on a page. Not recognition, not a replacement for
   `ocrs`'s detection model — this consumes that model's existing output, which the pipeline
   currently throws away.
2. **No new dependency and no new model.** `detect_text_pixels` is already public on the `ocrs`
   version we pin, and `chargrid` is already in the tree. Nothing is trained; a published geometry
   is matched against a measured one.
3. **Method.**
   - Take the `[H, W]` probability map from `detect_text_pixels`.
   - Find candidate horizontal bar structures by row-wise response, filtered to near-equal height
     and plausible aspect.
   - Fit pitch and phase per candidate with `chargrid`'s existing ink-profile machinery, and score
     how well a fixed pitch of 30 / 36 / 44 cells explains it.
   - Prefer candidates whose gaps show the periodic filler signature, which is the self-check: a
     correct fit makes filler runs land on cell boundaries.
   - Return the band, the format hypothesis, the orientation, and the grid — so the crop that
     follows arrives with its pitch already measured rather than re-derived.
4. **Arms.** Cargo feature `mrz-locate`, off by default, and `SYNTHPASS_OCR_MRZ_LOCATE=off|on|control`,
   matching the pattern `SYNTHPASS_OCR_CHARGRID` established in #334. `control` does the work and
   discards the result, so the A/B separates the treatment from its cost.
5. **Byproduct, deliberately sought:** a document that has a band we cannot read becomes
   distinguishable from one that has no band. Today both are `no_mrz_found`. That distinction is
   worth more than it sounds — it is the difference between an accuracy target and an
   off-denominator document, and the corpus currently resolves it only by hand.

## Go / no-go

Measured on the real corpus and on a fixed-seed synthetic set, as a same-binary A/B. All must hold:

1. **Closes at least two of the three frozen detection misses** — the band is located on France and
   Italy. Moldova cannot be closed by any locator, because there is no band; a locator that claims
   one there is a false positive and fails outright.
2. **No Tier-1 loss**, document for document, against the committed baseline.
3. **No false bands.** Zero new `false_positive_mrz` across the refusal population. A locator that
   invents bands is worse than one that misses them, and this threshold is not negotiable against
   a gain elsewhere.
4. **Latency:** p50 added ≤ 100 ms per document. It should be cheaper than what it replaces, because
   it removes recognition passes spent on scoring candidate lines — establish whether
   `detect_text_pixels` reuses `detect_words`'s computation or costs a second pass before claiming it.
5. **Determinism:** two runs over the same corpus produce byte-identical reports.

**A no-go sets this ADR to Rejected and keeps the numbers.** Measuring that geometry cannot locate
the band would say the residue is in the detection *model* rather than in how we consume it, which
is a different and more expensive problem — and worth knowing before anyone pays for it.

## Alternatives considered

- **More retry variants (the status quo).** Rejected on measurement: in run `35169813105`, **13 of
  the 14 scored misses exhausted the pass list without approaching the 52-second clock**. The
  budget is not the constraint; the variant set is. More time buys nothing.
- **Add a contrast-stretch variant to the existing chain.** Cheap, and the browser evidence says it
  would likely close France on its own. Worth doing regardless — but it is a patch on the circular
  design, not a fix for it: the next document defeated by a different transform fails the same way.
  Recommended as a separate small change, not as the answer here.
- **Corner-chevron matched filter** (the ICAO geometry prior plus a matched filter for the periodic
  `<<<<` run in the four corner strips, proposed 2026-09-17). Same family as this proposal and
  probably the same implementation: the filler-run periodicity it searches for is the signature this
  ADR's step 3 relies on. Treat as an arm of this ADR rather than a competing one.
- **Train a band detector.** Rejected: [`VISION.md`](../VISION.md) §2 rules out custom-trained
  models, and a published fixed-pitch geometry does not need one.

## Consequences

- Detection stops depending on recognition succeeding, which removes a failure mode that is
  currently invisible in the metrics — a readable band that our recogniser cannot read is reported
  identically to no band at all.
- The crop handed to recognition arrives with a measured pitch and phase. That is exactly what
  `chargrid` and any [ADR-0014](ADR-0014-per-cell-ocrb-classification.md) arm need, and the
  2026-09-18 A/B traced 13 synthetic regressions to grid-origin error — so a locator that supplies a
  verified origin may fix a defect in work that already exists.
- One more feature-gated module, reversible by deleting it, with no new dependency.
- If it does not work, we learn whether the residue is the detection model or our use of it.

## Open questions

- Whether `detect_text_pixels` reuses `detect_words`'s forward pass or costs a second one. This
  decides whether the locator is cheaper or more expensive than what it replaces, and it is the
  first thing to measure.
- Whether the locator should return a *refusal* — "a band-shaped region exists that I could not
  read" — as a distinct outcome, which would change the corpus taxonomy and the baseline's bucket
  set. Probably yes, but it is a metric change and belongs in its own decision.
- Whether the same geometry can decide orientation outright, replacing `choose_rotation`'s four
  detection passes, which [`adr-0008-1c-attribution`](ADR-0008-mrz-detection-track.md) identified as
  the cause of an earlier failure class.

## Sequencing

After the frozen M6 list is evaluated, since this changes detection and
[ADR-0011](ADR-0011-split-m6-packaging-into-m8.md)'s amendment keeps M6 scored on the v1.5.0
snapshot. It is not an OCR engine replacement, a vision provider or a new dependency, so the M6
prohibition in [`ROADMAP.md`](../ROADMAP.md) does not bar it — but a detection change measured
against a frozen detection list needs the list settled first.

Independent of that: the contrast-stretch variant above is small, testable now, and should not wait
for this ADR.

## Amendment (2026-09-24) — filler-run geometry is evidence, not a guarantee

[`ocrb-filler-geometry-2026-09-23.md` §5](../benchmarks/ocrb-filler-geometry-2026-09-23.md#5-band-geometry-standard-real-and-synthetic)
measured the priors this ADR's Decision and "What an MRZ band looks like" section assume, on 16
real TD3 crops, and several do not hold as tightly as written here:

- **"Vertically adjacent at a known spacing"** does not hold as a single number: line pitch
  ranges **1.65-2.99 cap** across the 16 specimens (nominal 2.58 cap), and TD1's nominal spacing
  (1.72 cap) is not the same prior as TD3/TD2's. A locator needs a band of spacings per format,
  not one constant.
- **"A correct fit makes filler runs land on cell boundaries"** is contradicted by the same
  measurement: filler glyphs are *centred* in their cells, not aligned to boundaries, and a
  filler cell still carries 0.6-0.7× a letter cell's ink — a low-ink cell, not a gap. Whether a
  given cell reads as "filler" depends on the OCR line-box height used to sample it, which that
  note flags as still unmeasured.
- **"Two or three long horizontal bars of near-equal height"** holds only loosely: line 2 prints
  5-10% taller than line 1 (it carries digits).

None of this changes the Decision above — it is a design note pending `mrz-locate`'s existence,
per that benchmark's own framing ("flagged for the architect, not changed here"). The broader
point is principle 1's: a periodic filler-run signature is *evidence* a candidate grid fit is
correct, not a guarantee. A name that fills its entire line (Doc 9303 Part 4 allows this) prints
no filler run at all, so a locator that requires one to accept a fit will refuse a valid MRZ.
