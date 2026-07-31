- **M7 is complete: a multi-provider benchmark harness has landed (`provider-bench`, new binary in
  `synthpass-bench`).** It runs every reader in the real M7 `ProviderCatalog` — the deterministic
  `MrzReader` and the LLM's `LlmFieldReader` — against the same OCR'd synthetic corpus and reports,
  per provider: field-match rate and mean CER (reusing `synthpass-bench`'s existing `cer()`),
  speed (mean/p50/p95), JSON validity (`synthpass_llm::repair::repair_fallbacks()` delta, `None`
  for deterministic providers), an unsupported-assertion rate (does each answered field value
  appear, verbatim, in the OCR text the provider saw), and resident memory —
  `Capability::estimated_resident_bytes` always, plus an optional coarse `sysinfo` process-RSS
  delta behind an off-by-default `measure-memory` Cargo feature and `--measure-memory` flag.

  Reaching the real catalog needed one new accessor, `Pipeline::catalog()`
  (`crates/synthpass-pipeline/src/lib.rs`) — `LlmFieldReader` is `pub(crate)`, so a benchmark
  outside `synthpass-pipeline` had no other way to reach the registered instance.
  `ProviderCatalog::readers()`/`.profiles()` already supported full enumeration; no catalog
  changes were needed.

  The Tier-1-only corpus-generation loop previously inlined in `synthpass-bench.rs`'s `main()` is
  now `synthpass_bench::generate_corpus`, shared by both binaries.

  A first real run (5 documents, clean profile, the shipped Qwen2.5 GGUF) measured `mrz` at 70%
  field-match / 3 ms mean / zero JSON-repair fallbacks, and `llm` at 56% field-match / ~24 s mean —
  both in line with existing measurements (M4's 55% Tier-1 hit rate on OCR noise; M5's GBNF parity
  run's ~45% field match), not an artifact of the new measurement code.

  Ships standalone: `cargo run -p synthpass-bench --release --bin provider-bench`. Not wired into
  CI in this PR — a separable follow-up once the metrics have more runs behind them.
