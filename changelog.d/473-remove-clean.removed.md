- **`SYNTHPASS_OCR_STOP` and `SYNTHPASS_OCR_CONFIRM_PASSES` are gone.** The `clean` retry-stop
  arm they configured held a checksum-valid reading that only `mrz`'s damaged-capture search had
  recovered, instead of stopping on it, and spent up to two more passes trying to confirm it.
  Re-measured against the default `first-valid` on a build that also fixed how Tier 1 read the
  held reading, it changed no real-specimen outcome, cost one real name read and gained 1 of 500
  synthetic seeds
  ([re-run](knowledge/benchmarks/retry-stop-rerun-2026-09-27.md)), so it was removed rather than
  promoted. Default runs are unchanged. If either variable is still set, loading the OCR models
  prints one stderr line per variable saying it no longer has any effect (#473).
