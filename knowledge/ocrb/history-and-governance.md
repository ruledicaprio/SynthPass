# History and governance of the OCR-B standards

This note explains why the drawing that actually defines `<` is not in any document we hold, and
who could supply it. The tags are defined in [`README.md`](README.md#sources).

## Timeline

| When | What | Source |
| --- | --- | --- |
| June 1961 | ECMA TC4 begins OCR standardisation | ECMA-15, ECMA-18, ECMA-21, ECMA-30 Brief Histories [R][T] |
| 1961-1965 | ECMA settles on two fonts, a stylised Class A (ECMA-8, OCR-A) and a conventional Class B. ECMA sends its full OCR-B proposal to ISO/TC97/SC3/WG1 in February 1965, and TC97 accepts it as the ISO OCR-B font (ISO Recommendation 1073) in Tokyo, October 1965. ECMA adopts ECMA-11 in November 1965 | ECMA-11 3rd ed., Brief History [R] |
| 1969-1971 | ECMA-11 revision: new characters, minor aesthetic changes, five shapes deleted. The 2nd edition (October 1971) drops size II and adds size IV, with the same aspect ratio as size I | ECMA-11 3rd ed., Brief History, §3.4 [R] |
| May 1968 | ECMA-15, print specification for OCR-A and OCR-B | ECMA-15 Brief History [R] |
| June 1969 | ECMA-19 (coding) and ECMA-21 (journal tape) | ECMA-19, ECMA-21 [R] |
| 1969 | ISO/R 1073-1969, which included size II | ISO 1073-2 Foreword [R] |
| 1971 | ECMA-30, 1st edition | ECMA-30 [T] |
| 1976 | ECMA-11 3rd ed. and ECMA-30 2nd ed. (March). ISO 1073/II-1976 (1st ed., December): size II dropped, size IV added, ZERO redesigned | ECMA-11, ECMA-30, ISO 1073-2 [R][T] |
| January 1977 | ECMA-18 2nd edition (single-line documents) | ECMA-18 [R] |
| June 1979 | Corrected reprint of ISO 1073-2, the copy held here | ISO 1073-2 cover [R] |
| October 1980 | ISO 1831, 1st edition | ISO 1831 [R] |
| 1983 | ISO 2033, 2nd edition | ISO 2033 [R] |
| 1993-1999 | Turkey asks for Turkish letters. SC 2/WG 3 opens a revision of ISO 1073-2 and three CDs follow (1994, 1995, 1996), the repertoire growing each time. The editor halts it in 1997 because the industry could not test the new glyphs. The fallback Technical Report is cancelled in 1999 | L2/00-265 [H] |
| 2000-2001 | Responsibility for ISO 1073 and ISO 1831 moves from JTC 1/SC 2 to **SC 31** (automatic identification and data capture) | L2/00-265; TC304 N982 [H] |
| 2001 | CEN/TC 304 tests a Euro sign in OCR-B (125,000 triglyphs, RecoStar) and plans an EN(V) based on the last revision CD | TC304 N982 [H] |

## What this means for us

- **The reference drawings are the standard.** For OCR-A, ISO 1073-1 contains complete glyph
  specifications. For OCR-B, ISO 1073-2 instead refers to separately held reference drawings
  (L2/00-265 [H]; ISO 1073-2 §11 [R]).
  - Those drawings were produced at ECMA and archived at ECMA and NBS/NIST, and "have now been
    transferred to JISC" (L2/00-265 [H]).
  - The 1976 standard itself offered 100:1 duplicates on request from ECMA and NBS (ISO 1073-2
    §11.2 [R]).
- **ISO 1073-2 has not been substantively revised since 1976.** The 1990s revision died, and
  ICAO still cites the 1976 edition
  ([Part 5 references](../docs9303/Doc_9303_Part5_Specs_for_TD1_MROTDs.md)).
- **The font files in circulation are digitisations, not the standard.** The glyph shapes an
  issuer prints depend on which digitisation its printer uses. The spread across fonts and
  specimens in
  [`ocrb-filler-geometry-2026-09-23.md`](../benchmarks/ocrb-filler-geometry-2026-09-23.md) is
  what that produces.
- **TC304's test method reads like a SynthPass benchmark.** It deliberately used a generic,
  non-OCR-B-tuned recognition package, with printed and scanned triglyphs, and read its results
  as "on the safe side". It found `O`/`0` and `I`/`1` to be the dominant confusions. It also found
  that tall diacritics broke line finding until the line spacing grew (TC304 N982 [H]). That is
  a 2001 precedent for detection, not classification, being the fragile step
  ([ADR-0015](../decisions/ADR-0015-geometric-mrz-band-location.md)).
