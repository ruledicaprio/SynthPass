- **`synthpass-pipeline` now consults the `synthpass-die` catalog for Tier 2, instead of calling
  the inferer directly.** `Pipeline::process_document` used to hand off to Tier 2 with a bare
  `self.extract_via_inferer(&stage.markdown)`. It now looks up a `FieldReader` via
  `catalog.find_reader(stage.escalation_budget, |c| c.fields && !c.deterministic)` — the new
  `LlmFieldReader`, registered alongside `MrzReader` at `Pipeline` construction — and calls
  `.read(&ctx)` on it. `RoutingPolicy`'s `Decision::Escalate { budget, .. }`, previously discarded,
  now selects the cost ceiling that lookup runs under.

  `LlmFieldReader` wraps the same `Arc<dyn InferBackend>` and semaphore/queue-depth/metrics
  plumbing `extract_via_inferer` always used, through a newly shared `run_tier2` seam — there is
  still exactly one place that can invoke the model. Proven behaviour-unchanged: every existing
  Tier-2 test (JSON shape, provenance, trace, `SYNTHPASS_JSON_V1` shim, metrics) passes unmodified,
  plus a new round-trip test covering the one real trap — `ExtractionV2` has no reverse conversion
  to the legacy v1 `Extraction` shape, and a naive one would have wrongly turned an absent
  `mrz_checksums_valid` into `Some(false)`.

  **Not in this PR:** `process_document_stream` still calls `InferBackend::extract_stream`
  directly — `FieldReader::read` has no delta-forwarding hook, and adding one is separable, scoped
  work (see `knowledge/technical_debt.md`'s "Streaming bypasses the provider contract"). Versioned
  prompts and the multi-provider benchmark harness remain unshipped.
