# evaluation/

How we decide something is good enough to ship.

`../benchmarks/` holds *how we measure* and *what we measured*. This folder holds
*what the numbers have to say before a thing ships* — the gates, and the argument
for where each is set.

## What belongs here

- **Gate definitions and their rationale.** The M4 CI gate is
  `--min-hit-rate 0.30` against a measured rate well above it. That margin is
  deliberate: OCR inference varies across machines with floating-point ordering,
  and a gate pinned just under the observed value fails on unrelated hardware.
  A gate is a floor against regression, not a target.
- **Acceptance criteria per milestone** — the Definition of Done columns in
  `../ROADMAP.md`, and what evidence closed them.
- **What we chose not to gate on**, and why. Gating on a noisy metric trains
  people to re-run CI until it passes, which is worse than not gating.

## The honesty rule

When a measured number falls short of an aspiration, the aspiration gets
corrected — not the presentation. M4's original target was 95%; it measured ~55%,
and the roadmap says so in the row where the target used to be. That record is the
point of this folder.
