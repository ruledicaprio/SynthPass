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
PDFs. ECMA-30 is complete; ECMA-11, ISO 1073-2, ECMA-15, ECMA-18 and ECMA-21 are in progress. The
PDFs are cited by standard identifier, never by local path, and are never committed. This is the
same rule [`../docs9303/`](../docs9303/README.md#what-does-not-belong-here) applies to the ICAO
PDFs.

ISO 1073-2's outline-coordinate tables (informative Annexes A and B) cover only the **superseded**
ZERO design and the **deleted** size-II digits. They contain nothing for `<` or for any letter, so
there is nothing in them to record for the current MRZ glyphs. See
[`glyph-dimensions.md`](glyph-dimensions.md#reference-drawings-and-what-the-standard-reproduces).
