# ADR-0025 — Tier 1 reads a single OCR pass only when two passes agree

**Status:** Accepted (owner, 2026-10-03)
**Date:** 2026-09-27

**2026-10-03: accepted by the owner.** This answers the question the 2026-10-01 note below left
open, which OCR pass Tier 1 reads. It changes no code:

- Rules (1) and (2) have no trigger on `main`. Both need a retry stop that confirms or holds a
  pass, and the retry loop has had neither since `SYNTHPASS_OCR_STOP=clean` and
  `SYNTHPASS_OCR_CONFIRM_PASSES` were removed on 2026-09-28 (#550). `synthpass-ocr` now warns once
  if either is still set, and every run stops on the first checksum-valid reading.
- Tier 1 therefore parses the full concatenation of every pass, rule (3). That is what `main`
  does today.
- Rules (1) and (2) apply again only if a stop that confirms or holds a pass returns. That stop
  needs its own A/B first. The re-run note's lever, a `clean` that stops on a superseding reading
  only when a second pass agrees, stays Hypothesized and unbuilt
  ([note](../benchmarks/retry-stop-rerun-2026-09-27.md)).
- ADR-0024's build step 6 (schema 2) no longer waits on this question (ADR-0024, "Where the build
  stands", 2026-10-03).

**2026-10-01: not adopted, and kept as the record.** This ADR lands on `main` as Proposed (the
owner's decision of 2026-10-01). The handoff it proposes was never adopted:

- The #473 re-run ([note](../benchmarks/retry-stop-rerun-2026-09-27.md)) found that
  `SYNTHPASS_OCR_STOP=clean` changed no real-specimen outcome, so #550 removed it on 2026-09-28.
  That is the removal the Consequences below provide for.
- Cases (1) and (2) of the Decision cannot occur now. Tier 1 parses the full concatenation, which
  is case (3) and what `main` does today.
- The question this ADR answers, which OCR pass Tier 1 reads, stays open. ADR-0024's schema 2
  (build order item 6) waits on it.

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
