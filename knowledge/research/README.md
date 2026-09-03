# research/

Distilled, actionable summaries drawn from `../papers/`.

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
| [document-pipeline-stage-taxonomy.md](document-pipeline-stage-taxonomy.md) | A commercial capture-vision SDK's published stage enumeration (vendor API docs, not a paper) | Documents carry printed *security texture* under their text, and a pipeline that never models it separately from illumination bakes it into the binarized image. We had no such stage; `preprocess::texture_variants` is it. Also records why document-quad/homography rectification is deferred, and the structural reason we can speculate where a commercial pipeline must commit: the ICAO check digit is a free correctness oracle. |
