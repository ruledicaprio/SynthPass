- **`synthpass-pipeline` now consults the `synthpass-die` catalog for Tier 1, instead of its own
  inline gate.** `ocr_and_tier1` used to decide MRZ-vs-LLM with a bare
  `mrz::find_and_parse(&text).ok().filter(|m| m.valid())`. It now builds a `synthpass_die::Evidence`
  via the catalog's registered `MrzReader` and asks `RoutingPolicy::default().decide(&evidence)` for
  an `Accept`/`Escalate` `Decision`. The default policy is `v1_2_0_compatible()`, so the accept/
  escalate outcome and every field produced are unchanged — proven by three pipeline tests exercising
  a clean MRZ, a checksum-valid-but-structurally-flawed one, and a checksum-*invalid* one (the
  corrupted-check-digit case the old inline gate also escalated on, now asserted explicitly).

  The pipeline's own private `extraction_v2_from_mrz` — duplicated on purpose since
  `synthpass-die::mrz_reader` landed with "byte-for-byte the same output" — is deleted. The
  catalog's `MrzReader` is now the single place that mapping lives.

  **`ExtractionV2.trace` is populated for the first time**, on every processed document:
  `providers` always starts with `"mrz"` (the catalog reader the pipeline always consults first),
  with `"llm"` appended when Tier 2 ran, and `escalation` carrying the `EscalationKind` that sent it
  there. This is additive (`skip_serializing_if`) and does not change which fields are populated or
  any field's value or confidence, but it is a real, visible change to on-disk JSON: documents
  processed after this PR gain a `trace` key that was never present before.

  **Not in this PR:** Tier 2 (the LLM) is still called directly (`self.infer.extract_via_inferer`),
  not through the catalog — it is not yet a registered `FieldReader`. Wrapping it as one, versioned
  prompts, and the multi-provider benchmark harness remain later chunks.
