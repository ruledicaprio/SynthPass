# mrz

[![crates.io](https://img.shields.io/crates/v/mrz.svg)](https://crates.io/crates/mrz)
[![docs.rs](https://docs.rs/mrz/badge.svg)](https://docs.rs/mrz)
[![downloads](https://img.shields.io/crates/d/mrz.svg)](https://crates.io/crates/mrz)
[![MSRV](https://img.shields.io/badge/MSRV-1.82-blue.svg)](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#versioning-and-msrv)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#license)

Zero-dependency (by default) [ICAO Doc 9303](https://www.icao.int/publications/pages/publication.aspx?docnum=9303)
Machine Readable Zone parser, emitter and check-digit validator for Rust — passports, ID cards
and visas.

A check digit is arithmetic, not a model score: it agrees with the read or it does not. `mrz`
verifies every printed digit under the standard 7-3-1 weighting and reports which one agreed and
which one failed, field by field. It is just as exact about what the arithmetic cannot see.

**Why use it**

- **Evidence per field.** Document number, date of birth, expiry, personal number and composite each
  report verified, refuted or not printed.
- **Repairs only with proof.** `find_and_parse` fixes a misread only when a check digit agrees with the fix.
- **Writes conformant zones.** All five formats, with Latin and Cyrillic names transliterated as Doc 9303 prescribes.
- **States its limits.** The substitutions no check digit can catch are a public API.
- **Small and safe.** No dependencies in the default build, no `unsafe`, no clock, no network. Builds for
  `wasm32-unknown-unknown` and is property-tested never to panic on arbitrary input.

**▶ [Try it in your browser](https://ruledicaprio.github.io/SynthPass/)** — live WASM MRZ validator.

## Contents

- [Install](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#install) · [Formats](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#formats) · [Quick start](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#quick-start) · [Reading OCR output](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#reading-ocr-output) · [Emitting](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#emitting)
- [Consistency versus validity](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#consistency-versus-validity) · [What a check digit cannot prove](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#what-a-check-digit-cannot-prove) · [What a passing parse guarantees](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#what-a-passing-parse-guarantees)
- [Occluded cells](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#occluded-cells-apply_occlusion) · [Checking line 1](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#checking-line-1-select_line1) · [Conformance](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#conformance)
- [Feature flags](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#feature-flags) · [Versioning and MSRV](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#versioning-and-msrv) · [Changelog and roadmap](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#changelog-and-roadmap) · [License](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#license)

## Install

```toml
[dependencies]
mrz = "0.9"
```

**Upgrading from 0.8?** 0.9 is a breaking release for one input: a document number whose first
cell is the filler `<` is now refused. The
[migration guide](https://github.com/ruledicaprio/SynthPass/blob/main/MIGRATION.md#mrz-08--09)
says what to change, and the
[changelog](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/CHANGELOG.md)
says why each change was made.

## Formats

All five formats parse and emit.

| Format | Document                        | Layout       |
| ------ | ------------------------------- | ------------ |
| TD3    | Passports                       | 2 lines × 44 |
| TD2    | Official travel documents / IDs | 2 lines × 36 |
| TD1    | ID cards                        | 3 lines × 30 |
| MRV-A  | Visas (passport-book)           | 2 lines × 44 |
| MRV-B  | Visas (smaller)                 | 2 lines × 36 |

## Quick start

```rust
let doc = mrz::parse_td3(
    "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
    "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
).unwrap();

assert_eq!(doc.surname, "ERIKSSON");
assert_eq!(doc.date_of_birth.to_string(), "1974-08-12"); // expanded to ISO 8601

// Per-field evidence, not a single boolean. `Some(true)` verified,
// `Some(false)` refuted, `None` this format prints no such check digit.
assert_eq!(doc.checks.document_number, Some(true));
assert_eq!(doc.checks.date_of_birth, Some(true));
assert_eq!(doc.checks.composite, Some(true));
assert!(doc.valid()); // every check digit this format prints verified
```

`parse_td1`, `parse_td2`, `parse_mrv_a` and `parse_mrv_b` cover the other formats. Each has a
`*_with` variant taking `ParseOptions`.

## Reading OCR output

```rust
let text = "## REPUBLIC OF UTOPIA\n\
            P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n\
            L898902C36UTO7408122F1204159ZE184226B<<<<<10";

let doc = mrz::find_and_parse(text).expect("an MRZ");
assert_eq!(doc.surname, "ERIKSSON");
assert_eq!(doc.given_names, "ANNA MARIA");
assert!(doc.valid());
```

[`find_and_parse`](https://docs.rs/mrz/latest/mrz/fn.find_and_parse.html) locates an MRZ in noisy
OCR text and runs a check-digit-guided repair pass. It accepts a repaired reading only when its
check digits agree with it. When nothing validates you get the best-scoring partial read with its
honest `Checks`, or `NotFound` if the text only looked like an MRZ.

Edge cases, each with a runnable example on docs.rs:

- **Noisy text.** HTML-escaped fillers and lines merged onto one physical line are handled.
- **A valid zone that may hold the wrong line.** An issuing state in no registry, two near-identical
  lines, or a digit in a two-line format's name field ranks the zone below any unflagged valid zone in
  the same text. It is still returned when there is no other; the rank never refuses a valid read
  unless you opt in to `ParseOptions::refuse_repeated_line` (off by default), which drops a zone whose
  lines repeat one another and returns `MrzError::RepeatedLine` when nothing else is left.
- **Long document numbers.** A number longer than the 9-character field can overflow into optional
  data under Parts 5/6 note j. TD3 uses that form by crate policy; visas do not.
  [`full_document_number`](https://docs.rs/mrz/latest/mrz/struct.MrzData.html#method.full_document_number)
  reassembles it.
- **Unknown and partial birth dates.** Part 3 §4.8 lets an issuer fill an unknown date of birth with
  `<`, and §4.9 gives a filler the value zero, so `<<<<<<` with check digit `0` is a *valid*
  birth-date field. The parser also accepts fillers in expiry dates as a tolerance policy; §4.8 does
  not authorize them there. A parsed date is an
  [`MrzDate`](https://docs.rs/mrz/latest/mrz/enum.MrzDate.html) (`Calendar`, `OutOfCalendar`,
  `PartiallyUnknown`, `Unknown` or `Malformed`), so an issuer's unknown never looks like an OCR
  failure.
  [`date_completeness`](https://docs.rs/mrz/latest/mrz/fn.date_completeness.html) classifies a raw
  field the same way.
- **A sex cell other than `M`, `F` or `<`.** It is kept as read, as
  [`Sex::NonConformant`](https://docs.rs/mrz/latest/mrz/enum.Sex.html), not rewritten to `X`. No
  check digit covers the cell, so a misread there is otherwise invisible.

## Emitting

All five formats emit from a `*Fields` struct with typed `MrzDate` and `Sex` values plus MRZ-native
text fields such as 3-letter codes. Every check digit is computed for you, and dates print as
`YYMMDD`. With a document code the matching parser accepts, the output parses back as `valid()`.

```rust
use mrz::{format_td3, parse_td3, Td3Fields};

let zone = format_td3(&Td3Fields {
    issuing_country: "UTO".into(),
    document_number: "L898902C3".into(),
    surname: "Müller".into(),
    given_names: "Anna Maria".into(),
    nationality: "UTO".into(),
    date_of_birth: mrz::MrzDate::Calendar(mrz::Date::new(1974, 8, 12)),
    sex: mrz::Sex::Female,
    date_of_expiry: mrz::MrzDate::Calendar(mrz::Date::new(2030, 12, 31)),
    ..Default::default()
});
assert!(zone.starts_with("P<UTOMUELLER<<ANNA<MARIA<<")); // transliterated, not dropped

let (line1, line2) = zone.split_once('\n').unwrap();
assert!(parse_td3(line1, line2).unwrap().valid());
```

National characters are transliterated, not dropped:

- **Latin**, Doc 9303 Part 3 §6 A: `MÜLLER` emits as `MUELLER`, not `MLLER`. Five characters have more
  than one recommended form; see
  [`transliterate`](https://docs.rs/mrz/latest/mrz/fn.transliterate.html).
- **Cyrillic**, §6 B: `ИВАНОВ` emits as `IVANOV`, not fillers. Twelve rows (plus five word-initial
  rules) depend on the name's language: Serbian `Ж` is `Z`, Russian `Ж` is `ZH`. The emitters apply
  the base (≈ Russian) column. If you know the language, run
  [`transliterate_cyrillic`](https://docs.rs/mrz/latest/mrz/fn.transliterate_cyrillic.html)
  with the right
  [`CyrillicLanguage`](https://docs.rs/mrz/latest/mrz/enum.CyrillicLanguage.html) first.

The crate can *produce* a conformant transliteration but cannot *validate* one: the standard admits
several correct answers. §6 C (Arabic) is not implemented.

## Consistency versus validity

A verified composite constrains the *read*. Whether the document is in date is a separate question,
and the crate never reads the clock: you pass "today" in.

```rust
use mrz::Date;

let doc = mrz::parse_td3(
    "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
    "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
).unwrap();

assert!(doc.valid());                                   // every printed digit agrees ...
assert!(!doc.validity(Date::new(2026, 9, 14)).in_date); // ... and the specimen expired in 2012
```

## What a check digit cannot prove

A check digit is a 7-3-1 weighted sum taken **mod 10** (Part 3 §4.9), so it sees only each
character's value mod 10. Two characters are indistinguishable to *every* check digit exactly when
their values are congruent. `blindspot` says so outright:

```rust
use mrz::{blindspot, collisions, Blindspot};

// The OCR confusion everyone worries about is CAUGHT ...
assert!(matches!(blindspot('O', '0'), Blindspot::Caught { delta_mod10: 4 }));
// ... and the quiet one nobody mentions is not.
assert!(blindspot('K', '<').is_blind());

// The complete blind set for a character, not a sample of it.
assert_eq!(collisions('K'), vec!['0', 'A', 'U', '<']);
```

| Pair    | Verdict    | Why                        |
| ------- | ---------- | -------------------------- |
| O ↔ 0   | **caught** | 24 vs 0 — differ mod 10    |
| I ↔ 1   | **caught** | 18 vs 1                    |
| B ↔ 8   | **caught** | 11 vs 8                    |
| S ↔ 5   | **caught** | 28 vs 5                    |
| Z ↔ 2   | **caught** | 35 vs 2                    |
| K ↔ `<` | **blind**  | 20 vs 0 — congruent mod 10 |
| I ↔ S   | **blind**  | 18 vs 28                   |
| B ↔ L   | **blind**  | 11 vs 21                   |
| A ↔ K   | **blind**  | 10 vs 20                   |

The table is per swap, not per character. Across several positions the shift is
`Σ Δᵢ·wᵢ (mod 10)`, so swaps that are individually caught can cancel: two O↔0 at weights 7 and 3
give `24·(7+3) = 240 ≡ 0`. Measured over 76 real document-number mismatches, the undetectable ones
were *dominated* by pairs this table calls caught. Read it as "which single swaps are safe", never as
"which characters are safe".

So the crate layers structural checks on top of the arithmetic: country-code recognition and date
plausibility in `find_and_parse`'s repair gates, plus one guard every direct `parse_*` call applies.
A document number whose first cell is the filler `<` (including an all-filler field) is refused
outright as
[`MrzError::LeadingFiller`](https://docs.rs/mrz/latest/mrz/enum.MrzError.html), never merely a
failed check digit: Doc 9303 enters data from each field's left-hand position, so nothing was printed
to the left of it. The full derivation and the corpus numbers are on
[`Blindspot`](https://docs.rs/mrz/latest/mrz/enum.Blindspot.html), and
`cargo run -p mrz --example checksum_blindspots` demonstrates the law against the real parser.

## What a passing parse guarantees

Every cell of the five formats' fixed grids falls into one of three buckets. A passing `parse_*`
call means something different for each.

**Check-digit-covered.** The document number, date of birth and date of expiry on every format,
TD3's personal number, and the composite (TD1, TD2, TD3), each under its own digit; TD1 and TD2
optional data sit under the composite alone.

- *Guarantees:* the printed digit agrees with the 7-3-1 weighted sum of the cells it covers (Part 3
  §4.9), so the field is internally consistent. The document number's first cell also gets a content
  check: a leading filler is refused (`MrzError::LeadingFiller`).
- *Does not guarantee:* byte-identity with what was issued. `1`↔`L` and `6`↔`G` are the only
  `CONFUSABLES` pairs sharing a residue class, so they are the only single-cell misreads no check
  digit catches. Every other listed confusable is caught alone, though two can cancel in combination
  ([above](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#what-a-check-digit-cannot-prove)).
  Nor does it guarantee a real calendar day: `MrzDate::OutOfCalendar` is a valid, checksum-consistent
  outcome for a month of `13`, a `30`th of February, or the `000000` placeholder some specimens print.

**Structural.** Line 1's first cell, on every format.

- *Guarantees:* it matches the format's admissible document-code table: `P` (TD3), `V` (MRV-A/B), or
  one of `I`/`A`/`C` (TD1/TD2). Together with the document number's leading cell, this is one of two
  content checks a direct `parse_*` call makes outside the check digits.
- *Does not guarantee:* anything about the document code's second cell, which is accepted exactly as
  printed.

**Unverifiable.** Everything else: the document code's second cell, issuer, name, nationality, sex,
and MRV-A/MRV-B optional data (visas print no composite).

- *Guarantees:* membership in the MRZ alphabet (`0`-`9`, `A`-`Z`, `<`). Anything else is
  `BadCharacter`.
- *Does not guarantee:* that the issuing country or nationality is a real ICAO-registered code (a
  direct parse does no registry lookup), or that the sex cell is `M`, `F` or `<` (anything else
  survives as `Sex::NonConformant`, never rejected).

No check digit covers the document code, issuer, name, nationality or sex on any of the five formats,
so there is no arithmetic to hold them to. On TD2, TD3 and both visas that is all of line 1. TD1's
line 1 also carries the document number with its check digit, and optional data the composite covers.

**Direct parse versus `find_and_parse`.** A direct `parse_*` call reports what the check digits
establish, which is consistency, plus the one structural refusal above. It never applies the
country-registry or date-plausibility guards: those live in `find_and_parse`'s repair gates, and
plausibility is left to them.

## Occluded cells: `apply_occlusion`

When an image shows an occluder over part of an already-parsed zone,
[`apply_occlusion`](https://docs.rs/mrz/latest/mrz/fn.apply_occlusion.html) takes a
[`CellMask`](https://docs.rs/mrz/latest/mrz/struct.CellMask.html) of the covered cells and
withholds what the arithmetic cannot back up. Doctests on docs.rs show both outcomes.

- **Check-covered or structural cell masked:** the whole call fails with
  `MrzError::OccludedCheckedCell`. The cell is never rebuilt from check-digit arithmetic, even where
  it could be solved for uniquely.
- **Unverifiable cell masked:** its field is withheld, listed in `Occluded::fields` and blanked in
  `Occluded::data`, never returned as a value.
- **Name field:** a masked cell before a *visible* `<<` withholds the surname and the given names
  together, since a covered separator could either complete itself or continue a multi-part surname.
  A masked cell after it but before the given names' own visible end withholds the given names alone.
  A mask confined to trailing filler withholds nothing.
- **Empty mask:** always the identity.

Three things to know before you use the result:

- **`mrz_lines` keeps the covered cells.** `Occluded::data.mrz_lines` is the validated read, not a
  redaction (ADR-0026 rejects rewriting covered cells). It must never stand in for a withheld
  field.
- **A blank can look like a printed value.** A withheld `sex` reads `Sex::Unspecified` and a
  withheld optional-data slot reads `None`, exactly like a printed filler. Check `Occluded::fields`.
- **Run `select_line1` first.** On an `Occluded::data` whose names were withheld it can `Propose`
  them back from the OCR text.

See the function's documentation for the full semantics, including the overflow-layout case.

## Checking line 1: `select_line1`

On a two-line format the one field no check digit covers is line 1's name field. `select_line1` is
an opt-in look at it. Give it the OCR text and the zone `find_and_parse_with` accepted.

```rust
use mrz::{find_and_parse_with, select_line1, Line1Verdict, ParseOptions};

let text = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n\
            L898902C36UTO7408122F1204159ZE184226B<<<<<10";
let opts = ParseOptions::default();

let accepted = find_and_parse_with(text, &opts).unwrap();
let selection = select_line1(text, &accepted, &opts);
assert_eq!(selection.verdict, Line1Verdict::Kept); // the accepted name field is well formed
```

It reports whether the accepted name field is well formed (`Kept`), whether exactly one other line
of the same width, document code and issuing state would replace an ill-formed one without changing
anything else (`Proposed`), or why it left the zone alone. It never applies a change, never touches
a field other than the names, and nothing in `mrz` calls it.

It has been measured. In SynthPass's
[replayed A/B](https://github.com/ruledicaprio/SynthPass/blob/main/knowledge/benchmarks/line1-selection-ab-2026-09-29.md)
on the 261 public specimens, applying its proposals changed no outcome and took names exact among
hits from 13 of 33 to 15 of 33. All five synthetic formats were identical. That is one corpus, so
A/B it against a control on your own data before relying on it. `ParseOptions::class_sweep` remains
unmeasured.

## Conformance

`mrz` is verified against the ICAO Doc 9303 text, not against memory of it. Worked examples published
in the standard are pinned as test vectors: composite check digits, name encodings, transliterations,
the published TD1/TD2/visa specimens and TD3 fields from Part 3 §3.2. The older-edition `P<` TD3 line 1
differs from the current Part 4 `PP` specimen. Where the standard is **silent**, the crate says so
rather than inventing conformance:

- Two-digit-year century inference has no rule anywhere in Doc 9303. The pivot is this crate's
  policy, documented as such, and configurable per call.
- Name truncation is issuer-discretionary; Doc 9303 defines several strategies and states that
  truncation is not reliably detectable.
- §6 A transliteration is deliberately multi-valued for five characters. §6 B is language-dependent.
  Part 3 §6 B has no worked example, while Part 4 Appendix A Figure A-2 illustrates one. Its 48 rows
  are pinned by table-integrity checks.

The corroboration record — which passages are relied on, how each was verified, and which remain
unverified — is kept alongside the source corpus in
[`CONFORMANCE_BASIS.md`](https://github.com/ruledicaprio/SynthPass/blob/main/knowledge/docs9303/CONFORMANCE_BASIS.md).

## Feature flags

Both are off by default, keeping the base crate zero-dependency and wasm-clean.

- **`serde`** derives `Serialize` + `Deserialize` on the data types: `MrzData`, `Checks`, `Format`,
  `Field`, `SequenceCompleteness`, `ParseOptions`, `Date`, `DateValidity`, `DateCompleteness`,
  `ZoneField`, `Occluded`, and the five emitter inputs (`Td3Fields`, `Td2Fields`, `Td1Fields`, `MrvAFields`, `MrvBFields`). `MrzDate`
  and `Sex` implement both by hand as their text form: a date serialises as `"1974-08-12"` (or its
  raw field, such as `"74<<12"`) and sex as its zone character.
- **`zeroize`** derives `ZeroizeOnDrop` on `MrzData`, wiping its PII-bearing fields from memory when
  the value is dropped. Best-effort: `format`, `document_number_legacy_encoding` and `checks` are
  skipped, `MrzDate` keeps its variant after wiping its payload, `Sex` is fully overwritten, and
  copies made before the drop are out of reach.

## Versioning and MSRV

`mrz` is pre-1.0, so **the minor version is the breaking slot**: `mrz = "0.9"` picks up every `0.9.x`
fix and addition, and never a breaking `0.10.0`. CI diffs every change against the published API with
`cargo-semver-checks`, so a break cannot ship as a patch.

**Returned types are `#[non_exhaustive]`**, and so is the one options struct: `MrzData`, `Checks`,
`Date`, `DateValidity`, `ParseOptions` and every public enum. They grow without breaking you. Build
`ParseOptions` with `ParseOptions::default().with_pivot_yy(..)`. Functional update syntax is *not* an
escape hatch on a non-exhaustive struct: `ParseOptions { pivot_yy: 30, ..Default::default() }` is
rejected with `E0639`, just as the bare literal is.

The five emitter inputs (`Td3Fields`, `Td2Fields`, `Td1Fields`, `MrvAFields`, `MrvBFields`) are
**deliberately exhaustive**: they mirror field layouts ICAO 9303 fixes, and you build them with
struct expressions. Future tunables go in a separate non-exhaustive companion rather than as a new
field on one of them.

The minimum supported Rust version is **1.82**, set by `std::iter::repeat_n` and
`Option::is_none_or`. CI's `msrv` job enforces it for the zero-dependency build and for
`--all-features`. The `serde` feature follows its own upstream MSRV, comfortably below 1.82. The
`zeroize` feature would not: `zeroize` 1.9.0 and `zeroize_derive` 1.5.0 both moved to edition 2024
and rust-version 1.85, which cargo 1.82 cannot even parse. So `crates/mrz/Cargo.toml` caps both to
their last edition-2021 releases (`zeroize` `>=1.8.1, <1.9`, `zeroize_derive` `>=1.4, <1.5`). Raising
the floor is a minor-version change; the cap can be lifted once it reaches 1.85.

## Changelog and roadmap

- **[CHANGELOG](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/CHANGELOG.md)** —
  every published version, with the pull requests behind each entry.
- **[ROADMAP](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/ROADMAP.md)** — what
  patch, minor and major releases are for, the planned breaking window, and what 1.0 has to mean.

## License

MIT
