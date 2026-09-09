- **CI gates every extraction-path PR against a real-specimen Tier-1 no-regression baseline.**
  A new `real-specimen-gate.yml` workflow runs `provider-bench --real-specimens --mrz-only`
  (the deterministic `mrz` provider only — the ~1-minute pass, no LLM, no GGUF) over the whole
  `samples/` corpus and fails the build if the Tier-1 HIT count drops or any miss bucket
  (`checksum_failed`, `no_mrz_found`, …) grows past the committed, CI-measured baseline in
  `knowledge/benchmarks/real-specimen-mrz-baseline.json`. Path-filtered to extraction code and
  the corpus; advisory at first. Extends the `m4-hit-rate` synthetic-TD3 gate to the real
  corpus every recent Tier-1 change actually moved. See `knowledge/benchmarks/README.md` for
  the design and the baseline re-bless flow.
- **`provider-bench` gains `--mrz-only`, `--write-baseline PATH`, and `--assert-baseline PATH`.**
  `--mrz-only` registers only the deterministic reader, skipping the Tier-2 LLM provider and
  its model entirely — useful for any quick local Tier-1 measurement, not just CI.
  `--write-baseline` emits the `mrz` provider's HIT count + miss-kind histogram as JSON;
  `--assert-baseline` compares against one and exits non-zero on a regression (a missing path
  is written and passes, for first-run bootstrap).
