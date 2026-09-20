# ADR-0019 — Typed dates and sex on `MrzData`

**Status:** Proposed
**Date:** 2026-09-20

> **This ADR deliberately reaches no recommendation.** The options are laid out with their honest
> costs, and the three questions that would settle the choice are named at the end. The decision is
> the maintainer's, on acceptance.

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

## What this ADR deliberately does not decide

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

The acceptance test was run over the 64 ground-truth zones, counting from each raw `mrz_line` rather
than only the curated fixture fields. Four of 64 `date_of_birth` values are not real calendar dates,
four of 64 `date_of_expiry` values are not real calendar dates, and three of 64 sex cells are outside
`M`/`F`/`<`. These are lower bounds: ground truth records the printed zone, and OCR can only add
malformed reads beyond them.

Neither branch of the original test holds. The evidence therefore points to Option C, with the
serde shape permitted to change in 0.8.0, a hard zeroize guarantee for date-of-birth data, and the
`date_of_expiry_completeness` asymmetry fixed in the same change. The A/B/C implementation details
remain open for the maintainer; this ADR stays Proposed.

## Timing — this is a deadline, not background

`crates/mrz/Cargo.toml` already reads **0.8.0**, but `CHANGELOG.md`'s `[Unreleased]` section is empty
and the last released version is 0.7.1. **The version is staged, not published.**

That has a precise consequence. `cargo-semver-checks` derives the permitted bump from `Cargo.toml`,
which already sits in the breaking slot, so this change costs **one `!` changelog fragment and zero
version movement** if it lands before 0.8.0 publishes — and **a whole additional 0.9.0** if it lands
after, at which point "we broke once" stops being true.

The window is open. It is not open indefinitely, and nothing about it is self-enforcing.

## Consequences

Stated per option, since none is chosen.

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
