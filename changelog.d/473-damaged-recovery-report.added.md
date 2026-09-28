- **Both bench harnesses record, per document, whether a reading came from `mrz`'s
  damaged-capture search.** `synthpass-bench` and `provider-bench` report `retry_damaged_recovery`
  (the reading the OCR retry loop accepted or held) and `tier1_damaged_recovery` (the harness's own
  Tier-1 parse), and `synthpass-bench` gains the per-document `retry_variant_id` `provider-bench`
  already carried. The keys are always written, `null` when there is nothing to report, and
  `tools/synth_ab_diff.py` breaks moved seeds down by `retry_stop` × `retry_damaged_recovery`.
  Instrumentation only: extraction, the committed real-specimen baseline and its outcome ledger
  are unchanged (#473).
