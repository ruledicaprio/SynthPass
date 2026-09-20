# ADR-0018 — Name the optional-data field for what it holds

**Status:** Proposed
**Date:** 2026-09-20

## Context

[`MrzData::personal_number`](../../crates/mrz/src/lib.rs) is `Option<String>`, and it holds three
different things depending on format. The field's own doc comment says so:

> TD3: the personal-number field. TD1: optional data 1 and 2, joined. TD2, MRV-A and MRV-B: the
> optional-data field.

Only TD3 prints a *personal number*. On the other four formats the value is **optional data**, which
ICAO 9303 defines as issuer-discretionary and semantically unspecified. The crate names four of five
formats after a field they do not have.

### TD1's join is lossy

For TD1 the parser concatenates two distinct printed fields
([`parser.rs:437`](../../crates/mrz/src/parser.rs)):

```rust
let optional2 = line2[18..29].trim_end_matches('<');
let personal = [optional1, optional2]
    .iter()
    .filter(|s| !s.is_empty())
    .cloned()
    .collect::<Vec<_>>()
    .join(" ");
```

Empty fields are filtered before joining, so **an empty optional data 1 with a populated optional
data 2 produces exactly the same string as the reverse.** A caller receiving `"ZZZ"` cannot recover
which of the card's two fields it was printed in. The crate's own round-trip test documents the
collapse in passing: *"optional_data_2 is empty here, so no `" "` separator survives."*

This is not a cosmetic naming problem. A TD1 card back carries two separately-positioned fields, one
per line, and the parse output has one slot for both.

### The crate already disagrees with itself, and with the spec

The **emit** side of this same crate uses the honest names
([`emit.rs`](../../crates/mrz/src/emit.rs)): `Td1Fields::optional_data_1` and `optional_data_2`,
`Td2Fields::optional_data`, `MrvAFields::optional_data`, `MrvBFields::optional_data`. Only `Td3Fields`
says `personal_number` — correctly, because TD3 is the format that has one.

So `mrz` emits `optional_data_1` and parses it back as `personal_number`.

[`knowledge/docs9303/Mrz_Field_Layout.md`](../docs9303/Mrz_Field_Layout.md), the ICAO spec
transcription this repo treats as source of truth under
[ADR-0003](ADR-0003-docs9303-source-of-truth.md), agrees with the emitters exactly: `optional_data_1`
/ `optional_data_2` for TD1, `optional_data` for TD2 and the MRVs, and — **importantly** —
`personal_number` for TD3 line 2 `[28,42)`. `synthpass-bench`'s `MrzFieldSpan` tables use the same
five names.

So the defect is not that `personal_number` is the wrong word. **It is that one field is standing in
for five**, and the rest of the codebase already models them separately. The crate can *emit* a
shape it cannot *report*.

That reframing matters, because the obvious move — rename everything to `optional_data` — would fix
four formats and break the fifth, putting `mrz` at odds with its own emitters, its own spec
transcription, and the benchmark span tables, all three of which currently agree.

The honest complication on the other side: ICAO Doc 9303 Part 4 §4.2.2 titles TD3 positions 29–42
*"Personal number **or other optional data elements**"*. The standard's prose is broader than the
key the transcription chose.

## The question

Should `MrzData::personal_number` be renamed to say what it holds — and if so, should TD1's two
printed fields be separated — given that the name is also a published schema key elsewhere in the
workspace?

## Options

- **A — change nothing.** Free, and the doc comment already discloses the mismatch. Rejected: a
  field name is read far more often than a field doc, and this one is wrong on four formats out of
  five while the same crate spells it correctly on the emit side.

- **B — rename to `optional_data`, keep TD1's join.** Honest name, smallest possible diff, and it
  brings parse into line with emit for TD2 and the MRVs. Its merit is that it fixes the *naming*
  defect without touching the data shape, so no caller has to think about arity. Cost: TD1 callers
  still cannot tell which printed field a value came from, so the lossy join survives under a better
  name.

- **C — add the new fields and deprecate `personal_number`.** No hard break; downstream migrates on
  its own schedule, which is a real kindness for a published crate, and `MrzData` is
  `#[non_exhaustive]` precisely so additions are free. Cost: three fields where the format has at
  most two, permanently — unless a later break removes the deprecated one, which is the break this
  was avoiding, deferred and made harder. `#[deprecated]` on a field also warns at every *read*
  site, including TD3 reads where the value is still correct.

- **D — split per format: `optional_data_1` / `optional_data_2` as fields, with `personal_number`
  kept as a TD3-only accessor.** Lossless *and* spec-aligned: TD1's two fields separate, TD3 keeps
  the name the transcription and the emitters give it, and nothing in `Mrz_Field_Layout.md` or
  `provider_bench.rs`'s span tables needs amending. The overflow carve-out gains an unambiguous home,
  since TD1's overflow reads from optional data 1 only. Cost: the largest surface change — two new
  fields, one removed, one new accessor — and a two-slot shape that is over-general for the four
  formats with one field. It needs an explicit rule for which slot a single optional-data field
  occupies, and getting that wrong puts TD2's data in the TD1-line-2 slot.

- **E — one `optional_data: OptionalData` value**, where `OptionalData` is an enum over the arities.
  Makes arity a property of the value, so TD1's second field is unreachable on a TD3. Rejected: a new
  public enum and a tagged-union JSON shape for a field only some formats populate is more machinery
  than the fact warrants — but it is the shape a reader will propose, so it is recorded rather than
  omitted.

## Recommendation

**Option D.**

1. **It is the only option that recovers information.** A, B and C all leave `("", "ZZZ")` and
   `("ZZZ", "")` indistinguishable. That — not the name — is the defect worth a breaking slot.
2. **It costs nothing in spec alignment**, which is what rules out B. Because D keeps
   `personal_number` for TD3, the transcription, `Td3Fields`, and the benchmark span tables all stay
   as they are. A flat rename was the obvious move; reading the spec authority ruled it out, and
   that is worth recording rather than quietly discarding.
3. **Symmetry with the emitters is the whole argument.** "The crate can emit a shape it cannot
   report" needs no appeal to taste.

**C is the runner-up, and the condition that would promote it is mechanical:** if 0.8.0 publishes
before this lands, D's cost roughly triples — it stops being one `!` fragment inside an already-open
window and becomes a 0.9.0 of its own. At that point the additive path is the better trade.

## What this ADR deliberately does not decide

**Whether the rename propagates past `mrz` — and this is the load-bearing open question, not a
detail.** `personal_number` is also:

- `CoreField::PersonalNumber` and `ExtractionFields::personal_number` in `synthpass-core`, whose
  serialised shape [`schema_keys.rs`](../../crates/synthpass-core/tests/schema_keys.rs) calls **"a
  published contract"** and pins by test;
- a published dataset key in [`EXPORTS.md`](../EXPORTS.md)'s JSONL and Hugging Face export schemas;
- one of the **three parallel field-name lists** recorded in [`technical_debt.md`](../technical_debt.md)
  (`ExtractionFields`, `COMPARED_FIELDS`, `prompt::FIELDS`), pinned against each other by `const _`
  compile-time assertions — so a partial rename does not compile, which is the guard working as
  designed;
- a key in the `mrz-wasm` demo payload, which serialises `MrzData` whole and is read directly by
  `web/index.html`.

The defensible position is that **the rename stops at `mrz`**: `mrz` names what the *document
prints*, while the v2 schema names what the *extraction produced*, and those vocabularies are
allowed to differ across a bridge that already translates. The bridging assignments would fail to
compile until updated — loudly, which is the desired behaviour.

But that leaves `mrz` and the schema disagreeing on a name, and someone will eventually try to
"fix" it. **Whichever way it goes, it must be written down rather than left implicit**, because the
next reader will otherwise assume the divergence is an oversight.

Also not decided here: whether the `mrz-wasm` payload key follows the crate or is pinned
independently, and whether `EXPORTS.md`'s dataset keys move at all — a published dataset schema has
consumers that no compiler will warn.

**One collision to record before review finds it.** `Checks::personal_number` and
`MrzData::personal_number` currently share a name. Under D the data field moves and the check-digit
field does not — which is *correct*, since only TD3 prints that check digit, but it leaves
`Field::PersonalNumber`'s `as_str()` returning `"personal_number"`, a string that no longer names any
field on `MrzData`. That is acceptable and should be stated, not discovered.

## Timing — this is a deadline, not background

`crates/mrz/Cargo.toml` already reads **0.8.0**, but `CHANGELOG.md`'s `[Unreleased]` section is empty
and the last released version is 0.7.1. **The version is staged, not published.**

That has a precise consequence. `cargo-semver-checks` derives the permitted bump from `Cargo.toml`,
which already sits in the breaking slot, so this change costs **one `!` changelog fragment and zero
version movement** if it lands before 0.8.0 publishes — and **a whole additional 0.9.0** if it lands
after, at which point "we broke once" stops being true.

The window is open. It is not open indefinitely, and nothing about it is self-enforcing.

## Consequences

- **Positive:** the field name stops misdescribing four of five formats; TD1's two printed fields
  become separately recoverable; parse, emit and the spec transcription agree for the first time;
  and the document-number overflow carve-out gains an unambiguous home.

- **Negative:** a real break for every reader of `MrzData::personal_number`, and — if TD1 splits —
  an arity change, not just a rename, so the migration is not mechanical. The `mrz-wasm` JSON key
  changes and the demo page reads it directly. `mrz` carries **zero `#[serde(...)]` attributes**, so
  there is no `alias` cushioning deserialisation of previously-written JSON unless one is added
  deliberately.

- **Documents that must change with it:** the field's own doc comment, which currently documents the
  mismatch as a feature; the `MrzData` type-level example, which asserts a `personal_number` value;
  [`EXPORTS.md`](../EXPORTS.md) if the dataset keys follow; and [`SYNTHPASS.md`](../SYNTHPASS.md),
  which phrases an accuracy claim in terms of "the longer `personal_number` field". Benchmark prose
  already uses `optional_data_2` — see
  [`twelve-scored-misses-2026-09-19.md`](../benchmarks/twelve-scored-misses-2026-09-19.md) — so the
  measurement vocabulary moves *toward* the crate rather than away.

- **What would reverse it:** a decision that the v2 schema key must follow the crate name. At that
  point the change stops being a crate rename and becomes a published-wire-format break with dataset
  consumers attached, and the cost/benefit is a different calculation entirely — one that should be
  taken on its own evidence, not inherited from this ADR.
