# docs9303/

ICAO Doc 9303 (*Machine Readable Travel Documents*, Eighth Edition, 2021),
Parts 1–13, converted to clean Markdown. This is the source of truth behind
`crates/mrz` and the M7 `synthpass-die` provider work, and supersedes the
informal, partial ICAO references that used to be scattered across
`crates/mrz/README.md` and the MRZ-handling policy in
`knowledge/ARCHITECTURE.md` §13 — those stay as the project's *policy* on MRZ
handling; this tree is the *spec* they implement.

## Contents

| File | Part | Relevance to SynthPass |
|---|---|---|
| `Doc_9303_Part1_Introduction.md` | 1 — Introduction | Glossary and acronyms; §5.1 is a one-paragraph map of all 13 parts and the canonical cross-reference hub. |
| `Doc_9303_Part2_Security_of_MRTDs.md` | 2 — Security of the Design, Manufacture and Issuance of MRTDs | Physical/procedural document security and machine-assisted authentication (Appendix C). Not implemented in code; background for document-authenticity claims. |
| `Doc_9303_Part3_Specs_Common_to_all_MRTDs.md` | 3 — Specifications Common to all MRTDs | The check-digit algorithm (§4.9, 7-3-1 weighting) — `crates/mrz/src/checksum.rs`. OCR-B character set, ICAO/ISO 3166-1 country codes — `crates/mrz/src/countries.rs`. |
| `Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md` | 4 — MRPs and other TD3 Size MRTDs | The TD3 (passport) format — `crates/mrz/src/parser.rs::parse_td3`, `emit.rs::format_td3`. |
| `Doc_9303_Part5_Specs_for_TD1_MROTDs.md` | 5 — TD1 Size MROTDs | The TD1 (ID card, 3-line MRZ) format — `parse_td1` / `format_td1`. |
| `Doc_9303_Part6_Specs_for_TD2_MROTDs.md` | 6 — TD2 Size MROTDs | The TD2 (2-line MRZ) format — `parse_td2` / `format_td2`. |
| `Doc_9303_Part7_Machine_Readable_Visas_MRVs.md` | 7 — Machine Readable Visas | MRV-A/MRV-B visa formats — `crates/mrz`'s MRV-A/B parse and emit support. |
| `Doc_9303_Part8_Emergency_Travel_Documents.md` | 8 — Emergency Travel Documents | Guidance material, not a normative format; informs which document types `synthpass-gen` does and doesn't need to cover. |
| `Doc_9303_Part9_Biometric_identification_and_Electronic_Data_storage_in_MRTDs.md` | 9 — Biometric Identification and Electronic Data Storage | eMRTD chip inspection flow. Relevant to future vision/chip-data providers in `synthpass-die`, not implemented yet. |
| `Doc_9303_Part10_LDS_for_Storage_of_Biometrics_and_Other_Data_in_the_Contactless_IC.md` | 10 — Logical Data Structure (LDS) | DG1–DG16 data group layout for the contactless chip. Relevant to future electronic-chip-data support; no code depends on it yet. |
| `Doc_9303_Part11_Security_Mechanisms_for_MRTDs.md` | 11 — Security Mechanisms for MRTDs | BAC, PACE, Chip/Terminal/Active/Passive Authentication. Future PKI and chip-authentication work. |
| `Doc_9303_Part12_Public_Key_Infrastructure_for_MRTDs.md` | 12 — Public Key Infrastructure for MRTDs | CSCA/Document Signer certificate chains. Future Passive Authentication verification. |
| `Doc_9303_Part13_Visible_Digital_Seals.md` | 13 — Visible Digital Seals | Barcode-based digital seal structure, used in Part 7's and Part 8's worked examples. Per `knowledge/ROADMAP.md`, barcode documents are their own project, not folded into the MRZ roadmap — kept here for reference regardless. |

## Conformance basis

[`CONFORMANCE_BASIS.md`](CONFORMANCE_BASIS.md) tracks, for the passages the
`crates/mrz` conformance program relies on, how each was corroborated
(worked example reproduced, cross-part agreement, or unverified) and records
the corpus's known defects (the §4.2.3.3c length discrepancy, Part 8's
Appendix B transcription damage). Living document, extended as later
conformance segments lean on new passages.

## Field layout

[`Mrz_Field_Layout.md`](Mrz_Field_Layout.md) is a machine-actionable distillation of Parts 3–7's
field-position tables for all five formats this crate handles (TD1, TD2, TD3, MRV-A, MRV-B) —
1-based ICAO positions alongside 0-based Rust-usable slices, the check-digit algorithm, composite
coverage ranges, and issuer options (e.g. an unused check digit printed as `0` or `<`), with a
normative JSON block in its §4 as the tiebreaker when its own human-rendered tables disagree.
Derived from the Part files above, not a replacement for them — read it for "what position is
this field at," and the Part files for the surrounding normative prose.

## Delegated standards

Part 3 does not define the typeface or the print quality itself. §4.4 requires OCR-B from
[ISO 1073-2], and §4.11 takes the print-quality measures (spectral band, PCS, range X) from
[ISO 1831]. The ISO documents cannot be reproduced here. Their facts are distilled, with
clause-level citations, in [`../ocrb/README.md`](../ocrb/README.md), and the Ecma twins of both
(ECMA-11, ECMA-15) are transcribed in [`../ecma/`](../ecma/README.md).

## Figures

`figures/` holds diagrams rendered from the source PDFs, prefixed with the
filename of the part they came from and the PDF page they were rendered
from: `<PartFileStem>_p<PageNumber>.png` (matching the naming convention in
`../papers/figures/`, adapted since these are full-page renders rather than
individually cropped images — several figures on the same PDF page share one
file). Wired into the Markdown as `<img src="./figures/...">` with real alt
text, immediately under the figure's caption; the caption's own bracketed
`[Diagram: ...]` description (where present) is left in place below the
image rather than removed, since it was independently verified against the
source PDF and is what a text-only reader gets.

Only the highest-value diagrams were extracted — MRZ zone-layout and
dimensional diagrams (Parts 4–7, the geometric basis for `crates/mrz`'s field
offsets), Part 2's Appendix C authentication-architecture diagrams (which had
been OCR'd into unreadable prose before this tree was cleaned up), the Chip
Inside symbol (Part 9), and the Digital Seal structure diagrams (Part 13).
Many bracketed `[Diagram: ...]` placeholders remain un-illustrated by design
— the text description was judged sufficient, or the diagram is decorative
(e.g. Part 2 Appendix C's GUI screenshot mock-ups, already described in
detail in prose). Extract more if a specific diagram turns out to matter for
the redesign; don't do it speculatively.

## Note on provenance

Converted from the official ICAO consolidated PDFs (Eighth Edition, 2021),
one per part. The raw conversion carried real damage that was cleaned up
here: three incompatible TOC/table styles were unified onto the cleanest one
found in the corpus (matching the original Parts 10–13); Part 2 had
duplicated front matter, PDF page header/footer text bled into the body, two
appendices missing their heading markup entirely, and diagrams OCR'd into
unreadable run-on prose; Part 9 opened with a leftover LLM chat preamble;
Part 13 had an editorial note embedded in its own TOC; several files were
also found, only after a cleanup script bug, to be missing real content at
the very end (a page-counting mistake, not a research one — caught by
diffing every file's ending against its source PDF rather than trusting the
script). Every part genuinely ends with `-- END --` in the official ICAO
text; that surprised the first pass at this cleanup enough to be worth
saying plainly here.

Every `Doc 9303-N` mention was made into a real Markdown link to that part's
file in the pass this note originally described (265 of them, still true
today) — a hand step, not part of `tools/docs9303_extract.py`'s output. The
2026-09-25 Part 11/12 rebuild below did not repeat it: cross-linking needs a
number-to-filename lookup this repo's naming has no mechanical rule for
(`Doc 9303-4` isn't `Doc_9303_Part4.md`, it's
`Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md`), so the rebuilt files carry
their 60 `Doc 9303-N` mentions as ICAO printed them, plain text, same as
every other word in that rebuild. `scripts/check-doc-links.sh` does not
require this — it checks that a link, once made, resolves, not that a
prose mention becomes one — so nothing is broken; it is a completeness gap
against this paragraph's original claim, worth an issue if the cross-links
matter enough to script rather than repeat by hand.

The source documents cite each other only at
whole-part granularity in prose (`Doc 9303-11`, `see Part 4`); they do not
cite specific section numbers across parts, so there was nothing finer to
link at that level there. Headings do carry their section number in the
heading text itself (e.g. `## 4.2.2 Document Number`), so a GitHub-style
anchor is always derivable — `Doc_9303_Part4_....md#422-document-number`.
`CONFORMANCE_BASIS.md`'s own file:line citations now use exactly this form —
a real link whose visible text is the file:line citation and whose target is
the section anchor, not a bare backtick citation — and
`scripts/check-doc-links.sh` verifies every such fragment resolves to a real
heading, not just a real file.
`crates/mrz/src/*.rs`'s doc comments still cite sections in prose only
(`ICAO 9303 part 4 §4.2.2.2`) rather than as real links — wiring those up
remains the one piece of this not done here.

### PDF audit, 2026-09-24: the conversion had invented content

A check against ICAO's own consolidated PDFs found content that **is not in Doc 9303**:

- "MRZ dimensions" tables under the Part 4 and Part 5 MRZ figures, where the PDF has only
  labels on the figure. Some of their values ("Line spacing 4.0", "2.54") appear nowhere in
  that Part.
- Invented table columns and labels: Part 7's "Optional" Zone IV, and Part 12's
  "`cA` MUST be `TRUE`".
- Made-up flowchart outcomes in Part 9.
- **The whole section and appendix skeleton of Part 10.**

Every Part was then audited against its PDF:

1. **Two deterministic detectors, run first.** One lists every number token that does not occur
   in that Part's PDF text. The other lists tables that sit directly under an image (the
   invention pattern). A third check flags lines whose word 5-grams are mostly absent from the
   PDF text.
2. **Triage of every flagged item against the rendered PDF page.** Each is judged faithful,
   wrong, invented or editorial. Wrong items were corrected, invented ones removed, and invented
   figure tables replaced by "Labels printed on Figure N (not a table in the original)".
   Converter descriptions of figures are marked `[Editorial description, not ICAO text: ...]`.
3. **Part 10 was rebuilt from a deterministic PyMuPDF text extraction.** Only structure was
   changed (headings, page furniture, table joins); nothing was reworded. The first repair
   attempt had paraphrased it, which is a quieter version of the same failure.

After the audit, no Part has an invented table under a figure, and Part 10 has 0 numbers absent
from its PDF. The numbers still flagged in other Parts are text-layer artefacts that were each
checked by hand: values the PDF splits (`23.` + `3`), MRZ strings with no word boundaries, and
hex dumps. **The rule this leaves:** before calling anything here an ICAO error, open the PDF
page. A table that reads well is not evidence that the source has a table.

### Parts 11-12 rebuilt, 2026-09-25: 68 missing clause titles, #424

Part 10's rebuild script from the audit above was never committed, so #424 asked for both:
rebuild Parts 11 and 12 the same way, and this time keep the tool. [`tools/docs9303_extract.py`](../../tools/docs9303_extract.py)
is a general PyMuPDF-based extractor for this corpus's PDFs — furniture stripping, numbered-clause
heading promotion by numbering depth, paragraph reflow with a hyphen-aware line join, and a table
pass via `page.find_tables()` — with pure text-transform functions unit-tested in
[`tools/test_docs9303_extract.py`](../../tools/test_docs9303_extract.py) against no PDF at all.
It reproduces Part 10's own "structure only, nothing reworded" method, so Part 10 itself was
**not** regenerated (it already passed that bar); only Parts 11 and 12, which had not yet been
rebuilt this way, were.

The old Part 11/12 text turned out to be summarized, not transcribed: whole tables were replaced
with "(Tables of OIDs for DH and ECDH mappings omitted for brevity, see Section 9.2.3 for full OID
trees)", and appendices were reduced to one-paragraph descriptions ("Demonstrates an RSA-based
Active Authentication flow...") instead of ICAO's own worked-example text. Measured against a
direct read of the PDF text layer:

- **Clause-title coverage: 175/175 (Part 11), 122/122 (Part 12), 0 missing.** The issue's own
  count was 68 missing titles; the rebuilt files have every numbered clause heading the PDF text
  layer has, cross-checked by running the same heading detector used to generate the Markdown
  against the raw PDF text.
- **Number tokens in the Markdown absent from the PDF: 0**, both Parts.
- **Determinism:** running the extractor twice over the same PDF produces byte-identical output
  (`diff` reports no difference).
- **Wording spot-check:** 10 paragraphs sampled at random, cross-checked verbatim against the PDF
  text layer (modulo the extractor's own whitespace/dehyphenation join) — no paraphrase, no
  invented word, one confirmed case of ICAO repeating the same sentence with different punctuation
  in two places in Part 12 (a real inconsistency in the source, not an extraction artefact).

Two known, documented limitations, both from PyMuPDF's own table-grid detector rather than from
guessing: a vertically-merged cell in a table (a cell whose content is shared with the row above
it) extracts as empty rather than reconstructing the merge, affecting a handful of cells in Part
11's Table 1 and Table 4; and Part 12's Table 6 ("Certificate Extensions Profile", pages 36-37)
is wide and dense enough that the grid detector fragments its header into 30+ spurious columns —
rather than render a table structure that confident, the tool falls back to the same page's plain
reading-order text there, verbatim but not tabulated. Neither is a hand-tuned fix to the output;
both are structural decisions in the tool itself (`MAX_TABLE_COLUMNS` in
`tools/docs9303_extract.py`), so they apply the same way if the tool is ever re-run.

Part 8's Visible Digital Seal byte-dump example (Appendix B) was previously
short about 13 of the 134 bytes the source PDF states it should have — a
16-column × 9-row grid that plain text extraction flattened into the wrong
column order and silently dropped one column from. Recovered in full by
clustering each word's on-page coordinates back into the source's true
grid (see [`CONFORMANCE_BASIS.md`](CONFORMANCE_BASIS.md)); the same
coordinate-clustering approach also fixed the same page's MRZ example, which
had literal spaces where filler characters belonged.

## Re-auditing

Run [audit_docs9303.py](../../tools/audit_docs9303.py) from the repository root with
`python tools/audit_docs9303.py --pdf-dir <local-pdf-dir>`; add `--part N` to narrow the run.
It checks
digit tokens, numbered clause coverage, tables under figure captions, and normalized prose
5-gram coverage against the local PDFs. It reports findings without changing files; the unit
tests need no PDFs.

## What does not belong here

PDFs. The point of this tree is that it is greppable, diffable text a model
can read in context. The source PDFs live outside the repository at
`pdf_docs9303/` on the machine this was converted on — convert on the way
in, don't commit the PDF itself.
