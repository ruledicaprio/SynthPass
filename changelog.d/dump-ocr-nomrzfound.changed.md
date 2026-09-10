- **`provider-bench --dump-ocr` now covers `no_mrz_found` misses too**, not only
  `checksum_failed`. Since the 2026-09-09 denominator correction, `no_mrz_found`
  means "an MRZ was expected and none was found" — a genuine detection failure —
  because the MRZ-less documents that used to pollute it are scored out as
  `no_mrz_expected` first. Those rows carry the MRZ band score and raw OCR text
  (no recovered zone: nothing parsed), which is the localization-vs-recognition
  evidence ADR-0008 chunk 1C needs. Each row gains a `miss_reason` field, and the
  dump file is renamed `provider-bench-miss-ocr-dump.jsonl` (was
  `provider-bench-checksum-failed-dump.jsonl`).
