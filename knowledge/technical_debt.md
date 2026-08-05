# Technical debt

Not bugs. Not TODOs. Things that work but whose shape will cost us later, with an
honest severity and an estimate. Reviewed at each milestone close.

Proposed in [archive/KNOWLEDGE.md](archive/KNOWLEDGE.md). Add an entry when you *choose* not to
fix something — the value of this file is the deferred decisions, not the list of
known imperfections.

---

## High

### Eleven source citations point at a document that has never existed

`docs/V2-DESIGN.md` is cited by section number in eleven doc comments across
`synthpass-core` (`lib.rs:28`, `v2.rs:1,74,97,123,128,143,271`),
`synthpass-pipeline` (`lib.rs:248,816`) and `synthpass-serve` (`main.rs:82`).
Verified against the full history — `git log --all -- docs/V2-DESIGN.md` returns
nothing. **The file was never committed to this repo.**

Those citations read as live references ("§11", "§9, B2/B3"), so a reader will go
looking and find nothing. Deliberately *not* rewritten to `knowledge/V2-DESIGN.md`
during the docs → knowledge migration: that would move a broken link rather than
fix it.

**Fix:** either write the document (the design it describes is real and shipped —
the v2 schema, the B2/B3 breaking changes, the §11 authenticity non-goal) or
replace each citation with the surviving doc that covers the claim. Writing it is
better; the content exists, scattered across `CHANGELOG.md` v2 entries and
`ARCHITECTURE.md`.

**Estimated effort:** 1 day to write, or 2 hours to re-point the citations.

### OCR confidence is a character-plausibility proxy, not a model score

`geometry::text_sanity` computes "fraction of plausible characters" because
`ocrs` exposes no per-character probability — its `TextChar`/`TextLine` carry
only `char` and `rect`, and the CTC decode probabilities are computed internally
and dropped. The module doc is honest about this, but every downstream consumer
still reads a field called `confidence`.

**Consequence:** the routing engine's `text_sanity` signal is weaker than its
name suggests, and no threshold on it can be better than the proxy.

**Fix:** upstream a patch to `ocrs` exposing the decode probabilities, or fork
the recognition loop. Neither is small.

**Estimated effort:** 3–5 days, mostly upstream negotiation.

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

### `deny.toml` and `about.toml` are policy files no CI job runs

Both exist and are well-formed. Neither `cargo deny check` nor
`cargo about generate` appears in any workflow — so the license allow-list and
the advisory `yanked = "deny"` rule are enforced by memory.

**Consequence:** a dependency with a disallowed license can land, and
`THIRD_PARTY_NOTICES.md` can silently go stale.

**Fix:** add a `deny` job. Deliberately not bundled into the M7 work — it would
block an unrelated milestone on whatever pre-existing findings it surfaces.

**Estimated effort:** 2 hours, plus unknown time to clear findings.

---

### Nothing in CI exercises the real inference engine

Both tests that load the actual GGUF — `synthpass-llm/tests/native_llm_e2e.rs` and
`tests/parity.rs` — are `#[ignore]`, and the `Native LLM (real model, opt-in)` CI
job skips. Everything CI runs against `synthpass-llm` type-checks or mocks. So a
change that alters *what the model produces*, while compiling cleanly and passing
all 28 unit tests, merges green.

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

### `ROADMAP.md`'s 45.2% parity baseline no longer reproduces

`ROADMAP.md`'s M5 execution note records the GBNF parity run's field match rate
as **45.2%** (~19/42). Running `parity` on `main` today gives **16/42 (38.1%)**,
on the same fixtures and the same floor.

This is not a regression from any recent change — it reproduces identically at
0.1.151 and 0.1.154, so the engine bump is not responsible. The likeliest
explanation is that the fixture set moved out from under the recorded number when
`samples/` was reorganised and the corpus gained TD1/TD2 rendering, but **that is
a guess, and guessing is how the number went stale in the first place.**

**Consequence:** a recorded baseline that does not reproduce reads as a live
regression to whoever next runs the test. It already cost one investigation: the
0.1.154 bump was held back from review while a control run ruled the bump out as
the cause.

**Fix:** re-run `parity`, record the number with the date and the corpus state
that produced it, and say in `ROADMAP.md` which corpus each figure belongs to.
Deliberately not folded into the bump PR that found it — that PR's claim is "this
changes nothing," and quietly editing a baseline inside it would undercut exactly
that claim.

**Estimated effort:** an hour, most of it the test run.

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
