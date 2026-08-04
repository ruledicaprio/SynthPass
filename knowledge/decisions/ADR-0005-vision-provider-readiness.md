# ADR-0005 — Vision-provider readiness: can `llama-cpp-2` drive a multimodal GGUF at all

**Status:** Proposed
**Date:** 2026-08-04

## Context

`knowledge/ROADMAP.md`'s v1.3.0 non-goals section is explicit: "No vision-language model
in v1.3.0. `Capability.vision` is `false` for every registered provider... Moondream is a
**v1.4.0 spike**, and the spike's first job is to establish whether `llama-cpp-2` can
drive a multimodal GGUF at a usable version at all."

That spike has not started. This ADR does the paper half of "establish whether it can at
all" ahead of it — inspecting the pinned `llama-cpp-2` dependency and its actual Rust API
surface, so the eventual spike starts from a verified answer instead of re-deriving it
under time pressure with weights already downloaded. **This ADR changes no code.** It
answers one question — is there anything to spike against — and records the answer.

Two things make this answerable without running anything: `crates/synthpass-llm/
Cargo.toml` pins an exact `llama-cpp-2` version, and that version's full source (via its
`llama-cpp-sys-2` dependency, which vendors llama.cpp itself) is present in the local
`cargo` registry checkout, not just its published API docs.

## Decision

**Record the following as the answer to "can `llama-cpp-2` drive a multimodal GGUF at
all," verified against the pinned version, not assumed from llama.cpp's general
reputation for supporting vision models:**

### Version and feature flags

- `crates/synthpass-llm/Cargo.toml` pins `llama-cpp-2 = { version = "0.1.151", features =
  ["sampler"] }`. No multimodal feature is enabled today.
- `llama-cpp-2` 0.1.151 declares an `mtmd` feature: `mtmd = ["llama-cpp-sys-2/mtmd"]`.
  Not on by default; nothing in the current dependency tree activates it. **Enabling it
  requires no version bump** — `mtmd` already exists at the exact version this repo pins.
- The latest published `llama-cpp-2` as of this writing is 0.1.153 (`cargo info
  llama-cpp-2`), two patch releases ahead of the 0.1.151 pin. Not required for anything
  below — `mtmd` is present at 0.1.151 already — but worth taking as a normal opportunistic
  bump before any spike work starts, checked against that bump's own diff rather than
  assumed safe here.

### What the `mtmd` feature actually provides

Enabling it is not a bare FFI surface — `llama-cpp-2` 0.1.151 ships a genuine, sizeable
safe Rust wrapper: `src/mtmd.rs`, roughly 980 lines, with doc comments and doctests of its
own (flagged `# Warning: This API is experimental and subject to breaking changes` in its
own module doc — noted here honestly, not smoothed over). Its public surface includes:

- `MtmdContext::init_from_file(mmproj_path, ...)` — loads a CLIP-style vision projector
  (`mmproj-*.gguf`) alongside the base language-model GGUF.
- `MtmdContext::support_vision()` / `support_audio()` — capability probes on the loaded
  projector.
- `MtmdBitmap::from_image_data(...)` / `from_file(...)` / `from_buffer(...)` — raw pixels
  or an image file in, ready for encoding.
- `MtmdContext::tokenize(...)` and `encode_chunk(...)` — turns interleaved text + image
  input into token/embedding chunks the base model can consume.
- `MtmdInputChunks::eval_chunks(...)` — runs the multimodal input through the context.
- `MtmdContextParams::use_gpu` — independent of, but composable with, ADR-0004's
  `cuda`/`vulkan` features.

This is not a stub. If accepted, a spike targeting a supported vision architecture would
be extending an existing, maintained wrapper — not writing FFI bindings from scratch.

### What is *not* confirmed: Moondream specifically

`llama-cpp-sys-2`'s vendored llama.cpp (under `tools/mtmd/`) recognizes vision
architectures by an internal `PROJECTOR_TYPE_*` enum in `clip.cpp` — Gemma3/Gemma4,
Qwen2VL/Qwen2.5VL/Qwen3VL, LLaVA-family (`PROJECTOR_TYPE_MLP`/`LDP`/`LDPV2`), Pixtral,
InternVL, MiniCPM-V, LLaMA4, and roughly two dozen others as of this vendored snapshot.
**No `PROJECTOR_TYPE_MOONDREAM` (or anything matching "moondream," case-insensitive)
appears anywhere in that file or the rest of the vendored `tools/mtmd/` tree.** This does
not prove Moondream cannot run — some vision encoders map onto one of the generic
projector types (the `MLP`/`LDP` family exists precisely because several LLaVA-derived
architectures share a projector shape) — but it means **no direct evidence exists, in this
snapshot, that a Moondream GGUF + its mmproj file loads and runs through this path**.
That is exactly the open question a real spike (with an actual Moondream GGUF and mmproj
file in hand) would have to answer, and it is the honest state of the evidence, not a
"probably fine."

### What an upgrade or the spike itself would drag into the tree

- Enabling `mtmd` alone adds no new *Rust* crate — it activates code already vendored
  inside `llama-cpp-sys-2`. The build-time cost is compiling llama.cpp's `tools/mtmd/`
  (primarily `clip.cpp`, a CLIP-family vision-encoder implementation) in addition to the
  base `llama.cpp` library already built today.
- It does add a **runtime artifact requirement** an eventual provider does not have
  today: a second GGUF file (the `mmproj-*.gguf` vision projector) alongside the base
  language model, both needing the same offline-checksum-verified bootstrap
  `synthpass-llm/src/verify.rs` already applies to the text-only Qwen weight
  (`SYNTHPASS_MODEL_PATH`/`SYNTHPASS_MODEL_SHA256`, per `knowledge/ROADMAP.md`'s Future
  Work section) — that verification path would need a second pinned hash, not a new
  mechanism.
- No new licensing category is introduced by the `mtmd` feature itself (same
  `llama-cpp-2`/`llama-cpp-sys-2` MIT OR Apache-2.0 terms); whatever Moondream checkpoint
  a spike eventually pulls in carries its own model license, to be checked against
  `knowledge/LICENSING.md` at that time — not evaluated here, since no weights are being
  proposed by this ADR.

### 5a is the readiness step already taken

Before any vision-capable provider could be useful, the pipeline has to actually hand it
pixels. `crates/synthpass-pipeline/src/lib.rs`'s `ocr_and_tier1` builds the `DocumentContext`
consulted at the Tier-1/routing stage; until this milestone that context never carried
`image` at all (`DocumentContext::from_text(&markdown).with_recognition(&recognition)`,
no `.with_image(...)`), so a vision-capable provider registered ahead of `MrzReader` would
have received `None` regardless of whether `llama-cpp-2` could technically drive a
multimodal model. That gap is now closed (`.with_image(input)` added to the Tier-1
context; see `crates/synthpass-pipeline/src/lib.rs`'s `ocr_and_tier1`, and the regression
test `tier1_context_carries_the_image_path_for_a_vision_capable_provider`, which registers
a `Capability::vision = true` test double — same fixture pattern as `provider_bench.rs`'s
`vision_provider_gets_not_applicable_instead_of_a_rate` — and asserts it receives the
on-disk image path). That work stands on its own regardless of what this ADR concludes
about `llama-cpp-2`: the pipeline no longer has a structural reason to starve a vision
provider of pixels, independent of whether one exists yet.

## Alternatives

**Skip the paper spike; go straight to downloading Moondream weights and trying `mtmd`
directly.** Rejected for this ADR's purpose specifically (not for the eventual v1.4.0
spike, which will still need to do exactly that): ROADMAP's stated non-goal is that
*nothing* pulls in vision weights or mmproj bindings in v1.3.0, and this document exists
to answer the "can it at all" half of the question without violating that boundary. The
actual download-and-try step belongs to the v1.4.0 spike this ADR is scoped ahead of.

**Assume `mtmd` support based on `llama-cpp-2`'s public reputation / llama.cpp's upstream
README, without checking the pinned version.** Rejected — this is exactly the failure
mode ADR-0004 documents for the original GPU draft (`rten::cuda` claims that do not exist
in the pinned version). Verifying against `Cargo.lock` and the local registry checkout
costs a few minutes and turns an assumption into a citation.

**Wait for a newer `llama-cpp-2` release before evaluating multimodal support at all.**
Rejected. `mtmd` is present at the exact version already pinned (0.1.151); there is
nothing to wait for.

## Consequences

**Positive:**
- The v1.4.0 Moondream spike now starts with a verified "yes, `llama-cpp-2` has a real
  multimodal API, no version bump required to reach it" instead of spending its own time
  budget re-deriving that.
- The specific open risk — whether Moondream's architecture is one `clip.cpp` recognizes —
  is named explicitly, so the spike's first concrete task is unambiguous: load a real
  Moondream mmproj file through `MtmdContext::init_from_file` and see whether
  `support_vision()` returns `true` or the load errors on an unrecognized projector type.

**Negative / open, honestly:**
- Moondream support is genuinely unconfirmed, not "probably fine" — this ADR does not
  reduce that uncertainty, only locates it precisely (architecture-recognition risk in
  `clip.cpp`, not an API-surface risk in `llama-cpp-2`'s Rust bindings).
- `mtmd`'s own module doc calls the API experimental; a spike built against it should
  expect breaking changes on a future `llama-cpp-2` bump, and pin accordingly.
- This ADR does not evaluate Moondream's model license, VRAM/RAM footprint, output
  quality, or latency — none of that is knowable without the weights in hand, which stays
  out of scope for v1.3.0 per ROADMAP's non-goal.

**Non-goals this ADR keeps true, unchanged:**
- No weights are downloaded or referenced by path/hash anywhere in this repo as a result
  of this ADR.
- No new Cargo dependency is added; `mtmd` is a feature of a dependency already present.
- No provider in this repo sets `Capability::vision = true` — `synthpass-die/src/
  mrz_reader.rs`'s `declares_itself_free_and_deterministic` test
  (`assert!(!cap.vision, "no provider ships vision in v1.3.0")`) stays green, and
  `synthpass-llm`'s `LlmFieldReader` stays text-only.

## References

- [`../ROADMAP.md`](../ROADMAP.md) — the v1.3.0 non-goals section this ADR's Context
  quotes verbatim, and the Moondream v1.4.0 spike it establishes readiness for.
- [`ADR-0004-gpu-acceleration.md`](ADR-0004-gpu-acceleration.md) — the `mtmd` finding
  first surfaced while verifying that ADR's `llama-cpp-2` GPU-feature claims; this ADR is
  where it is actually evaluated.
- `llama-cpp-2` 0.1.151 source, `src/mtmd.rs`: local registry checkout at
  `~/.cargo/registry/src/index.crates.io-*/llama-cpp-2-0.1.151/src/mtmd.rs` (not
  redistributed here; inspect the checked-out dependency directly to reproduce this
  ADR's findings).
- `llama-cpp-sys-2` 0.1.151's vendored llama.cpp, `tools/mtmd/clip.cpp` — the
  `PROJECTOR_TYPE_*` enum referenced above.
- `crates/synthpass-die/src/mrz_reader.rs` — `declares_itself_free_and_deterministic`,
  the test that keeps "zero vision implementations" true.
- `crates/synthpass-pipeline/src/lib.rs` — `ocr_and_tier1`'s `DocumentContext` construction
  (the 5a fix) and its regression test.
