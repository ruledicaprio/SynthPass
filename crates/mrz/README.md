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
- [Parsing](#parsing) · [Emitting](#emitting)
- [What a check digit cannot prove](#what-a-check-digit-cannot-prove)
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

[`find_and_parse`](https://docs.rs/mrz/latest/mrz/fn.find_and_parse.html)
locates an MRZ inside noisy OCR output — it tolerates HTML-escaped fillers and
lines merged onto one physical line — and drives a check-digit-guided repair
pass. A candidate reading is accepted only when its composite check digit
proves it.

```rust
let text = "## REPUBLIC OF UTOPIA\n\
            P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n\
            L898902C36UTO7408122F1204159ZE184226B<<<<<10";

let doc = mrz::find_and_parse(text).expect("an MRZ");
assert_eq!(doc.surname, "ERIKSSON");
assert_eq!(doc.given_names, "ANNA MARIA");
assert!(doc.valid());
```

Two cases the parser handles that are easy to get wrong, both documented with
runnable examples on docs.rs:

- **Document numbers longer than the 9-character field** overflow into
  optional data per Doc 9303, and
  [`full_document_number`](https://docs.rs/mrz/latest/mrz/struct.MrzData.html#method.full_document_number)
  reassembles them.
- **Unknown and partial dates** are conformant, not corrupt: §4.8 lets an
  issuer fill a date with `<`, and §4.9 gives a filler the value zero, so
  `<<<<<<` with check digit `0` is a *valid* field.
  [`date_completeness`](https://docs.rs/mrz/latest/mrz/fn.date_completeness.html)
  is how you tell that apart from an OCR failure.

## Emitting

All five formats emit — `format_td3` / `format_td2` / `format_td1` /
`format_mrv_a` / `format_mrv_b`, each taking its own `*Fields` struct in
MRZ-native form (`YYMMDD` dates, uppercase text). Every check digit is computed
for you, so output always round-trips back through the matching parser as
`valid()`. `Td1Fields` and `Td2Fields` follow the same shape, with
`optional_data_1` + `optional_data_2` and `optional_data` respectively.

National characters are **transliterated, not dropped**: Doc 9303 Part 3 §6 A
is applied automatically, so `MÜLLER` emits as `MUELLER` rather than `MLLER`.
Five of the table's 95 characters have more than one ICAO-recommended
transliteration and the issuing State chooses among them, so the crate can
*produce* a conformant transliteration but cannot *validate* one — see
[`transliterate`](https://docs.rs/mrz/latest/mrz/fn.transliterate.html).

## What a check digit cannot prove

Check digits are the crate's strongest guarantee, so its limits are worth
stating as precisely as the guarantee itself.

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

**The table above is per-swap, not per-character.** Across several positions
the shift is `Σ Δᵢ·wᵢ (mod 10)`, so substitutions that are individually
**caught** can cancel each other — two O↔0 at weights 7 and 3 give
`24·(7+3) = 240 ≡ 0`. Measured over a corpus of 76 document-number mismatches,
the undetectable set was in fact *dominated* by pairs this table calls caught,
O↔0 most of all. So read it as "which single swaps are safe", and do not infer
that a character listed as caught is safe wherever it appears.

`mrz::CLASSES` is the whole atlas: ten residue classes partitioning the
37-character alphabet. Because the arithmetic has a known blind set, the crate
layers structural checks on top of it — recognized country codes, date
plausibility, and name charset rules. A verified composite also proves only the
*read*, never that the document is in date; that is `MrzData::validity(today)`,
which takes an explicit reference date so the crate stays clock-free.

The full derivation, the corpus numbers, and why this makes the check digit a
strong *filter* but a weak *oracle* are on
[`Blindspot`](https://docs.rs/mrz/latest/mrz/enum.Blindspot.html).
`cargo run -p mrz --example checksum_blindspots` demonstrates the law
empirically — it mutates the ICAO specimen character by character and asks the
real parser, rather than asserting the algebra at you.

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
