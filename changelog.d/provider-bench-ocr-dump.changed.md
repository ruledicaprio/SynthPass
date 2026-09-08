- **`provider-bench --dump-ocr` dumps the full pre-parse OCR text for real-specimen
  `checksum_failed` misses.** It already printed the recovered MRZ zone and the failing check
  digit(s); it now also prints the complete OCR text the provider consumed and the MRZ band
  score, and writes one JSON row per miss to
  `artifacts/provider-bench-checksum-failed-dump.jsonl` so the population can be analysed from
  a file. This is the real-specimen equivalent of `synthpass-bench --dump-ocr`, and the input
  to the `checksum_failed` root-cause pass (`knowledge/benchmarks/README.md`, `ROADMAP.md`).
