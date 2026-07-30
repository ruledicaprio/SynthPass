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

### Part 5 §4.2.4 / Part 6 note j — long document number encoding

`Doc_9303_Part5_Specs_for_TD1_MROTDs.md:368` (note j), `:464-469` (§4.2.4),
`Doc_9303_Part6_Specs_for_TD2_MROTDs.md:340` (note j)

Establishes that a document number exceeding nine characters puts all **nine**
principal characters in the number field, a filler in the *check-digit*
position to signal truncation, and `remainder + check digit + filler` at the
start of the optional-data field — with the check digit computed over
positions "6 – 14, 16 – 28", i.e. the nine principal characters concatenated
with the remainder, skipping the position-15 filler.

Relied on by segment S1, which fixed an off-by-one here (the crate wrote eight
characters plus a filler). Pinned by `td1_hand_built_spec_form_overflow_parses`
in `crates/mrz/tests/roundtrip.rs`, whose zone is assembled by hand from these
position ranges rather than generated by the emitter, so it proves the parser
reads a zone this crate did not write.

**Status: cross-part agreement.** Parts 5 and 6 state the rule independently
of each other in the same terms, and Part 5 §4.2.4's check-digit range is
consistent with its own note j. Confirmed by hand arithmetic: for document
number `D23145890XYZ` the long-number check digit is 575 % 10 = 5 and the
composite is 720 % 10 = 0, both reproduced independently of the crate.

Note that **Part 4 contains no overflow rule at all** — searching the corpus
for "more than 9 characters" hits Parts 5, 6 and 7 only, and Part 4's
passport-number rows (`Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:415-425`)
define a flat nine-character field. Applying the TD1/TD2 rule to TD3, as this
crate does, is a deliberate extension by analogy because issuers do it in
practice, not something Part 4 licenses. Part 7 (`:320`, `:678`) explicitly
stops at "positions 1 to 9" with no spillover, which the crate already honours.

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

### Part 5 §4.2.4 long-number check-digit position, table vs note

`Doc_9303_Part5_Specs_for_TD1_MROTDs.md:464-469`

The table's "Check digit position (upper MRZ line)" cell for the long document
number reads "17 – 18 (one digit only)", but the note directly beneath it says
"Since the check digit follows the last digit of the document number, its
position is in the range of 17 – 29." Those disagree: the remainder may run to
position 28, so a check digit that follows it can sit anywhere up to 29.

The note is self-consistent with the same table's input range ("6 – 14,
16 – 28") and with note j, so the implementation follows the note and locates
the check digit dynamically as the character preceding the terminating filler.
The "17 – 18" cell is most likely transcription damage in a narrow table
column, but that is unconfirmed without the source PDF.

**Status: unverified — resolved in favour of the note, discrepancy recorded.**

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

### Part 3 §4.8 — silence on two-digit-year century inference

`Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:541-547`

States only the six-digit `YYMMDD` structure and the unknown-date filler
rule ("If all or part of the date of birth is unknown, the relevant
character positions shall be completed with filler characters"). It does
not say which century a two-digit year belongs to.

Segment S2 searched the whole corpus (not just Part 3) for a century-pivot
or "19xx/20xx" rule for any document type and found none. `crates/mrz`'s
`dates::CURRENT_YY` heuristic (birth years greater than the pivot roll back
to the 1900s) is therefore this crate's own invention, not an ICAO rule —
the doc comment on `CURRENT_YY` says so and cites this entry.

**Status: unverified — confirmed absent, not merely unchecked.** There is no
worked example or cross-part table to corroborate here because there is
nothing in the corpus to corroborate; the finding is the negative result
itself.

### MRV-A/MRV-B fixture provenance (`crates/mrz`)

`Doc_9303_Part7_Machine_Readable_Visas_MRVs.md:1104-1106` (MRV-A),
`:1136-1138` (MRV-B)

Segment S2 checked every fixture comment in `crates/mrz` claiming ICAO
specimen provenance against the corpus:

- The TD2 specimen (`I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<` /
  `D231458907UTO7408122F1204159<<<<<<<6`) is published verbatim as text in
  Part 6 (`Doc_9303_Part6_Specs_for_TD2_MROTDs.md:487-488`) — the "Official
  ICAO 9303 part 6 TD2 specimen" comment was already accurate.
- The TD3 and TD1 specimens (same Utopia/Eriksson identity, reshaped per
  format) are **not** extractable as text from Part 4 or Part 5: both
  parts' Appendix A/B worked-example figures are images in the source PDF
  (`Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:669-687`,
  `Doc_9303_Part5_Specs_for_TD1_MROTDs.md:504-534`), with no MRZ text
  captured in this corpus. The old "Official ICAO 9303 part 4/5 specimen"
  comments overclaimed direct textual provenance; corrected to say the
  identity is corroborated by cross-part agreement with the literal TD2
  text instead.
- **The MRV-A and MRV-B fixtures are not ICAO specimens at all.** They were
  hand-derived by this crate — `XK93054875BRA8502212F2703143R5T6U7V8W9<<<<<<`
  and `L234567897DEU9201017F2706306QW12ER34` — with the check-digit
  arithmetic worked in comments rather than taken from ICAO. Part 7 *does*
  publish real MRV-A/MRV-B worked examples at the citations above, using the
  same ERIKSSON/UTO identity but a different nationality, dates and optional
  data (`L898902C<3UTO6908061F9406236ZE184226B<<<<<<<` for MRV-A). The old
  comments said "specimen" without an explicit ICAO-provenance claim, which
  was not false but was easy to misread; corrected to state plainly that
  these are hand-derived, not published.

  Rather than stop at relabelling them, S2 **transcribed Part 7's published
  MRV-A and MRV-B specimens verbatim** and pinned them as new vectors
  (`MRV_A_ICAO_L1`/`_L2`, `MRV_B_ICAO_L1`/`_L2` in `crates/mrz/src/lib.rs`).
  All three of MRV-A's check digits were recomputed independently and match
  what Part 7 prints: document number `L898902C<` → 3, date of birth
  `690806` → 1, date of expiry `940623` → 6. The hand-derived pair is kept
  alongside them, since it exercises a different nationality and
  optional-data shape and the tamper tests are wired to its values.

  Note the published MRV specimen is a 1990s document, so it also pins the
  limit of this crate's century policy: with expiry always read as 20xx,
  `940623` renders as `2094-06-23`, not 1994. That assertion is deliberate —
  see the Part 3 §4.8 entry above for why the policy exists at all.

**Status: cross-part agreement** for the TD3/TD1 identity (Part 6's literal
text plus Part 4/5's figures, which agree on every value but weren't
independently extractable); **worked example reproduced** for the newly
transcribed Part 7 MRV-A/MRV-B specimens; **not applicable** for the
hand-derived MRV pair, now correctly labelled as this crate's own vectors.

### Check-digit vector provenance (`crates/mrz/tests/icao_vectors.rs`)

Following from the entry above, S2 re-audited the check-digit vectors this
file pins. Its header previously promised that "every entry is a value ICAO
9303 documents, not one derived from this crate" — a promise the corpus
cannot keep for two of the six:

| Vector | Corroboration |
|---|---|
| `520727` → 3 | Part 3 Appendix A Example 1, literal text |
| `AB2134<<<` → 5 | Part 3 Appendix A Example 2, literal text |
| `740812` → 2 | Part 6 TD2 specimen, literal text (`Doc_9303_Part6_Specs_for_TD2_MROTDs.md:488`) — **not** Part 4, as previously labelled |
| `120415` → 9 | Part 6 TD2 specimen, literal text, same line |
| `L898902C3` → 6 | **Not corroborated** — Part 4's specimen is image-only |
| `ZE184226B<<<<<` → 1 | **Not corroborated** — Part 4's specimen is image-only |

The arithmetic of all six is correct under the 7-3-1 rule regardless; what
was wrong was the provenance claim, not the values. The two uncorroborated
entries are kept (they almost certainly are ICAO's, from a specimen this
corpus simply could not extract) but are now labelled as uncorroborated
rather than credited to Part 4, and the file header no longer makes the
blanket claim.

**Status: worked example reproduced** for four of six; **unverified,
explicitly flagged** for the two document-specific TD3 values.

## Scope

This document is seeded by segment S0 with the passages the composite
check-digit and §4.2.3.1 length vectors depend on. It is a living document:
later segments (S1-S5 and beyond) should add an entry here whenever they
introduce a new test or implementation that leans on a specific corpus
passage, so the "how do we know this part of the corpus is right"
question stays answerable in one place rather than scattered across commit
messages.
