- **The benchmarks can record which OCR pass read each MRZ-shaped line.** `synthpass-bench
  --ocr-passes` adds `ocr_text` and `ocr_passes` to each `results[]` entry, and `provider-bench
  --real-specimens --dump-ocr-passes` writes `provider-bench-ocr-passes.jsonl` next to `--out`: one
  row per document, listing every executed OCR pass in order with its transform and the lines it
  read, each with its bounding box. Off by default and report-only: the OCR text, which retry passes
  run, where they stop, and every existing report field are unchanged. `--dump-ocr-passes` refuses
  `--include-private` and an output directory git does not ignore (#574).
