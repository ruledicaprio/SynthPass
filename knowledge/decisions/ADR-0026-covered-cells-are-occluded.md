# ADR-0026 — Covered MRZ cells are reported as `occluded`, never as text

**Status:** Proposed
**Date:** 2026-09-28

## Context

A box or a heavy blur inside a machine-readable zone is read today as if it were print. On
`passports/Malaysia_Passport_Specimen_P0_MYS_2019_redacted_mrz_blur.png` the zone parses
checksum-valid: line 2 is intact and genuinely validates
([ADR-0010](ADR-0010-benchmark-cost-split-by-role.md), Consequences). But the cell at the box's
edge is read as a letter and appended to the surname, and the covered given names come back empty.
Nothing in the record separates those fields from a real read. No ICAO check digit covers a name
on any format, so the arithmetic cannot object ([ADR-0013](ADR-0013-names-are-scored-against-mrz-form-truth.md)).

Real users send partly covered scans, typically with the names blacked out before sharing. The
corpus shows the publishers' methods vary: black fill, white fill, Gaussian blur, pixel mosaic and
textured patches all occur among the `redacted_mrz` specimens.

This ADR decides what the product says about such cells, in the record and in the trace. It does
not change the benchmark: `redacted_mrz` stays an off-denominator class decided by filename
([ADR-0008](ADR-0008-mrz-detection-track.md), `samples/README.md`).

## Decision

1. **Vocabulary.** A cell is *occluded* when the image shows it covered — a uniform fill of any
   tone, or blur measured against the zone's own verified line. The word describes pixels. It does
   not claim that anyone redacted anything, and it is never a judgement of tampering or
   authenticity, which are permanent non-goals ([VISION.md](../VISION.md) §2).
2. **Only an already-parsed zone is masked.** `mrz::apply_occlusion(&MrzData, CellMask)` classes
   each masked cell with the crate-private per-cell coverage map (#519):
   - a check-covered or structural cell fails closed with `MrzError::OccludedCheckedCell`;
   - an unverifiable cell makes its field occluded.

   The mask never enters `find_and_parse`, whose repairs move cells (#551). The function is
   additive, so `mrz` ships it as a 0.9.x patch.
3. **Names follow ICAO's grammar.** A name component is complete only when a visible terminator
   precedes the first covered cell in it. Otherwise it is occluded, and an occluded surname makes
   the given names occluded too.
4. **The record.** `ExtractionV2` gains `occluded: [CoreField]`, omitted when empty. A listed
   field's value is `null` and its confidence is `0.0`. So "covered" (null, listed), "read as
   nothing" (`""`) and "not read" (null, unlisted) are three distinct states. The v1 record, which
   has no status vocabulary, carries `null`.
5. **The trace.** `ExtractionTrace` gains `mrz_occlusion: {spans: [{line, first, last, kind}]}`,
   where `kind` is `fill` or `blur`. It is a per-document observation, so it sits beside
   `escalation`. `config_overrides` stays configuration only. Spans are positions of the occluder,
   never text, so the trace stays PII-free.
6. **No model fills a covered field.** After Tier 2 runs, the pipeline's shared deterministic step
   nulls and lists every occluded field. `Evidence::missing` excludes occluded fields, so no
   routing signal escalates on them.
7. **Covered content is never reconstructed.** The check-digit arithmetic is not used to rebuild a
   cell the image shows as covered: a covered cell is refused, never solved for.
8. **Default off until measured.** Detection runs behind `SYNTHPASS_OCR_OCCLUSION`. Every
   threshold is named and swept (`knowledge/benchmarks/README.md`, "Naming a threshold"). v1
   inspects the name line after a Tier-1 hit. Other lines, and reads that fail their check
   digits, need a later decision.

**Promotion to on by default requires all of the following:**
- on every synthetic non-redaction profile, zero false occlusions;
- on the redaction profiles, zero covered-field leaks;
- zero changed outcomes at the real-specimen gate;
- every real document that gains an occluded field is inspected and named in a dated note;
- the added p50 cost on Tier-1 hits is reported.

## Alternatives rejected

- **Pass the mask through `ParseOptions`.** `ParseOptions` derives `Copy`, so an owned mask breaks
  it. And `find_and_parse` would have to honour the mask across repairs that relocate cells, which
  is #551's unbuilt, repair-aware oracle. A tunable that a repair path silently ignores is the
  opt-in trap [ADR-0017](ADR-0017-checks-distinguish-absent-from-verified.md) documents.
- **Write covered cells as `<` in `mrz.lines`.** A filler asserts that an empty cell was printed.
  That fabricates the one thing the image does not show. `mrz.lines` stays the validated read, and
  the trace's spans say which of its cells carry no evidence.
- **Report the visible prefix of a partly covered name as the value.** A consumer that ignores the
  status gets a possibly truncated name and no sign of it. That is the defect this ADR exists to
  remove. A `partial` status can be added later without a break; withdrawing a value later cannot.
- **Call the status `redacted`.** Glare, a finger or a hologram also cover cells. `redacted` would
  claim an intent the pixels cannot show.
- **Detect only "dark" fills.** The existing ink threshold ignores white and grey fills, and the
  corpus contains both.
- **Let the LLM decide whether a field was covered.** A model is never asked something a
  deterministic check can answer (principle 1).

## Consequences

- **Positive.** A covered name can no longer pass as a clean read, and every covered field is
  explained in the record. The real specimen that motivated this keeps its verified line 2, and
  its names become explicit.
- **Negative.** A false occlusion blanks a correctly read name on a Tier-1 hit. A false occlusion
  of the document-code cell refuses a hit. Both are measured before promotion, but the real
  positive set is one document, so synthetic profiles carry the recall claim.
- **Negative.** Four parse sites (the pipeline, `MrzReader`, `synthpass-bench` and
  `provider-bench`) must call the same application function. A test pins that they agree.
- **Negative.** Mosaic and textured patches are not detected in v1. On a hit, those cells still
  read as text. The gap is documented and bounded by later generator profiles.
- **Neutral.** `redacted_mrz` stays off the denominator by filename, the ADR-0010 canary keeps
  firing on the same specimen, and no baseline count moves.

**Amends [ADR-0013](ADR-0013-names-are-scored-against-mrz-form-truth.md).** `occluded` joins the
`name_error` kinds. It is not a misread, and strict name metrics report it separately.

**What would reverse it.** The same trigger that ends a default-off arm. Either the synthetic sweep
finds no thresholds that keep false occlusions at zero on every non-redaction profile while leaks
stay at zero, or the real-specimen arm blanks names on hits that inspection shows are legible.
