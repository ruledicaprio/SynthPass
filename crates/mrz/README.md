# mrz

[![crates.io](https://img.shields.io/crates/v/mrz.svg)](https://crates.io/crates/mrz)
[![docs.rs](https://docs.rs/mrz/badge.svg)](https://docs.rs/mrz)
[![downloads](https://img.shields.io/crates/d/mrz.svg)](https://crates.io/crates/mrz)
[![MSRV](https://img.shields.io/badge/MSRV-1.82-blue.svg)](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#versioning-and-msrv)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#license)

Zero-dependency (by default) [ICAO Doc 9303](https://www.icao.int/publications/pages/publication.aspx?docnum=9303)
Machine Readable Zone parser, emitter and check-digit validator for Rust — passports, ID cards
and visas.

A check digit is **arithmetic, not a model score** — it agrees with the read or it does not.
`mrz` verifies every printed digit under the standard 7-3-1 weighting and tells you exactly
which one agreed and which one failed, field by field.

It is equally exact about its own edges. A check digit is a strong *filter* and a weak
*oracle*: what it passes is not a random remainder but precisely
[what the arithmetic cannot see](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#what-a-check-digit-cannot-prove) — a closed-form set,
measured against a real corpus, and a public API rather than a footnote.

- **Evidence, field by field** — document number, date of birth, expiry, personal number, composite.
- **Reads messy OCR** — finds the zone in free text, and repairs a misread only when a check
  digit agrees with the repair.
- **Writes conformant zones** — all five formats, with Latin and Cyrillic names transliterated
  the way Doc 9303 prescribes.
- **Honest about its limits** — the substitutions no check digit can catch are a public API.
- **Small and safe** — no dependencies in the default build, no `unsafe`, no clock, no network. Builds for
  `wasm32-unknown-unknown`, and is property-tested never to panic on arbitrary input.

**▶ [Try it in your browser](https://ruledicaprio.github.io/SynthPass/)** — live WASM MRZ validator.

---

## Contents

- [Install](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#install) · [Supported formats](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#supported-formats) · [Quick start](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#quick-start)
- [Reading OCR output](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#reading-ocr-output) · [Emitting](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#emitting) ·
  [A checksum-consistent read is not a valid document](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#a-checksum-consistent-read-is-not-a-valid-document)
- [What a check digit cannot prove](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#what-a-check-digit-cannot-prove) · [Conformance](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#conformance)
- [Feature flags](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#feature-flags) · [Versioning and MSRV](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#versioning-and-msrv) · [License](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md#license)

## Install

```toml
[dependencies]
mrz = "0.8"
```

**Upgrading from 0.7?** 0.8 is a breaking release. The
[migration guide](https://github.com/ruledicaprio/SynthPass/blob/main/MIGRATION.md#part-1-the-mrz-crate-07--08)
has a before/after for every change, and the
[changelog](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/CHANGELOG.md)
says why each one was made.

## Supported formats

| Format | Document                        | Layout       | Parse | Emit |
| ------ | ------------------------------- | ------------ | :---: | :--: |
| TD3    | Passports                       | 2 lines × 44 | ✅ | ✅ |
| TD2    | Official travel documents / IDs | 2 lines × 36 | ✅ | ✅ |
| TD1    | ID cards                        | 3 lines × 30 | ✅ | ✅ |
| MRV-A  | Visas (passport-book)           | 2 lines × 44 | ✅ | ✅ |
| MRV-B  | Visas (smaller)                 | 2 lines × 36 | ✅ | ✅ |

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

`parse_td1`, `parse_td2`, `parse_mrv_a` and `parse_mrv_b` cover the other formats, each with a
`*_with` variant taking `ParseOptions`.

## Reading OCR output

[`find_and_parse`](https://docs.rs/mrz/latest/mrz/fn.find_and_parse.html) locates an MRZ inside
noisy OCR text — HTML-escaped fillers, lines merged onto one physical line — and runs a
check-digit-guided repair pass. A repaired reading is accepted only when its check digits agree
with it. When nothing validates, you get the best-scoring partial read with its honest `Checks`, or
`NotFound` if the text only looked like an MRZ.

```rust
let text = "## REPUBLIC OF UTOPIA\n\
            P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n\
            L898902C36UTO7408122F1204159ZE184226B<<<<<10";

let doc = mrz::find_and_parse(text).expect("an MRZ");
assert_eq!(doc.surname, "ERIKSSON");
assert_eq!(doc.given_names, "ANNA MARIA");
assert!(doc.valid());
```

Two cases that are easy to get wrong, both documented with runnable examples on docs.rs:

- **Document numbers longer than the 9-character field** can overflow into optional data
  under Parts 5/6 note j; TD3 uses that form by crate policy, while visas do not.
  [`full_document_number`](https://docs.rs/mrz/latest/mrz/struct.MrzData.html#method.full_document_number)
  reassembles them.
- **Unknown and partial birth dates** are conformant: Part 3 §4.8 lets an issuer fill
  an unknown date of birth with `<`, and §4.9 gives a filler the value zero, so `<<<<<<` with check digit `0` is a
  *valid* birth-date field. The parser also accepts fillers in expiry dates as a
  tolerance policy; Part 3 §4.8 does not authorize them there. A parsed date is an
  [`MrzDate`](https://docs.rs/mrz/latest/mrz/enum.MrzDate.html) (`Calendar`, `OutOfCalendar`,
  `PartiallyUnknown`, `Unknown` or `Malformed`), so an issuer's unknown never looks like an
  OCR failure. [`date_completeness`](https://docs.rs/mrz/latest/mrz/fn.date_completeness.html)
  classifies a raw field the same way.
- **A sex cell other than `M`, `F` or `<`** is kept as read, as
  [`Sex::NonConformant`](https://docs.rs/mrz/latest/mrz/enum.Sex.html), not rewritten to `X`. No
  check digit covers the cell, so a misread there is otherwise invisible.

## Emitting

All five formats emit from a `*Fields` struct with typed `MrzDate` and `Sex` values,
plus MRZ-native text fields such as 3-letter codes. Dates print as `YYMMDD` in the zone.
Every check digit is computed for you. With a document code accepted by the matching parser,
the output parses back as `valid()`.

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

National characters are **transliterated, not dropped**:

- **Latin**, Doc 9303 Part 3 §6 A — `MÜLLER` emits as `MUELLER`, not `MLLER`. Five characters
  have more than one recommended form; see
  [`transliterate`](https://docs.rs/mrz/latest/mrz/fn.transliterate.html).
- **Cyrillic**, §6 B — `ИВАНОВ` emits as `IVANOV`, not fillers. Twelve rows (plus five
  word-initial rules) depend on the name's language — Serbian `Ж` is `Z`, Russian `Ж` is `ZH` —
  so the emitters apply the base (≈ Russian) column. If you know the language, run
  [`transliterate_cyrillic`](https://docs.rs/mrz/latest/mrz/fn.transliterate_cyrillic.html)
  with the right
  [`CyrillicLanguage`](https://docs.rs/mrz/latest/mrz/enum.CyrillicLanguage.html) first.

Either way the crate can *produce* a conformant transliteration but cannot *validate* one: the
standard admits several correct answers. §6 C (Arabic) is not implemented.

## A checksum-consistent read is not a valid document

A verified composite constrains the *read*. Whether the document is in date is a separate
question, and the crate never reads the clock to answer it — you pass "today" in:

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
character's value mod 10. Two characters are indistinguishable to *every* check digit exactly
when their values are congruent — and `blindspot` says so outright:

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

**The table is per swap, not per character.** Across several positions the shift is
`Σ Δᵢ·wᵢ (mod 10)`, so substitutions that are individually caught can cancel — two O↔0 at
weights 7 and 3 give `24·(7+3) = 240 ≡ 0`. Measured over 76 real document-number mismatches,
the undetectable ones were *dominated* by pairs this table calls caught. Read it as "which
single swaps are safe", never as "which characters are safe".

That is why the crate layers structural checks on top of the arithmetic — recognized country
country-code recognition and date plausibility in `find_and_parse` repair
gates. Direct `parse_*` calls do not apply these structural guards. The full derivation and the corpus numbers are on
[`Blindspot`](https://docs.rs/mrz/latest/mrz/enum.Blindspot.html), and
`cargo run -p mrz --example checksum_blindspots` demonstrates the law against the real parser.

## Conformance

`mrz` is verified against the ICAO Doc 9303 text, not against memory of it. Worked examples
published in the standard — composite check digits, name encodings, transliterations, the
published TD1/TD2/visa specimens and TD3 fields from Part 3 §3.2 — are
pinned as test vectors. The older-edition `P<` TD3 line 1 differs from the
current Part 4 `PP` specimen. Where the standard is
**silent**, the crate says so rather than inventing conformance:

- Two-digit-year century inference has no rule anywhere in Doc 9303. The pivot is this crate's
  policy, documented as such, and configurable per call.
- Name truncation is issuer-discretionary; Doc 9303 defines several strategies and states that
  truncation is not reliably detectable.
- §6 A transliteration is deliberately multi-valued for five characters. §6 B is
  language-dependent. Part 3 §6 B has no worked example, while Part 4 Appendix A
  Figure A-2 illustrates one. Its 48 rows are pinned by table-integrity checks.

The corroboration record — which passages are relied on, how each was verified, and which
remain unverified — is kept alongside the source corpus in
[`CONFORMANCE_BASIS.md`](https://github.com/ruledicaprio/SynthPass/blob/main/knowledge/docs9303/CONFORMANCE_BASIS.md).

## Feature flags

Both are off by default, keeping the base crate zero-dependency and wasm-clean:

- **`serde`** — derives `Serialize` + `Deserialize` on the data types: `MrzData`, `Checks`,
  `Format`, `Field`, `SequenceCompleteness`, `ParseOptions`, `Date`, `DateValidity`,
  `DateCompleteness`, `DateRole`, `RawDateField`, `InvalidRawDateField`,
  `ParseMrzDateError`, `ParseSexError`, and the five emitter inputs (`Td3Fields`, `Td2Fields`, `Td1Fields`,
  `MrvAFields`, `MrvBFields`). `MrzDate` and `Sex` implement both by hand as their text form:
  a date serialises as `"1974-08-12"` (or its raw field, such as `"74<<12"`) and sex as its zone
  character.
- **`zeroize`** — derives `ZeroizeOnDrop` on `MrzData`, wiping its PII-bearing fields from memory
  when the value is dropped. Best-effort: `format`, `document_number_legacy_encoding` and
  `checks` are skipped, `MrzDate` keeps its variant after wiping its payload, `Sex` is fully
  overwritten, and copies made before the drop are out of reach.

## Versioning and MSRV

`mrz` is pre-1.0, so **the minor version is the breaking slot**: `mrz = "0.8"` picks up every
`0.8.x` fix and addition, and never a breaking `0.9.0`. CI diffs every change against the
published API with `cargo-semver-checks`, so a break cannot ship as a patch.

**Types this crate returns, and its one options struct, are `#[non_exhaustive]`** — `MrzData`,
`Checks`, `Date`, `DateValidity`, `ParseOptions` and every public enum — so they grow without
breaking you. Build `ParseOptions` with `ParseOptions::default().with_pivot_yy(..)`; note that
functional update syntax is *not* an escape hatch on a non-exhaustive struct, so
`ParseOptions { pivot_yy: 30, ..Default::default() }` is rejected with `E0639` just as the bare
literal is.

The five emitter inputs (`Td3Fields`, `Td2Fields`, `Td1Fields`, `MrvAFields`, `MrvBFields`) are
**deliberately exhaustive**: they mirror field layouts ICAO 9303 fixes, and you build them with
struct expressions. Future tunables go in a separate non-exhaustive companion rather than as a
new field on one of them.

The minimum supported Rust version is **1.82**, set by `std::iter::repeat_n` and
`Option::is_none_or`, and enforced by CI for the zero-dependency default build. The optional
features follow their own upstream MSRVs. Raising the floor is a minor-version change.

- **[CHANGELOG](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/CHANGELOG.md)** —
  every published version, with the pull requests behind each entry.
- **[ROADMAP](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/ROADMAP.md)** — what
  patch, minor and major releases are for, the planned breaking window, and what 1.0 has to mean.

## License

MIT
