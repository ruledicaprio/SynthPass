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
  distinction is load-bearing). On 2026-09-05, with `qwen2.5-1.5b-instruct-q4_k_m`
  and the deterministic normalizers folded in: reviewed **95/162 (58.6%)**, derived
  **85/162 (52.5%)**, **55.6% overall** (vocabulary `74aee114001a9ed2`).

  **Current, 2026-09-27, prompt v3:** the corpus is now **118** fixtures (65
  reviewed, 53 derived), vocabulary `b6bd1f9a5fdd108e`: reviewed **297/585
  (50.8%)**, derived **88/159 (55.3%)**, 385/744 (51.7%) overall. This is not a
  move from the 2026-09-05 figures: the fixture set, the normalizer vocabulary and
  some fixtures changed in between, so the two are different measurements, not two
  points on one trend. Full per-field figures and the version history are in
  `crates/synthpass-llm/tests/parity.rs`'s module doc and
  [`knowledge/benchmarks/README.md`](../benchmarks/README.md#current-headline-numbers).

  Per version, newest first:

  - **v3** (2026-09-27, [#506](https://github.com/ruledicaprio/SynthPass/issues/506),
    PR #533): drops OCR lines holding a whitespace-free run over 64 characters
    (guilloche microprint), and caps the content so the prompt plus 500 output
    tokens fits `n_ctx`, always keeping MRZ-shaped lines. The template and digest
    are unchanged. **Before/after, paired on the 84 fixtures the v2 run reached
    before it overflowed on Romania `PE_ROU_2024`: 281/522 → 281/522, zero field
    verdicts flipped** (reviewed 218/405, derived 63/117, both arms). Two field
    values changed, wrong to a different wrong, both on fixtures whose prompt v3
    shortened. On the 81 fixtures whose prompt v3 left byte-identical, every
    answer was byte-identical, so the two CI runs showed no run-to-run noise. v3
    then completes all 118 fixtures; Romania scores 3/9. Record:
    [`parity-prompt-v3-2026-09-27.md`](../benchmarks/parity-prompt-v3-2026-09-27.md).
  - **v2** ([#102](https://github.com/ruledicaprio/SynthPass/issues/102)): the
    MRZ-hint slot. The hint is off by default and the harness passes none, where
    the prompt is byte-identical to v1, so no parity measurement moves with it.
    Its one full-corpus run (2026-09-26, CI run 36252511121) stopped at fixture
    85 of 118 on a context-window overflow, which opened #506.

  Note when comparing against anything recorded before that date: the fixtures
  themselves changed. The `.md` inputs came from `docling` (retired in v0.7.5) and
  now come from `NativeOcr::recognize`, which is what the pipeline actually feeds
  Tier 2 — worth roughly a doubling of the measured rate on the same six
  documents. A pre-2026-09-02 number is not comparable to a later one.
- **Rejected phrasings**, with what they did to the numbers.
