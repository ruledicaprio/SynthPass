# Technical debt

Not bugs. Not TODOs. Things that work but whose shape will cost us later, with an
honest severity and an estimate. Reviewed at each milestone close.

Proposed in [archive/KNOWLEDGE.md](archive/KNOWLEDGE.md). Add an entry when you *choose* not to
fix something — the value of this file is the deferred decisions, not the list of
known imperfections.

---

## High

### OCR confidence is a character-plausibility proxy, not a model score

`geometry::text_sanity` computes "fraction of plausible characters" because
`ocrs` exposes no per-character probability — its `TextChar`/`TextLine` carry
only `char` and `rect`, and the CTC decode probabilities are computed internally
and dropped. The module doc is honest about this, but every downstream consumer
still reads a field called `confidence`.

**Consequence:** the routing engine's `text_sanity` signal is weaker than its
name suggests, and no threshold on it can be better than the proxy.
`RoutingPolicy.sanity_floor` (`synthpass-die/src/routing.rs`) is the one
place that would act on it, and it is deliberately `None` in
`v1_2_0_compatible()` — the routing policy's own doc comment cites this exact
gap as the reason no threshold has been set.

**Fix:** not "upstream a patch to `ocrs`, or fork the recognition loop" —
`ocrs`/`rten` are plain, unmodified crates.io dependencies with no
`[patch]` or vendor infrastructure in place today, so either option means
standing up and maintaining a fork before writing a line of the actual fix.
`knowledge/research/long-horizon-parsing.md` §2 works out a better path that
avoids upstream negotiation entirely: swap the recognizer to PaddleOCR
PP-OCRv5 converted to `.rten` (already a direct dependency) and write our
own CTC decode, so per-character probabilities land in code we control.
Costs are real and stated there honestly — PP-OCRv5's MRZ/OCR-B accuracy
vs. `ocrs` is unverified, the MRZ-charset beam-search retry pass would need
reimplementing against the new engine, and it's a second model to
download/hash-pin — so the recommended sequencing is a `synthpass-bench`
bake-off first, not a blind swap.

**Estimated effort:** the bake-off is the next concrete step; the 3–5 day
figure for the full swap (patch/fork framing) is retired along with that
framing — see `long-horizon-parsing.md` for the actual cost breakdown.

---

## Medium

### Three parallel lists of ICAO field names

- `synthpass_core::v2::ExtractionFields` — the schema, 10 fields
- `synthpass_bench::COMPARED_FIELDS` — what the benchmark scores
- `synthpass_llm::prompt::FIELDS` — what the prompt asks for, and the source
  `grammar.rs` generates the GBNF from

The third **deliberately differs** (it asks for `mrz_line`, omits
`personal_number`), and `grammar.rs`'s "prompt and grammar cannot drift"
invariant depends on it staying a Rust const. So this is not simply
de-duplicable. `v2::CoreField` unifies the first two; the third stays separate.

**Consequence:** adding an ICAO field means editing three places, and nothing
fails if you edit two.

**Fix:** a compile-time assertion that `CoreField` ⊇ `prompt::FIELDS` minus the
documented exceptions, so divergence is deliberate rather than accidental.

**Estimated effort:** half a day.

### Streaming bypasses the provider contract

`FieldReader::read` is unary. Streaming still goes through
`InferBackend::extract_stream` directly, because putting a
`tokio::sync::mpsc::Sender` on the trait would drag `tokio` into
`synthpass-die` and into every out-of-tree provider — for a concern that belongs
to `synthpass-serve`'s SSE transport.

**Consequence:** a third-party provider cannot stream. Acceptable while exactly
one provider streams; a problem the moment a second one wants to.

**This has already cost us once.** Because the streaming path assembles its own
`ExtractionV2` rather than receiving one from a `FieldReader`, a fix applied to
`process_document`'s Tier-2 assembly did not reach `process_document_stream` —
and `synthpass-serve` uses only the latter, so every web upload kept emitting the
record the fix had supposedly removed. The guard against a repeat is
`both_tier2_paths_produce_the_same_extraction` in `synthpass-pipeline`, which
asserts the two paths agree rather than checking either against a hand-written
expectation. That is a test, not a structure: it catches the next divergence
instead of preventing it.

**Fix:** a `StreamingReader` sub-trait in a separate crate, or a runtime-agnostic
sink abstraction.

**Estimated effort:** 2 days, and it should wait for a second streaming provider
to exist so the abstraction is designed against two cases rather than one.

---

### Nothing in CI exercises the real inference engine

Both tests that load the actual GGUF — `synthpass-llm/tests/native_llm_e2e.rs` and
`tests/parity.rs` — are `#[ignore]`, and the `Native LLM (real model, opt-in)` CI
job skips. Everything CI runs against `synthpass-llm` type-checks or mocks. So a
change that alters *what the model produces*, while compiling cleanly and passing
all 28 unit tests, merges green.

This now applies identically to the optional `cuda` feature
(`knowledge/decisions/ADR-0004-gpu-acceleration.md`): both tests were run
manually against `--features cuda` on a GTX 970 and produced byte-identical
output to the CPU run, but that check is a one-time manual pass on one machine,
not a CI gate. A future engine bump that changes CUDA-path output specifically
(e.g. a kernel numerics change) gets exactly the same silence this entry
already describes for the CPU path.

The `llama-cpp-2` 0.1.151 → 0.1.154 bump is the worked example. It vendors a new
llama.cpp (b10200), decoding is `LlamaSampler::greedy()` and therefore
deterministic, and new kernels are exactly what moves a greedy decode. CI had
nothing to say about it. The bump turned out clean — verified by running `parity`
on both versions on one machine (16/42 either way) — but *that verification was
manual and nothing required it*. The next engine bump gets the same silence, and
the person doing it may not think to check.

**Consequence:** the accuracy of the shipped Tier-2 path is unguarded between
releases. `synthpass-bench`'s CI gate covers Tier 1 (deterministic MRZ) and does
not run the LLM.

**Fix:** not simply "un-ignore them." They need the ~1 GB GGUF and ~4 minutes,
which is why they are opt-in, and `SYNTHPASS_MODEL_PATH` bootstrapping is
deliberately not a runtime fetch. The realistic shapes are a scheduled (not
per-PR) workflow that provisions the weight and records `parity`'s rate as a
tracked number, or a required manual checklist item on any PR touching
`synthpass-llm`'s dependencies. Recording the rate over time is the more valuable
half — a single pass/fail at a 25% floor would not have caught anything here
either.

**Estimated effort:** half a day for the scheduled workflow, plus whatever the
weight-provisioning story costs in CI.

## Low

### `ProviderId` is `&'static str`

This keeps metric-label cardinality bounded (a hard requirement — see
`CONTRIBUTING.md`'s PII checklist), but it means a provider loaded from a config
file or a dynamic library cannot register. That is currently fine: everything is
compiled in.

**Fix, when it matters:** an interned-string table with a bounded capacity, so
labels stay a closed set without requiring `'static`.

### `Method` has two variants and answers a question that is becoming three-valued

`Method::{MrzDeterministic, Llm}` is matched exhaustively in
`synthpass-cli/src/main.rs` twice. It answers "which tier produced the final
JSON", which stays true while there are two tiers. Once a vision provider lands,
the honest answer is a provider id, not a tier.

**Fix:** deprecate `Method` in favour of reading `ExtractionTrace.providers`,
after the trace has shipped long enough for consumers to migrate.

### A v1 Tier-2 record mixes two sources with no way to tell them apart

Since `apply_deterministic_mrz`, an escalated record's `mrz_line` is the
deterministic read while its scalar fields (`document_number`, the names, the
dates) are the model's. That is the right precedence — the deterministic read is
always the better source — but it means a v1 consumer can see a `document_number`
the accompanying `mrz_line` spells differently, with nothing marking which came
from where. `ExtractionV2` carries `trace` and per-field provenance and has no
such problem; v1 has no field to put it in.

**Consequence:** bounded. v1 is emitted only behind `SYNTHPASS_JSON_V1=1`, and a
consumer that cares can compare the two itself.

**Fix:** none planned — adding provenance to v1 would defeat the point of the
legacy shape. This entry exists so the mixing is a recorded decision rather than
a surprise.

### We build `llama-cpp-2`'s `common` feature for one try/catch

`common` is on because it is in `llama-cpp-2`'s default feature set, not because
anything chose it — `crates/synthpass-llm/Cargo.toml` asks only for `sampler`. It
sets `LLAMA_BUILD_COMMON=ON` in llama.cpp's CMake and compiles the crate's
`wrapper_common.cpp`, so it is not free in build time or binary size.

From `llama-cpp-2` 0.1.154's `src/sampling.rs`, the only part of it this codebase
reaches is `LlamaSampler::grammar`, which compiles to a different call per
feature:

```rust
#[cfg(feature = "common")]      llama_rs_sampler_init_grammar(...)   // crate shim
#[cfg(not(feature = "common"))] llama_sampler_init_grammar(...)      // raw upstream
```

and that shim is `try { llama_sampler_init_grammar(...) } catch (...) { return
nullptr; }`. Same sampler; the difference is a C++ exception guard. Without
`common`, an exception thrown during grammar init unwinds across an `extern "C"`
boundary instead of arriving at `synthpass-llm/src/lib.rs:76` as
`Err(GrammarError::NullGrammar)`. Since 0.1.154 (PR #1086) the grammar samplers
work without `common` at all, so dropping it is now merely *possible* — which is
exactly why this needs writing down before someone reads that release note as an
invitation.

**Consequence:** we pay build time and binary size — including in the musl
single-file air-gapped release — for an exception guard on a grammar that
`grammar.rs` generates from a Rust const and that therefore should never be
malformed. "Should never" is what the guard is for.

**Decision:** keep `common`. §2's priority order puts correctness and security
above performance, and binary size is not on the list at all; trading a safety
net for bytes inverts that. Revisit only with a measured size delta *and* a
reason the guard is redundant — not on the strength of the size number alone.

**Estimated effort:** 10 minutes to change, which is the trap. The measurement
and the argument are the work.
