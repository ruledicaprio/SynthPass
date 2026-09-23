# Technical debt

Not bugs. Not TODOs. Things that work but whose shape will cost us later, with an
honest severity and an estimate. Reviewed at each milestone close.

Proposed in [archive/KNOWLEDGE.md](archive/KNOWLEDGE.md). Add an entry when you *choose* not to
fix something — the value of this file is the deferred decisions, not the list of
known imperfections.

---

## High

### The licensing public key is still a placeholder

`crates/synthpass-license/pubkey.b64` is the Ed25519 verifying key compiled into every shipped
binary. `keys.rs`'s own comment says: *"Placeholder — generate a real keypair … and replace this
file before issuing any real licenses."* It has not been touched since the v2 rebrand commit
(2026-07-21).

**Consequence:** no real customer license can be issued against a shipped binary today. The
licensing crate itself is sound — Ed25519 `verify_strict`, fail-closed feature gating, expiry,
32 unit tests — but it is verifying against a key nobody holds the private half of in anger.
This is a hard blocker on the "official binary" distribution path, not a code-quality nit.

**Fix:** generate the real keypair out-of-band, store the private half where it is never in the
repo, replace `pubkey.b64`, and delete the placeholder comment in the same commit so the file
cannot be misread as still-fake. Cheap to do, easy to forget, expensive to discover late.

**Related:** the machine fingerprint is real only on Linux (`/etc/machine-id` with a
persisted-random fallback). On Windows it degrades to `windows-dev-{COMPUTERNAME}` /
`unbound-dev-windows`, which binds nothing. Any Windows distribution needs a real fingerprint
source first.

**Decision (2026-09-13): distribution is source-build only** until a real keypair and a real Windows
fingerprint exist. No official binary is a deliverable of any milestone as things stand;
[`ADR-0011`](decisions/ADR-0011-split-m6-packaging-into-m8.md) writes M8's air-gapped Definition of
Done against a source build for exactly this reason. Shipping a binary later is its own decision,
and the Fix above is its first step. The entry stays High because it still blocks that path — what
changed is that the blockage is now a recorded position rather than an unnoticed gap.

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

**Fix:** `ocrs::OcrEngine::prepare_recognition_input` is public and returns the exact
preprocessed line tensor the model consumes, so run the already-pinned `.rten` model through
`rten` directly to obtain its CTC log-probability matrix: no fork, vendored patch, or new
dependency is needed.
`knowledge/research/long-horizon-parsing.md` §2 works out a better path that
avoids upstream negotiation entirely: swap the recognizer to PaddleOCR
PP-OCRv5 converted to `.rten` (already a direct dependency) and write our
own CTC decode, so per-character probabilities land in code we control.
Costs are real and stated there honestly — PP-OCRv5's MRZ/OCR-B accuracy
vs. `ocrs` is unverified, the MRZ-charset beam-search retry pass would need
reimplementing against the new engine, and it's a second model to
download/hash-pin — so the recommended sequencing is a `synthpass-bench`
bake-off first, not a blind swap.
A raw CTC log-probability is a model score and strictly better than the current plausibility
proxy, but it is uncalibrated. Under [`project_principles.md` §2](project_principles.md#2-every-claim-exposes-the-strength-of-its-evidence), it may route work but cannot be published as
confidence without a reliability diagram. Obtaining that score is necessary but not sufficient to
retire this debt; neither this entry nor ADR-0014 §5 previously said so.

**Estimated effort:** the bake-off is the next concrete step; the 3–5 day
figure for the full swap (patch/fork framing) is retired along with that
framing — see `long-horizon-parsing.md` for the actual cost breakdown.

---

## Medium

### A valid line 2 still leaves four ambiguities no checksum resolves

Recorded 2026-09-17 when the strict-name work (ADR-0013) scoped itself to names. A checksum-valid
MRZ proves the document number and the dates; it does not prove everything on line 2.

- **Nationality and sex are unchecked on every format.** Neither sits inside a check digit or
  the composite, so a misread there passes as a Tier-1 hit. The strict metric scores names only;
  it should grow into a `strict_fields` list (nationality, sex) once name repair has a baseline.
- **Compound substitutions are invisible to the check digits.** `mrz::blindspot` and
  `blind_positions` (`crates/synthpass-die/src/mrz_reader.rs`) see single-character collisions;
  most observed `document_number` mismatches were compound (ROADMAP, "Check-digit blind spot").
  Checksum-guided repair over `mrz::CONFUSABLES` can likewise stop on a wrong-but-valid read.
  Only labelled fixtures catch either, which is why W2's ground truth matters.
- **The century pivot is a guess near its boundary.** A `YYMMDD` date close to the pivot can
  resolve to the wrong century with valid check digits; `scripts/check-century-pivot.sh` keeps
  the pivot current but cannot remove the ambiguity.
- **A dropped `<` in the sex position** is lost by the recognizer like any isolated filler. It
  is the one line-2 case the fixed-grid repair (`synthpass_ocr::chargrid`) could fix provably,
  because the surrounding check-digit-covered cells pin its position — deliberately out of
  scope for the name-line wiring.

**Why deferred:** names are the larger, measured gap, and each item above needs ground truth
before its fix can be judged. **Severity:** Medium — a silent wrong field on a hit. **Estimate:**
`strict_fields` ~1 d after W2; the rest per item.

### Three parallel lists of ICAO field names

- `synthpass_core::v2::ExtractionFields` — the schema: the 12 ICAO fields (ten, plus
  `optional_data_1`/`optional_data_2` since ADR-0018) plus the two derived `*_name` keys
  (`issuing_country_name`, `nationality_name`)
- `synthpass_bench::COMPARED_FIELDS` — what the benchmark scores
- `synthpass_llm::prompt::FIELDS` — what the prompt asks for, and the source
  `grammar.rs` generates the GBNF from

The third **deliberately differs** (it asks for `mrz_line`, omits `personal_number` and both
optional-data fields), and `grammar.rs`'s "prompt and grammar cannot drift" invariant depends on
it staying a Rust const. So this is not simply de-duplicable. `v2::CoreField` names the 12 ICAO
fields; the third stays separate.

**Guarded since the `icao-fields-guard` change:** `CoreField::as_str` is `const`,
and two `const _` blocks pin the lists at compile time — `synthpass-bench`
asserts `COMPARED_FIELDS` equals `CoreField::ALL` name for name and in order;
`synthpass-llm/src/grammar.rs` asserts every prompted field is a `CoreField` or
listed in `PROMPT_ONLY_FIELDS`, and every `CoreField` is prompted or listed in
`CORE_FIELDS_NOT_PROMPTED`. Adding a field to one list and not the others no
longer compiles.

**Still open:** the derived `*_name` keys have no `CoreField` variant, and
`schema_keys.rs` checks `CoreField` → `ExtractionFields` keys in one direction
only, so a new `ExtractionFields` key outside `CoreField` is still unguarded.

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

**Partly addressed 2026-09-02, and the unaddressed half is now the whole
entry.** `parity` is no longer six documents — it is 72, and it reports a
per-field breakdown plus separate rates for hand-verified and generated fixtures
(`crates/synthpass-bench/examples/ground_truth_candidates.rs`,
`crates/synthpass-llm/tests/parity.rs`'s module doc and
[`knowledge/benchmarks/README.md`](benchmarks/README.md#current-headline-numbers) for the current
baseline). So the "16/42 either way" check that
cleared the 0.1.151 → 0.1.154 bump would now be a 162-field measurement with
visible per-field movement, which is the difference between a number that can
detect a regression and one that cannot. What has *not* changed is the part this
entry is actually about: it still requires someone to remember to run it. The
fix below — a scheduled workflow that provisions the weight and records the rate
as a tracked number — is unbuilt, and the entry stays open until it exists.
Note the run now takes ~31 minutes, not ~4, which makes "just un-ignore them"
even less viable than when this was written.

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

### `#![forbid(unsafe_code)]` is declared in one crate of fourteen

Only `crates/mrz/src/lib.rs` forbids `unsafe`. The other thirteen crates merely happen to contain
none in production: measured 2026-09-17, every `unsafe` in the workspace sits inside a
`#[cfg(test)]` module — `std::env::set_var`/`remove_var` (unsafe since Rust 2024) in
`synthpass-ocr` and `synthpass-llm`, and a hand-rolled executor in `synthpass-die`'s
`mrz_reader` tests. The image ingest path carries zero.

**History:** this replaces a High entry that claimed 18 `unsafe` blocks sat on `synthpass-ocr`'s
untrusted-image path. The count was right and the location was wrong — all 18 are test-only — and
the entry stood for weeks because nobody checked the blocks against the `#[cfg(test)]` boundary.
Kept here as the record rather than deleted, so the claim is not rediscovered and re-filed.

**Fix:** make the property compile-time instead of measured: plain `#![forbid(unsafe_code)]` in
the crates with no `unsafe` at all, and `#![cfg_attr(not(test), forbid(unsafe_code))]` in the
three whose tests mutate the environment or spin an executor (`forbid` cannot be re-allowed
inside the crate, so the test-only sites need the `cfg_attr` form). An afternoon.

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
