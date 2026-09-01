- **Gated experiment: surface a composite-only-failed MRZ read's proven fields (off by
  default).** `MrzReader` previously discarded every field whenever any ICAO check digit
  failed, even when the *composite* check digit was the only failure and every
  individually check-digited field (`document_number`/`date_of_birth`/`date_of_expiry`/
  `personal_number`) independently verified — `mrz::Checks::only_composite_failed` names
  this exact, narrow case. New opt-in constructor `MrzReader::with_partial_read_surfacing()`
  reports those four fields at a new `FieldConfidence` band (`mrz_checksum_scope_partial`,
  strictly between `MRZ_STRUCTURAL` and full `PROVEN` — real arithmetic backing, but the
  record's own composite check still failed) instead of discarding the whole record; paired
  with new `RoutingPolicy::accept_composite_only_failure` (also off by default) to let a
  routing decision act on the narrowed `evidence.missing`. Neither is wired into
  `synthpass-pipeline`'s production catalog — `MrzReader::new()`/`RoutingPolicy::default()`
  are untouched, byte-identical to before (verified: the existing exhaustive
  `default_policy_is_exactly_the_v1_2_0_predicate` test still passes, plus new dedicated
  composite-only-failure regression tests in both `mrz_reader.rs` and `routing.rs`).

  Deliberately narrower than "any partially-valid read" — only fires when *every*
  individually check-digited field's own digit passed, not merely some of them. Reachability
  measured on the synthetic corpus (150 documents × 5 OCR-noise profiles): TD3 0%, TD1 7.3%,
  TD2 4.7%, not applicable to MRV-A/MRV-B (no composite check digit exists for those formats).
  A full pipeline-level Tier-2-escalation-rate A/B needs separate `synthpass-pipeline`/
  `provider-bench` wiring, deliberately not bundled into this change — see
  `knowledge/MRZ_SEQUENCE_COMPLETENESS.md`'s Chunk 7 for the full writeup and product-risk
  sign-off this shipped under.
