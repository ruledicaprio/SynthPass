# providers/

One note per intelligence provider — anything implementing `Recognizer` or
`FieldReader` from `crates/synthpass-die`.

## What a provider note contains

- **Capability profile** — the declared `Capability` values and, separately, what
  was *measured*. Declared and measured are different columns; keep them apart.
- **Licence of the weights**, not just the code. This is the thing that ruled
  Qwen2.5-3B out (non-Apache "Qwen Research" licence) despite it being technically
  attractive. `deny.toml` governs crates; nothing governs weights except this note.
- **Measured behaviour** on the benchmark corpus: per-field accuracy, speed,
  resident memory, JSON validity, unsupported-assertion rate.
- **Failure modes** — what it gets wrong, and whether the failure is detectable.
- **Hardware envelope** — what it needs to run at acceptable latency.

## What does not belong here

Marketing comparisons and star ratings. If a claim is not backed by a run of
`synthpass-bench`, it is an anecdote, and principle 6 says we don't keep those.
