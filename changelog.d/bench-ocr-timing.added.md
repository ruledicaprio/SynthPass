- **`provider-bench` now records how long each document's OCR pass took.** Every row of the
  JSON report's `documents_detail` carries `ocr_ms`, and the summary prints one `ocr:` line
  (total, mean, p50, p95, max). The existing `speed` block times only the reader itself, which
  for the deterministic `mrz` provider is microseconds. So a real-specimen run that took forty
  minutes reported a mean of 3 ms, and nothing said which documents the time went to. That
  question gates [ADR-0010](knowledge/decisions/ADR-0010-benchmark-cost-split-by-role.md)'s
  benchmark split, whose step 5 requires measuring before building. The field is additive, so
  older reports parse as before.
