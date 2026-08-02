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

**Status: worked example reproduced** for line length. Segment S3 went
further and ran these (plus the TD1/TD2/MRV-A/MRV-B equivalents) through the
real emitter, including example (d), `O'CONNOR` — see the dedicated S3 entry
below ("Part 3 §4.6 / Part 4 §4.2.3.1, §4.2.3.4 — name-field punctuation and
non-truncating worked examples") for the full account.

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

### Part 3 §4.6 / Part 4 §4.2.3.1, §4.2.3.4 — name-field punctuation and
non-truncating worked examples

`Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:495-535` (punctuation rules),
`Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:479-584`,
`Doc_9303_Part5_Specs_for_TD1_MROTDs.md:427-457`,
`Doc_9303_Part6_Specs_for_TD2_MROTDs.md:399-429`,
`Doc_9303_Part7_Machine_Readable_Visas_MRVs.md:338-402` (MRV-A),
`:690-760` (MRV-B)

Segment S3 implemented `crates/mrz/src/emit.rs`'s `clean_name_half` (per-half
name cleaning: hyphen/comma/whitespace → single filler `<`, apostrophe
dropped with **no** filler, all other punctuation dropped with no filler,
digits passed through — see that function's doc comment) and wired it into
`name_field`, replacing the old blanket `clean()` call that mapped every
non-alphanumeric character — apostrophe included — to a filler. That old
behavior turned `O'CONNOR` into `O<CONNOR`; the corpus's own worked example
(`Doc_9303_Part4...:504`) says it must be `OCONNOR`.

All 33 of the corpus's non-truncating worked name examples across TD3, TD1,
TD2, MRV-A, and MRV-B were transcribed verbatim from the citations above into
`crates/mrz/tests/icao_vectors.rs`'s `TD3_NAME_VECTORS` / `TD1_NAME_VECTORS`
/ `TD2_NAME_VECTORS` / `MRV_A_NAME_VECTORS` / `MRV_B_NAME_VECTORS` and driven
through the real `format_td3`/`format_td1`/`format_td2`/`format_mrv_a`/
`format_mrv_b` emitters (`name_encoding_matches_icao_worked_examples`), not
merely length-checked as the pre-S3 `SECTION_4_2_3_1_NAME_EXAMPLES` test
already did. Every one reproduced byte-for-byte, including both `O'CONNOR`
occurrences (TD3 `:504`, MRV-B `:708`) and the `PAPANDROPOULOUS` "fits
exactly, must be assumed truncated" case (TD3 `:584`, MRV-A `:402`, MRV-B
`:760`, TD1/TD2 equivalents `:427`/`:399`).

Truncation itself (the §4.2.3.2/§4.2.3.3-family worked examples, and Part 7's
equivalents) is **not** pinned as emit assertions: Part 4 §4.2.3
(`Doc_9303_Part4...:463-587`) documents several issuer-discretionary
truncation strategies with no single canonical algorithm. `emit.rs`'s
`truncate_name_components` implements one conformant choice (whole
`<`/`<<`-separated components, cutting the last one short when needed) and
documents it as such, not as "the" ICAO behavior. It guarantees the two rules
Part 4 *does* make normative — a name that fits is never truncated; a
truncated field's last character is always alphabetic `A`-`Z`
(`Doc_9303_Part4...:465`, `:467`) — the latter checked both by the pinned
`PAPANDROPOULOUS` non-truncating vectors above and by a dedicated property
test, `truncated_td3_name_field_always_ends_alphabetic`
(`crates/mrz/tests/fuzz_props.rs`), which forces truncation on every case via
oversized random surname/given_names and asserts the field's last character
is always a letter.

Numeric characters are forbidden in MRZ name fields
(`Doc_9303_Part3...:507`), but the corpus defines no mapping for one that
appears anyway. `clean_name_half` passes digits through unchanged rather than
silently mapping them to a filler — see that function's doc comment for the
reasoning (mirrors `clean()`'s existing "alphanumeric survives untouched"
behavior on the document-number/optional-data fields; upstream validation is
this crate's answer, not a silent rewrite inside emission).

**Status: worked example reproduced** for all 33 golden vectors (the
non-truncating cases). **Not applicable** for truncation — Part 4 licenses
no single canonical algorithm to reproduce, so this crate's own documented
choice is verified against its own two normative constraints instead, via
property test rather than a worked example.

### Part 3 §6 A — transliteration of multinational Latin-based characters

`Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:703-807` (Table A, 95 rows,
U+00C0…U+1E9E, sorted ascending by code point, no duplicate keys),
`:1568-1574` (Appendix B, informative, "Térèsa CAÑON" worked example)

Segment S4 added `crates/mrz/src/translit.rs`'s `TABLE_A` — a verbatim
transcription of Table A, keyed on the Unicode column so binary search is
valid — and wired its `Expanded` style into `emit.rs`'s `clean_name_half`,
which previously dropped every Table A character as unrecognized punctuation
(`MÜLLER` emitted as `MLLER`). That silent data loss was non-conformant
against `:493` ("The issuing State or organization shall transliterate
national characters using only the allowed OCR-B characters"); this segment
fixes it.

Table A itself has no independent worked-arithmetic example to reproduce
(unlike the check-digit tables) — its only in-corpus corroboration is
Appendix B's "Térèsa CAÑON" → `CANXXON<<TERESA` name, which this crate
reproduces exactly (`crates/mrz/tests/icao_vectors.rs`,
`xxsuffix_golden_vector_teresa_canon`) using the `XxSuffix` style (`Ñ`→`NXX`)
plus the existing §4.6 name encoding. The `Expanded` style wired into
`clean_name_half` is a different, also-ICAO-listed choice for the same five
ambiguous rows (`Ñ`→`N` rather than `NXX`) — see
`crates/mrz/src/translit.rs`'s `TransliterationStyle` doc comment for the
full table of what each named style picks and why the grouping into three
named styles is this crate's own organisation of a choice ICAO leaves open,
not something Doc 9303 itself names.

Table integrity (95 rows, strictly ascending, no duplicate keys, every
variant `[A-Z]+`, `Expanded == variants[0]` for all rows, exactly the five
documented code points are multi-valued) is asserted directly against the
table in `crates/mrz/src/translit.rs`'s unit tests, and the
uppercase-before-lookup design is checked by a lowercase-round-trip test
over all 95 rows (with one documented, harmless exception — see the defect
entry below).

**Status: worked example reproduced** for the Appendix B golden vector.
**Unverified** for the remaining 94 rows beyond internal table-integrity
checks — there is no further worked example in this corpus to check them
against.

### Part 3 §5 — issuing-state/nationality code registry, full cross-check

`Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:618-696` (Parts A-H)

Segment S5 diffed every 3-letter code the registry lists (Parts A-H) against
`crates/mrz/src/countries.rs`'s `CODES` table. 24 codes already present and
correct (`GBD GBN GBO GBS GBP RKS D EUE UNO UNA UNK XBA XIM XCC XCO XEC XPO
XOM XXA XXB XXC XXX UTO`, plus `IAO`/`ANT`/`NTZ` which the program plan had
already flagged as candidates before this sweep). Seven were missing and
added by this segment:

| code | entity | corpus line | §5 part |
|---|---|---|---|
| `XCE` | Council of Europe | `:656` | D |
| `XES` | Organization of Eastern Caribbean States (OECS) | `:660` | D |
| `XMP` | Parliamentary Assembly of the Mediterranean (PAM) | `:661` | D |
| `XDC` | Southern African Development Community | `:663` | D |
| `ANT` | Netherlands Antilles | `:678` | F (deprecated) |
| `NTZ` | Neutral Zone | `:679` | F (deprecated) |
| `IAO` | International Civil Aviation Organization | `:695` | H |

Four of the seven (the Part D codes) were a new finding from doing the full
registry sweep rather than checking only the three codes the conformance
program's plan had predicted (`IAO`/`ANT`/`NTZ`) — Part D had only 7 of its
11 codes present before this segment. `IAO` is used only when ICAO itself
digitally signs a master list (`:691`), which the new table entry's comment
records. None of the seven collide with an existing table entry's name, so
`code_for_name`'s reverse-lookup / first-match-wins behaviour is unaffected.

**Status: cross-part agreement** — the registry table (Part 3 §5) is the
sole source for these codes; there is no separate worked example to check
them against, but each addition was verified by re-reading its corpus line
directly, not by pattern-matching neighboring entries.

### Part 3 §4.8/§4.9 — filler dates and the check-digit interaction

`Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:547` (unknown-date filler rule),
`:563` (filler value zero for check-digit purposes)

`:547` states that if all or part of the **date of birth** (not expiry) is
unknown, the relevant positions are completed with `<`. `:563` states a `<`
"shall be given the value of zero for the purpose of calculating the check
digit." `crates/mrz/src/checksum.rs`'s `char_value` already mapped `'<' =>
Ok(0)` before this segment, so an all-filler date of birth (`<<<<<<`) sums to
zero under the 7-3-1 weighting and its printed check digit is legitimately
`0` — `<<<<<<0` is a *valid* field, not a corrupt read. This segment does not
change that arithmetic; it pins it with `checksum.rs`'s
`all_filler_date_check_digit_is_zero` and `lib.rs`'s
`td3_all_filler_dob_is_a_valid_field_not_a_bad_read` /
`td1_all_filler_dob_reports_unknown` tests, and adds
`dates::DateCompleteness` / `dates::date_completeness` plus
`MrzData::date_of_birth_completeness` so a caller can distinguish this case
from an OCR-garbage read, which was previously impossible: a complete date
is reshaped from six characters to ten (`YYYY-MM-DD`) by `expand_date`, so
the original raw `YYMMDD` field is not recoverable from `date_of_birth`
after the fact for a `Complete` read, and for a non-`Complete` read
`date_of_birth` holds the raw field verbatim with no marker distinguishing
"conformantly unknown" from "garbage" until this field existed.

**Status: worked example reproduced** for the arithmetic (`<<<<<< → 0`,
hand-verified independently of the crate: 0·7+0·3+0·1+0·7+0·3+0·1 = 0, 0 mod
10 = 0) — no independent ICAO-published all-filler worked example exists in
this corpus, but the rule (`:547`, `:563`) and the general 7-3-1 check-digit
definition (already corroborated above) are both literal corpus text, so the
combination is a direct application rather than an inference.

## Known corpus defects

### Part 7 MRV-A §4.2.3 examples (a) and (d) — length discrepancies

`Doc_9303_Part7_Machine_Readable_Visas_MRVs.md:332` (example a),
`:350` (example d)

Two more corpus length defects, found by segment S3 while transcribing Part
7's MRV-A name examples (same family of defect as the §4.2.3.3c entry
below, found by an earlier segment):

- `:332` (example (a), `V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<<`)
  measures 45 characters, one over the 44-character MRV-A upper-line length
  every other example in the same section satisfies (including the very same
  VIZ, `ERIKSSON, ANNA MARIA`, printed correctly at 44 characters in the
  MRV-B section's `:690` and in Part 4's TD3 section's `:480`).
- `:350` (example (d), `V<UTOOCONNOR<<ENYA<SIOBHAN<<<<<<<<<<`) measures 36
  characters — not 44. That is exactly MRV-B's line width, not MRV-A's,
  suggesting the MRV-B version of this example (`:708`, identical VIZ,
  correctly 36 characters there) was pasted into the MRV-A section by
  mistake during either the original typesetting or this corpus's
  conversion.

Both are excluded from `MRV_A_NAME_VECTORS` in
`crates/mrz/tests/icao_vectors.rs` rather than "fixed" by guessing the
missing/wrong character, consistent with how the §4.2.3.3c entry below was
handled.

**Status: unverified — flagged as known defects, not corrected.**

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

### Part 3 §6 A Table A — four damaged "National character" cells

`Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:748` (U+0110),
`:765` (U+0131), `:766` (U+0132), `:772` (U+013F)

Four Table A rows had a "National character" cell that disagreed with the
row's own Unicode code point — corpus transcription damage, not an ICAO
erratum, since the Unicode column is unambiguous and each fix makes the two
columns agree instead of merely picking a plausible-looking glyph:

- `:748` — U+0110 should render `Đ` (D WITH STROKE); the corpus showed `Ð`
  (U+00D0, ETH — already a *different* Table A row, `:723`).
- `:765` — U+0131 should render `ı` (LATIN SMALL LETTER DOTLESS I); the
  corpus showed ASCII `I` (U+0049).
- `:766` — U+0132 should render `Ĳ` (LATIN CAPITAL LIGATURE IJ); the corpus
  showed the two-character decomposition `IJ` (U+0049 U+004A).
- `:772` — U+013F should render `Ŀ` (LATIN CAPITAL LETTER L WITH MIDDLE
  DOT); the corpus showed `L·` (U+004C U+00B7, a plain L followed by a
  middle-dot punctuation mark).

Segment S4 corrected all four cells in place (four individual edits, verified
by `git diff` to touch exactly those four lines and nothing else — see the
segment's own verification). The "Recommended transliteration" column was
already correct for all four rows and is unaffected.

**Status: corrected in the corpus** (not merely worked around in code) —
the Unicode column is authoritative and unambiguous for all four.

### Part 3 §6 A / Appendix B — "nine characters" count matches nothing

`Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:1568` (Appendix B, informative)

Appendix B's prose says there is "a group of **nine** characters that are
treated specially" in transliteration. The normative Table A
(`:703-807`) has **five** multi-valued rows (U+00C4, U+00C5, U+00D1, U+00D6,
U+00DC) and **eleven** rows with at least one multi-character variant
(the five multi-valued rows above, plus U+00C6, U+00D8, U+00DE, U+0132,
U+0152, U+1E9E, each single-valued but recommending a 2-3 letter
transliteration). Neither count is nine, and no obvious subset or
combination of the two produces nine either.

Appendix B is explicitly informative; Table A is normative. This crate's
`translit.rs` implements Table A as transcribed and does not attempt to
reverse-engineer which nine characters Appendix B might have meant.

**Status: unverified — discrepancy recorded, not resolved.** Table A (the
normative table) governs.

### Part 3 §6 A Table A — U+0130 lowercase round-trip exception

`Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:764`

U+0130 (İ, LATIN CAPITAL LETTER I WITH DOT ABOVE) is a genuine exception to
the otherwise-universal invariant that every Table A row's lowercase
counterpart uppercases back to something this crate transliterates
identically to the row itself. Rust's `char::to_lowercase` has no single-char
*simple* lowercase mapping for İ; it uses the Unicode *full* case mapping
instead, which is the two-character sequence `i` + COMBINING DOT ABOVE
(U+0307). Neither `transliterate`'s bare per-char table lookup nor its
ASCII-passthrough branch recombines that sequence back to İ's own output —
only `emit.rs`'s `clean_name_half`, one layer up, which drops unrecognized
punctuation entirely, happens to discard the stray U+0307 and land on the
same result (`I`) İ's own row prescribes.

This was found while implementing the table-integrity test suite, not
flagged in advance — see `crates/mrz/src/translit.rs`'s
`lowercase_round_trip` test, which documents and explicitly skips this one
row rather than silently weakening the assertion for all 95.

**Status: unverified — confirmed as a genuine, narrow, harmless exception,
not a bug in this crate's Table A transcription or lookup logic.**

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
