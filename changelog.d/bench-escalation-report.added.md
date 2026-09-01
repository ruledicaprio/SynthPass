- **`synthpass-bench --escalation-report`: the Tier-2 escalation A/B for Chunk 7's
  `accept_composite_only_failure`, and the measurement that says not to ship it.** Reports the
  escalation rate under the shipped default routing policy *and* under the opt-in, plus whether
  the documents the opt-in would newly accept are actually correct.

  Pure post-processing of the `SeedResult`s a run already produces — no pipeline change, no
  `RoutingPolicy` call, no new dependency. Sound because
  `synthpass_die::routing::default_policy_is_exactly_the_v1_2_0_predicate` proves exhaustively
  that under the default, `Decision::Escalate` ⇔ `!(mrz_found && mrz_checksums_valid)`, which in
  this harness's vocabulary is exactly `ocr_error` + `no_mrz_found` + `checksum_failed`. Both arms
  therefore come from the *same run of the same binary*, satisfying
  `knowledge/benchmarks/README.md`'s same-binary A/B rule by construction rather than by
  discipline. `document_number_mismatch` is deliberately excluded from the escalation set: those
  documents' check digits verified, so the router accepts them — they are the separate
  compound-cancellation blind spot, which this opt-in neither helps nor worsens.

  **Measured, 300 documents per format, `--profile all --seed 0`, sequential on an idle machine:**

  | Format | escalation (default) | escalation (+opt-in) | delta | newly accepted | carrying a wrong check-digited field |
  |---|---|---|---|---|---|
  | TD1 | 42.7% | 34.0% | −8.67pp | 26 | **26/26** |
  | TD2 | 21.7% | 17.0% | −4.67pp | 14 | **13/14** |
  | TD3 | 30.0% | 29.7% | −0.33pp | 1 | **1/1** |

  **40 of 41 (97.6%) of the documents the opt-in would newly accept carry a field that is
  provably wrong.** Every one would be surfaced at `CHECKSUM_PARTIAL` (0.95) with
  `Decision::Accept`.

  This is structural, not statistical. On TD1/TD2/MRV-A/MRV-B, `Checks::personal_number` is
  hardcoded `true` — `parser.rs:386` (`// TD1 has no personal-number check digit`), `:299`,
  `:467`, `:538` — because no such check digit exists in those formats. `Checks::failed()` only
  lists a field whose bool is `false`, so a corrupted optional-data field can *never* appear in
  `failed()`. But the TD1/TD2 composite digit **does** cover that field. So on those formats
  `only_composite_failed()` is largely the signature of *"the one field with no independent check
  is wrong, and the composite — the only digit covering it — caught that"*. Observed
  substitutions are the classic OCR confusions (`0↔O`, `5↔S`), which are not congruent mod 10 and
  so are reliably caught by the composite and by nothing else.

  TD3 is the contrast that confirms the mechanism rather than contradicting it: its
  `personal_number` genuinely is check-digited (`parser.rs:223`), reachability collapses to 1 in
  300, and that single document is wrong in `document_number` via `Q`→`G` — values 26 and 16,
  congruent mod 10, therefore invisible to that field's own check digit. The composite is the
  only reason the record was flagged at all.

  Either way the conclusion is the same: a composite-only failure is evidence that something in
  the zone is wrong, not evidence that everything except the composite digit is right.
  `FieldConfidence::mrz_checksum_scope_partial`'s justification for the 0.95 band — "this field
  still carries a real, independently-passing arithmetic proof that a structural-only field never
  has" — does not hold for `personal_number` on four of the five formats, where the passing proof
  is vacuous.

  The decision rule was fixed in the plan before the run ("any wrong check-digited field ⇒ revert
  #173"); the revert follows separately.
