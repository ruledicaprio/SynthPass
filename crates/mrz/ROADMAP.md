# `mrz` roadmap — to 1.0 and after

Where the crate is going, what each version slot is for, and what 1.0 has to mean. Every item
says whether it is delivered, planned or only proposed; none of it carries a date.

`mrz` versions independently of the SynthPass workspace it lives in (see
[`RELEASING.md`](../../RELEASING.md)), so this file covers the crate alone. The application's
roadmap is [`knowledge/ROADMAP.md`](../../knowledge/ROADMAP.md), and it wins any disagreement
about SynthPass itself.

## What the crate is for, and what it will not become

The job: turn MRZ text into verified fields, and fields into conformant MRZ text, with every
claim backed by the arithmetic or the text of ICAO Doc 9303 — deterministically, offline, and
with no runtime dependencies.

Non-goals, written down so they are not re-proposed:

- **OCR.** The crate takes text. Pixels belong to the caller.
- **Authenticity.** A check digit proves a faithful *read*, never a genuine *document*. No
  forgery detection and no eMRTD chip signature checks.
- **A clock, a network, or a runtime dependency.** "Today" is always a parameter, the code
  registry is compiled in, and the default build pulls in nothing.
- **Per-country special cases.** Rules come from Doc 9303 and apply to every issuer. A code table
  is used for recognition, never for rejection.

## How version numbers move

| Change | Before 1.0 | From 1.0 |
| --- | --- | --- |
| **Breaking** — removes or renames public API, changes a signature, adds a field to an exhaustive type, or tightens a contract (input that used to parse no longer does) | minor — `0.7` → `0.8` | major — `1.x` → `2.0` |
| **Additive** — a new function or type, a variant or field on a `#[non_exhaustive]` type, a new transliteration table or format | patch | minor |
| **Fixes and data** — bug fixes, repair heuristics, code-registry updates, the yearly century-pivot bump, documentation | patch | patch |
| **MSRV raise** (today 1.82) | minor | minor |

The mechanics are enforced, not remembered. Each change drops a fragment in
[`changelog.d/mrz/`](../../changelog.d/mrz/README.md) whose name carries the slot (a trailing
`!` marks a break). CI checks the version bump against the fragments
(`scripts/check-changelog.sh`) and against the published API (`cargo-semver-checks`).

## Where it stands — 0.7.x

Delivered:

- All five Doc 9303 layouts (TD1, TD2, TD3, MRV-A, MRV-B) parse and emit, and emitted zones
  round-trip through the parsers as valid.
- Per-field check-digit proof, and a free-text scanner that repairs OCR damage only under
  check-digit proof.
- Long document numbers, unknown and partial dates, Part 4 §4.4 passport types, and the
  Part 3 §5 code registry.
- Part 3 §6 A (Latin) and §6 B (Cyrillic) transliteration.
- The check-digit blind-spot atlas (`Blindspot`, `CLASSES`).
- CI gates for the MSRV, semver and rustdoc warnings. Every public item is documented, and
  every item docs.rs counts carries a runnable example.

## Patch track — 0.7.x

Additive work and fixes that break nothing. None of it is ordered or scheduled.

- **Repair heuristics measured on real specimens** — new `CONFUSABLES` rows, new shift and
  drop repairs. Each one ships with the measurement that justified it.
- **Registry data** — new or retired §5 codes, and the `CURRENT_YY` century pivot, bumped every
  year. CI fails once the pivot falls two years behind the clock.
- **Part 3 §6 C (Arabic) transliteration** — the last missing table. It needs a verifiable
  source the way §6 A had its Appendix B example. Not started.
- **`serde` on the remaining public types** — `PassportType`, `CyrillicLanguage`,
  `TransliterationStyle`, `Blindspot`, `Resolution`, `FieldKind` and `MrzError` carry no
  derives today. Anyone serializing diagnostics wants them.
- **A strict emit path** — `try_format_*` functions that return an error for input the
  emitters currently map to fillers or truncate. The existing total functions stay as they are.
- **`no_std` + `alloc`** — worth investigating before 1.0. The core is integer arithmetic and
  string slicing, which suits embedded document readers. `core::error::Error` has been stable
  since 1.81, below this crate's MSRV, so the switch may be possible without a break. What it
  needs is an audit of the `std::` paths and of the `serde` feature's `alloc` configuration.

## The breaking window — 0.8.0 (proposed)

Pre-1.0 is the cheap time to break, and the plan is to break **once**: collect every change
below into a single release rather than spreading them across several minors. Each is a
proposal to be decided on its merits, in an issue or an ADR of its own. None is a commitment.

1. **`ParseOptions` becomes `#[non_exhaustive]`, with a builder. — DONE, and it is what opens
   this window.** Built with `ParseOptions::default().with_pivot_yy(..)`; adding a second
   tunable — a repair budget, a strictness switch — is now additive.

   Two corrections this change established, both worth keeping. **"Every output type in the
   crate is already non-exhaustive" was not true** — an audit of all 21 public types found
   `Date` and `DateValidity` exhaustive as well, and both are now fixed in the same release,
   free, because `Date::new`/`Date::from_epoch_days` already reach every field and
   `DateValidity` is never built by callers. And **functional update syntax is not an escape
   hatch**: `T { field: x, ..Default::default() }` is rejected with `E0639` exactly as the bare
   literal is, so a `with_*` builder is the only ergonomic path. Policy recorded in
   `knowledge/ARCHITECTURE.md` §13.4.

   The five emitter inputs stay **deliberately exhaustive**: they mirror layouts ICAO 9303
   fixes, so future tunables belong in a separate non-exhaustive companion instead.
2. **Say what "not applicable" means in `Checks`.** `personal_number` and `composite` report
   `true` on formats that print no such check digit. The docs say so, but the type does not,
   and a caller can read "absent" as "verified". A tri-state would make the difference visible.
3. **Name the fields for what they hold. — DONE (ADR-0018).** `MrzData::personal_number`
   carried TD1, TD2 and MRV *optional data*, and TD1's two optional fields arrived joined into
   one. Now `optional_data_1` is every format's primary optional-data element,
   `optional_data_2` is TD1's second, and `personal_number()` names the TD3 element and returns
   `None` elsewhere — the shape the emitters and the spec transcription always had. The join
   was the defect worth the breaking slot: with one slot empty it lost which printed field held
   the value, and two tracked specimens sat on opposite sides of it.
4. **Typed values alongside the strings.** `date_of_birth` and `date_of_expiry` hold ISO text,
   or the raw field when a date is incomplete; `sex` is `"M"`, `"F"` or `"X"`. Typed accessors
   (`Date`, a `Sex` enum) can be added in a patch. Making them the primary representation is the
   breaking part, and it needs a decision on what the `serde` shape becomes.
5. **Richer errors.** `MrzError::BadCharacter` names the character but not its line or its
   position.

## 0.9.0 — release candidate

- Nothing breaks after 0.8.0. The 0.9 line exists to prove that: real downstream use runs on it
  for at least one full release cycle without needing a breaking fix.
- The `serde` representation — field names, enum tagging — is documented as API and pinned by
  a round-trip test per type. From 1.0, renaming a JSON field is a major change.
- The 1.x MSRV and semver policy is written into this file.

## 1.0.0 — when, not a date

1.0 ships when all of these hold:

- [ ] The 0.8.0 breaking window has closed, and a 0.9.x line shipped with no further break.
- [x] Every public item is documented, and every item docs.rs counts carries a runnable example.
- [x] Every Doc 9303 worked example the crate relies on is pinned as a test vector
      ([`CONFORMANCE_BASIS.md`](../../knowledge/docs9303/CONFORMANCE_BASIS.md)).
- [x] Parsing and emitting are property-tested never to panic on arbitrary input, and
      coverage-guided fuzz targets exist.
- [ ] Fuzzing runs on a schedule, not only on demand.
- [ ] Every published version from 0.7.1 on has a git tag and a changelog section.
- [ ] The `serde` format is pinned by tests.
- [ ] The 1.x MSRV and semver policy is published here.

## After 1.0

- **Major** — only when Doc 9303 itself changes in a way the types cannot absorb, or when a
  correctness fix cannot be made additively. Expected to be rare.
- **Minor** — new capability: §6 C, formats ICAO adds, new diagnostics, new optional features.
- **Patch** — fixes, repair heuristics, registry and pivot updates, documentation.

## Out of scope for this crate

- **eMRTD chip data** (Doc 9303 Parts 10–12) — signed binary data groups, not text. If it is
  ever built, it belongs in a separate crate.
- **AAMVA PDF417 and ISO/IEC 18013-5 mDL** — different standards with different mechanisms,
  tracked in SynthPass's roadmap rather than here.
