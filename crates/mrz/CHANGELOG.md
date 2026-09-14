# Changelog — `mrz`

All notable changes to the [`mrz`](https://crates.io/crates/mrz) crate. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The crate is pre-1.0, so **the minor
slot is the breaking slot** — `^0.7` resolves any `0.7.x` but never `0.8.0`.
[`ROADMAP.md`](ROADMAP.md) says what each slot is for and what 1.0 has to mean.

`mrz` versions independently of the SynthPass workspace it lives in, so this file lists only
what a user of the crate can observe. Test-only and CI-only changes are left out.

**Where entries come from.** A pull request that changes the crate adds a fragment to
[`changelog.d/mrz/`](../../changelog.d/mrz/README.md) instead of editing this file. The pull
request that releases a version assembles them into `[Unreleased]` with
`scripts/assemble-changelog.sh --scope mrz --write`, then renames that heading to the version.
Every entry names the pull request or commit it came from, so it traces back to git.

## [Unreleased]

## [0.7.1] — 2026-09-14

### Added

- **`is_leap_year` is reachable.** The 0.3.0 entry below calls it public, but it was declared
  in a private module and never re-exported, so no published version let a caller use it. It is
  now `mrz::is_leap_year`.
- **A runnable example on every item docs.rs counts** — 78 of 78, where 0.7.0 had 44 of 60 —
  and on the methods it does not count. Four of them are ICAO's own published specimens, reproduced byte for byte:
  `Td2Fields` emits Doc 9303 Part 6's TD2 specimen, `Td1Fields` its TD1 counterpart, and
  `MrvAFields` and `MrvBFields` Part 7's two visa specimens.
- **[`ROADMAP.md`](ROADMAP.md)** — what each version slot is for, the proposed 0.8.0 breaking
  window, and the conditions 1.0 has to meet.
- **This changelog covers every published version.** It previously started at 0.7.1 and sent
  readers to the workspace changelog. The sections below are reconstructed from the published
  crates themselves: each `.crate` records the commit it was packaged from.

### Changed

- **`#![forbid(unsafe_code)]`.** The crate never contained `unsafe`; now the compiler keeps it
  that way.
- **The crate-level docs have a map.** They gained "Where to start", feature-flag and stability
  sections, and their module list names all nine modules — it named five.

### Fixed

- **Two more OCR glyph confusions recover in a checksum-failed read**
  ([#245](https://github.com/ruledicaprio/SynthPass/pull/245)). The damaged-read repair path
  (`find_and_parse` only, after nothing validated) now knows `M`↔`N` and `2`↔`7`, both measured
  on real specimen scans. As before, a repair is accepted only when the field's own check digit
  proves it and no other reading also verifies.
- **Documentation that said the wrong thing.**
  - `transliterate` said Cyrillic (§6 B) was out of scope. It has been implemented since 0.6.6,
    as `transliterate_cyrillic`.
  - `format_td3`'s national-character section covered Latin (§6 A) only. It now covers
    Cyrillic, including why a Serbian or Ukrainian name should be transliterated first.
  - The TD1, TD2 and TD3 emitter inputs said a long document number is silently truncated.
    Since 0.4.0 it overflows into the optional-data field whenever the remainder fits.
  - `Checks` said `personal_number` is `true` "for TD1/TD2". It is `true` on every format but
    TD3, and `composite` is likewise `true` on MRV-A and MRV-B.
  - `Format` said MRV-A and MRV-B arrived in 0.3.0. The published crates show 0.2.0.
  - Seventeen public doc comments pointed at private module documentation, which docs.rs never
    renders ("see the module docs"). They now point at public items, and the
    "recognition, never rejection" rationale moved onto `PassportType`, where it renders.
  - The README's install line said `mrz = "0.6"`, and its `serde` list named 7 of the 14 types
    that derive it.

## [0.7.0] — 2026-09-08

### Changed

- **BREAKING (contract): `find_and_parse` no longer returns non-MRZ text as a checksum-failed
  record** ([#238](https://github.com/ruledicaprio/SynthPass/pull/238)). When the best-scoring
  reading never validates *and* fails every structural signal at once — issuing state and
  nationality both unrecognized, date of birth not even six digits — it is OCR that matched
  MRZ-shaped text elsewhere on the document, and the result is now `Err(MrzError::NotFound)`.
  A genuine non-conformant line 1 still parses. No signature changed. The minor bump is because
  input that used to return `Ok` no longer does.

## [0.6.6] — 2026-09-08

### Added

- **Cyrillic transliteration, Doc 9303 Part 3 §6 B**
  ([#231](https://github.com/ruledicaprio/SynthPass/pull/231)). `transliterate_cyrillic`,
  `transliterate_cyrillic_char` and `CyrillicLanguage` (Russian — the default — Belarusian,
  Bulgarian, Serbian, Ukrainian, Macedonian). The emitters now write `ИВАНОВ` as `IVANOV`
  instead of dropping it to fillers, using §6 B's base column.
- **Part 4 §4.4 secondary document codes**
  ([#189](https://github.com/ruledicaprio/SynthPass/pull/189)). `PassportType`,
  `passport_type()` and `MrzData::passport_type()` cover the ten codes (`PP`, `PE`, `PD`, …).
  Recognition only: no code is ever rejected, and `P<` stays conformant.
- **`codes()`** ([#209](https://github.com/ruledicaprio/SynthPass/pull/209)) — every
  `(code, name)` pair behind `country_name` and `code_for_name`, so a caller can detect when
  the table changes.
- Runnable examples on the fifteen public functions that had none
  ([#234](https://github.com/ruledicaprio/SynthPass/pull/234)).

### Fixed

- A German passport that lost its line-1 position-1 filler now repairs: the three line-1 repair
  gates looked up `D<<` literally and never found Germany's single-letter code `D`
  ([#189](https://github.com/ruledicaprio/SynthPass/pull/189)).

## [0.6.5] — 2026-09-01

### Added

- **`MrzError::IncompleteSequence`** ([#165](https://github.com/ruledicaprio/SynthPass/pull/165)).
  `find_and_parse` tells "found line 1 of a format but never its companion" apart from
  `NotFound`.
- **`SequenceCompleteness`** ([#166](https://github.com/ruledicaprio/SynthPass/pull/166)) — one
  value spanning both arms of a `find_and_parse` result, with `MrzData::sequence_completeness()`
  and `SequenceCompleteness::from_parse_result()`.
- Every public item documented, enforced by `#![warn(missing_docs)]`
  ([#133](https://github.com/ruledicaprio/SynthPass/pull/133)).

### Fixed

- **Line 1 shifted right by a spurious inserted character** on TD3, MRV-A and MRV-B now
  repairs, arbitrated by the issuing-state lookup
  ([#138](https://github.com/ruledicaprio/SynthPass/pull/138)).
- **A TD2 line 1 that dropped its position-1 filler** now repairs, gated the same way
  ([#171](https://github.com/ruledicaprio/SynthPass/pull/171)).
- **A long name keeps its secondary identifier when truncated**
  ([#174](https://github.com/ruledicaprio/SynthPass/pull/174)). The emitters now shrink the
  primary identifier first, as Doc 9303 requires, and reproduce ICAO's worked examples byte for
  byte. Emitted bytes change for names that overflow the field; names that fit are unchanged.
- Six doc links pointed at private items and rendered dead on docs.rs; CI now fails on any
  rustdoc warning ([#179](https://github.com/ruledicaprio/SynthPass/pull/179)).

### Changed

- The README became a landing page, and its long-form prose moved onto the public items it
  describes, where docs.rs renders it ([#180](https://github.com/ruledicaprio/SynthPass/pull/180)).

## [0.6.4] — 2026-08-16

### Fixed

- **A dropped line-1 position-1 filler on TD3, MRV-A and MRV-B** (`P<BRA…` read as `PBRA…`,
  shifting every later field) is offered as a repair candidate ahead of the as-read line
  ([#130](https://github.com/ruledicaprio/SynthPass/pull/130)).
- **A collapsed `<<` name separator** — OCR dropping one of its fillers — no longer sends
  `given_names` to an empty string; the parser falls back to a single `<`
  ([#130](https://github.com/ruledicaprio/SynthPass/pull/130)).

The workspace changelog labelled both of these 0.6.3; the published 0.6.3 crate does not
contain them.

## [0.6.3] — 2026-08-05

### Added

- **Single-glyph substitution repair**
  ([`cdc3feb`](https://github.com/ruledicaprio/SynthPass/commit/cdc3feb)). `CONFUSABLES`,
  `substitution_candidates` and `solve_substitution` recover a field of the right width that
  carries one misread glyph, when its check digit proves the reading.
- **`codes_equivalent` and `encode_name_component`**
  ([`f3ffb3d`](https://github.com/ruledicaprio/SynthPass/commit/f3ffb3d)) — compare two codes
  that name the same state (`D` and `DEU`), and encode a name component exactly as the
  emitters do.

### Fixed

- **TD1's name line is read again** ([#124](https://github.com/ruledicaprio/SynthPass/pull/124)).
  A dropped position-1 filler on line 1 — which carries TD1's document-number check digit — is
  repaired, and the three-line scan no longer requires the lines to be strictly adjacent.
- The blind-spot documentation is scoped to single substitutions: swaps that are individually
  caught can cancel each other out
  ([#127](https://github.com/ruledicaprio/SynthPass/pull/127),
  [#129](https://github.com/ruledicaprio/SynthPass/pull/129)).

## [0.6.2] — 2026-07-30

### Changed

- The README was restructured and three inaccuracies in it corrected
  ([#96](https://github.com/ruledicaprio/SynthPass/pull/96)). Documentation only.

## [0.6.1] — 2026-07-30

### Added

- **`DateCompleteness`, `date_completeness` and `MrzData::date_of_birth_completeness`**
  ([#95](https://github.com/ruledicaprio/SynthPass/pull/95)) — tell a conformantly unknown date
  of birth (`<<<<<<` with check digit `0`) apart from an OCR failure.
- Seven missing Part 3 §5 registry codes: `XCE`, `XES`, `XMP`, `XDC`, `ANT`, `NTZ` and `IAO`
  ([#95](https://github.com/ruledicaprio/SynthPass/pull/95)).

## [0.6.0] — 2026-07-30

### Changed

- **BREAKING: long document numbers follow Doc 9303**
  ([#91](https://github.com/ruledicaprio/SynthPass/pull/91)). The emitters print all nine
  principal characters before the filler, where 0.5 printed eight. The parsers read that form,
  fall back to the pre-0.6 one (flagged by the new `MrzData::document_number_legacy_encoding`),
  and report a failed check rather than silently truncating.
- **Names are encoded per Part 3 §4.6**
  ([#93](https://github.com/ruledicaprio/SynthPass/pull/93)). Apostrophes are dropped with no
  filler (`O'CONNOR` → `OCONNOR`), other punctuation is dropped, and hyphens, commas and spaces
  each become one `<`.
- **Latin national characters are transliterated, Part 3 §6 A**
  ([#94](https://github.com/ruledicaprio/SynthPass/pull/94)). New `transliterate`,
  `transliterate_char`, `transliterations` and `TransliterationStyle`. The emitters write
  `MÜLLER` as `MUELLER`, where they used to write `MLLER`.

### Fixed

- Every Doc 9303 citation in the crate's documentation was checked against the standard's text,
  and nine were corrected ([#92](https://github.com/ruledicaprio/SynthPass/pull/92)).

## [0.5.1] — 2026-07-24

### Added

- **Recovery of lines that lost a character to damage**
  ([#70](https://github.com/ruledicaprio/SynthPass/pull/70)). When nothing else validates,
  `find_and_parse` sweeps the missing position across the line and lets the check digits rule.
  The primitives are public: `width_candidates`, `solve_field`, `Resolution`, `FieldKind`,
  `UNKNOWN` and `MRZ_ALPHABET`. A recovered record must also have well-formed dates, and two
  lost positions are reported as `Ambiguous` rather than guessed.

## [0.5.0] — 2026-07-22

### Added

- **The check digit's blind spots, as an API**
  ([#67](https://github.com/ruledicaprio/SynthPass/pull/67)): `blindspot`, `Blindspot`,
  `collisions`, `class_of` and `CLASSES`. O↔0, I↔1, B↔8, S↔5 and Z↔2 are caught; K↔`<`, I↔S,
  B↔L and A↔K are not.
- `rust-version = "1.82"`, enforced in CI.

## [0.4.0] — 2026-07-22

### Added

- **Document numbers longer than nine characters** are emitted into, and read back from, the
  optional-data field: `MrzData::document_number_full` and `full_document_number()`
  ([#65](https://github.com/ruledicaprio/SynthPass/pull/65)).
- `Field` and `Checks::failed()` name which check digit failed, and `MrzError::BadChecksum`
  carries one.
- `ParseOptions { pivot_yy }` and the `*_with` parsers pin the two-digit-year century pivot.

### Changed

- **BREAKING:** `MrzData`, `Checks`, `Format` and `MrzError` are `#[non_exhaustive]`, so later
  additions to them are not breaking; `MrzData` gained `document_number_full`
  ([#65](https://github.com/ruledicaprio/SynthPass/pull/65)).
- When no reading validates, `find_and_parse` returns the best-scoring one, not the first found.

## [0.3.0] — 2026-07-22

### Added

- **MRV-A and MRV-B emit** — `format_mrv_a`, `format_mrv_b`, `MrvAFields`, `MrvBFields` — and
  `find_and_parse` scans free text for visa lines
  ([#62](https://github.com/ruledicaprio/SynthPass/pull/62)).

### Fixed

- `Date::is_well_formed` applies the real Gregorian calendar: Feb 30, Feb 29 in a common year
  and April 31 are rejected ([#62](https://github.com/ruledicaprio/SynthPass/pull/62)).

## [0.2.0] — 2026-07-21

### Added

- **TD1 and TD2 emit** — `format_td1`, `format_td2`, `Td1Fields`, `Td2Fields`
  ([#61](https://github.com/ruledicaprio/SynthPass/pull/61)).
- **MRV-A and MRV-B parse** — `parse_mrv_a`, `parse_mrv_b`
  ([#61](https://github.com/ruledicaprio/SynthPass/pull/61)).
- `examples/checksum_blindspots.rs`, which demonstrates the check digit's blind spots against
  the real parser ([#59](https://github.com/ruledicaprio/SynthPass/pull/59)).

### Changed

- **BREAKING:** `Format` gained `MrvA` and `MrvB`. It was not yet `#[non_exhaustive]`, so an
  exhaustive `match` on it stopped compiling.

## [0.1.0] — 2026-07-21

First release as a standalone crate ([#58](https://github.com/ruledicaprio/SynthPass/pull/58)):
TD1, TD2 and TD3 parsing with per-field `Checks`; TD3 emission; `find_and_parse` with
check-digit-guided OCR repair; date expansion and plausibility (`MrzData::validity`); the ICAO
code registry (`country_name`, `code_for_name`); and the optional `serde` and `zeroize`
features.

[Unreleased]: https://github.com/ruledicaprio/SynthPass/tree/main/crates/mrz
[0.7.1]: https://crates.io/crates/mrz/0.7.1
[0.7.0]: https://crates.io/crates/mrz/0.7.0
[0.6.6]: https://crates.io/crates/mrz/0.6.6
[0.6.5]: https://crates.io/crates/mrz/0.6.5
[0.6.4]: https://crates.io/crates/mrz/0.6.4
[0.6.3]: https://crates.io/crates/mrz/0.6.3
[0.6.2]: https://crates.io/crates/mrz/0.6.2
[0.6.1]: https://crates.io/crates/mrz/0.6.1
[0.6.0]: https://crates.io/crates/mrz/0.6.0
[0.5.1]: https://crates.io/crates/mrz/0.5.1
[0.5.0]: https://crates.io/crates/mrz/0.5.0
[0.4.0]: https://crates.io/crates/mrz/0.4.0
[0.3.0]: https://crates.io/crates/mrz/0.3.0
[0.2.0]: https://crates.io/crates/mrz/0.2.0
[0.1.0]: https://crates.io/crates/mrz/0.1.0
