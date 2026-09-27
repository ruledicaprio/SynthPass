# ADR-0025 — Tier 1 reads a single OCR pass only when two passes agree

**Status:** Proposed
**Date:** 2026-09-27

## Context

The retry loop accepts a reading from one pass's lines, but `OcrPage.text` accumulates every
pass and Tier 1 re-parses all of it. Under `SYNTHPASS_OCR_STOP=clean`, `mrz`'s `single()` then
refuses the loop's decision
([replay](../benchmarks/retry-handoff-replay-2026-09-27.md)). Handing Tier 1 the accepted pass
alone (#535) was reverted: at `first-valid` the single pass was a worse witness on every
document that moved ([A/B](../benchmarks/retry-handoff-ab-2026-09-27.md)). The concatenation
lets the ordinary scan pair lines across passes and lets `single()` refuse when passes
disagree.

## Decision

Tier 1 parses `OcrPage::tier1_text()`: (1) on `variant_valid_confirmed`, the confirming pass's
lines; (2) on a held reading accepted unconfirmed, `text` as it stood when the reading was
first held; (3) otherwise, the full concatenation. Under `first-valid` only (3) occurs. `mrz`,
`Checks`, `valid()` and `MrzReader` are unchanged. Tier 2 keeps the concatenation. The
pipeline, `synthpass-bench` and `provider-bench` call the one function.

## Alternatives

**The accepted pass on every stop** (#535): reverted, a checksum-valid wrong document number
and more synthetic wrong accepts. **The concatenation's parse as a veto**: equals today's
output except where the concatenation refuses, and there it returns the single pass that
produced #535's wrong reads. **A veto on checked fields only**: lets one pass decide names,
which no digit covers. **Confirming every variant stop**: unmeasured; deferred.

## Consequences

`clean` becomes measurable on its own, and its unconfirmed outcome equals `first-valid`'s. No
default change. The three real name corrections #535 made are not recovered. If `clean` shows
no gain in the #473 re-run, (1) and (2) are removed along with it.
