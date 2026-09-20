- **`bench-report` binary.** Turns a `provider-bench` gate report JSON, the committed
  `real-specimen-mrz-baseline.json`, and `samples/corpus.jsonl` into a deterministic Markdown
  benchmark report — no corpus image is read and no OCR runs. Covers provenance (MAIN/`samples-data`
  SHAs, provider/OCR configuration), the two Tier-1 hit rates (scored and whole-corpus, both
  denominators stated), the full outcome-bucket table, a per-format and per-issuing-state accuracy
  cut (joining `documents_detail[].mrz_format` against the manifest's `mrz.issuing_state`, every row
  reconciling exactly to the headline totals), a separate corpus-composition breakdown by
  `samples/corpus.jsonl`'s `dir`/`provenance`/`year`/licence fields, the ADR-0013 strict-name rates
  when measured, methodology, limitations, and reproduction commands. `cargo run -p synthpass-bench
  --bin bench-report -- --report PATH --baseline PATH --corpus PATH [--out PATH]`.
