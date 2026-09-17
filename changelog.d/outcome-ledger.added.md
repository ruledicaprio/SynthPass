- **Real-specimen outcome ledger.** `provider-bench --write-baseline` now also writes
  `real-specimen-outcomes.jsonl` next to the baseline: one JSON row per document (`asset_id`,
  `outcome`, the full miss reason, format, checksum/name outcomes, OCR timing, native-retry
  telemetry), sorted deterministically by `asset_id`. Its SHA-256 is pinned in the baseline's new
  `outcomes_sha256` field, and `--assert-baseline` now fails if a committed ledger no longer
  hashes to it, and otherwise prints an informational per-document outcome diff against it. The
  ledger exists so a dated finding stays re-derivable after the `real-specimen-gate-report` CI
  artifact expires (it carries no `retention-days`).
- **Every miss bucket is now explicit in the baseline, including zero.** The committed baseline's
  `by_miss_kind` always states every known bucket (`false_positive_mrz` included) as an explicit
  `0` rather than an absent key, so "0 false accepts across N documents" is a visible claim, not
  an inference from silence.
- **New baseline field: `refusal_population`.** The off-denominator population (`documents -
  scored`) a false accept could come from, report-only like ADR-0013's `strict_names`.
  `tools/rebless.py` installs the ledger alongside the baseline and rewrites
  `knowledge/benchmarks/README.md`'s new False accepts row once a baseline carries it;
  `scripts/check-headline-numbers.sh` gained checks 15-16 for the ledger's integrity and this
  field's arithmetic, both skipped cleanly on a baseline that predates them.
