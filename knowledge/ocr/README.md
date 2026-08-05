# ocr/

Recognition, preprocessing, layout analysis, confidence scoring — the Tier-1
input path.

## What belongs here

- **Preprocessing** — deskew, local thresholding, MRZ-band isolation, and which
  passes are proven vs. additive. The retry loop is budgeted
  (`SYNTHPASS_OCR_MAX_PASSES`, `SYNTHPASS_OCR_MAX_SECONDS`) so a hopeless scan
  degrades instead of burning minutes.
- **Confidence scoring** — and its honest limits. `ocrs` exposes no per-character
  probability: `TextChar`/`TextLine` carry only `char` and `rect`, and the CTC
  decode probabilities are computed internally and never returned. What we call
  per-line confidence is `text_sanity`, a *character-plausibility proxy*. Notes
  here should not let that distinction blur; see `technical_debt.md`.
- **Layout analysis** — MRZ band detection, portrait detection, orientation.
- **Open-set recognition** — distilled from `papers/`: what the OSR literature
  offers a document pipeline that must reject the unknown rather than
  force-classify it.
- **Recognition-engine candidates** — what would replace or sit beside
  `ocrs`/`rten`, and on what measured evidence. The live proposal is PaddleOCR's
  PP-OCRv5 converted ONNX → `.rten` (no onnxruntime, so no C++ dependency),
  wanted less for accuracy than because decoding CTC ourselves is what turns the
  `text_sanity` proxy above into a real per-character score. See
  [`../research/long-horizon-parsing.md`](../research/long-horizon-parsing.md) §2
  — and note it is a bake-off first, not a swap.

## The question every note answers

Does this become a trait, a struct, an algorithm, a pipeline stage, a benchmark,
a configuration knob, or a heuristic? If none of those, it does not get kept.
