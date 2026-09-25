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

## [0.8.1] — 2026-09-25

### Fixed
- **Damaged-zone recovery no longer invents the format letter.** A passport whose line 1 OCR
  dropped a character could be returned as a checksum-valid MRV-A visa: restoration inserted
  `V` at cell 0, and the visa layout lacks TD3's personal-number and composite checks.
  Restoration now keeps the observed first cell while still recovering missing interior cells.
- **Damaged-zone recovery no longer picks one of several disagreeing readings.** When
  `find_and_parse` rebuilds a damaged zone, it returns a reading only if every recovered
  candidate is the same answer. It used to compare six fields: the document number, the two
  dates, the two names and the nationality. Candidates that differed in sex, issuing state,
  document code, optional data, the full document number or format counted as unanimous, and the
  first candidate won. The rest of the zone decided which one that was.

  No check digit covers sex, the issuing state, the document code or the format, so such a
  reading was checksum-consistent and still wrong. Candidates now have to agree on every
  `MrzData` field except `mrz_lines`, format included. Otherwise the recovery refuses and the
  ordinary checksum-failed result is returned.
- **A TD3 line 1 one cell too long no longer defaults to an unresolved issuing state.** Line 1
  carries no check digit, so when the as-read issuing state doesn't resolve in `country_name`,
  `find_and_parse` now tries deleting each single cell of the three-letter issuing-state slot on
  its own. Exactly one resolving deletion is used; two or more real, distinct countries are
  equally admissible, so the read is refused instead of guessed. An already-resolving issuing
  state is left untouched either way.
- **A genuine Part 4 §4.4 TD3 document code is no longer rewritten into another country.**
  `find_and_parse` was unshifting a real second document-code letter (`PP`, `PD`, `PS`, `PR`, ...)
  whenever the code's letter plus the issuer's first two letters happened to spell a different
  real ISO/ICAO state — e.g. a genuine Nigerian `PP` passport (`PPNGA...`) was read back as
  document code `P` and issuer `PNG` (Papua New Guinea). The as-read line is now kept whenever it
  already carries a resolving §4.4 code and issuer, before the unshifted reading is even tried.

## [0.8.0] — 2026-09-24

**Upgrading from 0.7:** follow the
[migration guide](https://github.com/ruledicaprio/SynthPass/blob/main/MIGRATION.md#part-1-the-mrz-crate-07--08).
It has a before/after for every change below.

**Why this release matters.** In 0.7, one parsed result could not answer three questions a
caller has to answer:
- Was a check digit absent, or did it fail?
- Which optional-data field held a value?
- Was the sex cell empty, or misread?

0.8 makes each answer a type you can match on:
- `Checks` reports `Option<bool>`: `None` when the format does not print that digit
  (ADR-0017).
- `optional_data_1` and `optional_data_2` replace a `personal_number` field that held three
  different things (ADR-0018).
- `MrzDate` and `Sex` keep a placeholder date, a partly unknown date and a misread cell apart
  from real values. The emitters take the same types (ADR-0019, ADR-0020).

Every factual claim in the crate's documentation was also checked against the code and
against the Doc 9303 PDFs. Each one is now true, or it was removed.

**Who needs to act.**
- Rust callers get a compile error at most changed sites. The migration guide also lists the
  value-level changes that still compile, such as error payloads and the sex cell.
- JSON consumers see four differences under the `serde` feature: `checks.*` can be `null`;
  `personal_number` becomes `optional_data_1`/`optional_data_2`; `sex` is the zone character
  (`"<"` for unspecified); `date_of_birth_completeness` is gone.
- JSON written by 0.7 still deserializes.

**What did not change.**
- No dependencies by default, and the optional `serde` and `zeroize` features are the same.
- MSRV is 1.82, and the crate still builds for `wasm32-unknown-unknown`.
- Date strings serialize byte for byte as before.
- SynthPass's real-specimen CI gate (261 documents) passed on every change in this release.

Breaking entries come first in each section.

### Added
- **`MrzData::checksum_consistent()`** — the named form of what `valid()` computes, and the
  preferred spelling from here on. Both return `checks.all_valid()`; neither is going away
  inside 0.8. The short name is the problem: `valid` sits three letters from `validity`,
  which answers an unrelated question (is the document in date?), so it reads as a verdict on
  the document when it is a verdict on the arithmetic. Its doc example now also shows what
  the verdict does *not* establish — TD3's composite spans line 2 only, so an altered surname
  on line 1 is exactly as checksum-consistent as the genuine one.
- **`ParseOptions::class_sweep` (off by default) reaches `solve_class_sweep` from `find_and_parse`.**
  Built with `ParseOptions::default().with_class_sweep(true)` — the first option added since the
  struct became `#[non_exhaustive]`, and serde-compatible with 0.7 options JSON because the new field defaults to false.

  The sweep runs in its own pass at the head of the damaged-capture search, for the fields that
  carry a check digit of their own (TD1 line 1's document number and line 2's dates; line 2's
  document number, dates and personal number on TD3; the same minus the personal number on TD2 and
  the MRV formats). Fields covered only by the composite are excluded: the composite cannot
  localise an error to a field, so sweeping one would be a guess the format cannot arbitrate.

  **It is a separate pass, preferred over the ordinary search, and that is a measured decision
  rather than a stylistic one.** Pooled into the existing damaged-capture shapes, the sweep loses
  to the machinery it complements. On the motivating shape — a nine-cell document number of one
  character read as its lookalike, check digit included — the single-substitution search finds
  **three** further readings that also validate, because swapping one cell of that run shifts the
  checksum by exactly the amount needed at any weight-3 position. Four disagreeing readings then
  reach the ambiguity guard, which correctly refuses to choose, and nothing is recovered at all.
  The sweep worked and still lost.

  So the tie-break is explicit, and it is a **prior, not arithmetic**: on the checksum evidence
  alone the four readings are equal. The sweep explains every differing cell with one decision
  about one glyph class, while each rival requires the recogniser to have read eight of nine
  identical glyphs correctly and exactly one of them differently — contradicting the uniformity the
  capture actually shows. That judgement is stated in `class_sweep_pass`'s own documentation rather
  than buried in an ordering.

  **Unmeasured on real documents.** It repairs a shape observed on one card back; how many
  documents it *breaks* is what the arm exists to find out. Off by default, and the damaged pass
  runs only after an ordinary read has already failed to validate.
- **`examples/bih_mrz_audit.rs`** — a worked example of layering issuer policy *over* the
  generic parser rather than inside it, which is the arrangement this crate's roadmap asks
  for. It recomputes each ICAO 9303 check digit independently, applies an opt-in
  Bosnia-and-Herzegovina profile, and never promotes an observational corpus pattern to a
  conformance requirement — its `<<<` optional-data check is opt-in, reported as a warning,
  and says in the message that it is not an ICAO failure. Run with
  `cargo run -p mrz --example bih_mrz_audit -- --profile auto FILE`.
- **`solve_class_sweep`.** Resolves a check-digited field where OCR misread every occurrence of
  one character as its `CONFUSABLES` partner, including the check-digit cell itself — the shape
  measured on a TD1 card back whose document number is nine cells of one digit plus a
  check-digit cell of the same digit, all read as one confusable letter. Field-scoped only (never
  a whole-line sweep), requires at least two occurrences of the swept character across the field
  and its check digit, and is additive alongside `solve_substitution`'s single-position repair —
  `MAX_SUBSTITUTIONS` is unchanged.
- **`MrzDate`, `Sex`, `RawDateField` and `DateRole`: typed date and sex values** (ADR-0019).
  - `MrzDate` gives each thing an MRZ date field can hold its own variant:
    - `Calendar(Date)` for six digits naming a real day;
    - `OutOfCalendar(Date)` for six digits naming none, such as the `000000` placeholder some
      specimens print;
    - `PartiallyUnknown` and `Unknown` for the issuer fillers Doc 9303 Part 3 §4.8 permits;
    - `Malformed` for a misread.
  - `Sex` separates ICAO's three zone values (`M`, `F`, and `<` for unspecified) from a
    `NonConformant(char)` kept as read, rather than collapsing every other character into `"X"`.
  - Their text form is a contract (ADR-0020): `Display`, `FromStr` and serde agree with the zone.
    A six-digit date renders byte-identically to `expand_date_with_pivot`, which a property test
    checks for every field, role and pivot. Values produced by the parser round-trip; caller-built variants can normalize or be rejected.
  - Under the `zeroize` feature, `Date`, `RawDateField`, `MrzDate` and `Sex` implement `Zeroize`,
    so a typed date of birth is still wiped. `RawDateField` is six inline bytes, so all four types
    stay `Copy`.

### Changed
- **Make check-digit absence explicit.** `mrz::Checks` now reports each digit as `Some(true)`, `Some(false)`, or `None` when the layout does not print it; consumers must handle the breaking optional check-state API and JSON `null` values.
- **`MrzData::personal_number` is gone: `optional_data_1` and `optional_data_2` replace it, and
  `personal_number()` names the TD3 element** (ADR-0018). The field held three different things
  under one name — TD3's personal number, TD2/MRV-A/MRV-B optional data, and TD1's two
  optional-data elements *joined with a space*, a join that could not be undone: a TD1 whose
  value sat in slot 1 was indistinguishable from one whose value sat in slot 2, and two tracked
  specimens (Belgium 2021, Serbia 2008) sit on opposite sides of exactly that. Now
  `optional_data_1` is the format's primary optional-data element — TD1 line 1 `[15,30)`, TD2
  `[28,35)`, TD3 `[28,42)`, MRV-A `[28,44)`, MRV-B `[28,36)` — and the document-number overflow
  target on every format that defines one; `optional_data_2` is TD1's second element (line 2
  `[18,29)`) and `None` on every other format; `personal_number()` returns `optional_data_1` on
  TD3 and `None` elsewhere, because only TD3 prints a personal number (Doc 9303 Part 4 §4.2.2
  titles that field "personal number or other optional data elements"). The parser now reports
  the shape the emitters (`Td1Fields`, `Td2Fields`, `MrvAFields`, `MrvBFields`, `Td3Fields`) have
  always taken. `Checks::personal_number` and `Field::PersonalNumber` are unchanged: they name
  the check digit, which only TD3 prints. The `serde` shape follows the fields — `personal_number`
  is no longer a key, `optional_data_1` and `optional_data_2` are. Migration: TD3 readers call
  `personal_number()` or read `optional_data_1`; TD2 and visa readers read `optional_data_1`;
  TD1 readers read both slots.
- **`MrzError::BadCharacter` now reports where the character was.** The variant carries
  `character`, `line` and `position` instead of a bare `char`; both indices are zero-based and
  counted in `char`s. `line` is `Option<usize>` and is `None` when there was no zone to count lines
  in -- the standalone `check_digit` helper is handed a single field, so its `position` is
  field-relative. Absence is not spelled `Some(0)`, because line 0 is a real line of a real zone.
- **Make parser error payloads consistent.** A line's length is measured in characters, not bytes: a line of the right length that contains a non-MRZ character such as `É` is now a `BadCharacter` at that character's position, not a `BadLength`, and `BadLength.got` counts characters; and `BadDocumentCode` carries both raw document-code cells for all five formats, including a trailing `<`. Callers comparing these payloads should update their matches.
- **`ParseOptions`, `Date` and `DateValidity` are now `#[non_exhaustive]`, and `ParseOptions` is built with `ParseOptions::default().with_pivot_yy(..)`.**
  These were the last three types in this crate's public surface that could grow only by breaking.
  Every public enum already carried the attribute, and so did `Checks` and `MrzData` — `Checks` for a
  reason that applies to all of them: *"a future MRZ format may carry a check digit these five fields
  don't name, and adding it should not be a breaking change."*

  **Breaking once, deliberately, while it is cheap.** `mrz` is pre-1.0, so cargo's breaking slot is
  the minor version; from 1.0 it becomes the major, and a major bump on a published crate splits the
  ecosystem. Paying for all three now costs one release. Paying per type later costs three.

  **Only `ParseOptions` changes anything for callers.** Replace `ParseOptions { pivot_yy: 30 }` with
  `ParseOptions::default().with_pivot_yy(30)`. `Date` is constructed through `Date::new` and
  `Date::from_epoch_days`, which already reach every field, and `DateValidity` is returned by
  `validity()` and never built by callers — so neither needs a builder and no workspace crate
  changes.

  **Functional update syntax is not a workaround, and this is the part worth knowing.**
  `ParseOptions { pivot_yy: 30, ..Default::default() }` is rejected with the same `E0639` — the
  restriction covers struct expressions including the `..base` form. Verified by compiling it rather
  than by reading the reference, after a first attempt to check it turned out vacuous. The `with_*`
  builder is the pattern instead, and each future option adds one method additively.

  **The five `*Fields` emit structs stay exhaustive, on purpose.** `Td1Fields`, `Td2Fields`,
  `Td3Fields`, `MrvAFields` and `MrvBFields` mirror field layouts ICAO 9303 fixes; they do not grow,
  and callers build them with struct expressions. Future tunables belong in a separate
  `#[non_exhaustive]` companion rather than as a new field on one of these — the separation
  `ParseOptions` already has from the data it parses. Each of the five now says so in its own
  rustdoc, and the policy is recorded in `knowledge/ARCHITECTURE.md` §13.4.

  No behaviour changes. `Default` still yields `CURRENT_YY`, `with_pivot_yy` is `const`, and no
  parser is touched.
- **Breaking:** `Td3Fields`, `Td2Fields`, `Td1Fields`, `MrvAFields`, and `MrvBFields` now take `MrzDate` for birth and expiry and `Sex` for the sex cell. Build dates with `MrzDate::Calendar(Date::new(...))` or classify six zone characters with `MrzDate::from_field`; use `Sex::Unspecified` for the MRZ filler `<`. Emitted zone bytes and check digits are unchanged for equivalent values. A `Sex::NonConformant` cell is written as given when it is an MRZ character (`A`-`Z`, `0`-`9`, `<`), and as `<` otherwise, so every emitted line is its exact width in the MRZ alphabet.
- **`MrzData`'s dates are `MrzDate` and its sex is `Sex`; `date_of_birth_completeness` is gone**
  (ADR-0019).
  - `date_of_birth` and `date_of_expiry` change from `String` to `MrzDate`, and `sex` from
    `String` to `Sex`. Read a date with `calendar()` (the day, when it is a real one), branch on
    its variant, or render it with `to_string()`, which gives exactly the string the field used
    to hold: ISO `YYYY-MM-DD` for six digits, the raw field otherwise.
  - `MrzData::date_of_birth_completeness` is removed. `date_of_birth.completeness()` returns the
    same `DateCompleteness`, and `date_of_expiry.completeness()` now answers the same question
    for expiry. `SequenceCompleteness::Complete { date_of_birth }` is unchanged.
  - `validity()` reads the typed dates. `dates_well_formed` is true exactly when both dates are
    `MrzDate::Calendar`, as before.
  - A misread sex cell is no longer indistinguishable from an unspecified one: `<` is
    `Sex::Unspecified`, and any character other than `M`, `F` and `<` is `Sex::NonConformant(c)`,
    `X` included. Callers that want the visual zone's vocabulary (`M`/`F`/`X`) map it themselves.
  - **The `serde` shape changes in two places** (ADR-0020). The `date_of_birth_completeness` key
    is gone. `sex` serialises as the zone character: `"M"` and `"F"` are unchanged, an
    unspecified cell is `"<"` where it was `"X"`, and a non-conformant cell keeps its character
    where every such cell used to be `"X"`. For example, a zone whose sex cell reads `1` used to
    serialise `"sex":"X"` and now serialises `"sex":"1"`. Dates serialise byte-identically.
  - Under the `zeroize` feature the three fields are wiped through their own `Zeroize` impls.
    `MrzData`'s documentation states that the wipe is best-effort and what it skips:
    `MrzDate`'s payload is cleared while its kind survives, and `Sex` is fully overwritten.
  - Migration: `d.date_of_birth == "1974-08-12"` becomes `d.date_of_birth.to_string() ==
    "1974-08-12"`, or `d.date_of_birth == MrzDate::Calendar(Date::new(1974, 8, 12))`;
    `d.date_of_birth_completeness` becomes `d.date_of_birth.completeness()`; `d.sex == "X"`
    becomes a match on `Sex::Unspecified | Sex::NonConformant(_)`.
  - JSON written by 0.7 still deserialises, but should be re-parsed from `mrz_lines`:
    old boolean `Checks` values can become `Some(true)` even for checks absent from the
    format, and the old `personal_number` key is dropped rather than mapped to
    `optional_data_1`. The removed `date_of_birth_completeness` key is
    ignored, but its `"sex": "X"` now reads as `Sex::NonConformant('X')`. 0.7 wrote `"X"` for
    `<` and for every misread cell alike, so the original cannot be recovered from old JSON;
    Re-parsing `mrz_lines` also restores the correct check applicability and optional data.

### Fixed
- **Correct ICAO authority names.** The `UNK`, `XCO` and `XPO` codes are present with their Part 3 §5 names, including the full UNMIK holder description for `UNK`.
- **Preserve legal second-position `K` codes.** OCR filler repair changes `PK` to `P<` only for passport codes; TD1, TD2 and visa issuer-defined `K` codes remain intact.
- **Restore old options JSON.** `ParseOptions` deserializes a 0.7-shaped object containing only `pivot_yy`; the new `class_sweep` flag defaults to `false`.
- **The documentation no longer claims a check digit proves a faithful read.** It does not: a
  valid check digit establishes that the candidate is *checksum-consistent* with the printed
  check digit, over the field that digit covers. It does not establish byte-identity with the
  printed zone — two substitutions the crate classifies as `Caught` can cancel mod 10, and on
  TD2 and TD3 the whole of line 1 (document code, issuing state, both name fields) carries no
  check digit at all. `Blindspot` and `CLASSES` already documented the arithmetic correctly;
  the crate-level, `README`, `checksum`, `parser` and `dates` headlines contradicted them, as
  did `Checks::all_valid`'s own summary line. All are restated in terms of what the
  arithmetic establishes. `MrzData::mrz_lines` likewise no longer calls itself "raw": it holds
  the repaired zone when `find_and_parse` normalized the input.

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
