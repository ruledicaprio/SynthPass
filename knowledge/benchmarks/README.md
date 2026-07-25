# benchmarks/

Methodology, metric definitions, and dated results — **including the candidates
that were rejected.**

Principle 6: a constant with no measurement behind it does not ship.

## What belongs here

- **Methodology** — how a run is configured so two runs are comparable. The
  corpus runner is deliberately single-threaded; a parallelised version measured
  38% where the honest sequential number was 55–56%, because oversubscribing
  `rten`'s internal threads corrupts the OCR retry budget. That is a measurement
  invariant, not a style preference.
- **Metric definitions** — precisely what each number counts. "Hit rate" means
  *the MRZ provider produced a checksum-valid record whose document number matches
  ground truth*, not "the extraction was correct."
- **Dated sweeps** — `routing-sweep-YYYY-MM-DD.md`, `provider-comparison-*.md`.
  Name the exact invocation that produced them.

## Record the rejections

The most valuable entries are the signals that looked promising and did not
survive contact with the corpus. Precedent in the codebase:
`synthpass-core/src/fusion.rs` documents a name-reconstruction integrity check
that was measured, false-positived on genuine specimens, and was thrown away —
with the reason. Without that note, someone re-proposes it every year.

## Naming a threshold

When a sweep produces a constant, the constant's doc comment names this file and
the invocation. See `MRZ_BAND_CONFIDENT_SCORE` in
`crates/synthpass-ocr/src/geometry.rs` for the pattern.
