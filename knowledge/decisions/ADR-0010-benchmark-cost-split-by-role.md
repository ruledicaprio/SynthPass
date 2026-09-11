# ADR-0010 — Split the real-specimen benchmark by role, not by random sample

**Status:** Proposed. Records where the benchmark's cost actually is, and why the obvious
speed-up — sampling the corpus — cannot coexist with the gate's tolerance of zero.
**Date:** 2026-09-11

## Context

The real-specimen gate has become the slowest thing on a pull request: **39m29s** on
[#262](https://github.com/ruledicaprio/SynthPass/pull/262) (`Real-specimen Tier-1 no-regression
gate`), and ~25–35 minutes per arm locally. Note that
[`real-specimen-gate.yml`](../../.github/workflows/real-specimen-gate.yml)'s own header still
describes it as "the ~1-minute deterministic pass". That comment is stale by a factor of forty,
and it has already caused one wrong estimate to be repeated as fact in review.

A proposal followed: sample **one image per country and document type** each run, rotating which
one, including the private specimens, and make the baseline tolerance mandatory and zero.

Before redesigning the corpus, the cost was measured — over
`artifacts/ab-rotate-default.json`, the full 254-specimen run:

| | documents | shape of the cost |
| --- | --: | --- |
| Yields a checksum-valid MRZ | 144 | retry loop **breaks early** — one or two variants |
| Yields none | **110** | runs the **entire** retry chain — ~12 variants |
| …and is **scored out of the hit rate** | **95** | fully OCR'd, then discarded from the metric |

The 95 are 43 `no_mrz_expected`, 35 `redacted_mrz` and 17 `checksum_failed_specimen`. They never
produce a valid MRZ, so they never break early: they pay the **worst-case** cost on every run to
produce a number the headline rate excludes by construction.

Two denominators are in play here and they are not the same number, so both are stated once:
the corpus splits **157 scored + 97 off-denominator = 254**, and separately **95 + 15 = 110**
never yield a valid MRZ. The off-denominator set is 97; the 95 above is the subset of it that
never reads a valid MRZ and therefore pays the full chain. The remaining 2 are off-denominator
documents that *do* read a checksum-valid MRZ — see the canary note under Consequences. The
committed baseline reconciles both ways: 142 hits + 15 scored misses = 157, and 142 + 2 = 144
checksum-valid.

**Cost tracks failures, not document count.** That single fact decides the shape of the fix, and
it is what makes sampling a poor lever: a uniform sample removes cheap and expensive documents in
proportion, so halving the corpus roughly halves the time, while removing the off-denominator set
removes the part that dominates.

Modelling variant-runs as `cheap × ~1.5 + expensive × 12`:

```
today                 144×1.5 + 110×12  ≈ 1536 variant-runs
without the 95        144×1.5 +  15×12  ≈  396 variant-runs   (-74%)
uniform half-sample    72×1.5 +  55×12  ≈  768 variant-runs   (-50%, and see below)
```

**This is a model, not a measurement, and that is itself a finding.** The bench report's `speed`
block reports a 6 ms mean over 254 documents — which cannot be OCR, since a single variant costs
hundreds of milliseconds. It times the `mrz` provider call only. **The benchmark does not measure
the expensive stage**, which is precisely why "the build got slow" has never been attributable to
anything in particular.

## Decision

**Split the benchmark by the role each document plays, and keep the gate deterministic.**

1. **The per-PR gate scores the scored population only** — the 157 documents that can yield a
   hit. Full coverage of the metric the gate actually asserts, still deterministic, so
   `tolerance: 0` stays valid and meaningful.
2. **The off-denominator refusal set runs on a schedule, not per PR.** All 97 of those documents
   verify *correct refusal*, which is a real property and must not be deleted — only moved off
   the critical path. (95 of the 97 are also the expensive ones; the split matters for cost, not
   for what the set is for.)
3. **Random sampling is rejected for the gate** (see below). If per-PR cost is still too high
   after 1 and 2, the next step is **deterministic slots**: slot *k* always selects the same
   documents, seeded by slot index and never by the clock, with a per-slot expected HIT in the
   baseline. Reproducible, so tolerance 0 survives; rotation gives coverage over time.
4. **Private specimens stay opt-in and stay out of CI**, permanently. `--include-private` is the
   only way in.
5. **Measure before building.** [`ADR-0008`](ADR-0008-mrz-detection-track.md) chunk 2 removed four
   `detect_words` passes per document from the default path — a corpus-wide saving nobody has
   measured yet. Implementing this ADR before measuring that would be optimizing against an
   out-of-date number, which is what `project_principles.md` P6 exists to prevent.

## Alternatives rejected

**Random rotation with `tolerance: 0`.** Rejected because the two requirements are mutually
exclusive, and the incompatibility is the most important thing in this document. Tolerance 0
works today *only* because the corpus is fixed and the pipeline deterministic — #246/#247
established that two CI runs are byte-identical. Draw a different subset each run and the HIT
count moves according to **which documents were drawn**, not according to the code. The gate then
fails on sampling noise, and a genuine regression hides inside that same noise. It would become
louder and less informative at once, which is strictly worse than either failure alone.

**Uniform one-per-(country, document type) sampling.** Rejected on its own arithmetic. It removes
cheap and expensive documents in proportion, so it buys about half of what the role split buys
(-50% against -74%) while giving up the deterministic gate. Paying more and getting less.

**Including private specimens in the default or CI rotation.** Rejected as a privacy regression
that reverses a decision merged days earlier. #254 removed the 13 real-PII images from the
default corpus walk and #255 made them opt-in. In CI this would put real personal data on
hosted runners, in job logs and in uploaded artifacts — against `CONTRIBUTING.md`'s "No PII in
logs or fixtures" and the whole compliance posture [`VISION.md`](../VISION.md) §4 sells. There is
no version of this that is worth 74% of a build.

**Raising the tolerance instead.** Rejected: the tolerance being zero *is* the gate's value. A
tolerance wide enough to absorb sampling noise is also wide enough to absorb the regressions the
gate exists to catch.

**Doing nothing and only measuring.** Partially adopted — measurement is step 5 and gates the
rest. Rejected as the whole answer, because the 95-document finding does not depend on the
pending measurement: those documents are excluded from the metric by construction, whatever the
absolute timings turn out to be.

**Rotation as corpus diversification.** Rejected as a category error, noted because it motivated
the proposal. Rotation varies *which slice is measured*; it does not diversify the corpus.
[`CORPUS_COVERAGE.md`](../CORPUS_COVERAGE.md) records 158 of 238 country codes with no specimen
at all — diversity is gained by ingest, not by sampling what is already there.

## Consequences

**Positive**

- The per-PR critical path loses the part of the work that dominates it, without weakening the
  assertion: the scored population stays complete, deterministic and tolerance-0.
- The stale "~1-minute" claim gets corrected, so the next person estimating this work starts from
  a real number.
- It forces the per-document timing the report currently lacks, which is the instrument any
  further tuning needs.

**Negative**

- **The hallucination canary moves from per-PR to scheduled.** The off-denominator set is what
  would catch a document tagged as unreadable reading a checksum-valid MRZ anyway. Two documents
  trip it today, and — stated plainly, because an alarm whose current firings are all explained
  is easy to over-sell — **both are already accounted for**:
  - `Argentina_Passport_Specimen_P0_ARG_2026_mrz` (`checksum_failed_specimen`): its printed MRZ is
    internally checksum-consistent but its values are placeholders, which is exactly why
    [#259](https://github.com/ruledicaprio/SynthPass/pull/259) scored it off the denominator.
  - `Malaysia_Passport_Specimen_P0_MYS_2019_redacted_mrz_blur` (`redacted_mrz`): line 2 is intact
    and genuinely validates; only line 1's `given_names` is covered.

  So the canary's value is **prospective** — it is the only thing positioned to catch a *new*
  invented zone — not that it is flagging an open mystery now. Moving it to a schedule delays that
  detection by up to a day. That is the real cost of this decision, and it is cheaper than the
  alternative only because nothing currently depends on same-PR detection.
- The baseline gains a shape change (scored-only per-PR, full on the schedule), and two baselines
  are two things that can drift apart.
- New work in `provider-bench`: a scored-only mode. `--limit` takes the first N and is not a
  substitute — it is not stratified and not role-aware.

**Explicitly not licensed by this decision:** no random sampling anywhere in the gate, no private
specimens in CI or in the default walk, no change to `tolerance: 0`, and no implementation before
the post-chunk-2 measurement in step 5.
