# Long-horizon parsing, and what an end-to-end OCR model is worth here

Distilled from [`../papers/Unlimited_OCR_Works.md`](../papers/Unlimited_OCR_Works.md)
(Baidu, [arXiv:2606.23050](https://arxiv.org/abs/2606.23050), Jun 2026). Code and
weights: [github.com/baidu/Unlimited-OCR](https://github.com/baidu/Unlimited-OCR),
MIT.

## Start here: the headline does not apply to us

The paper's contribution is Reference Sliding Window Attention — each generated
token attends to *all* reference tokens (the visual tokens and the prompt) but
only to the preceding 128 output tokens, so the decode-side KV cache is bounded
by a constant instead of growing with output length. The payoff is parsing
"dozens of pages in a single forward pass"
([§3.4](../papers/Unlimited_OCR_Works.md#34-reference-sliding-window-attention),
[§5.4](../papers/Unlimited_OCR_Works.md#54-long-horizon-parsing)).

**A passport is one page.** SynthPass has never had a long-horizon parsing
problem and adding one is not on any milestone. Nothing in the constant-KV-cache
result is actionable here, and no amount of re-reading changes that.

Three things underneath the headline *are* worth having. They are ranked by what
they cost us, cheapest first.

---

## 1. The data engine — the one to act on ⭐

**The idea, in our terms.**
[§4.1](../papers/Unlimited_OCR_Works.md#41-data-engine) describes how the 2M-sample
training corpus was built: single-page PDFs annotated **with PaddleOCR**, each
block's coordinates and content concatenated into end-to-end ground truth,
coordinates normalised to **0–1000**, multi-page samples synthesised by
concatenating single pages with `<page>` as the separator, everything packed to
32K tokens.

Read that again with our tree in mind. Baidu pseudo-labelled with an OCR engine
because they had no generator, and inherited that engine's errors as ground
truth. `synthpass-gen` produces labels that are **100% accurate by construction**
— that is M2's Definition of Done, not an aspiration. On the axis that limited
the SOTA model's training data, we are strictly ahead.

**What it becomes.** [`ROADMAP.md`](../ROADMAP.md#m6--expansion--enterprise-readiness)
commits M6 to "dataset exports (COCO / YOLO / JSONL / Hugging Face), consumed by
at least one external trainer end-to-end" — and specifies no format for the JSONL
and Hugging Face halves. Adopt this one. Normalised 0–1000 coordinates,
block-concatenated content, `<page>` separators. Then "consumed by an external
trainer" stops being a hurdle we have to go find a trainer for: a corpus in this
shape is drop-in for anyone following the DeepSeek-OCR / Unlimited-OCR recipe,
which is currently a well-travelled path.

**What it costs.** Nothing. No dependency, no model, no runtime. It is a choice
about what the exporter writes, made before the exporter is written, which is the
only cheap moment to make it.

**Why it might not work.** The convention could be superseded before M6 lands —
it is a 2026 recipe, not a standard. That risk is bounded: it costs one
serialisation function either way, and a format nobody else reads is the status
quo we would be choosing instead.

---

## 2. PaddleOCR — and the prize is the confidence signal, not the accuracy ⭐

Two corrections to the premise before the proposal.

**PaddleOCR is Apache-2.0, not MIT.** This does not weaken the case. Apache-2.0
is exactly the bar [`ROADMAP.md`](../ROADMAP.md#future-work) already applies to
weights — the bar that ruled Qwen2.5-3B out for carrying a "Qwen Research"
licence despite being technically attractive. PaddleOCR clears it.
[`providers/README.md`](../providers/README.md) is where a weights licence gets
recorded; `deny.toml` governs crates and has never governed weights.

**The C++ dependency is avoidable, which is what makes this viable at all.**
PP-OCRv5 models export to ONNX, and `rten` — already a direct dependency of
`synthpass-ocr` — is end-to-end Rust and ships `rten-convert` to turn ONNX into
its own `.rten` format. So PaddleOCR is adoptable **without** pulling in `ort` /
onnxruntime. That matters: an onnxruntime dependency is a C++ toolchain in a
workspace whose musl single-file air-gapped build
([`ARCHITECTURE.md`](../ARCHITECTURE.md) §10) is a shipped feature. The Rust
crates that wrap PaddleOCR today (`oar-ocr`, `paddle-ocr-rs`) all route through
`ort` and are therefore the wrong door for us, even though they are the obvious
one.

### Path A — a second recognition engine

Conversion output drops into machinery that already exists:
`crates/synthpass-ocr/src/download.rs`, `known_good_hashes.rs`, `embedded.rs`
and `build.rs` are already a hash-pinned model-fetch-and-cache path with an
`embedded-models` feature that bakes weights into the binary. A second engine is
new weights through an existing pipe, not a new pipe.

### Path B — the actual reason to do this

[`technical_debt.md`](../technical_debt.md#ocr-confidence-is-a-character-plausibility-proxy-not-a-model-score)
carries a **High**-severity entry: `ocrs` computes CTC decode probabilities
internally and drops them — `TextChar`/`TextLine` expose only `char` and `rect` —
so `geometry::text_sanity` is a character-plausibility proxy wearing the name
`confidence`. Its recorded fix is "upstream a patch to `ocrs` exposing the decode
probabilities, or fork the recognition loop… 3–5 days, **mostly upstream
negotiation**."

Running PaddleOCR's *recognition* model through `rten` directly means writing our
own CTC decode. The per-character probabilities are then **in our own code**,
because we computed them. The debt's cost estimate is dominated by a negotiation
this route does not have.

That is worth more than any accuracy delta. `text_sanity` becomes a real model
score, and M7's evidence-driven escalation gets an honest signal to route on
instead of a proxy no threshold can outperform.

**What it costs, stated honestly.**

- PP-OCRv5's recogniser is Chinese-centric multilingual. Its **OCR-B and MRZ
  performance versus `ocrs` is unverified** and could go either way. OCR-B is a
  narrow, machine-readable typeface; a model tuned for natural-scene and
  CJK document text is not obviously better at it.
- The MRZ-charset-constrained retry pass in `crates/synthpass-ocr/src/lib.rs`
  (`A–Z 0–9 <`, beam-search decoding) is written against `ocrs`'s engine and
  would need re-implementing against a different dictionary and decode loop.
- `rten` operator coverage for Paddle's DBNet detector is unverified. Detection
  and recognition can be adopted independently — and Path B only needs the
  recogniser, so a detector that will not convert is not a blocker.
- Two engines means two downloads, two hash pins, and a larger binary for the
  musl `embedded-models` build.

**Sequencing.** Measure, then adopt.
[`providers/README.md`](../providers/README.md) is explicit that a claim not
backed by a `synthpass-bench` run is an anecdote, and both the runner and its
`--dump-ocr` probe already exist. The first step is a bake-off on the existing
corpus with the result recorded in [`../benchmarks/`](../benchmarks/) — not a
swap, and not a swap behind a feature flag either.

---

## 3. Unlimited-OCR as a vision-provider candidate — and the runway is shorter than it looks

[`vision/README.md`](../vision/README.md#what-belongs-here) framed the open
question as: can `llama-cpp-2` drive a multimodal GGUF at a usable version, or
does that mean patching `llama-cpp-sys-2`?
[`ADR-0005`](../decisions/ADR-0005-vision-provider-readiness.md) has since
answered the paper half of it, and the answer is more permissive than the
framing assumed: `mtmd` is a declared **off-by-default feature at the exact
version already pinned** — no version bump, no patching `llama-cpp-sys-2` — and
it ships a ~980-line safe Rust wrapper (`MtmdContext::init_from_file(mmproj_path)`,
`support_vision()`, `MtmdBitmap::from_file`, `tokenize`/`encode_chunk`,
`eval_chunks`), not a bare FFI surface. That was established at the 0.1.151 pin
and re-verified at 0.1.154 when the pin moved: same entry points, same roles,
987 lines. The claim is about the crate, not about one version of it.

So the cost of *trying* a vision provider is enabling a feature flag, not
building a binding. That changes this item's economics: it is the one proposal
here whose blocker turned out to be smaller than advertised. This model is a
legitimate candidate for that spike, and it belongs on the candidate list for
two reasons the others do not share.

**For.** The weights are **MIT** — no research-licence problem, the thing that
disqualified Qwen2.5-3B. And it activates **0.5B parameters** of a 3B MoE per
token, which is CPU-plausible in a way that a 3B dense VLM is not; on a CPU-only
build that is the difference between a candidate and a curiosity. DeepSeek-OCR
support reached llama.cpp (PR #17400), and community GGUF quantisations of
Unlimited-OCR with `mmproj` projectors exist.

**Against.** The remaining unknowns are empirical, not structural. ADR-0005 did
the source inspection, not a run: nothing has yet loaded weights through
`MtmdContext` on our CPU envelope, and the ADR is careful that its own module
doc is flagged *"experimental and subject to breaking changes."* Whether
llama.cpp implements R-SWA is
unverified and most likely it does not, so a GGUF run would use ordinary
attention. For one page that is fine — but be clear that it makes the paper's
mechanism irrelevant to the deployment, leaving only the DeepSeek-OCR baseline
plus whatever the continued training bought. And the GGUF repos are community
uploads rather than vendor-published, so the `SYNTHPASS_MODEL_SHA256` pin in
`crates/synthpass-llm/src/verify.rs` is doing real work here, not ceremony.

**The tension to settle before, not during.** This model emits Markdown from
pixels. Under the M7 contract it is a `Recognizer`, not a `FieldReader` — it
would *be* the OCR engine. [`vision/README.md`](../vision/README.md) says plainly
that the VLM is not the OCR engine, and [`../../CLAUDE.md`](../../CLAUDE.md)'s OCR
philosophy puts deterministic preprocessing and multiple passes first. Those are
not obstacles to route around; they are the project's position. Registering an
end-to-end VLM as a `Recognizer` is an ADR
([`../decisions/`](../decisions/)), not a PR.

---

## Rejected, recorded so nobody re-derives them

- **OmniDocBench as a yardstick.** It scores table structure, formula
  recognition and reading order over dense documents. A TD3 passport has no
  tables, no formulas, and a reading order that is fixed by ICAO 9303. The
  benchmark the paper wins is not measuring anything we ship.
- **R-SWA as something to implement.** It is an attention-kernel and
  KV-cache-eviction change inside a 3B CUDA decoder. There is no artefact in
  this workspace it maps onto. The analogy to `vision/README.md`'s narrow-ask
  design — static reference context, bounded output window — is real and is
  still only a framing, and the 80/20 filter in
  [`README.md`](README.md) says a framing that becomes no trait, struct,
  algorithm, stage, benchmark or knob does not earn a note.
- **A caution the paper hands us for free.**
  [§5.4](../papers/Unlimited_OCR_Works.md#54-long-horizon-parsing) reports that
  its multi-page errors concentrate where "small text in the PDF is difficult to
  discern" at DeepEncoder's 1024×1024 Base mode — the authors' own diagnosis,
  and they are explicit it is the resolution rather than R-SWA losing its place.
  On a passport the small text *is* the payload: the MRZ. That is a specific,
  sourced reason to doubt that any end-to-end VLM at a fixed page resolution
  replaces deterministic MRZ reading, and it argues for the posture
  [`project_principles.md`](../project_principles.md) already holds rather than
  against it.
