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
  it is indistinguishable from noise: the parity corpus is 6 documents at 19/42
  field matches, so drift hides for months.
- **Rejected phrasings**, with what they did to the numbers.
