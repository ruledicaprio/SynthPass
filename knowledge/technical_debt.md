# Technical debt

Not bugs. Not TODOs. Things that work but whose shape will cost us later, with an
honest severity and an estimate. Reviewed at each milestone close.

Proposed in [KNOWLEDGE.md](KNOWLEDGE.md). Add an entry when you *choose* not to
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
