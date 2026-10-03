# research/

Distilled, actionable notes. Most summarise a paper in `../papers/` or a tool's or vendor's
documentation; a design note may instead start from this project's own measurements, as
[mrz-geometric-elimination.md](mrz-geometric-elimination.md) does.

## The 80/20 filter

Reading a paper, ask one question: **can this become a trait, struct, algorithm,
pipeline stage, benchmark, configuration knob, or heuristic?** If not, it does not
get a note.

The value is extracting *design principles*, not *definitions*. A summary that
restates the abstract is worse than nothing — it costs context and teaches
nothing. Twenty pages of actionable extract beats four hundred and fifty pages of
source.

## What a research note contains

- The one idea worth stealing, stated in terms of *this* pipeline.
- What it would replace or improve, named by file and type.
- What it would cost — dependencies, latency, memory, complexity.
- Why it might not work here. Most won't; say so and keep the note anyway, so the
  next person doesn't re-derive the same dead end.

## Current source material

`../papers/` holds seven papers: six on open-set recognition and the related
divide-and-conquer decomposition, and one on end-to-end OCR.

OSR is relevant because a document pipeline must be able to say *"I don't know"*
rather than force a read into the nearest known class — the same posture as
`fusion::Verdict::NeedsReview`. Of those six, the closest to actionable are the
unified-OSR uncertainty-fusion result and the gradient-based unknown detector.
**Neither has been distilled yet.**

## Notes

| Note | Source | The one idea |
|---|---|---|
| [long-horizon-parsing.md](long-horizon-parsing.md) | `../papers/Unlimited_OCR_Works.md` | The paper's headline (dozens of pages in one forward pass) is irrelevant to a one-page passport. Its *data engine* is not: it describes the training-corpus format a `synthpass-gen` export should adopt. Carries the PaddleOCR-via-`rten` proposal and the case for Unlimited-OCR as a vision-provider candidate. |
| [mrz-geometric-elimination.md](mrz-geometric-elimination.md) | The vendored OCR-B's outlines, the MRZ repair tables in `crates/mrz`, ECMA-11 and ISO 1073-2 figures, and 16 public TD3 bands | Read the band by elimination rather than recognition. The fixed grid, the closed alphabet, glyph height and ink, and the check digits rule candidates out, and where two remain the ambiguity is localised instead of guessed. Font geometry is observed; its effect on real recognition is still a measurement plan. **Measured since, against parts of it:** the real confusion matrix is a filler problem and a zero problem ([2026-09-21](../benchmarks/observed-ocrb-confusions-2026-09-21.md)); the vendored filler sits about 0.025 cap lower than ISO 1073-2 and real print place it ([2026-09-23](../benchmarks/ocrb-filler-geometry-2026-09-23.md), [2026-09-24](../benchmarks/ocrb-standards-vs-priors-2026-09-24.md)); both OCR engines do emit an isolated `<` ([2026-09-29](../benchmarks/ocr-filler-unknown-trace-2026-09-29.md)), yet on clean synthetic renders the filler after `P` reads as `S` in 96 of 100 ([2026-09-30](../benchmarks/glyph-atlas-p-filler-2026-09-30.md)); and a per-cell filler-or-letter classifier on real names is a no-go, 17 / 45 strict names against a 20 / 45 gate, with no correct field broken ([2026-10-01](../benchmarks/filler-or-letter-2026-10-01.md)); and the probe's items 1 and 2 are measured on the 46 public specimens it locates: the top edge orders digits above letters in distribution but by a sub-pixel gap at the corpus's resolution, and the left quarter separates every undisputed `K` from every filler down to 9 px per cell ([2026-10-03](../benchmarks/elimination-probe-items-1-2-2026-10-03.md)). **What the work builds on:** the per-document archive ([ADR-0024](../decisions/ADR-0024-per-document-benchmark-archive.md)), whose [`archive_query.py`](../../tools/archive_query.py) `cell` counts the classes read at one zone cell across a run without a new OCR pass, and [`promotion_gate.py`](../../tools/promotion_gate.py) ([#664](https://github.com/ruledicaprio/SynthPass/issues/664)), which a per-cell treatment must pass before it becomes a default; `mrz::position_class`, the layout's character class at one cell ([ADR-0014](../decisions/ADR-0014-per-cell-ocrb-classification.md)'s 2026-10-01 amendment); and the bench-only [`elimination_probe`](../../crates/synthpass-bench/examples/elimination_probe.rs), which measures the note's items 1 and 2 on the public real bands, and [`elimination_offsets.py`](../../tools/elimination_offsets.py), which scores item 3 from its output. |
| [document-pipeline-stage-taxonomy.md](document-pipeline-stage-taxonomy.md) | A commercial capture-vision SDK's published stage enumeration (vendor API docs, not a paper) | Documents carry printed *security texture* under their text, and a pipeline that never models it separately from illumination bakes it into the binarized image. We had no such stage; `preprocess::texture_variants` is it. Also records why document-quad/homography rectification is deferred, and the structural reason we can speculate where a commercial pipeline must commit: the ICAO check digit is a free correctness oracle. |
| [Tesseract_OCR_studies.md](Tesseract_OCR_studies.md) | Tesseract/tessdoc documentation and the `tessdata_ocrb` MRZ traineddata project | Do not run the [`ADR-0008`](../decisions/ADR-0008-mrz-detection-track.md) comparison as "Tesseract vs `ocrs`/`rten`". Tesseract's result is the product of segmentation, Leptonica preprocessing, model family, character whitelisting and retry budget, and its apparent MRZ advantage may be a *localization* advantage rather than a recognition one. Proposes a five-level factorial to attribute it. Two of its four confounders turned out to be controlled already — `synthpass-imageprep` compiles to wasm so both stacks run the same preprocessing, and `MRZ_CHARSET` is applied on both sides — which is what shrank the experiment to the three cells the track now runs. |
