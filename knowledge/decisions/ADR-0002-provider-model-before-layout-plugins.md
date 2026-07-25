# ADR-0002 — Build the provider model (M7) ahead of M6

**Status:** Accepted
**Date:** 2026-07-25

## Context

`ROADMAP.md` opened with a commitment: *"The evolution is linear, M1 through M6 — no parallel
tracks."* That rule exists for a good reason. It was written against a real failure mode — a
project that starts five things and finishes none — and it has held through five milestones.

Two facts made it worth revisiting exactly once.

**M6 already contained M7's work, without the interface to hang it on.** M6's deliverables list
read *"…declarative document layouts; dataset exports; **plugin architecture**; air-gapped
deployment guide; commercial Pro closed beta"*, with the DoD *"a third-party plugin builds against
a stable interface following the docs."* There was no such interface, and nothing in M6's other
deliverables would produce one. "Plugin architecture" was a placeholder for a subsystem, sitting
inside a milestone otherwise about document formats and packaging.

**M6's concrete work is exactly what a provider model absorbs.** M6 adds TD1, TD2, MRVA and MRVB;
its own scoping note observes that driving licences cannot be read by any amount of MRZ work and
belong to a barcode decoder behind `ExtractionV2.barcodes`. Today the extraction path is a
hardcoded two-tier branch in `synthpass-pipeline`: `mrz::find_and_parse(..).filter(|m| m.valid())`
and, on failure, one LLM call. Adding four MRZ formats plus a barcode decoder to that shape means
adding branches to a conditional that already knows the name of every engine it can call.

## Decision

Introduce **M7 — Document Intelligence Engine** as a numbered milestone and build it before M6.
Amend the linearity statement to record the exception and its reason rather than quietly breaking
it.

Narrow M6's "plugin architecture" to **declarative *layout* plugins** — data-driven document
definitions, which is what the rest of that milestone is actually about — and move the
"third-party plugin builds against a stable interface" criterion to M7, which owns the interface.

Milestones remain numbered by **dependency, not build date**. M7 is numbered after M6 because it
was conceived later; it is built first because M6 depends on it.

## Alternatives rejected

**Keep strict M1→M6 order; M7 stays design-only until M6 ships.** Honours the existing commitment
exactly, and the linearity rule has earned deference. Rejected because it inverts the dependency:
M6 would add four document formats and a barcode decoder as branches in the hardcoded two-tier
conditional, and M7 would then rewrite all of them into providers. That is the same work twice,
and the second pass is a refactor of shipped, customer-visible behaviour rather than of code
nobody depends on yet. A design left unimplemented across a whole milestone also goes stale
against the code it describes.

**Fold the provider model into M6 as another deliverable.** No new milestone, no amendment to the
linearity claim. Rejected because M6 is already large — four document formats, declarative
layouts, four export formats, an air-gapped deployment guide, and a commercial beta — and a
subsystem with its own contract, catalog, routing engine and benchmark harness inside that list
gets neither a Definition of Done that means anything nor a release it can be attributed to. It
would also leave the misleading "plugin architecture" line unexamined.

**Renumber so the built-first milestone is M6.** Superficially tidier. Rejected because
`ROADMAP.md`, `CHANGELOG.md`, closed pull requests and commit messages already refer to M6 by
number; renumbering rewrites the meaning of existing references, which is a worse cost than one
out-of-order build with a written reason.

## Consequences

**Positive**

- M6's formats arrive as providers registered against an existing contract, not as new branches
  in a growing conditional.
- The barcode slot driving licences need becomes something a contributor can write without
  touching the pipeline.
- M6's third-party-plugin DoD becomes satisfiable, because an interface exists.
- The escalation decision becomes inspectable — which is principle 7, and not achievable inside
  the current two-tier branch.

**Negative**

- The linearity rule now has an exception, and an exception invites a second one. Mitigated by
  requiring that any future exception arrive as an ADR arguing a dependency inversion this
  concrete — not a preference about what is more interesting to build.
- M6 slips by roughly one milestone of calendar time.
- The roadmap's milestone table is no longer in build order, which is mildly confusing to read.
  Mitigated by saying so directly in the table and the timeline.

**Explicitly not licensed by this decision:** parallel tracks. M7 is built *instead of* M6 for a
period, not *alongside* it. One milestone at a time remains the rule.
