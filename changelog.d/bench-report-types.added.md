- **`synthpass_bench::report` is now public.** The `provider-bench` report and real-specimen
  baseline types (`Report`, `ProviderRow`, `OutcomeRow`, `RealSpecimenSnapshot`,
  `RealSpecimenBaseline`, and the rest of the JSON vocabulary) are library API instead of being
  private to the `provider-bench` binary's `main`, so other tools can build or read the same
  report/baseline JSON without duplicating its shape. Pure move: the serialized JSON — including
  the committed `knowledge/benchmarks/real-specimen-mrz-baseline.json` — is byte-for-byte
  unchanged.
