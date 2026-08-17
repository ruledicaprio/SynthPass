# ADR-0004 — GPU acceleration as an optional, feature-gated path

**Status:** Accepted (2026-08-17, `cuda` only — see amendment below)
**Date:** 2026-08-04 (supersedes the unnumbered `ADR-00XX-GPU-Integration.md` draft of
2026-08-03, which was never indexed and contained several unverified claims — see
"What changed from the original draft" below)

## Relationship to the constitution

`SYNTHPASS_ENGINEERING_CONSTITUTION.md` §4.1 lists **GPU acceleration for OCR and LLM**
under "Future Vision" and is otherwise silent on GPU support one way or the other — the
constitution's actual non-goals (§4) name cloud integration, model training, real-time
video, and non-ICAO documents (for the currently committed milestones); GPU acceleration
is not among them. This ADR does not contradict or override anything currently in force:
it **proposes** filling in that Future Vision item, with CPU staying the default and only
path until a follow-up ADR (or this one, amended) records it as Accepted and something
ships. Nothing here authorizes writing GPU code today.

## Context

### Current state (verified against this workspace's actual pinned dependencies)

- SynthPass is CPU-only today. Every OCR and LLM code path runs on CPU; no GPU feature
  flag exists anywhere in the workspace.
- OCR backend: `synthpass-ocr`, built on `ocrs` 0.12.2 and `rten` 0.24.0
  (`crates/synthpass-ocr/Cargo.toml`).
- LLM backend: `synthpass-llm`, built on **`llama-cpp-2`** 0.1.151 with only the
  `sampler` feature enabled (`crates/synthpass-llm/Cargo.toml`) — **not**
  `llama-cpp-sys` as the original draft of this ADR named it. `llama-cpp-sys` is an
  unrelated, unmaintained crate; this workspace depends on `llama-cpp-2`, which in turn
  depends on `llama-cpp-sys-2` (a different, actively maintained sys crate, currently
  pinned at the matching 0.1.151). Getting this name right matters because every feature
  flag below is spelled against `llama-cpp-sys-2`, not `llama-cpp-sys`.
- Performance budgets: `SYNTHPASS_ENGINEERING_CONSTITUTION.md` §15 — OCR < 700 ms,
  LLM repair < 2 s, end-to-end < 3 s (typical).
- M4 Tier-1 hit rate: **the original draft's "Currently 68%" line is not in version
  control, and is not the canonical baseline.** (This sentence originally read "is not a
  number found anywhere in this repo," which was too strong — see the amendment under
  point 4 below. It was reproducible; it was just never committed.)
  `knowledge/ROADMAP.md`'s M4 entry records the measured,
  honestly-reported figure as **~55% (100-seed, clean profile) as of PR #30**, with a 30%
  CI floor. This ADR cites that figure and no other. (A much smaller ad hoc sample taken
  while drafting this ADR — 30 seeds, TD3, clean profile, `synthpass-bench` — measured
  23/30 = 76.7%; that is a 30-document sample against a corpus that has since gained
  TD1/TD2 rendering, not a replacement for ROADMAP's 100-seed baseline, and is recorded
  here only so a reader can see where the discrepancy between drafts came from — it is
  not a claim about the current, canonical hit rate.) GPU acceleration changes *speed*,
  not the OCR/LLM algorithms themselves, so it is not expected to move this number in
  either direction; that expectation is untested, see Consequences.

### What changed from the original draft

The original `ADR-00XX-GPU-Integration.md` (unnumbered, not indexed, dated 2026-08-03)
made four claims that do not hold against this repository as it actually exists, checked
against the pinned `Cargo.lock` versions and the local `cargo` registry checkout rather
than against `rten`'s/`llama-cpp-2`'s documentation in the abstract:

1. **`rten/cuda` and `rten/vulkan` do not exist.** The pinned `rten` 0.24.0's complete
   feature list is `all-ops`, `default`, `fft`, `mmap`, `onnx_format`, `random`,
   `rten_format`, `wasm_api` — no `cuda`, no `vulkan`. A `grep` across the crate's vendored
   source for `cuda`/`vulkan`/`gpu` (case-insensitive) returns nothing. `rten::cuda::
  is_available()` and `rten::CudaError`, both quoted as code in the original draft, do not
   exist in this version and would not compile.
2. **`llama-cpp-sys` was the wrong crate name** (see above) — the correct name is
   `llama-cpp-2` (with `llama-cpp-sys-2` underneath), and the corrected name is used
   throughout this document.
3. **`llama-cpp-2`'s GPU claims, by contrast, verify.** The pinned 0.1.151 genuinely
   exposes `cuda`, `vulkan`, `metal`, `rocm`, and `opencl` Cargo features, each mapping
   directly to the matching `llama-cpp-sys-2` feature (e.g. `cuda = ["llama-cpp-sys-2/
   cuda"]`). This is the one part of the original draft's GPU story that was actually
   right, once the crate name is corrected.
4. **The "68% Tier-1 hit-rate" figure is not the canonical baseline and does not match
   ROADMAP.md.** See above.

   **Amended 2026-08-05.** This point originally read "has no citation and does not match
   ROADMAP.md," which overshot. The figure was not invented: a `bench-report.json` sat
   untracked in the repo root — **2026-07-30 00:59:46 UTC, `--count 50 --seed 0 --profile
   clean`, `hits: 34`, `hit_rate: 0.68`** — reproducing 68% exactly. That is CI's own
   invocation (`.github/workflows/ci.yml`'s accuracy-gate step), so the draft's author had
   run the gate locally and quoted the result.

   The substantive objection is unchanged and still correct: a 50-seed clean-profile run
   is not ROADMAP's 100-seed baseline, so citing it as "currently 68%" was wrong. But the
   file was gitignored and never committed, which is why nobody could find it — "no
   citation" was fair as to *provenance in the tree* and unfair as to *fabrication*. The
   artifact has since been deleted as repo-root clutter; its numbers are recorded here so
   the provenance survives the file.

   Worth keeping as a method note: "I could not find it" and "it does not exist" are
   different claims, and an untracked file sitting in the working tree is exactly the gap
   between them.

The original draft's broken References section (linking
`SYNTHPASS_ENGINEERING_CONSTITUTION.md` and `ROADMAP.md` as if this file lived at the
repo root, when it lives in `knowledge/decisions/`) is also fixed below —
`scripts/check-doc-links.sh` now passes against this file.

### Customer motivation (unchanged from the original draft)

A customer requested GPU acceleration for batch throughput and single-document latency,
citing available hardware: a ZOTAC GTX 970 (Maxwell, compute capability 5.2, 3.5 GB
GDDR5, CUDA 11.x/12.x and Vulkan 1.1+ capable). This remains the concrete motivating case;
no GPU hardware has been used to validate any number in this document, which is why every
performance figure below is marked as an *expectation*, not a measurement.

## Decision

**Propose** GPU acceleration as an optional, compile-time-gated path for the LLM (Tier 2)
backend only, via `llama-cpp-2`'s existing `cuda`/`vulkan` Cargo features — CPU stays the
default and the only path shipped until a follow-up records this ADR (or a successor)
Accepted, an actual implementation lands, and it is benchmarked on real GPU hardware.

**OCR (`synthpass-ocr`/`rten`) is explicitly out of scope for this proposal**, not a
parallel work item: the pinned `rten` has no GPU path at all (see above), so "OCR GPU
acceleration" would first require either an `rten` upgrade to a version that adds one (no
such version is confirmed to exist as of this writing — this was not checked against
`rten`'s upstream changelog, only against the version this repo currently pins) or a
different OCR engine entirely. Either is a materially bigger decision than turning on an
existing feature flag, and deserves its own ADR with its own verification once (or if)
that becomes concrete, rather than being bundled into this one under an "OCR GPU" section
whose only working code today would be for the LLM half.

### What would actually ship, if accepted

```toml
# crates/synthpass-llm/Cargo.toml — additive, default-off features
[features]
cuda = ["llama-cpp-2/cuda"]
vulkan = ["llama-cpp-2/vulkan"]
```

No new dependency is added — `cuda`/`vulkan` are features of the dependency already in
the tree. Enabling either requires the matching system-level toolkit (CUDA Toolkit /
Vulkan SDK + driver) to be present on the *build* machine; neither is vendored, matching
`SYNTHPASS_ENGINEERING_CONSTITUTION.md` §13's "no external dependencies except those
vetted and pinned" — a system toolkit is not a new Rust dependency, but it is a new build
precondition, which is why this stays feature-gated and off by default rather than
becoming a default build requirement.

Runtime selection (sketch, **not implemented, not compiled, and not asserted to be
correct API usage** — this is the shape a follow-up implementation PR would need to fill
in and verify against the real `llama_cpp_2` API surface, not code to copy verbatim):

```rust
// crates/synthpass-llm/src/backend.rs (does not exist yet)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmBackend {
    Cpu,
    #[cfg(feature = "cuda")]
    Cuda,
    #[cfg(feature = "vulkan")]
    Vulkan,
}
```

`llama-cpp-2`'s actual GPU knob is `LlamaModelParams`'s `n_gpu_layers` (how many
transformer layers to offload to the device) plus the backend the crate was *compiled*
with — GPU selection in this ecosystem is a build-time feature choice, not a pure runtime
switch the way the original draft's `OcrBackend::auto()`/`rten::cuda::is_available()`
sketch implied for OCR. A real implementation needs to read `llama_cpp_2`'s actual
`LlamaModelParams`/context-params API (`crates/synthpass-llm/src/*` already wraps
`llama_cpp_2`; whatever this becomes should extend that wrapper, not bypass it) rather
than the enum sketch above, which exists only to show the shape of "one new default-off
feature flag per backend," not a working design.

### Multimodal note (informs, does not authorize, ADR-0005)

While verifying `llama-cpp-2`'s GPU features for this ADR, its `mtmd` feature
(`mtmd = ["llama-cpp-sys-2/mtmd"]`) was also found to carry a real, non-trivial Rust
module (`llama_cpp_2::mtmd`, ~980 lines) wrapping llama.cpp's multimodal (vision/audio)
support — `MtmdContext::init_from_file`, `MtmdBitmap::from_image_data`/`from_file`, GPU
toggle (`MtmdContextParams::use_gpu`), and image-token budget controls. This is
independent of `cuda`/`vulkan` (it is its own feature) and independent of this ADR's
LLM-GPU proposal, but it is the concrete finding `ADR-0005-vision-provider-readiness.md`
builds on for evaluating whether `llama-cpp-2` can drive a multimodal GGUF at all — noted
here because it surfaced during the same verification pass, not proposed for
implementation by this ADR.

## Alternatives

**Do nothing (stay CPU-only indefinitely).** Rejected for now, not permanently: the
customer request is concrete and the feature-gate cost is low (two lines in one
`Cargo.toml`, default-off). But "reject nothing" is also not automatic — this ADR
proposes evaluating it, not committing to ship it; see Consequences for what has to be
true first.

**Ship GPU as the default backend, falling back to CPU.** Rejected. Breaks the "same
input → same output, regardless of environment" expectation implicit in
`SYNTHPASS_ENGINEERING_CONSTITUTION.md` §2's determinism priority the moment two machines
resolve to different backends by default, and makes every user's first run depend on
undocumented system-toolkit detection succeeding. Feature-gated and explicit stays
closer to "a local dependency beats a cloud API" — the constitution's own worked
example — read as "the simplest correct default beats an auto-detected one."

**Vendor/statically link CUDA or Vulkan into the binary.** Rejected. Both toolkits are
large, platform- and driver-version-specific, and would turn a ~50 MB binary-size budget
(§15) into a multi-GB one, or require runtime dynamic loading of system libraries the
build didn't control — the opposite of this project's offline-first, single-binary
posture (`SYNTHPASS_ENGINEERING_CONSTITUTION.md` §3/§13).

**Write a new pure-Rust GPU backend instead of using `llama-cpp-2`'s existing features.**
Rejected. `llama-cpp-2`/`llama.cpp` already carries mature CUDA and Vulkan backends,
audited and maintained upstream; re-implementing GPU tensor ops in-house is a multi-month
undertaking with no offsetting benefit — "a local dependency beats a cloud API" scales
down to "a maintained dependency beats a from-scratch reimplementation" here too.

**Solve OCR-GPU by bundling it into this same ADR with speculative `rten` API calls.**
Rejected, which is the whole reason this draft exists: the original draft did exactly
this, and every one of those calls (`rten::cuda::is_available`, `rten::CudaError`,
`rten/cuda`, `rten/vulkan`) is unverifiable against the pinned version and would not
compile. Splitting OCR out, rather than patching the fabricated calls with real ones,
is deliberate: nobody has yet checked whether *any* released `rten` version has a GPU
path, and that check belongs in whatever ADR eventually proposes it, backed by its own
verification, not folded into this one's LLM-only, verified proposal.

## Consequences

**Positive, if eventually implemented and accepted:**
- Materially faster Tier-2 (LLM repair) latency and batch throughput on GPU-equipped
  hardware, for users who opt in.
- No effect on users who don't build with the feature — `cuda`/`vulkan` off by default
  keeps every existing deployment, including fully air-gapped ones with no GPU driver at
  all, unchanged.
- No new runtime dependency category: the Rust dependency graph is unchanged; only a
  system-level build precondition is added, and only for opt-in builds.

**Negative / open, honestly:**
- **Nothing here is benchmarked.** Every throughput/latency number in the original
  draft's "Performance Expectations" table was a projection, not a measurement, and has
  been removed from this version rather than carried forward uncorrected — see Principle
  6 in `knowledge/benchmarks/README.md`: "a constant with no measurement behind it does
  not ship." A real number requires a GPU-equipped CI runner or a manual run on hardware
  like the customer's GTX 970, neither of which exists today.
- **Determinism is unverified across backends.** `SYNTHPASS_ENGINEERING_CONSTITUTION.md`
  §2 ranks deterministic behavior above performance; floating-point reduction order
  commonly differs between CPU and GPU kernels, which can change LLM sampling outcomes at
  the margin even at temperature 0. This has to be checked (same prompt, same seed, CPU
  vs. GPU output) before this ADR could move to Accepted, not assumed.
- **CI has no GPU runner.** Testing this path at all requires either a self-hosted
  GPU-equipped runner or manual verification before every release that touches
  `synthpass-llm`'s GPU-feature code — an ongoing maintenance cost this ADR does not yet
  have a concrete answer for.
- ~~**`llama-cpp-2` is pinned at 0.1.151; 0.1.153 is the latest available on crates.io as
  of this writing.**~~ **Done — the pin is now 0.1.154** (vendoring llama.cpp b10200).
  This was never a blocker: the `cuda`/`vulkan`/`mtmd` features this ADR and ADR-0005
  rely on were already present at 0.1.151 and remain present at 0.1.154. The bump was
  taken on its own, checked against its own `cargo update -p llama-cpp-2` diff as this
  note asked — lockfile movement was confined to the two llama crates. 0.1.154's three
  ported upstream API breaks do not reach this workspace; see the bump's commit message
  and ADR-0005's version note for the itemised reasoning.
- **`rten` GPU support remains completely unscoped.** This ADR explicitly does not claim
  a path exists; a customer expecting OCR speedup from "GPU acceleration" would not get
  one from what this ADR proposes.

## Amendment 2026-08-17 — Accepted, `cuda` only, validated on real hardware

This ADR moves to **Accepted** for the `cuda` feature only. `vulkan` stays
proposed-but-unimplemented — nothing in this amendment validates it, and there
is no reason to ship an unverified path alongside a verified one.

**What changed since Proposed:** a customer machine running this workspace
turned out to have the exact hardware this ADR's motivating case named — an
NVIDIA GTX 970 (Maxwell, compute capability 5.2, 4 GB VRAM) — plus the full
build chain `llama-cpp-sys-2`'s CMake build needs on Windows: CUDA Toolkit
13.3, driver 582.28, cmake 4.4.0, and Visual Studio 2022 Community's C++
(VC.Tools.x86.x64) workload. That closes the ADR's two biggest listed gaps —
"nothing here is benchmarked" and "determinism is unverified across
backends" — with real measurements instead of projections; see
`crates/synthpass-llm/Cargo.toml`'s `cuda` feature and
`crates/synthpass-llm/src/lib.rs`'s `#[cfg(feature = "cuda")]` GPU-layer-offload
wiring in `NativeLlm::load` for what actually shipped.

**Determinism result: confirmed identical.** `tests/parity.rs --ignored
--nocapture` run against the same 6-fixture sample set, CPU vs `--features
cuda`: field match rate **16/42 (38.1%) on both**, and the actual (not just
matched/mismatched) extracted text is character-for-character identical
across every field on every fixture (e.g. the malformed date renderings
`"13 ABR/AVR 90"` and `"1JUL/JUI/JUL31"` on the `2022_cetis_terra_condifea...`
fixture appear verbatim on both backends). Greedy decoding (`build_sampler`'s
`LlamaSampler::greedy()`) is stable across the CPU/CUDA split on this
`llama-cpp-2` 0.1.154 build. This does not generalize to future engine
bumps — see `knowledge/technical_debt.md`'s existing "Nothing in CI exercises
the real inference engine" entry, unchanged by this finding.

**Benchmark result: real, measured, on a card with no Tensor Cores.** The
GTX 970 is Maxwell — FP32 ALUs only, no Tensor Cores, no native FP16 math —
so this is not a "modern GPU" number, but it is a real one:

| Test | CPU | CUDA (GTX 970) | Speedup |
|---|---|---|---|
| `native_llm_e2e` (1 unary + 1 streaming extract) | 51.49s | 20.22s | ~2.5x |
| `parity` (6 documents) | 181.33s (3m01s) | 73.67s | ~2.46x |

All 28 transformer layers offloaded to `CUDA0` (confirmed via llama.cpp's own
`load_tensors: layer N assigned to device CUDA0` logging), well within the
970's fast 3.5 GB VRAM segment (model + KV cache measured at ~991 MiB
resident on-device).

A larger real-world sample confirms the same shape: `provider-bench
--real-specimens --features cuda` (no `--limit`, the full 221-document
`samples/` corpus) reports the `llm` provider's mean latency at **11.4s/doc**
on this card — slower than the clean 6-fixture parity set's per-doc average
(longer, messier real-world OCR text), but the full run (both providers,
221 docs) still completed in 1h37m wall time, a corpus size that would not
have been practical to run to completion on CPU in one sitting at the
30-60s/doc rate observed earlier. Field-match rate on this larger, unfiltered
sample (37.6%) lines up with the 6-fixture parity harness's 38.1%, i.e. the
GPU path isn't quietly producing worse extractions at scale.

**Build-chain notes for anyone repeating this** (not part of the shipped
code, but real friction hit during validation, worth recording so the next
person doesn't re-derive it):
- **CUDA 13.0+ dropped Maxwell/Pascal/Volta support entirely** — `nvcc
  -arch=sm_52` fails with `Unsupported gpu architecture 'sm_52'` on the
  13.3 toolkit this machine had installed. CUDA 12.9 is the last release
  that still targets `sm_52`; a second toolkit install (compiler/libraries
  only, no display-driver component — the existing driver already supports
  the newer CUDA runtime and must not be touched) was required alongside
  the existing 13.3 install.
- **`llama-cpp-sys-2`'s CMake build must be pointed at the right toolkit
  explicitly.** With the default Visual Studio generator, CMake's CUDA
  compiler detection picked the newest installed toolkit (13.3) regardless
  of the `CUDA_PATH` environment variable, because MSBuild's CUDA
  "BuildCustomizations" props/targets resolve independently of the process
  environment. Switching the generator to Ninja (`CMAKE_GENERATOR=Ninja`,
  via `pip install ninja`) made the build respect `CUDA_PATH`/`PATH`
  correctly.
- **`CUDAARCHS=52`** (a CMake ≥3.20 environment-variable seed for
  `CMAKE_CUDA_ARCHITECTURES`, no source changes needed) keeps the build from
  compiling kernels for architectures this card can't use.
- A stale CMake cache directory from a prior (wrong-toolkit) build attempt
  silently kept using the old toolkit's headers even after `CUDA_PATH` was
  corrected — `cargo clean -p llama-cpp-sys-2 -p llama-cpp-2` (the package
  name only, not `--features`, which `cargo clean` does not accept) is
  required after any toolkit change, not just a plain rebuild.

**Still open:**
- `vulkan` remains unimplemented and unverified.
- No CI runner has this hardware — the GPU path is manually validated per this
  amendment, not continuously tested. `knowledge/technical_debt.md`'s "Nothing
  in CI exercises the real inference engine" entry still applies to both
  backends.
- `rten`/OCR GPU support remains completely out of scope, unchanged from the
  original proposal.
- The CUDA 12.9 vs. 13.3 toolkit split is a real, ongoing maintenance cost on
  Maxwell-class hardware specifically — newer cards would build cleanly
  against whatever toolkit is already installed. This is hardware-specific
  friction, not something the crate/feature-flag design can absorb.

## References

- [`SYNTHPASS_ENGINEERING_CONSTITUTION.md`](../../SYNTHPASS_ENGINEERING_CONSTITUTION.md)
  — §2 (priority order), §3/§13 (offline-first, dependency vetting), §15 (performance
  budgets).
- [`../ROADMAP.md`](../ROADMAP.md) — M4's measured Tier-1 hit-rate baseline (~55%,
  100-seed, clean profile, PR #30) and the M6 GPU/Future Vision context.
- [`knowledge/benchmarks/README.md`](../benchmarks/README.md) — "Principle 6: a constant
  with no measurement behind it does not ship," the standard this ADR's Consequences
  section holds itself to.
- `llama-cpp-2` 0.1.151 on crates.io: <https://crates.io/crates/llama-cpp-2/0.1.151> —
  feature list verified via `cargo info llama-cpp-2` and the local registry checkout of
  its source (`~/.cargo/registry/src/.../llama-cpp-2-0.1.151/Cargo.toml`), not from
  docs.rs prose.
- `rten` 0.24.0 on crates.io: <https://crates.io/crates/rten/0.24.0> — feature list and
  absence of any `cuda`/`vulkan`/GPU source verified the same way.
- [`ADR-0005-vision-provider-readiness.md`](ADR-0005-vision-provider-readiness.md) — the
  `mtmd` finding this ADR surfaced but does not itself propose acting on.
