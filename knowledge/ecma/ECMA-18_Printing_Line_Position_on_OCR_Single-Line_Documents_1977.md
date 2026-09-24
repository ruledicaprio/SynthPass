<!--
Source: Standard ECMA-18, Printing Line Position on OCR Single-Line Documents, ECMA-18, 2nd edition (January 1977). © Ecma International. Ecma publishes its standards free of
charge (https://ecma-international.org/publications-and-standards/standards/).
Transcribed for SynthPass from a scan of the original by reading page images. Text is
copied as printed: source typos are kept, uncertain characters are marked [?], and every
'page N' anchor is the PDF page. Figures are not reproduced (1 in this document);
each is replaced by a pointer to its source page. See knowledge/ecma/README.md.
-->

# Standard ECMA-18 — Printing Line Position on OCR Single Line Documents

<!-- page 1 -->

**ECMA — EUROPEAN COMPUTER MANUFACTURERS ASSOCIATION**

**STANDARD ECMA-18**

**PRINTING LINE POSITION ON OCR SINGLE LINE DOCUMENTS**

2nd Edition — January 1977

<!-- page 2 -->

Inside front cover. Boxed text:

"Free copies of this document are available from ECMA, European Computer Manufacturers Association, 114 Rue du Rhône — 1204 Geneva (Switzerland)"

<!-- page 3 -->

(Title page, repeated)

**ECMA — EUROPEAN COMPUTER MANUFACTURERS ASSOCIATION**

**STANDARD ECMA-18**

**PRINTING LINE POSITION ON OCR SINGLE LINE DOCUMENTS**

2nd Edition — January 1977

<!-- page 4 -->

### BRIEF HISTORY

ECMA TC4 started their standardization work in the field of Optical Character Recognition in June 1961. This work led to the adoption of the Standards ECMA-8 (Nominal Character Dimensions of the OCR-A Font), ECMA-11 (Alphanumeric Character Set for OCR-B) and ECMA-15 (Printing Specification for OCR). In order to ensure better information interchange, further work has been undertaken on the arrangement of the information on specific data media. This Standard ECMA-18 is directed to documents bearing a single line of characters recognizable by machine. Its 1st edition was issued in November 1968. Since then, all three Standards mentioned above have been revised and re-issued. The present 2nd edition of Standard ECMA-18 has taken these revisions into account.

The main differences between the two editions relate to the replacement of Size II by Size IV and to the suppression of the clauses related to mixed sizes in the same line.

THIS 2nd EDITION SUPERSEDES THE EDITION DATED NOVEMBER 1968.

<!-- page 5 -->

## 1 SCOPE

The purpose of this Standard ECMA-18 is to establish the position of the printing line for documents containing a single line of information to be read by an optical character reader. It contains the basic definition and recommendations concerning the position of the printing line.

## 2 SIZES

This Standard is applicable to characters printed in Size I, Size III or Size IV. These sizes are defined by their width and their height as follows:

| Size | Width | | Height |
|---|---|---|---|
| Size I | 1,40 mm | x | 2,40 mm |
| Size III | 1,52 mm | x | 3,20 mm |
| Size IV (OCR-A) | 2,04 mm | x | 3,80 mm |
| Size IV (OCR-B) | 2,10 mm | x | 3,60 mm |

## 3 REFERENCE EDGE

The Reference Edge of a document shall be the bottom edge.

## 4 CLEAR AREA

The Clear Area (see Note 1) shall be located at the bottom of the document: it shall extend over the whole length of the document and have a height H1 of at least 16 mm for Size I and Size III and of 20 mm for Size IV.

## 5 PRINTING LINE POSITION

The different fields which compose the printing line shall all be contained in the Printing Area (see Note 2). The horizontal centreline of the Printing Area shall be located at a height H2 = 9,6 mm from the Reference Edge; this is independent of the height of the Printing Area.

The nominal position of the printing line (horizontal centreline of the characters), shall coincide with the horizontal centreline of the Printing Area.

## 6 PRINTING AREA HEIGHT

The height H3 of the Printing Area depends upon the font size as follows:

| Size | H3 |
|---|---|
| I | 5,8 mm |
| III | 7,2 mm |
| IV | 8,4 mm |

<!-- page 6 -->

The values specified for the Printing Area height H3 are obtained taking into account the vertical misalignment and the stroke width tolerances permitted in 4.5.8 and 5.8 of Standard ECMA-15. A tolerance of 1,6 mm has been added in order to accomodate misalignment among fields resulting from printing and possible guillotining.

## 7 MARGINS

The right and left hand Margins (see Note 3) shall be at least 6 mm, measured in the direction parallel to the Reference Edge.

<!-- page 7 -->

### NOTES

1. Definition of the Clear Area (Standard ECMA-15, 5.10):

   "The Clear Area is that region of a document reserved for the OCR characters and the clear space around these characters."

2. Definition of the Printing Area (Standard ECMA-15, 5.9):

   "The Printing Area is a rectangle that has one side parallel to the document reference edge and is intended to contain only machine readable characters of one line."

3. Definition of the Margin (Standard ECMA-15, 5.11):

   "The distance between any boundary of the Printing Area and the nearest parallel paper edge is called the Margin."

<!-- page 8 -->

> *[Figure 1 - Printing line position diagram: drawing not reproduced here; see the original, page 8.]*
<!-- VERIFY: figure, source p.8 -->

The page is printed in landscape orientation (rotated 90° relative to the other pages); the crop above has been rotated back to upright for readability. Unnumbered figure (no caption or figure number printed on the page) illustrating the definitions of clauses 3–7. Labels and values legible in the figure:

- CLEAR AREA (leader line to the outer, taller rectangle spanning the full page width)
- PRINTING AREA (leader line to the inner rectangle, drawn with a dash-dot horizontal centreline)
- NOMINAL POSITION OF THE PRINTING LINE (leader line to the dash-dot centreline of the Printing Area)
- REFERENCE EDGE (leader line to the lower boundary line of the Printing Area's outer rectangle)
- MARGIN (two horizontal double-headed arrows, one at the left edge and one at the right edge, each spanning from the outer page boundary to the inner rectangle)
- H3 (vertical double-headed arrow at the left, spanning the height of the Printing Area's outer rectangle, i.e. the Clear Area boundary to the Printing Area's lower line)
- H1 (vertical double-headed arrow at the right, spanning from the Clear Area's upper boundary line down to the Reference Edge/lower line)
- H2 (vertical double-headed arrow at the right, spanning from the dash-dot centreline down to the Reference Edge/lower line, nested inside H1)

No numeric dimension values are printed directly on this figure; all values (H1, H2, H3, Margin) are given as symbols only, cross-referenced to the numeric values in the body text on pages 5–6 (clauses 2, 4–7).

<!-- page 9 -->

Inside back cover. Blank (no text).

## Transcription notes (pages 1-9)

- Page 8: figure is printed in landscape orientation on an otherwise-portrait page; the embedded crop `p08-fig1.png` was rotated 90° for readability. Marked `<!-- VERIFY: figure, source p.8 -->` — the figure carries no printed figure number or caption in the source, so "Figure 1" in the embed caption is an editorial label for reference, not transcribed text.
- Page 8, figure label "CLEAR AREA": the first word's third letter ("E") is faint/marked with what appears to be a stray printing mark in the scan (renders close to "CL[?]AR AREA"). Read as "CLEAR AREA" based on the unambiguous match to the term defined and used identically elsewhere on pages 5, 6, 7 (clause 4, Note 1). Flagged here as an uncertain character resolved by context.
- No numeric tables beyond the two simple two/three-column lists on page 5 (clause 2 sizes, clause 6 heights); both were double-checked against the source crop and are not flagged VERIFY.
- No other illegible text or `[?]` marks encountered on pages 1-9.
- Pages 2, 3, and 9 are cover material (inside covers, repeated title page) with minimal or no running text; transcribed briefly per instructions rather than word-for-word cell layout.
