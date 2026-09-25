# ocrb/: the OCR-B and OCR-print standards behind Doc 9303's MRZ

**Date:** 2026-09-24 · **MAIN:** `5adde9f` · **DATA:** n/a (standards only; no corpus read) ·
**Evidence:** Observed (page renders of the scanned standards, read and measured by hand) ·
**Status:** current

ICAO Doc 9303 is this project's normative truth for the MRZ
([`../docs9303/README.md`](../docs9303/README.md)). For the glyphs and the print, it hands off to
other standards instead of defining them itself. This tree records what those standards say, as
facts with citations, and ties each fact to the SynthPass code, measurement or ADR it constrains.

## The citation chain

Doc 9303 invokes two standards by name, and each has an ECMA ancestor. ECMA-11 §1 sets the
boundary between them: **ECMA-11 defines only the nominal shapes of the printed images, and
print quality and positioning tolerances are "covered elsewhere", in ECMA-15, ECMA-18, ECMA-21 and
ECMA-30** (ECMA-11 3rd ed., §1, p. 1 [R]).

```
ICAO Doc 9303-3 §4.4, §4.5, §4.10, §4.11 (8th ed., 2021)
 ├─ ISO 1073-2 (1976, corr. 1979) ── shapes and sizes ──────────── ≈ ECMA-11 (3rd ed., 1976)
 │     └─ numeric sub-sets (repertoire, no geometry) ───────────── ECMA-30 (2nd ed., 1976)
 ├─ ISO 1831 (1980) ── paper, ink, print quality, positioning ──── ≈ ECMA-15 (1968)
 │                                                                  + ECMA-18 (single-line docs)
 │                                                                  + ECMA-21 (journal tape)
 └─ (coding; not invoked by Doc 9303) ISO 2033 (1983) ──────────── ≈ ECMA-19 (1969)
                                       ISO 646 ───────────────────── = ECMA-6
```

- **Doc 9303-3 §4.4** sets the MRZ in OCR-B, size 1, with constant stroke width, at a fixed pitch
  of 2.54 mm. Its Figure 4 is "a subset of OCR-B characters from ISO 1073-2"
  ([Part 3 §4.4](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#44-print-specifications)).
  Part 1's glossary defines OCR-B as the font defined in ISO 1073-2
  ([Part 1](../docs9303/Doc_9303_Part1_Introduction.md)). Parts 5 and 6 list ISO 1073-2:1976 among
  their references, and Part 7 lists ISO 1831:1980.
- **Doc 9303-3 §4.5, §4.10 and §4.11** take the B900 band, the substrate and image properties,
  and print quality Range X from ISO 1831. On top of that they set the MRZ's own values:
  PCS/min ≥ 0.6 in B900, CVR < 1.50, void threshold d = 0.4 and skew ≤ 3°
  ([§4.11](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#411-quality-specifications-of-the-mrz);
  see [`print-quality-iso1831.md`](print-quality-iso1831.md#what-doc-9303-adopts)).
- **ECMA-30 §2** splits the work the same way. It specifies only which characters are used: the
  printed image is in ECMA-11, print quality in ECMA-15, and positioning in ECMA-18 and ECMA-21
  (ECMA-30 2nd ed., §2 [T]). The ISO pair follows the same split. ISO 1073-2 covers shapes, and
  ISO 1831 covers paper, ink, print quality and positioning in one document (ISO 1831 §1,
  p. 2 [R]).
- **Equivalences.**
  - ECMA submitted its OCR-B proposal to ISO in February 1965, and ISO accepted it as the ISO
    OCR-B font (ISO Recommendation R 1073). ECMA-11 "corresponds to" that font (ECMA-11 3rd ed.,
    Brief History [R]).
  - ECMA-30's history calls ECMA-8 and ECMA-11 "compatible with" ISO 1073 [T].
  - ISO 1073/II-1976, together with part I, cancels and replaces ISO/R 1073-1969 (ISO 1073-2
    Foreword [R]).
  - ECMA-15 (1968) is the ancestor of ISO 1831 (1980). The scope is the same, and so is the chapter
    structure: spectral bands, paper, printed image with PCS, spots and voids, and character
    positioning (ECMA-15 table of contents and §1.2 [R]). This tree did **not** compare the two
    value by value.
- **History and maintenance:** [`history-and-governance.md`](history-and-governance.md).

## Notes in this tree

| Note | What it holds |
| --- | --- |
| [`glyph-dimensions.md`](glyph-dimensions.md) | Sizes I/III/IV, character heights, stroke width, scale factors, reference drawings, the two type styles, and what the ISO illustration measures |
| [`filler-and-symbols.md`](filler-and-symbols.md) | Everything the standards say about `<`, and a new measurement of it on ISO 1073-2's own 4:1 illustration |
| [`print-quality-iso1831.md`](print-quality-iso1831.md) | Spectral bands, paper, character outline limits, PCS, CVR, voids, spots and edges, and what Doc 9303 adopts |
| [`line-and-pitch.md`](line-and-pitch.md) | Pitch, character spacing and separation, alignment, skew, line spacing and clear area, compared with Doc 9303's per-format figures |
| [`character-subsets.md`](character-subsets.md) | ISO 1073-2 sub-sets 1-5, the ECMA-30 numeric sub-sets, the MRZ subset, and the standards' own confusable warnings |
| [`code-positions.md`](code-positions.md) | Where the MRZ's 37 characters sit in ISO 2033 and ISO 646/ECMA-6, and why the code-structure standards do not matter here |
| [`history-and-governance.md`](history-and-governance.md) | ECMA TC4 (1961) to ISO, the aborted 1990s revision, and the SC 2 → SC 31 transfer |
| [`fonts-vs-standard.md`](fonts-vs-standard.md) | Six OCR-B font files measured against the nominal values |
| [`../benchmarks/ocrb-standards-vs-priors-2026-09-24.md`](../benchmarks/ocrb-standards-vs-priors-2026-09-24.md) | The findings: the standards' numbers set against SynthPass's priors and measurements |

## Template for a new distilled-facts page

A note added to this tree is a page of **facts**, not a transcription — the private `ocr-b`
repository holds the transcription this page is drawn from. Follow `glyph-dimensions.md` or
`print-quality-iso1831.md` as worked examples; every new page has these parts, in this order:

1. **Header.** Same shape as this file's own: `**Date:**`, `**MAIN:**` (the SynthPass SHA the page
   was written against), `**DATA:**` (`n/a` for a standards-only page — no corpus was read),
   `**Evidence:**` (one line naming the verification tags used below), `**Status:**`.
2. **Source citation.** Standard identifier, edition and date, exactly as in this file's own
   [Sources](#sources) table below — add a row there for a standard used for the first time.
   Every fact in the page cites a clause and page number from that row.
3. **Fact tables, in our own words.** Dimensions, tolerances and definitions paraphrased — never
   the standard's own sentences beyond a short quoted phrase, and never a reproduced figure,
   table or outline-coordinate dataset (see
   [Copyright, and what is deliberately not here](#copyright-and-what-is-deliberately-not-here)).
   Every value carries one of the verification tags defined under
   [Sources](#sources) ([R]/[M]/[T]/[P]/[H]) so a reader knows how the number was obtained, not
   just what it is.
4. **Measured values SynthPass relies on.** The standard's nominal value next to what was actually
   measured on a font, a specimen crop or a generator output — character pitch, stroke width, cap
   height, filler-glyph geometry, whatever the page's clause covers. State the method (font
   outline, page-render pixel count, generator constant) inline; a bare number with no method is
   exactly the "constant with no measurement behind it" `benchmarks/README.md` principle 6 rules
   out.
5. **Used by.** Real links, not prose mentions, into the code and the ADRs the page's facts
   constrain — for example `crates/synthpass-ocr/src/lib.rs`'s `MRZ_CHARSET`/`build_mrz`,
   `crates/mrz`'s registries and check program, [ADR-0014](../decisions/ADR-0014-per-cell-ocrb-classification.md)
   (per-cell OCR-B classification masks), [ADR-0015](../decisions/ADR-0015-geometric-mrz-band-location.md)
   (the geometric locator tracked in
   [#433](https://github.com/ruledicaprio/SynthPass/issues/433)), and
   [ADR-0021](../decisions/ADR-0021-fixed-grid-mrz-strips.md) (the fixed-grid strip template and
   its check-digit anchors). A page with no code or ADR to constrain has not yet cleared the 80/20
   filter [`../README.md`](../README.md#subject-folders) states for this tree.
6. **Add the file to [Notes in this tree](#notes-in-this-tree)** above, and update the
   per-standard table in [`../LIBRARY.md`](../LIBRARY.md) if the new page changes a standard's
   distillation status.

## Standards awaiting source

Doc 9303's own citations are covered (row above the [Sources](#sources) table has the full
chain). These are the ones a future landing needs, per `ocr-b/standards/incoming/INVENTORY.md`'s
survey of what the maintainer has not yet obtained. **Awaiting source — no content yet:**

| Standard | Governs | Why it would matter here | State (per `ocr-b`'s inventory) |
| --- | --- | --- | --- |
| ISO/IEC 7501-1:2008 | ISO endorsement of Doc 9303 Part 4 (TD3/MRP) | Cross-check for the TD3 template alongside Doc 9303 itself | Not held; must-have per the maintainer's priority order |
| ISO/IEC 30116:2016 | OCR-B print-quality testing, written for the MRZ | The modern companion to ISO 1831 — a newer measurement method for the same print-quality question `print-quality-iso1831.md` already covers from the 1980 standard | Not held; must-have |
| ISO/IEC 7810:2019 (+Amd 1:2024) | ID-1/ID-2/ID-3 physical card sizes | The physical frame TD1/TD2/TD3's field-position tables assume; Doc 9303 Part 3 itself cites the withdrawn 2003 edition | Not held; must-have |
| ISO/IEC 18013-1:2018 | ISO-compliant driving licence, ID-1 card with optional machine-readable technologies | Peripheral to the MRZ proper; relevant only if a future driving-licence track needs its own layout reference | Not held; "nice" priority |

ISO 1073-2, ISO 1831 and ISO 2033 are **not** in this table — they already have facts distilled
above, drawn from partial reads of a complete private transcription (see
[Copyright, and what is deliberately not here](#copyright-and-what-is-deliberately-not-here)).
A future, more thorough pass over those same three standards extends the existing notes; it does
not start a new landing zone.

## Sources

Page numbers are the document's own printed page numbers; for scans the PDF page follows where it
differs.

| Key | Standard, edition | Governs | How we have it | Verified for this tree |
| --- | --- | --- | --- | --- |
| ISO 1073-2 | ISO 1073/II-1976 (E), 1st ed. 1976-12-01, corrected reprint 1979-06-15 | OCR-B shapes, sizes, sub-sets, reference drawings | Image-only scan, 44 PDF pages. Printed p. N = PDF p. N+2 up to p. 21; the scan omits blank versos after that | Body pp. 1-21 read in full [R]. §13 4:1 illustration measured [M]. Drawings (pp. 23-41) and Annexes A-C (pp. 43-54) skimmed |
| ECMA-11 | ECMA-11, 3rd ed., March 1976 | OCR-B shapes (the ECMA twin of ISO 1073-2) | Image-only scan, 40 PDF pages. Printed p. N = PDF p. N+5 | Brief History, §1-§5.2, index ref. 79 and §9-§11 read [R]. §13 illustration measured earlier [P] |
| ISO 1831 | ISO 1831-1980 (E), 1st ed. 1980-10-15 | Paper, ink, print quality and character positioning for OCR | Scan with a poor OCR text layer. Printed p. N = PDF p. N+4 | §1-§6 and Annexes B and D read on page renders [R]. No number is taken from the text layer |
| ECMA-15 | ECMA-15, May 1968 | Print specification for OCR (ancestor of ISO 1831) | Image-only scan, 52 PDF pages | Table of contents, history and §1 only |
| ECMA-18 | ECMA-18, 2nd ed., January 1977 | Printing-line position on single-line documents | Image-only scan, 9 PDF pages | §2-§7 read [R] |
| ECMA-21 | ECMA-21, June 1969 | Character positioning on journal tape | Image-only scan, 12 PDF pages | Scope and definitions only. Not relevant to the MRZ |
| ECMA-30 | ECMA-30, 2nd ed., March 1976 | OCR-B sub-sets for numeric applications | Scan, plus a complete local transcription | Transcription read [T] |
| ECMA-19 | ECMA-19, June 1969 | 7-bit/4-bit coding of MICR and OCR sets (ancestor of ISO 2033) | Image-only scan, 15 PDF pages | History and scope read [R] |
| ISO 2033 | ISO 2033-1983 (E), 2nd ed. | Coding of machine-readable characters | Scan with a poor OCR text layer | Table 8 (p. 8) read on the page render [R] |
| ECMA-6 | ECMA-6, 6th ed., December 1991 (= ISO 646). The 1973 and 1985 editions are scans only | 7-bit coded character set | Clean born-digital text | §6.4 allocations [T] |
| ECMA-35 / -43 / -48 | 6th ed. 1994 (= ISO/IEC 2022); 3rd ed. 1991 (= ISO 4873); 5th ed. 1991 (= ISO/IEC 6429). ECMA-43 1974 and ECMA-48 1984/1986 are scans | Code extension, 8-bit structure, control functions | Clean text and scans | Equivalences only |
| L2/00-265 | K. I. Larsson, *Notes on transfer of responsibility for OCR-B standards*, Unicode L2/00-265, 2000-08-08 | Governance history | User-made transcription of the public HTML | Read [H] |
| TC304 N982 | CEN/TC 304 N982 (Unicode L2/01-259), June 2001 | OCR-B recognition testing and the Euro sign | User-made transcription of the public HTML | Read [H] |

**Verification tags** used in every note:

- **[R]**: read by eye on a page render made for this tree, at 110-300 dpi.
- **[M]**: measured on a page render. The method is stated where the number appears.
- **[T]**: taken from a clean born-digital text layer or a verified transcription.
- **[P]**: from an earlier SynthPass measurement, linked where it is used.
- **[H]**: from a user-made historical transcription.

Every [R] and [M] fact is a candidate for cross-checking against the full transcriptions.

## Copyright, and what is deliberately not here

ISO standards are copyrighted. ECMA's older editions are free to download, but they are still under
copyright. This tree therefore records **facts**, in our own words and tables: dimensions,
tolerances, and definitions paraphrased. Each fact carries a clause and page, and quotation is
limited to short phrases. There are no page images, reproduced figures, whole tables or
outline-coordinate data.

**Full transcriptions exist only locally**, outside this public repository, next to the source
PDFs, in the private `ocr-b` repository. As of that repository's own README (2026-09-24): ISO
1073-2, ISO 1831, ISO 2033, ECMA-11, ECMA-15, ECMA-18, ECMA-21 and ECMA-30 are complete
vision transcriptions; the code-set family (ECMA-6, ECMA-35, ECMA-43, ECMA-48) is complete for
its born-digital editions and its 1974 ECMA-43 scan, with ECMA-19 and the older ECMA-6/ECMA-48
scans not yet transcribed. Completing a transcription does not by itself add facts here — a fact
is added only once it is distilled, cited and tied to SynthPass code, per the template above; see
[`../LIBRARY.md`](../LIBRARY.md) for what each standard currently contributes. The PDFs are cited
by standard identifier, never by local path, and are never committed. This is the same rule
[`../docs9303/`](../docs9303/README.md#what-does-not-belong-here) applies to the ICAO PDFs.

ISO 1073-2's outline-coordinate tables (informative Annexes A and B) cover only the **superseded**
ZERO design and the **deleted** size-II digits. They contain nothing for `<` or for any letter, so
there is nothing in them to record for the current MRZ glyphs. See
[`glyph-dimensions.md`](glyph-dimensions.md#reference-drawings-and-what-the-standard-reproduces).
