- **`provider-bench --replay-ocr-passes DIR` scores a captured real-specimen run without running OCR.**
  With `--real-specimens --mrz-only`, it reads the `provider-bench-ocr-passes.jsonl` an earlier
  `--dump-ocr-passes` run wrote, rebuilds each public-corpus document's page from its row, and runs the
  same scoring, so a change downstream of the OCR text can be compared on byte-identical text instead of
  on two live runs whose wall-clock-budgeted retry loops may read different passes. It writes the same
  report, the same `--dump-ocr` and `--dump-ocr-hits` dumps and the outcome ledger, and its run manifest
  names the capture in `replay_of` (the capture's run-manifest name and SHA-256). It refuses, naming the
  problem, a capture that does not cover the corpus exactly once, that read other image bytes or another
  `samples/corpus.jsonl`, or that lacks the keys below, and `--include-private`, `--include-local`,
  `--write-baseline`, `--assert-baseline` and `--dump-ocr-passes`. Each pass-file row gains two text-free
  keys at its end, `retry_damaged_recovery` and `mrz_band_score`, so a replay reproduces the report's
  `retry_damaged_recovery` and the dump's band score; every earlier key is unchanged, so a capture from
  before this change cannot be replayed and must be recaptured. `tools/bench_ab_diff.py` prints each
  arm's `replay_of`, and its `--expect-identical` mode gains `--check-report`, which also compares each
  document's `report.json` row (#574).
