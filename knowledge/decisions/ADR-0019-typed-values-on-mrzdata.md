# ADR-0019 — Typed dates and sex on `MrzData`

**Status:** Accepted (2026-09-23) — Option D, a fourth option recorded under [Decision](#decision)
**Date:** 2026-09-20

> **As proposed, this ADR reached no recommendation.** The options below are kept as written, with
> their honest costs and the questions that would settle them. The maintainer's choice, and the
> evidence it rests on, are recorded under [Decision](#decision). None of A, B or C was accepted as
> written.

## Context

`MrzData` exposes `date_of_birth`, `date_of_expiry` and `sex` as `String`
([`lib.rs`](../../crates/mrz/src/lib.rs)). The typed vocabulary for the dates **already exists and is
public** — `Date`, `DateCompleteness`, `DateValidity`, `expand_date_with_pivot` — and `MrzData`
already carries `date_of_birth_completeness`. What does not exist is a `Sex` type, or any typed
accessor for either date.

Two properties of the current representation are worth stating plainly, because they are what a
typed change would either fix or inherit.

### The crate already converts back

`MrzData::validity` calls `parse_iso(&self.date_of_birth)` — the parser formats a date into a
`String` at parse time, and the crate immediately re-parses that string into a `Date` to answer any
question about it. **The typed value is what the crate knows; the string is a rendering it then has
to undo.** That, more than any argument about ergonomics, is what puts this item on the list.

### The date strings are not always dates

`date_of_birth` holds ISO `YYYY-MM-DD` **or the raw `YYMMDD` field**, and which one depends on
`date_of_birth_completeness`. The field doc explains why this is deliberate rather than sloppy:

> An unknown or partially unknown date of birth is not a bad read: Part 3 §4.8 explicitly lets an
> issuer fill unknown positions with `<`, and a filler counts as zero for check-digit purposes, so
> an all-filler date of birth with check digit `0` verifies. This field is how a caller
> distinguishes "legitimately unknown, per the issuer" from "garbage OCR".

So the string is a union, discriminated by a sibling field. A typed representation has to decide what
it does with the non-date case — that is the substance of this ADR, not a detail of it.

**And it is a union of three things, not two, because the two existing signals can disagree.**
`expand_date_with_pivot` expands any six ASCII digits, and `date_completeness` classifies any six
ASCII digits as `Complete`. So a read of `"134599"` yields:

```
date_of_birth              == "2013-45-99"     an ISO-shaped non-date
date_of_birth_completeness == Complete         says it is fine
validity().dates_well_formed == false          says it is not
```

Telling which of the three kinds the string holds requires two further calls, **and those two calls
do not agree with each other.**

There is also an asymmetry: **`date_of_birth_completeness` exists; `date_of_expiry_completeness` does
not.** The crate acknowledges this in `SequenceCompleteness`'s own doc, which notes the gap is
"inherited from `DateCompleteness` itself, not introduced here."

### `sex` loses information, silently

[`clean_sex`](../../crates/mrz/src/parser.rs) is:

```rust
match c {
    'M' => "M".into(),
    'F' => "F".into(),
    _   => "X".into(),
}
```

Every other character collapses to `"X"` — including the **conformant `<` filler**, which ICAO
defines as "unspecified", and including OCR garbage such as a misread `H`. A caller receiving `"X"`
cannot tell "the document says unspecified" from "we could not read this cell".

**Sex is the one field where the crate has no verification and also destroys the evidence.** No
check digit covers it on any format — it falls outside every composite range — so the substitution
can never be caught arithmetically. And `synthpass-core`'s `fusion.rs` then raises
`LlmContradictsMrzStructural` against a Tier-2 model that may have read the printed character
*correctly*, on the strength of a value `mrz` invented.

The emitters complete the loop: `"X"` is emitted as the filler `<`, so a misread `M`→`H` parses to
`"X"`, emits as `<`, and produces a conformant zone whose check digit verifies. The generator's
round-trip property tests cannot catch it, because they start from `M`/`F`/`X`.

This is the same class of conflation [ADR-0017](ADR-0017-checks-distinguish-absent-from-verified.md)
addresses for check digits, and `DateCompleteness` already exists to preserve exactly this
distinction for dates. A `Sex` enum is the natural place to separate them — but only if it carries
more than three variants.

## The question

Should typed values become the **primary** representation on `MrzData`, be added **alongside** the
strings, or something between?

## Options

- **A — additive accessors only.** Add a `Sex` enum and `birth_date()` / `expiry_date()` /
  `sex_typed()` beside the existing fields. **This needs no breaking window at all** — it ships as a
  patch, since `MrzData` is `#[non_exhaustive]` and nothing is removed. The serde shape is
  unchanged, the `ZeroizeOnDrop` invariant is untouched, and the ~12 downstream sites that clone
  these strings into JSON fields keep working unaltered. Cost: the lossy `"X"` collapse and the
  ISO-or-raw duality both survive; the typed accessors are a *view* over a representation that still
  cannot express what it needs to. A caller who wants the truth still has to consult
  `date_of_birth_completeness` to know whether `birth_date()` returning `None` means "unknown by the
  issuer" or "unparseable".

- **B — typed as primary.** `date_of_birth: Date`, `date_of_expiry: Date`, `sex: Sex`. The cleanest
  long-term API, and the one a 1.0 crate would want: the type says what the value is, and the
  ISO-versus-raw ambiguity disappears because a `Date` cannot hold `"74<<12"`. Costs, all real: the
  serde shape changes, and `mrz-wasm` serialises `MrzData` whole into a payload `web/index.html`
  reads directly; roughly twelve downstream sites clone these values into `String` JSON fields and
  every one needs rewriting; the non-date case needs somewhere to live, or the information is simply
  lost; and **a typed `Date` holding a date of birth is PII that would stop being wiped** —
  `MrzData` derives `ZeroizeOnDrop`, and its doc states the rule that only fields which "carry no
  PII and are `Copy`" are `zeroize(skip)`. A `Date` is `Copy` and *is* PII, which the stated
  invariant does not currently contemplate.

- **C — typed primary with `*_raw` string escape hatches.** `date_of_birth: Date` plus
  `date_of_birth_raw: String`, so the raw field stays recoverable precisely when it matters. Keeps
  B's clarity without destroying the incomplete-date information. Cost: two representations of every
  date on the same struct, which invites them to disagree; the serde shape *grows* rather than
  merely changing, so the payload gets larger for every consumer including the browser demo; and it
  does not resolve the zeroize question — it doubles it.

## Decision

**Accepted: Option D — typed values as the primary representation, with a wire form that says
exactly what the zone says.** Taken 2026-09-23 under the maintainer's instruction to prefer the
option with the widest blast radius if it gives the cleaner crate. The pre-1.0 window is the only
time a break is this cheap, and `mrz` has **zero reverse dependencies on crates.io** (checked
2026-09-22), so every consumer the break reaches is in this workspace.

### The shape

```rust
pub struct RawDateField(/* private: [u8; 6] */);   // six MRZ-charset characters, as printed

#[non_exhaustive]
pub enum MrzDate {
    Calendar(Date),                     // six digits naming a real day
    OutOfCalendar(Date),                // six digits that do not: 000000, 110229
    PartiallyUnknown(RawDateField),     // digits and `<`: the issuer left part unknown
    Unknown,                            // six `<`: the issuer left it all unknown
    Malformed(RawDateField),            // any other character: a misread
}

#[non_exhaustive]
pub enum Sex {
    Male,                // zone `M`
    Female,              // zone `F`
    Unspecified,         // zone `<` — Part 4 note p: `<` in the MRZ, `X` in the VIZ
    NonConformant(char), // any other zone character, `X` included, kept verbatim
}
```

`MrzData::date_of_birth` and `date_of_expiry` become `MrzDate`, and `sex` becomes `Sex`.
**`date_of_birth_completeness` is removed**: `MrzDate::completeness()` returns the same
`DateCompleteness` from the value itself. `DateCompleteness` stays, because
`SequenceCompleteness` and the `mrz-wasm` payload use it.

### Why D and not A, B or C

- **Why not B's bare `Date`?** It was the first draft of this option. It fails on the first
  partially unknown date: `Date` is three integers and cannot hold `74<<12`, so B must either
  invent a value (the fabrication this ADR exists to remove from `sex`) or discard the field. The
  enum gives each of the five things a date field can hold its own variant.
- **Why a fifth variant for six-digit non-dates?** `looks_like_non_mrz_text` rejects a candidate
  whose date field has a character that is neither a digit nor `<`, and that rule carries a
  measured 18/19 no-MRZ rejection rate. With `OutOfCalendar` separate, `Malformed` keeps exactly
  that meaning and the rule's behaviour cannot move.
- **Why not C's `*_raw` strings?** The raw characters live inside the variants that need them,
  so there is nothing to disagree with. The verbatim zone is still `mrz_lines`.
- **Why not A?** A would leave the collapse of every non-`M`/`F` character into `"X"` in place,
  and the evidence below shows that every `"X"` it produces on this corpus asserts a meaning no zone printed.
- **The wipe is kept, not traded.** `RawDateField` is six bytes, so `MrzDate` and `Sex` stay
  `Copy`, carry no heap, and get a direct `Zeroize` impl (`Date`, `RawDateField`, `MrzDate`,
  `Sex`) under the existing `zeroize` feature. A date of birth is still wiped on drop. The type
  doc's claim is reworded to **best-effort**: `mrz-wasm` never enables `zeroize`, so the browser
  build has never wiped anything, and the doc promised more than any build delivered.
- **Question 3 answers itself.** Expiry has the same type, so the missing
  `date_of_expiry_completeness` asymmetry disappears without adding a field.
- **The emitter follows.** `Td3Fields` and its siblings take `Sex` and `MrzDate` too, so parse and
  emit are symmetric by type. Today `emit` takes `sex: String`, where `"X"` means `<`, which is the
  same loose vocabulary. Doing it now keeps the break to one release.

### The wire form

[ADR-0020](ADR-0020-mrz-value-wire-contract.md) records it as a contract: **`Display`, serde and
the zone agree.** A date serialises as today's ISO string when it is six digits, and otherwise as
its six raw characters. Every date that reaches JSON today reaches it byte-identically. Sex
serialises as the zone character. That is the one deliberate wire change: `"<"` where the zone
says unspecified, and the misread character itself (`"1"`, `"S"`) where today's `"X"` hid it. The
product schema in `synthpass-core` keeps ICAO's VIZ vocabulary (`M`/`F`/`X`) through one mapping
function.

### Order of work

1. The types land unwired, with a JSON snapshot of today's output for every fixture.
2. The field swap, which must reproduce that snapshot except for the removed key and the four
   sex cells below. The real-specimen gate must show no movement.
3. Separately, and on its own measurement: a non-conformant sex cell stops reaching the product
   as `"X"`, and Tier-1 promotion is gated on `MrzDate::Calendar` for both dates. Expiry is
   promoted today without any completeness gate.
4. The emitter inputs.

Steps 2 and 3 are split so that a benchmark movement can always be attributed to one of them.

## What this ADR deliberately did not decide, as proposed

**Anything.** The recommendation is withheld on purpose, because the choice turns on three
judgements that are the maintainer's rather than the analysis's:

1. **Is the `serde` shape allowed to change in 0.8.0?** [`ROADMAP.md`](../../crates/mrz/ROADMAP.md)
   freezes the serde representation as API at 1.0 and pins it by test in 0.9 — but says nothing
   about 0.8. If the shape is already effectively frozen by downstream practice, B and C are both
   out and A is the only option; if 0.8 is genuinely the last chance to move it, the calculus
   inverts.
2. **Is `MrzData`'s zeroize guarantee a hard invariant or a best-effort one?** If PII-bearing fields
   must be wiped, a typed `Date` for a date of birth needs a zeroizing type or an exemption stated
   in the doc. If the guarantee is best-effort, B is much cheaper than it looks.
3. **Is the `date_of_expiry_completeness` asymmetry fixed in the same change, or left standing?**
   Adding it is additive and independent — but a typed representation makes the gap louder, because
   `birth_date()` and `expiry_date()` would return `None` for reasons a caller can distinguish in
   one case and not the other.

**Three of those are countable rather than arguable, and none needs new instrumentation** — all are
derivable from an existing `--dump-ocr` run over the 261-document real-specimen population:

- How many reads have a `date_of_birth` that `parse_iso` rejects, split by `DateCompleteness`? If
  that is ~0, the string's polymorphism is theoretical and option B is nearly free.
- How many have `date_of_birth_completeness == Complete` **and** `dates_well_formed == false`? That
  is the exact size of the ISO-shaped-non-date hole.
- How many sex cells are a character other than `M`, `F` or `<`? That separates "the issuer said
  unspecified" from "we could not read it", and if it is zero across the corpus then `clean_sex`'s
  loss is theoretical and option A suffices.

**So this ADR does make one recommendation, and it is about process rather than about the options:
count those three before accepting any option here.** A fourth question is a one-minute lookup that
nobody has done — whether `mrz` has reverse dependencies on crates.io, which converts every
downstream-churn estimate above from a bounded number into an unbounded one.

Also not decided: the arity of `Sex`. If it is only `M`/`F`/`X` it reproduces today's collapse in a
new type and buys nothing; if it distinguishes the conformant filler from an unreadable cell, it
fixes a real defect — but that is a fourth variant with its own downstream consequences, and it is
arguably [ADR-0017](ADR-0017-checks-distinguish-absent-from-verified.md)'s argument applied to a
different field rather than part of this one.

### Corpus evidence for the choice

Counted over the raw zones of all **118** ground-truth fixtures (`samples/ocr_fixtures/**`,
`derived/` included) by
[`adr_0019_evidence.rs`](../../crates/synthpass-bench/examples/adr_0019_evidence.rs), which
asserts every number below, so a corpus change that moves one fails instead of silently
outdating this record:

| Field | Calendar | Out of calendar | Partially unknown | Unknown | Malformed |
| --- | ---: | ---: | ---: | ---: | ---: |
| `date_of_birth` | 114 | 3 | 0 | 0 | 1 |
| `date_of_expiry` | 114 | 3 | 1 | 0 | 0 |

Sex cells: `M` 58 · `F` 56 · `<` 0 · **non-conformant 4** (`1` ×2, `0`, `S`). Today every one of
the four becomes `"X"`, ICAO's visual-zone word for *unspecified*. But an issuer that means
unspecified prints `<`, and no zone in the corpus does. So **every `"X"` `mrz` produces over this
corpus asserts a meaning the zone never printed.** Two of the four sit in checksum-valid zones
(Croatia 2021, Kazakhstan 2004): sex is outside every check-digit range, so the checksum cannot
flag them. (What the visual zone says is not established for all four: two hand-reviewed fixtures
record sex as `null`, one records `M`, and Kazakhstan's `"X"` was itself derived from the MRZ
without review.)

**Read the date kinds with each zone's own checksum verdict.** A slice is only a field when the
zone is aligned. Türkiye's 2024 passport specimen prints a deliberately broken zone: the
document-number check digit is `<`, and every later field sits one column off. Its
partially-unknown expiry `48<123` and its malformed birth date `R38473` are cuts across two
fields, not an issuer's choice or a misread. Restricted to the 99 checksum-valid zones (198
dates), the only non-calendar dates are **3 six-digit specimen placeholders**: Croatia's `000000`
twice, and Czechia's `110229`, a 29 February in 2011. No issuer in the corpus prints a filler
date.

So the evidence for the date variants is uneven, and the ADR says so plainly:

- `OutOfCalendar` carries real, checksum-valid traffic.
- `PartiallyUnknown` and `Unknown` exist because ICAO Part 3 §4.8 permits them, not because this
  corpus contains them.
- `Malformed` exists because OCR produces misreads. These counts are lower bounds for OCR output:
  ground truth records the printed zone, and a recogniser can only add malformed reads.

(An earlier count over the 64 top-level fixtures alone found 4/4/3; it missed `derived/`.)

## Timing

As proposed, this section warned that the 0.8.0 window was open but not self-enforcing. It was
used. By the time of acceptance, `mrz` 0.8.0 already carried three breaking fragments
(`changelog.d/mrz/*.changed!.md`) and had not yet been published. This decision adds its break to
the same release, which costs one more fragment and no version movement.

## Consequences

**Of the decision (Option D):**

- **Positive:** the type says what the value is, and a caller branches on one `match` instead of
  consulting a second field. The fabricated `"X"` is gone at the source. The wipe guarantee holds
  for the new types and is described honestly for the first time. Parse and emit share one
  vocabulary.
- **Negative:** the widest churn of any item in the window. About 13 production sites and 23 tests
  outside `mrz` touch these fields, plus `mrz`'s own tests and doctests. Two new public types and
  a newtype enter the 1.0 surface. The `mrz-wasm` payload loses `date_of_birth_completeness`, and
  its `sex` can now be `"<"` or a raw character. The demo's check-in form already maps anything
  other than `M`/`F` to `X`.
- **What would reverse it:** a consumer outside this workspace that the wire change broke. There
  is none today (zero reverse dependencies). After 0.8.0 publishes, that is precisely what the
  pre-1.0 window can no longer absorb.

**Of each option as proposed** (kept for the record):

- **Positive, under A:** immediate, costs nobody an upgrade, and can ship before 0.8.0 publishes
  without consuming the window. **Under B or C:** the type finally says what the value is, and the
  "is this ISO or raw?" question stops being answerable only by consulting a second field.

- **Negative, under A:** the window closes with this item unresolved, and making typed values
  primary later costs a break of its own — which is exactly what the "break once" rule exists to
  avoid. **Under B:** the widest downstream churn of any item in the window, plus an unresolved
  PII-wiping question on a crate whose selling point includes not leaking document data. **Under
  C:** a struct carrying the same fact twice, which is a maintenance liability and a larger payload
  for the wasm demo.

- **Documents that must change with it:** under B or C, `MrzData`'s type doc — specifically its
  `ZeroizeOnDrop` rule about which fields are `zeroize(skip)` and why; the `date_of_birth` and
  `date_of_expiry` field docs, which currently document the ISO-or-raw duality as the contract; and
  the `mrz-wasm` projection, which
  [`MRZ_SEQUENCE_COMPLETENESS.md`](../MRZ_SEQUENCE_COMPLETENESS.md) records as needing the same
  deliberate treatment `DateCompleteness` received when it shipped.

- **What would settle it:** the three questions above. **What would reverse it, once taken:** for A,
  evidence that callers are re-parsing the strings into `Date` at most sites anyway, which would mean
  the typed form should have been primary; for B or C, evidence that the JSON shape change broke a
  consumer the workspace does not control — which, for a published crate, is precisely the risk the
  pre-1.0 window is meant to contain.
