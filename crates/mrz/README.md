# mrz

[![crates.io](https://img.shields.io/crates/v/mrz.svg)](https://crates.io/crates/mrz)
[![docs.rs](https://docs.rs/mrz/badge.svg)](https://docs.rs/mrz)
[![MSRV](https://img.shields.io/badge/MSRV-1.82-blue.svg)](#minimum-supported-rust-version)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](#license)

Zero-dependency [ICAO Doc 9303](https://www.icao.int/publications/pages/publication.aspx?docnum=9303)
Machine Readable Zone parser, emitter and check-digit validator for Rust.

A valid composite check digit is **mathematical proof** that an OCR read is
faithful to the printed document — not a probability, not a model score. `mrz`
verifies every printed check digit under the standard 7-3-1 weighting and
reports the result per field, so you always know *which* digit proved the read,
and which one failed.

The crate has **no runtime dependencies** and compiles to
`wasm32-unknown-unknown` as readily as to native targets.

**▶ [Try it in your browser](https://ruledicaprio.github.io/SynthPass/)** — live WASM MRZ validator.

---

## Contents

- [Install](#install) · [Supported formats](#supported-formats) · [Quick start](#quick-start)
- [Parsing](#parsing) — [OCR text](#scan-free-form-ocr-text) · [long document numbers](#long-document-numbers) · [unknown dates](#unknown-and-partial-dates)
- [Emitting](#emitting) — [national characters](#national-characters-are-transliterated-not-dropped)
- [What a check digit cannot prove](#what-a-check-digit-cannot-prove) — [expiry](#it-does-not-prove-the-document-is-in-date) · [blind character pairs](#it-cannot-distinguish-characters-congruent-mod-10) · [structural guards](#structural-guards)
- [Conformance](#conformance) · [Feature flags](#feature-flags) · [MSRV](#minimum-supported-rust-version) · [License](#license)

## Install

```toml
[dependencies]
mrz = "0.6"
```

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
assert_eq!(doc.date_of_birth, "1974-08-12"); // expanded to ISO 8601

// Per-field proof, not a single boolean.
assert!(doc.checks.document_number);
assert!(doc.checks.date_of_birth);
assert!(doc.checks.composite);
assert!(doc.valid()); // every check digit verified
```

`parse_td1`, `parse_td2`, `parse_mrv_a` and `parse_mrv_b` cover the other
formats, each with a `*_with` variant taking `ParseOptions`.

## Parsing

### Scan free-form OCR text

`find_and_parse` locates an MRZ inside noisy OCR output — it tolerates
HTML-escaped fillers and lines merged onto one physical line — and drives a
check-digit-guided repair pass. A candidate reading is accepted only when its
composite check digit proves it.

```rust
let text = "## REPUBLIC OF UTOPIA\n\
            P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n\
            L898902C36UTO7408122F1204159ZE184226B<<<<<10";

let doc = mrz::find_and_parse(text).expect("an MRZ");
assert_eq!(doc.surname, "ERIKSSON");
assert_eq!(doc.given_names, "ANNA MARIA");
assert!(doc.valid());
```

### Long document numbers

When a document number exceeds the 9-character field, Doc 9303 puts the nine
principal characters in the field, a filler in the check-digit position, and
the remainder plus its check digit at the front of the optional-data field.
`mrz` emits and reads that form, exposing both the printed field and the
reassembled number:

```rust
use mrz::{format_td3, Td3Fields};

let lines = format_td3(&Td3Fields {
    issuing_country: "UTO".into(),
    document_number: "L898902C31234".into(), // 13 chars, overflows the 9-char field
    surname: "ERIKSSON".into(),
    given_names: "ANNA MARIA".into(),
    nationality: "UTO".into(),
    date_of_birth: "740812".into(),
    sex: "F".into(),
    date_of_expiry: "120415".into(),
    ..Default::default()
});

let (l1, l2) = lines.split_once('\n').unwrap();
let doc = mrz::parse_td3(l1, l2).unwrap();
assert!(doc.valid());
assert_eq!(doc.document_number, "L898902C3");            // the printed 9-char field
assert_eq!(doc.full_document_number(), "L898902C31234"); // the reassembled number
assert!(!doc.document_number_legacy_encoding);           // read as the Doc 9303 form
```

This is normative for TD1 and TD2 (Part 5 and Part 6, note j). **Part 4 defines
no overflow rule for TD3**, so applying it there is a deliberate extension by
analogy, made because issuers do it in practice; Part 7 defines none for
MRV-A/MRV-B, so visas never overflow. When the remainder does not fit the
optional field, the number truncates to 9 characters instead.

The parser also accepts the eight-character encoding this crate emitted before
0.6.0, flagging it via `document_number_legacy_encoding`.

### Unknown and partial dates

Doc 9303 Part 3 §4.8 lets an issuer complete an unknown date of birth with
filler characters, and §4.9 gives a filler the value zero for check-digit
purposes. An entirely unknown date of birth with check digit `0` is therefore a
**valid** field, not a corrupt read:

```rust
use mrz::{check_digit, date_completeness, DateCompleteness};

assert_eq!(check_digit("<<<<<<").unwrap(), 0);

assert_eq!(date_completeness("740812"), DateCompleteness::Complete);
assert_eq!(date_completeness("7408<<"), DateCompleteness::PartiallyUnknown);
assert_eq!(date_completeness("<<<<<<"), DateCompleteness::Unknown);
assert_eq!(date_completeness("7X0812"), DateCompleteness::Malformed);
```

`MrzData::date_of_birth_completeness` carries this for a parsed document. It is
how a caller separates "the issuer conformantly recorded an unknown date" from
"the OCR failed" — both leave `date_of_birth` holding the raw field rather than
an ISO date, and the raw `YYMMDD` is not recoverable afterwards, since a known
date has been reshaped to ten characters.

## Emitting

All five formats emit — `format_td3` / `format_td2` / `format_td1` /
`format_mrv_a` / `format_mrv_b`, each taking its own `*Fields` struct in
MRZ-native form (`YYMMDD` dates, uppercase text). Every check digit is computed
for you, so output always round-trips back through the matching parser as
`valid()`.

```rust
use mrz::{format_td3, Td3Fields};

let lines = format_td3(&Td3Fields {
    issuing_country: "UTO".into(),
    document_number: "L898902C3".into(),
    surname: "ERIKSSON".into(),
    given_names: "ANNA MARIA".into(),
    nationality: "UTO".into(),
    date_of_birth: "740812".into(), // YYMMDD, MRZ-native
    sex: "F".into(),
    date_of_expiry: "120415".into(),
    personal_number: Some("ZE184226B".into()),
    ..Default::default()
});

let (l1, l2) = lines.split_once('\n').unwrap();
assert!(mrz::parse_td3(l1, l2).unwrap().valid());
```

`Td1Fields` and `Td2Fields` follow the same shape, with `optional_data_1` +
`optional_data_2` and `optional_data` respectively:

```rust
use mrz::{format_td1, Td1Fields};

let td1 = format_td1(&Td1Fields {
    issuing_country: "UTO".into(),
    document_number: "D23145890".into(),
    surname: "ERIKSSON".into(),
    given_names: "ANNA MARIA".into(),
    nationality: "UTO".into(),
    date_of_birth: "740812".into(),
    sex: "F".into(),
    date_of_expiry: "120415".into(),
    ..Default::default() // document_code defaults to "I"
});
assert_eq!(td1.lines().count(), 3); // three 30-char lines
```

### National characters are transliterated, not dropped

Doc 9303 Part 3 §6 A defines a recommended transliteration for Latin national
characters that do not fit the MRZ's `[A-Z0-9<]` alphabet. The emitters apply
it automatically, so accented letters survive instead of being deleted:

```rust
use mrz::{format_td3, Td3Fields};

let lines = format_td3(&Td3Fields {
    issuing_country: "DEU".into(),
    surname: "MÜLLER".into(),
    given_names: "TÉRÈSA".into(),
    ..Default::default()
});
let line1 = lines.split_once('\n').unwrap().0;
assert!(line1.contains("MUELLER"));
assert!(line1.contains("TERESA"));
```

Five of the table's 95 characters have **more than one** ICAO-recommended
transliteration, and the issuing State chooses among them. `transliterate`
exposes that choice rather than pretending to a single canonical answer:

```rust
use mrz::{transliterate, TransliterationStyle};

assert_eq!(transliterate("Ñ", TransliterationStyle::Expanded), "N");
assert_eq!(transliterate("Ñ", TransliterationStyle::XxSuffix), "NXX");
assert_eq!(transliterate("ß", TransliterationStyle::Simple), "SS"); // only one ICAO answer
```

This crate can *produce* a conformant transliteration; it cannot *validate*
one, because the standard admits several correct answers. Only Latin-based
characters (§6 A) are implemented — Cyrillic (§6 B) and Arabic (§6 C) are out
of scope.

## What a check digit cannot prove

Check digits are the crate's strongest guarantee, so its limits are worth
stating as precisely as the guarantee itself.

### It does not prove the document is in date

A verified composite proves the *read* is faithful. Whether the document is
expired, or its dates are internally consistent, is a separate and
non-cryptographic judgement — `MrzData::validity(today)`, which takes an
explicit reference date so the crate stays deterministic and clock-free:

```rust
use mrz::{parse_td3, Date};

// The ICAO specimen (Utopia / Anna Maria Eriksson): expires 2012-04-15.
let doc = parse_td3(
    "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
    "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
)
.unwrap();
assert!(doc.valid()); // the read is checksum-proven ...

let report = doc.validity(Date::new(2010, 1, 1));
assert!(report.in_date); // ... and, as of 2010, still in date
assert!(report.dob_before_expiry);

// The same faithful read, judged against a later day: still valid(), expired.
assert!(!doc.validity(Date::new(2020, 1, 1)).in_date);
```

### It cannot distinguish characters congruent mod 10

A check digit is a 7-3-1 weighted sum taken **mod 10** (Part 3 §4.9), so it
sees only each character's value mod 10. Two characters are indistinguishable
to *every* check digit exactly when their values are congruent — and
`blindspot` says so outright:

```rust
use mrz::{blindspot, collisions, Blindspot};

// The OCR confusion everyone worries about is CAUGHT ...
assert!(matches!(blindspot('O', '0'), Blindspot::Caught { delta_mod10: 4 }));
// ... and the quiet one nobody mentions is not.
assert!(blindspot('K', '<').is_blind());

// The complete blind set for a character, not a sample of it.
assert_eq!(collisions('K'), vec!['0', 'A', 'U', '<']);
```

That inversion is the point:

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

`mrz::CLASSES` is the whole atlas: ten residue classes partitioning the
37-character alphabet. Class 0 is `0 A K U <` — five members, because the
filler and the digit zero share a value. Any substitution *within* a row is
provably undetectable; any substitution *across* rows is caught.

`cargo run -p mrz --example checksum_blindspots` demonstrates this
empirically — it mutates the ICAO specimen character by character and asks the
real parser, rather than asserting the algebra at you.

### Structural guards

Because the arithmetic has a known blind set, the crate layers structural
checks on top of it: recognized country codes, date plausibility, and name
charset rules.

```rust
use mrz::{code_for_name, country_name};

assert_eq!(country_name("UTO"), Some("Utopia (ICAO specimen)"));
assert_eq!(country_name("XXA"), Some("Stateless person (1954 Convention)"));
assert_eq!(code_for_name("Germany"), Some("DEU"));
assert_eq!(country_name("ZZZ"), None);
```

The table covers the full Doc 9303 Part 3 §5 registry — ISO 3166-1 codes plus
the ICAO extensions, United Nations codes, stateless/refugee codes, deprecated
codes kept for backward compatibility, and the specimen code `UTO`.

## Conformance

`mrz` is verified against the ICAO Doc 9303 text rather than against memory of
it. Worked examples published in the standard — composite check digits, name
encodings, transliterations — are pinned as test vectors, and the crate
documents where the standard is **silent** rather than inventing conformance:

- Two-digit-year century inference has no rule anywhere in Doc 9303. The pivot
  is this crate's policy and is documented as such.
- Name truncation is issuer-discretionary; Doc 9303 defines several strategies
  and states that truncation is not reliably detectable.
- Transliteration is deliberately multi-valued for five characters.

The corroboration record — which passages are relied on, how each was verified,
and which remain unverified — is kept alongside the source corpus in
[`CONFORMANCE_BASIS.md`](https://github.com/ruledicaprio/SynthPass/blob/main/knowledge/docs9303/CONFORMANCE_BASIS.md).

## Feature flags

Both are off by default, keeping the base crate zero-dependency and wasm-clean:

- **`serde`** — derives `Serialize` + `Deserialize` on `MrzData`, `Checks`,
  `Format`, `Td3Fields`, `Date`, `DateValidity` and `DateCompleteness`.
- **`zeroize`** — derives `ZeroizeOnDrop` on `MrzData`, wiping the PII-bearing
  `String` fields from memory when the value is dropped.

## Minimum supported Rust version

**1.82.** The floor is set by `std::iter::repeat_n` and `Option::is_none_or` in
the OCR repair helpers, both stabilized in that release. It applies to the
zero-dependency default build and is enforced by CI on every push; the optional
`serde`/`zeroize` features and the dev-dependency test suite follow their own
upstream MSRVs, which are higher. The MSRV is a floor, not a pin — raising it
is a minor version bump.

## License

MIT
