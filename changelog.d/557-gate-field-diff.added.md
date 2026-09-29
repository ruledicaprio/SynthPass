- **`provider-bench --assert-baseline` reports every recorded per-document field that moved
  against the committed outcome ledger, not only the outcome.** After the existing `outcome`
  summary (unchanged), it prints a deterministic group (`miss_reason` kind, `mrz_format`,
  `mrz_found`, `mrz_checksums_valid`, `names_exact`, `name_error`, `retry_variant_id`,
  `retry_stop`) and a timing-sensitive group (`ocr_ms` as one summary line, and every change on a
  document that hit the retry-pass budget), to the job log and to `$GITHUB_STEP_SUMMARY`. The
  diff never fails the gate, and it prints asset ids, field names, enumerated values and counts
  only, never `miss_reason` text. An assert run also writes
  `real-specimen-outcomes-text-free.jsonl` under the `--out` directory (this run's ledger with each
  `miss_reason` reduced to its kind), and the gate workflow uploads it with the report. The
  committed ledger, `OutcomeRow` and `outcomes_sha256` are unchanged. `tools/rebless.py` prints
  the same diff between the old and the new ledger, carries it into the commit message and the
  findings entry, and stops (exit 3) when a document's outcome changed behind identical
  aggregates (#557).
