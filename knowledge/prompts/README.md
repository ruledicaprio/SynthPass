# prompts/

Prompt *philosophy* and per-version *evaluation results*.

## The prompts themselves are not here

They live in `crates/synthpass-llm/prompts/` and are compiled in with
`include_str!`. Two reasons, both load-bearing:

1. **Air-gapped single binary.** `include_str!` resolves relative to the source
   file. A prompt under `knowledge/` would make the crate un-buildable outside the
   workspace and add a runtime file read to a deployment story whose whole pitch
   is "copy one static binary to an isolated machine."
2. **A prompt swapped at runtime invalidates the version stamped in the output**,
   which is the entire point of versioning it. If you need a different prompt, you
   rebuild — that is what auditable means here.

## What belongs here

- **Philosophy** — the schema contract we ask models to honour: return only JSON,
  unknown values are `null`, never guess, every field carries `value` /
  `confidence` / `source`.
- **Evaluation per version** — when a prompt version bumps, the measured
  before/after on the benchmark corpus. A prompt change with no measurement behind
  it is indistinguishable from noise.

  The parity corpus used to be 6 documents, which is where the "drift hides for
  months" warning came from. As of 2026-09-02 it is **72**: 18 hand-verified
  fixtures scored on all nine prompt fields, and 54 generated ones scored only on
  the fields an ICAO check digit proves (see
  `crates/synthpass-bench/examples/ground_truth_candidates.rs` for why that
  distinction is load-bearing). Baseline at PROMPT_VERSION 2 with
  `qwen2.5-1.5b-instruct-q4_k_m`: reviewed **43/162 (26.5%)**, derived **47/162
  (29.0%)**, repair fallbacks **0**. Full per-field figures in
  `knowledge/ROADMAP.md`.

  Note when comparing against anything recorded before that date: the fixtures
  themselves changed. The `.md` inputs came from `docling` (retired in v0.7.5) and
  now come from `NativeOcr::recognize`, which is what the pipeline actually feeds
  Tier 2 — worth roughly a doubling of the measured rate on the same six
  documents. A pre-2026-09-02 number is not comparable to a later one.
- **Rejected phrasings**, with what they did to the numbers.
