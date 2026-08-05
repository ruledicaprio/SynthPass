- **`llama-cpp-2` bumped 0.1.151 → 0.1.154, vendoring llama.cpp b10200.** No API or CLI
  change, and no action needed on upgrade — recorded because the local inference engine
  underneath Tier 2 moved by three releases, and generated output is not guaranteed
  bit-identical across an engine bump even at a fixed seed.

  0.1.154 ports three upstream breaking API changes, none of which reach this workspace.
  `llama_model_params.use_mlock`/`use_mmap` became a single `load_mode` enum upstream, but
  all four public accessors keep their signatures and defaults. The other two are in
  `mtmd`, which is not enabled here, and are absorbed below the safe wrapper: 0.1.154's
  `src/mtmd.rs` still exposes every entry point
  `knowledge/decisions/ADR-0005-vision-provider-readiness.md` catalogued at 0.1.151, under
  the same names. That ADR's inventory and ADR-0004's feature findings both survive the
  bump; each carries a version note saying so, since the release-note wording suggests
  otherwise.

  The `common` feature was left enabled. It is now droppable — 0.1.154 lets the grammar
  samplers work without it — but doing so swaps `LlamaSampler::grammar` from the crate's
  exception-guarded shim to the raw upstream call, which would let a C++ exception unwind
  across an `extern "C"` boundary rather than surface as `Err(GrammarError::NullGrammar)`.
  Reasoning recorded in `knowledge/technical_debt.md`.
