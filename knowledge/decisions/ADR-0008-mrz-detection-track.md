# ADR-0008 — MRZ detection succeeds sequence completeness as M6's accuracy track

**Status:** Accepted
**Date:** 2026-09-09

## Context

[`ADR-0006`](ADR-0006-m6-accuracy-first.md) reframed M6 to lead with Tier-1 real-document
accuracy, and named the track: MRZ sequence completeness, scoped in
[`MRZ_SEQUENCE_COMPLETENESS.md`](../MRZ_SEQUENCE_COMPLETENESS.md). **That track is now closed** —
chunks 1–6 shipped, chunk 7 was measured and rejected, and the document's own banner says there is
nothing left to implement. M6's stated first item is finished, and nothing names what comes next.

Closing it changed the shape of the problem, which is the reason this ADR exists rather than a
line edit to the roadmap.

**The dominant miss inverted.** `ADR-0006` justified its priority with *"real-specimen
benchmarking shows `checksum_failed` (66) as the single largest miss category."* That was true
when written. It is no longer. The CI-written baseline
([`real-specimen-mrz-baseline.json`](../benchmarks/real-specimen-mrz-baseline.json), 2026-09-09):

| Outcome | Count | Share of 229 scored |
| --- | --- | --- |
| Tier-1 HIT | 119 | **52.0%** |
| `no_mrz_found` | **85** | **37.1%** |
| `checksum_failed` | 24 | 10.5% |
| `checksum_failed_specimen` | 1 | 0.4% |
| `redacted_mrz` (off-denominator) | 9 | — |

`no_mrz_found` now outnumbers `checksum_failed` **3.5:1**. This is not a regression — it is the
completed track's own result. `mrz` 0.7.0 began rejecting structurally implausible readings, and
roughly 38 documents that had been counted as `checksum_failed` were reclassified to what they had
always been: documents where no MRZ was ever found. Tier-1 HIT did not move (118 → 118 at that
step). We were not failing to *validate* those MRZs; we were failing to *find* them, and a
mis-categorised counter had been hiding it.

**The residual `checksum_failed` is no longer a parser problem.** Of the 24 that remain, the 23
hand-transcribed specimens split 16 non-conforming-by-design and 7 carrying a checksum-valid
printed MRZ. All 7 were individually attributed to native OCR on low-resolution scans — line-1
filler collapse with no checksum oracle, unreadable line 2, diffuse noise — and explicitly *not*
to `crates/mrz`. Further work inside the parser has no measurable target left.

**There is an unexplained result sitting in our own docs.** [`WEB_OCR_BASELINE.md`](../WEB_OCR_BASELINE.md)
records the browser demo's stack (tesseract.js + an OCR-B-trained model) reading **122/190 =
64.2%** against the native `ocrs`/`rten` pipeline's **113/190 = 59.5%** on the same corpus. The
throwaway demo out-reads the product. That has been recorded and never explained, and it is the
single most promising lead on `no_mrz_found` that exists — it suggests the gap is reachable
without new research.

## Decision

**MRZ detection and localization becomes M6's accuracy track**, succeeding sequence completeness.
The target metric is `no_mrz_found` (85 of 229); the constraint is that Tier-1 HIT must not
regress, enforced by the gate that already exists.

**The first chunk is a measurement, not a construction.** Before any detector is written, tuned or
replaced, establish *why* tesseract.js beats `ocrs`/`rten` on the same documents. The two stacks
differ in at least four ways at once — recognizer model, preprocessing path, the retry/variant
budget, and the corpora the two numbers were taken on are not identical. Attributing the gap
requires holding those constant one at a time, on one corpus, in one harness. Only the
same-binary A/B discipline counts: toggle by environment variable within one build against a
control variable, never a rebuild-based before/after.

That measurement decides the track's shape, and no commitment is made past it here. The plausible
outcomes are already visible — a preprocessing gap, a recognizer gap, or a band-localization gap —
and each implies different work. Naming the fix now, before the measurement, is exactly the move
[`project_principles.md`](../project_principles.md) P6 forbids.

## Alternatives rejected

**Move to packaging and the Pro beta instead.** M6's remaining items are layout plugins, dataset
exports, the air-gapped guide and a commercial beta, and they are genuinely blocked on nothing.
Rejected as premature: the deterministic Tier-1 core is what the product is sold on
([`product positioning`](../ROADMAP.md#current-state)), and packaging a reader that finds no MRZ
at all on 37% of real specimens sells the wrong thing at the wrong time. The packaging work is
also largely independent of extraction code, so it costs nothing to leave it sequenced behind.

**Throw a larger Tier-2 model at the residual.** Rejected on two grounds. P1 — deterministic
before probabilistic — is the constitution, and this would invert it. More decisively, it would
not work: `no_mrz_found` means the pipeline has no MRZ to hand Tier 2 either. A bigger model
cannot repair a string that was never extracted.

**Replace `ocrs`/`rten` with tesseract wholesale.** Tempting, given 64.2% vs 59.5%, and rejected
as a rewrite justified by a single uncontrolled comparison across two pipelines and two corpora.
It would also import a C++ dependency into a pure-Rust, wasm-clean pipeline — a cost
[`VISION.md`](../VISION.md) makes us argue for in writing. If the measurement shows the recognizer
genuinely is the gap, that argument can be made then, with a number behind it.

**Grow the corpus.** 158 of 238 country codes still have no specimen
([`CORPUS_COVERAGE.md`](../CORPUS_COVERAGE.md)). Worth doing, and it is not this. Adding documents
changes what the rate is measured over; it does not change the rate. Coverage and accuracy are
separate problems and conflating them would make both unmeasurable.

## Consequences

**Positive**

- M6's accuracy track points at the largest measured miss again, which is the property
  `ADR-0006` was trying to establish and which decayed the moment its premise changed.
- The target is defended before it is attacked: `real-specimen-gate.yml` and the committed
  baseline already exist, so any detection change is scored against a fixed denominator from
  the first commit — the completeness track had to build that mid-flight.
- It gives the `WEB_OCR_BASELINE.md` anomaly an owner. It has been sitting as a curiosity
  through two releases.

**Negative**

- The enterprise/packaging half of M6 is deferred again, a second time, by a second ADR.
  Accepted, but it should not become a habit: if accuracy work displaces packaging a third
  time, that is evidence M6 should have been split, and `ADR-0006`'s rejected M8 option
  should be revisited rather than re-rejected.
- The first chunk produces a measurement and possibly no code, which will look like a slow
  milestone from the outside. That is the correct trade against redesigning a detector on a
  hunch.

**Explicitly not licensed by this decision:** replacing the OCR engine, adding a vision
provider, or any new dependency. This ADR names a target and mandates a measurement. What the
measurement justifies is a later decision, argued on its own evidence.
