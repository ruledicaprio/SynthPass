# ADR-0001 — Adopt `knowledge/` as the documentation root

**Status:** Accepted
**Date:** 2026-07-25

## Context

`docs/` had accumulated fourteen files spanning four different genres: current
reference (`ARCHITECTURE.md`, `LICENSING.md`), forward-looking commitments
(`ROADMAP.md`, `VISION.md`), historical origin records (`FOUNDATIONAL_STRATEGY.md`,
`synthpass_v2_0.md`), and raw survey data (`*.jsonl`). Nothing distinguished them
beyond a hand-maintained index, and the index had already drifted.

Two pressures made this worse rather than merely untidy:

1. **The v1.3.0 Document Intelligence Engine work generates a genre `docs/` has
   no place for**: distilled paper summaries, per-provider capability notes,
   dated benchmark sweeps, prompt evaluation results, and the reasoning behind
   thresholds. These are engineering *memory*, not user-facing documentation.
   Dropping them into `docs/` alongside `LICENSING.md` makes both harder to find.

2. **A significant fraction of the reader population is now a model.** Context is
   budgeted. A flat folder mixing a licensing walkthrough with an academic paper
   forces whoever is reading — human or not — to load the wrong thing to discover
   it was the wrong thing. Modular, labelled structure measurably improves what
   gets retrieved.

The argument for treating this as institutional memory rather than documentation
is made at length in `../KNOWLEDGE.md`.

## Decision

Rename `docs/` to `knowledge/` and treat it as a technical library with subject
folders (`decisions/`, `providers/`, `prompts/`, `ocr/`, `vision/`,
`benchmarks/`, `evaluation/`, `hardware/`, `research/`, `papers/`), each carrying
a README stating what belongs in it and what does not.

Adopt three living artefacts named in `../KNOWLEDGE.md`:

- `../project_principles.md` — seven principles, each tied to the place in the
  codebase that enforces it, as the tiebreaker for contested design decisions.
- `../technical_debt.md` — deferred decisions with honest severity and effort.
- `decisions/` — ADRs, recording *why*, including rejected alternatives.

Performed with `git mv` so `git log --follow` keeps working across the rename.

## Alternatives rejected

**Keep `docs/` public-facing and add `knowledge/` for internal notes.** Clean
audience separation and zero reference churn. Rejected because the boundary is not
actually stable: `ARCHITECTURE.md` is both the public explanation of the system
and the internal record of what got deleted and why, and `ROADMAP.md` is both a
commitment and a working document. Splitting them would mean either duplicating
content or arbitrating an unclear line on every future edit. One root with labelled
subject folders answers "where does this go" more reliably than two roots do.

**Keep `docs/`, add subfolders inside it.** Achieves most of the structure with no
migration cost. Rejected for a weaker but real reason: the name sets the
expectation. `docs/` invites documentation — descriptions of what exists.
`knowledge/` invites the reasoning, the measurements, and the rejected
alternatives, which are the parts that are expensive to reconstruct and easy to
never write down. Renaming is cheap and happens once.

**Commit the tree as delete + recreate**, which is what the working copy already
contained. Rejected: it discards `git log --follow` for every file, which destroys
exactly the provenance this tree exists to preserve.

## Consequences

**Positive**

- New material has an obvious home, and each home states its own inclusion rule.
- ADRs make rejected alternatives durable. The codebase already demonstrates the
  value — `synthpass-core/src/fusion.rs` records an integrity check that was
  measured, false-positived on genuine specimens, and was discarded; without that
  note it would be re-proposed annually.
- `project_principles.md` gives contested decisions a written tiebreaker instead
  of relitigating philosophy per pull request.

**Negative**

- ~116 references across ~39 files had to be repaired, including source doc
  comments, workflow files, and historical `CHANGELOG.md` entries. Done in this
  one commit, with `scripts/check-doc-links.sh` added to CI so it cannot silently
  rot again.
- Any external link to `docs/…` on this repository now 404s. Acceptable: the
  project is pre-1.0 in adoption terms and the paths were never a published
  interface.
- Empty subject folders are a standing invitation to structure that never gets
  filled. Mitigated by each README stating a concrete inclusion rule, so an empty
  folder is a visible gap rather than decoration.

**Known gap, recorded not fixed:** eleven source citations reference
`docs/V2-DESIGN.md`, a document that has never existed in this repository
(verified against full history). They were deliberately *not* rewritten to
`knowledge/V2-DESIGN.md` — moving a broken link is not fixing it. Tracked in
`../technical_debt.md` and allowlisted in the link checker.
