# CONFORMANCE_BASIS.md

Living record of which passages in this corpus the ICAO Doc 9303 conformance
program (`crates/mrz` test/documentation segments, S0 onward) actually
relies on, and how each one was corroborated. Extended by later segments as
they lean on new passages — do not let it drift out of date with the tests
it backs.

## Why this exists

ADR-0003 (`../decisions/ADR-0003-docs9303-source-of-truth.md`) records, as
its own negative consequence, that a cleanup-script bug briefly clipped real
content from several files' endings, and that the corpus "should be
spot-checked against source again before being treated as fully
authoritative" rather than trusted just because it looks clean. There are no
source PDFs in this repository to diff against (see
[README.md](README.md)'s "What does not belong here"), so verification here
is necessarily internal to the corpus.

For the MRZ check-digit core specifically, that internal verification is
strong: Appendix A's worked examples (Part 3) are numeric, hand-checkable
arithmetic, and the per-part layout tables (Parts 4-6) that define the
composite check-digit ranges were produced independently of those examples.
When a worked example reproduces exactly under a range read from a separate
table in a separate part, the two corroborate each other — a transcription
error in either one would very likely have broken the match, rather than
happening to compensate for it.

## Corroboration levels

- **worked example reproduced** — ICAO publishes a fully worked numeric
  example (inputs, intermediate products, sum, final digit); this program
  recomputed it independently (not by calling the crate under test) and it
  matched exactly.
- **cross-part agreement** — a layout/offset claim in one part is consistent
  with how a related part or the worked examples use it, without there
  being a standalone worked example to check it against directly.
- **unverified** — taken on the strength of the corpus's own internal
  consistency and the cleanup process in ADR-0003, with no independent
  worked example or cross-part check performed yet.

## Entries

### Part 3 Appendix A — composite check-digit worked examples

`Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:1241-1383`

Establishes three fully worked composite check-digit calculations (Examples
3-5), each showing every per-character product and the final sum. Pinned
verbatim in `crates/mrz/tests/icao_vectors.rs`'s `COMPOSITE_VECTORS`.

| Example | Format | Lines | Composite input | Sum | Digit |
|---|---|---|---|---|---|
| 3 | TD3 | `:1247` | `HA672242<658022549601086<<<<<<<<<<<<<<0` | 448 | 8 |
| 4 | TD1 | `:1290, :1292` | `D231458907<<<<<<<<<<<<<<<34071279507122<<<<<<<<<<<` | 392 | 2 |
| 5 | TD2 | `:1346` | `HA672242<658022549601086<<<<<<<` | 448 | 8 |

**Status: worked example reproduced.** All three sums and digits were
recomputed independently under the 7-3-1 weighting (not by calling
`mrz::check_digit`) and matched ICAO's published values exactly.

These examples also corroborate the composite character ranges each parser
uses, cross-checked against the layout tables below:

- TD3: `crates/mrz/src/parser.rs:141-144` — `line2[0..10] + line2[13..20] +
  line2[21..43]`. Excludes nationality (positions 11-13) and sex
  (position 21) from the printed line, matching Part 4's data-element
  directory.
- TD2: `crates/mrz/src/parser.rs:205-208` — `line2[0..10] + line2[13..20] +
  line2[21..35]`. Same exclusions as TD3.
- TD1: `crates/mrz/src/parser.rs:278-287` — `line1[5..30] + line2[0..7] +
  line2[8..15] + line2[18..29]`. Excludes the *entire* third line (the
  30-character name field), not merely nationality/sex — confirmed against
  Part 5's own composite-range table, next entry.

### Part 5 §4.2.4 composite check-digit table

`Doc_9303_Part5_Specs_for_TD1_MROTDs.md:476-479`

States the TD1 composite check digit covers "6-30 (upper line), 1-7, 9-15,
19-29 (middle line)" and explicitly notes positions "1-30 (lower line)...
are excluded" — i.e. the whole third (name) line is out of scope, unlike
TD2/TD3 where only nationality and sex are excluded.

**Status: cross-part agreement.** Independently derived from Part 5's own
table, and it reproduces the same composite input Part 3's Example 4 worked
example publishes byte-for-byte (see table above) — the two parts agree
with each other without either one citing the other.

### Part 4 §4.2.3.1 — worked VIZ→MRZ name examples

`Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:479-533`

Nine worked examples (a-f, with (e) containing three) showing a VIZ name
truncated/formatted into a 44-character TD3 upper MRZ line. Pinned as
length-only assertions in `crates/mrz/tests/icao_vectors.rs`'s
`SECTION_4_2_3_1_NAME_EXAMPLES` — all nine measured at exactly 44
characters.

**Status: worked example reproduced** (for line length only; the examples
are not yet run through `format_td3` — that is segment S3's job, and
example (d), `O'CONNOR`, is known to fail against the crate's current name
encoder).

## Known corpus defects

### §4.2.3.3c length discrepancy

`Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:571-577`

The example (`PPUTOBENNEL<WOOLOOM<WARRAN<WARNAM<<DINGO<POTO`) measures 45
characters, one over the 44-character TD3 upper-line length every other
example in the same section satisfies. Either the source PDF's own
typesetting dropped/added a character in transcription, or ICAO's text
itself carries an erratum — undetermined without the source PDF, which is
not in this repository. Deliberately excluded from
`SECTION_4_2_3_1_NAME_EXAMPLES` rather than "fixed" by guessing which
character is wrong.

**Status: unverified — flagged as a known defect, not corrected.**

### Part 8 Appendix B transcription damage

`Doc_9303_Part8_Emergency_Travel_Documents.md:516-520`

The MRZ example's filler characters (`<`) render as literal spaces in the
converted text (`PUU OERIKSSONT <<ANNA MARIA< <<<<<<<<<<<` instead of a
clean `<`-filled MRZ line) — a two-column source layout that didn't extract
cleanly. Not usable as a pinned vector as transcribed.

`:542-547` carries an inline HTML comment, left in place from the original
cleanup pass, admitting the same section's Appendix B byte-dump table
accounts for only 121 of the 134 bytes the source PDF states it should have
— also noted in `README.md`'s "Note on provenance" and ADR-0003's negative
consequences.

**Status: unverified — known damage, documented in place rather than
guessed at.**

## Scope

This document is seeded by segment S0 with the passages the composite
check-digit and §4.2.3.1 length vectors depend on. It is a living document:
later segments (S1-S5 and beyond) should add an entry here whenever they
introduce a new test or implementation that leans on a specific corpus
passage, so the "how do we know this part of the corpus is right"
question stays answerable in one place rather than scattered across commit
messages.
