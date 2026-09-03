# Document-pipeline stage taxonomy

**Source:** the published API reference for a commercial capture-vision SDK — specifically its
`IntermediateResultUnitType` enumeration, which names every stage its document pipeline emits.
<https://www.dynamsoft.com/capture-vision/docs/server/programming/python/api-reference/core/enum-intermediate-result-unit-type.html>

**The one idea worth stealing:** documents carry printed *security texture* under their text, and
a pipeline that never models it separately from illumination will bake it into the binarized
image. We had no stage for this; we do now (`preprocess::texture_variants`).

> A note on the "Source" column of `README.md`'s index: every other note there cites a paper under
> `../papers/`. This one cites vendor API documentation instead. Recorded plainly rather than
> dressed up as something it isn't.

## Provenance

Clean-room. Derived solely from published documentation that *enumerates stage names*, plus
standard computer-vision techniques (median filtering, morphology, projection profiles, contour
and quadrilateral fitting) that long predate the SDK and are textbook material. No vendor source
was read, decompiled or copied — none is available — and no algorithm was recovered from a name.

Our naming deliberately does **not** mirror theirs: `texture_variants` and `median_filter`, not
`TEXTURE_REMOVED_BINARY`. A stage list is a checklist for gap analysis, not an API to clone.

## The surveyed taxonomy

Stripping the barcode-specific units, their document path runs:

```
COLOUR → SCALED_COLOUR → GRAYSCALE → TRANSFORMED_GRAY → ENHANCED_GRAY
  → PREDETECTED_REGIONS → BINARY
  → TEXTURE_DETECTION → TEXTURE_REMOVED_{GRAY,BINARY}
  → CONTOURS → SHORT_LINES → LINE_SEGMENTS → LONG_LINES → LOGIC_LINES
  → CORNERS → CANDIDATE_QUAD_EDGES → DETECTED_QUADS
  → DESKEWED_IMAGE / ENHANCED_IMAGE
  → TEXT_ZONES → LOCALIZED_TEXT_LINES → RAW_TEXT_LINES → RECOGNIZED_TEXT_LINES
```

Read it as a checklist, not a design. Two things in it we did not have.

## Gap analysis

### Already here

| Their stage | Ours |
| :-- | :-- |
| GRAYSCALE, TRANSFORMED/ENHANCED_GRAY | `preprocess::to_gray`, `contrast_stretched` |
| BINARY | `binarized` (global Otsu), `local_threshold` (integral-image adaptive) |
| PREDETECTED_REGIONS, TEXT_ZONES | three independent MRZ localizations: `bottom_band` (blind), `mrz_band` (row-density projection), `geometry::detect_mrz_band_scored` (content-scored line groups) |
| DESKEWED_IMAGE | `preprocess::deskew` — projection-profile variance over ±8° in 1° steps |
| LOCALIZED/RECOGNIZED_TEXT_LINES | `geometry::OcrPage.lines` — text plus per-line `BBox` |
| SCALED_COLOUR | `upscale_to_width` / `upscale_to_min_dim`, with a recognizer-dependent filter choice |

We are not behind on localization. Three geometric passes is more than "OCR the page and regex it."

### Genuinely absent

1. **Texture detection / removal.** Closed by this work — see below.
2. **Document-quad detection and homography rectification.** `CORNERS → CANDIDATE_QUAD_EDGES →
   DETECTED_QUADS → DESKEWED_IMAGE`. A grep for
   `homography|perspective|quadrilateral|contour|canny|sobel|warp|min_area_rect|hough` across every
   crate returns nothing; there is no `imageproc` and no `opencv`. Every transform we have is an
   axis-aligned crop, a resize, a quarter-turn, or a small centre rotation.
3. **Sub-degree skew.** `deskew` quantises to 1°.

### Deferred, with the reasoning

**Quad detection + perspective rectification is deferred, not rejected.** Our corpus is specimen
scans and flat stills, not angled handheld captures, so the expected yield is low against a
substantial build — edge detection, line fitting, corner grouping, quadrilateral scoring and a
four-point warp, all of which must be hand-rolled to stay inside the crate's wasm32 and
zero-dependency constraints.

It also introduces a failure mode the others do not: it *rewrites the input geometry*. Commit
`51c5a0c` is the cautionary precedent — an up-front orientation probe cost nine
previously-working documents and was removed. Any future quad work must therefore arrive as a
trailing candidate, never as a new first stage. Revisit if a live-camera capture path ships, where
angled input becomes the norm rather than the exception.

## What this note actually changed: texture suppression

**What it improves, by file and type.** New `preprocess::texture_variants` (returning one
`RgbImage`), backed by private `median_filter` and `median_radius_for_band`, chained in
`synthpass_ocr::NativeOcr::recognize_detailed` after `preprocess::geometry_band_variants`.

**Why it was missing rather than merely unbuilt.** Two helpers already *name* the pattern —
`contrast_stretched` calls itself "robust to the washed-out look of guilloche-patterned document
backgrounds", `text_bands` filters "guilloche-pattern noise" — but both address what the pattern
does to the *histogram*. A percentile stretch is a point operation and cannot remove a
spatially-varying pattern. `local_threshold` models the background with a box mean of radius
`min_dim/16`, tuned for illumination gradients and an order of magnitude coarser than a strand of
microprint. So the strands survive into the binarized image and merge with the strokes above them.

**The evidence it targets.** Among real-specimen misses, `checksum_failed` (66) outnumbers
`no_mrz_found` (51), and root-causing recorded the confusions as diffuse — `0`/`O`, `1`/`I`/`L`,
`5`/`S`, `8`/`B` scattered across many specimens with no single shared shape — with no follow-up
fix identified. That is the signature a textured background produces and a geometric fault does
not.

**Why a median and not the sharper instrument.** A morphological *closing* is the correct polarity
for thin dark strands on a light substrate (dilation takes the local max, so closing is what
removes dark detail; opening would attack the substrate and thicken ink). But it is only selective
while `strand < kernel < stroke`, and mis-sized by one pixel it erases stroke terminals and thins
`1` into nothing — manufacturing the exact confusion class we are trying to reduce. A median
deletes any structure occupying less than half its window and leaves wider structures and straight
edges alone, so it **fails safe**: if the texture is wider than the kernel the output is close to
the input and the pass is merely redundant. Closing stays documented as the follow-up if the median
under-delivers.

It composes with rather than duplicates `local_threshold`: median removes high-frequency structure
*before* thresholding, adaptive thresholding removes low-frequency illumination — disjoint parts of
the spectrum.

**Cost.** Zero new dependencies; hand-rolled on `image` primitives, and
`cargo check -p synthpass-imageprep --target wasm32-unknown-unknown` still passes. The kernel is
capped at radius 3, so the window holds at most 49 values and the naive form (rather than a Huang
sliding histogram) costs tens of milliseconds. It runs only on the failure path — the retry loop
breaks on the first checksum-valid MRZ and this variant is chained last — so the marginal cost is
one OCR pass per *failing* document.

**Budget, which is the part that bites.** `MAX_RETRY_VARIANTS` moved 8 → 9 and therefore
`DEFAULT_MAX_PASSES` 9 → 10. The retry loop seeds its counter at 1 and breaks on
`passes_run >= max_passes`, so without that bump the new variant is silently truncated on exactly
the documents it exists for and the measurement reads a confident zero.
`max_passes_reaches_the_last_worst_case_variant` catches it — verified: it failed with `left: 8,
right: 9` before the fixture was extended. The **wall-clock** half is not guarded by any test:
`SYNTHPASS_OCR_MAX_SECONDS` defaults to 45s against a measured worst case of ~30s for a single
specimen, and a ninth variant moves slow documents toward that ceiling.

## The structural point worth recording

A commercial pipeline must **commit** to one path at each stage, because it has no way to know
which was right. We have the ICAO check digit: a free, exact correctness oracle. That lets us run
N speculative treatments and let the checksum arbitrate — which is why the trailing-variant
discipline works here and is simply unavailable to them, and why "they have a stage we lack" does
not straightforwardly mean "they are ahead."

It is also the general form of `51c5a0c`'s lesson:

> Detection that can be wrong must never rewrite the input — but it may always *append a
> candidate*.

Every retry variant in `preprocess.rs` is an instance of that rule. It is the reason a new
preprocessing idea can be shipped here without risking the specimens that already pass.

## Measured, 2026-09-03: null on the Tier-1 fixture corpus

Full numbers in
[`../benchmarks/texture-suppression-ab-2026-09-03.md`](../benchmarks/texture-suppression-ab-2026-09-03.md).
Three arms (`off` / `control` / `on`) from one release binary over
`examples/mrz_corpus.rs`: **18/20 in all three**, zero false positives across 42 negative controls
in all three, and zero per-document flips in either direction between any pair of arms.

The safety properties all held, and two open questions closed with data:

- **The pass really executes.** `control` cost +67.3s over `off` and `on` cost +114.2s. A variant
  truncated by the budget cannot cost 114 seconds — cheaper and stronger evidence than a verbose
  log.
- **`SYNTHPASS_OCR_MAX_SECONDS` did not need raising.** Exactly one document reaches the 45s
  ceiling, a *negative* control already at 64.7s in the baseline; every positive sits at ~9-11s
  median.

**But the null says nothing about the hypothesis**, and the reason is worth being precise about:
this stage targets `checksum_failed` (MRZ found, then misread), and the corpus's only two misses
are `no MRZ found in text` — the other failure mode, on two already-known stale entries. The 18
passing positives validate on early variants and never reach variant 9. So the corpus contains
**zero instances of the failure class the treatment addresses**; no outcome it produced could have
confirmed or refuted the idea. The test is the real-specimen `provider-bench` corpus, where the 66
`checksum_failed` documents actually reach the trailing variants.

The stage therefore ships **default-off** and costs nothing until that measurement runs.

## Why this might not work

Kept per this directory's own rule — most ideas won't, and the dead end is worth recording.

- **The hypothesis may simply be wrong.** Diffuse `0`/`O`, `1`/`I` confusions are equally
  consistent with generic recognizer weakness on OCR-B at marginal resolution, with JPEG ringing,
  or with the Lanczos3 edge-ringing that `UpscaleFilter`'s own doc comment already documents as
  costing character accuracy. Texture is *a* physically-motivated explanation, not the only one the
  evidence supports.
- **The kernel may fall outside the window that matters.** Texture wider than 3px after upscaling
  is untouched by a clamped kernel; sub-pixel texture was already averaged away by the scanner, and
  there is nothing left to remove.
- **`local_threshold` may already capture most of the available gain.**
- **The remaining wins may be taken.** This runs ninth. Those 66 failures already survived eight
  treatments including two thresholdings, so the surviving population is enriched for glare,
  physical damage, genuinely absent MRZ, and ground-truth errors — none of which a filter reaches.
- **Ordering is untested in both directions.** Upscaling interpolates, smearing a 1px strand into a
  wider soft feature a small median may no longer catch. Applying the filter at native resolution
  *before* `upscale_to_width` would keep the physical relationship intact. The shipped order
  filters after upscaling; the alternative is unmeasured.
- **The corpus may be too small to resolve the effect** — except that this pipeline is
  deterministic, which is exactly why the three-arm same-binary design earns its cost: a
  two-document delta is readable as a real per-document flip list rather than a rate that moved.

A null result belongs in this section rather than in a deleted branch.
