- **Real-specimen strict-name counts now flow through to the baseline (report-only).** The
  committed `real-specimen-mrz-baseline.json` can carry ADR-0013's `strict_names` counts
  (`strict_hits`, `name_scorable_documents`, `name_scorable_hits`) once a CI re-bless measures
  them; `provider-bench --assert-baseline` warns, never fails, when they move — the strict-name
  rate is published for visibility, not gated on. `tools/rebless.py` and
  `scripts/check-headline-numbers.sh` carry the same field through to
  `knowledge/benchmarks/README.md`'s headline table.
