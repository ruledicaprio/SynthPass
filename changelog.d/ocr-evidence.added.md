- **`OcrPage` and the pipeline's `OcrResult` now carry the two signals the OCR engine already
  computed and then threw away**: `mrz_band_score` (how MRZ-shaped the winning band looked, in
  `[0, 1]`) and `text_sanity` (whole-page character plausibility).

  `detect_mrz_band_scored` has always returned `(BBox, f64)`, and the caller discarded the score
  with `.map(|(b, _)| b)` — so *where* the band was survived and *how convincing it was* did not.
  Those are different questions, and the second is the one an escalation decision needs: a band can
  be found and still be noise, which is precisely the failure the 0°/180° tie-break exists to
  handle. Per-line sanity likewise existed inside `OcrPage.lines` but never reached the pipeline,
  whose `OcrResult` dropped `lines` entirely.

  Both are `Option`, and `None` is kept distinct from `Some(0.0)` on purpose: "nobody scored this"
  and "this scored zero" are different facts, and an escalation policy acts differently on a blank
  scan than on a ruined one. `text_sanity` remains a **character-plausibility proxy, not a model
  confidence** — `ocrs` exposes no probabilities at all — and the doc comments say so at every
  level rather than letting the name imply otherwise.

  Additive: `OcrResult::from_text` leaves both unset, so `OcrEngine::recognize_detailed`'s default
  body keeps every engine that implements only `to_markdown` compiling and behaving identically.
  A new test asserts exactly that against a deliberately minimal in-tree engine, since no
  out-of-tree implementation is available to notice on our behalf.

  No behaviour change: nothing consumes the new fields yet.
