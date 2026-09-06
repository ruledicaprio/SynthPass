# ADR-0006 — Reframe M6 to lead with Tier-1 real-document accuracy

**Status:** Accepted
**Date:** 2026-09-07

## Context

M6 — "Expansion & Enterprise readiness" — carries seven tracks: the generator-format gap
(now closed), TD1/TD2/MRVA/MRVB as `synthpass-die` providers, MRZ sequence completeness
(Tier-1 real-document accuracy, scoped in [`MRZ_SEQUENCE_COMPLETENESS.md`](../MRZ_SEQUENCE_COMPLETENESS.md)),
declarative layout plugins, dataset exports, an air-gapped deployment guide, and a
commercial "Pro" closed beta.

Three facts made the framing worth fixing.

**The milestone table hid the accuracy track.** The M6 row's one-line summary read "layout
plugins, dataset exports, air-gapped guide and Pro beta still open" — the four
enterprise/packaging items, and nothing else. MRZ sequence completeness was a bullet in the
section body but absent from the table, the timeline, and every place a reader skims first.
The roadmap's own preamble names this failure mode: "the milestone table and the
per-milestone scoping notes are planning text ... not always revised after."

**Recent effort went entirely to the accuracy track.** Every PR from v1.3.0 to v1.4.0
(#177–#216) was Tier-1 MRZ parser correctness on real specimens (line-1 shift repairs,
name-separator-collapse recovery, century-pivot fixes, the corpus-ingest gate) or Tier-2
measurement and normalization. The four enterprise/packaging items are untouched. The
document that was supposed to describe the work did not describe the work.

**The product docs already rank it.** [`MRZ_SEQUENCE_COMPLETENESS.md`](../MRZ_SEQUENCE_COMPLETENESS.md)
line 12: *"SynthPass's product is the deterministic Tier-1 MRZ core; Tier-2 LLM inference is
the enterprise add-on for the residual ~1%, not the thing being sold."*
[`project_principles.md`](../project_principles.md) P1 (deterministic before probabilistic)
and P6 (benchmark everything) point the same way, and real-specimen benchmarking shows
`checksum_failed` (66) as the single largest miss category — the MRZ is *found* but does not
validate, more often than it is not found. `ADR-0002` already recorded that M6 was
overloaded ("M6 is already large ... a subsystem ... inside that list gets neither a
Definition of Done that means anything nor a release it can be attributed to").

## Decision

Reframe M6 so its stated priority order matches the effort and the constitution:

1. **Lead with the deterministic core** — MRZ sequence completeness / Tier-1 real-document
   accuracy, and TD1/TD2/MRVA/MRVB reading through registered `synthpass-die` providers.
2. **Sequence the enterprise/packaging track behind it** — declarative layout plugins,
   dataset exports, air-gapped deployment guide, commercial "Pro" beta — explicitly, in the
   table row, the section body, and the "suggested order".

This is a **reframe, not a split**. M6 stays one milestone with one number. The
`## M6 — Expansion & Enterprise readiness` heading keeps its exact text, because `ADR-0002`,
`CHANGELOG.md`, closed PRs and one inbound doc anchor all reference it — the same
"references are load-bearing, don't churn them" reasoning `ADR-0002` used to reject
renumbering. The milestone still delivers expansion and enterprise readiness; it does the
core work first.

The linearity rule is untouched: no new milestone, no parallel tracks, still one milestone
at a time.

## Alternatives rejected

**Split M6 into M6 (accuracy + formats) and a new M8 (enterprise & packaging).** Cleaner on
paper — each half gets a Definition of Done that means something, mirroring `ADR-0002`'s
"insert a number, don't renumber" move for M7. Rejected as too heavy for what the problem
is: `ADR-0002` was a *dependency inversion* (M6's formats could not be built correctly
before M7's contract existed), and it warns that "an exception invites a second one ...
Mitigated by requiring that any future exception arrive as an ADR arguing a dependency
inversion this concrete." There is no dependency inversion here — the packaging work is
merely *lower priority* than the accuracy work, which is a sequencing decision inside one
milestone, not a structural one. A second milestone-table change within four months, on a
weaker justification, spends more reader trust than it buys. M8 stays available as a clean
move later if the packaging track grows enough to need its own release.

**Leave the framing as it is.** Zero churn. Rejected because the table one-liner is a
factual defect — it omits an in-progress track — and it points readers, contributors and
any downstream planning at the enterprise items as "what M6 is", directly against
`MRZ_SEQUENCE_COMPLETENESS.md` and `project_principles.md`.

**Reorder the section silently, no ADR.** Rejected because this partially revises `ADR-0002`,
which framed M6 as the milestone "otherwise about document formats and packaging". A reader
comparing the two documents deserves the reason written down — that is what this directory
is for.

## Consequences

**Positive**

- The milestone table, the timeline, and the `## Open backlog` section all agree on what M6
  is doing now, and it agrees with the product principles.
- The Tier-1 accuracy work is attributable to a milestone position, not buried in a bullet.
- No anchor, reference, or numbering churn. `long-horizon-parsing.md`'s link to the M6
  dataset-exports commitment still resolves; the deliverable is still in M6, just later.

**Negative**

- M6's heading still says "Expansion & Enterprise readiness" while the section now leads
  with accuracy — a mild mismatch, accepted in exchange for reference stability. Anyone
  reading past the heading gets the corrected order immediately.
- M6 remains broad. The split would have bounded each half's Definition of Done more
  tightly; this reframe relies on the "suggested order" and the sequencing language holding.

**Explicitly not licensed by this decision:** parallel tracks, and a standing expectation
that milestones get re-carved whenever priorities shift. This is a one-time correction of a
milestone whose summary had drifted from its own body.
