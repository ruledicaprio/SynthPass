# vision/

Vision-language models and the multimodal track.

## Status: not implemented

As of v1.3.0 there is **no vision provider**. `Capability.vision` is `false` for
every registered provider, and that is stated as the deliverable rather than
hidden: the interface exists and has zero vision implementations.

`synthpass-llm` today is text-only — it consumes OCR Markdown, not pixels — and
`llama-cpp-2` is pinned at `0.1.151` with only the `sampler` feature, so there
are no mmproj/mtmd bindings in the tree. A true image → fields provider is new
capability, not a refactor.

## What belongs here

- **The mmproj/mtmd spike** — can `llama-cpp-2` drive a multimodal GGUF at a
  usable version, or does that mean patching `llama-cpp-sys-2`?
- **Candidate models** — Moondream2, Qwen2.5-VL, Gemma3 Vision, SmolVLM,
  InternVL — with weights licence and CPU-decode latency on the target hardware,
  not just leaderboard scores. Add **Unlimited-OCR** (Baidu, Jun 2026) to that
  list on two facts: its weights are **MIT**, and it activates 0.5B parameters of
  a 3B MoE per token, which is CPU-plausible where a dense 3B is not. It is also
  a document *parser* rather than a field extractor, so it lands on the wrong
  side of the line below — see
  [`../research/long-horizon-parsing.md`](../research/long-horizon-parsing.md) §3
  before treating it as a candidate for anything.
- **The narrow-ask design** — the strongest idea in the design notes: don't send
  a VLM the whole image and ask "extract this document." Send it the OCR text,
  the detected MRZ, bounding boxes, and an explicit list of missing fields, and
  ask it to recover *only those*. A 2B model asked a narrow question outperforms
  a larger one asked a vague one.

## What this is not

The VLM is not the OCR engine. Deterministic recognition runs first and the model
fills what deterministic code cannot recover — principle 1.
