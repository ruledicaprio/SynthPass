<!--
Source: Standard ECMA-30 for OCR-B Sub-Sets for Numeric Applications, ECMA-30, 2nd edition (March 1976). © Ecma International. Ecma publishes its standards free of
charge (https://ecma-international.org/publications-and-standards/standards/).
Transcribed for SynthPass from a scan of the original by reading page images. Text is
copied as printed: source typos are kept, uncertain characters are marked [?], and every
'page N' anchor is the PDF page. Figures are not reproduced (3 in this document);
each is replaced by a pointer to its source page. See knowledge/ecma/README.md.
-->

# Standard ECMA-30 for OCR-B Sub-Sets for Numeric Applications

<!-- page 1 -->

**ECMA**
EUROPEAN COMPUTER MANUFACTURERS ASSOCIATION

STANDARD ECMA-30
FOR
OCR-B SUB-SETS
FOR NUMERIC APPLICATIONS

2nd Edition — March 1976

<!-- page 2 -->

> Free copies of this ECMA standard are available from
> ECMA European Computer Manufacturers Association
> 114 Rue du Rhône — 1204 Geneva (Switzerland)

<!-- page 3 -->

**ECMA**
EUROPEAN COMPUTER MANUFACTURERS ASSOCIATION

STANDARD ECMA-30
FOR
OCR-B SUB-SETS
FOR NUMERIC APPLICATIONS

2nd Edition — March 1976

<!-- page 4 -->

## Brief History

ECMA/TC4 started their standardization work in the field of Optical Character Recognition in June 1961. 
This work led to the adoption of the Standards ECMA-8 (Nominal Character Dimensions of the OCR-A Font), 
ECMA-11 (Alphanumeric Character Set for OCR-B) and ECMA-15 (Printing Specification for OCR). ECMA-8 and ECMA-11 
are compatible with the International Standard ISO 1073 (Alphanumeric Sets for Character Recognition). 
The additional Standards ECMA-18 and ECMA-21 specify the positioning of characters on Single Line Documents 
and Journal Tape, respectively. In order to ensure better information interchange, further work has been 
undertaken on the standardization of the selection of OCR-B characters for those applications where the 
numerals alone are not sufficient but the full set is not required. This work led to the present Standard, 
issued in June 1971.

This 2nd Edition differs from the first one in that the character set for Single Line Documents has been reduced 
to 14 characters with only one additional optional character (SPACE). Furthermore, the letter Z has been selected 
instead of the letter V as one of the graphics for Journal Tape but has been put in the set of optional graphics. 
The basic set for Journal Tape is thereby reduced to 17 characters.

THIS 2nd EDITION SUPERSEDES THE EDITION DATED JUNE 1971.

<!-- page 5 -->

## 1 Purpose

The purpose of this Standard ECMA-30 is to determine the choice of characters for those Optical Character 
Recognition systems which do not require to make use of the complete OCR-B repertoire and with which a restricted 
set makes for economical working.

## 2 Scope

The Standard specifies the OCR-B characters to be used for the following applications:

- A. Single-Line documents with a restricted repertoire.
- B. Journal Tapes

Due account has been taken of the distinguisability of characters as well as of system and semantic needs. 
The provision of two sets is made necessary by the differing requirements of the two applications. All the characters 
in the set for Single Line Documents, with the exception of the PRE-PRINTED LONG VERTICAL MARK, are included in the 
set for Journal Tape. This Standard does not specify the printed image, nor does it specify the printing quality 
for interchange applications. These are defined by the Standards ECMA-11 and ECMA-15, respectively.

The positioning of the characters is specified in Standard ECMA-18 for Single Line Documents and in Standard ECMA-21 for Journal Tape.

## 3 Character set for single-line documents

### 3.1 Basic set

> *[Figure 1 - OCR-B basic character set for Single-Line documents: drawing not reproduced here; see the original, page 5.]*

- 10 numerals: 0 1 2 3 4 5 6 7 8 9
- PLUS: +
- GREATER THAN: >
- LESS THAN: <
- PRE-PRINTED LONG VERTICAL MARK: | (vertical bar)

<!-- VERIFY: figure, source p.5 -->

<!-- page 6 -->

### 3.2 Optional character

If required, e.g. for definition purposes, the character SPACE can be included in the set. Not all machines will necessarily read the character SPACE.

## 4 Character set for journal tape

### 4.1 Basic set

> *[Figure 2 - OCR-B basic character set for Journal Tape: drawing not reproduced here; see the original, page 6.]*

- 10 numerals: 0 1 2 3 4 5 6 7 8 9
- PLUS: +
- GREATER THAN: >
- LESS THAN: <
- LETTER C
- LETTER E
- LETTER N
- LETTER X

<!-- VERIFY: figure, source p.6 -->

### 4.2 Optional characters

If more characters are required by the application, the set can be extended by inclusion of:

> *[Figure 3 - OCR-B optional characters for Journal Tape: drawing not reproduced here; see the original, page 6.]*

- LETTER S
- LETTER T
- LETTER Z

<!-- VERIFY: figure, source p.6 -->

The use of these three letters is subject to a restriction: they may not appear in columns in which the numerals 1, 2 and/or 5 can appear.

<!-- page 7 -->

[blank page — inside back cover]

---

## Transcription notes

- Page 1: front cover (title, publisher, edition/date). No VERIFY marks.
- Page 2: inside-front-cover distribution notice. No VERIFY marks.
- Page 3: formal title page, duplicating the cover text on white stock. No VERIFY marks.
- Page 4: "Brief History" narrative text, fully legible. No VERIFY marks.
- Page 5: sections 1 (Purpose), 2 (Scope), 3/3.1 (Character set for single-line documents, basic set). Figure 1 (glyph shapes for the single-line basic set) cropped and embedded — marked `<!-- VERIFY: figure, source p.5 -->` since the shapes are a scanned facsimile of the OCR-B font rather than a specified drawing.
- Page 6: sections 3.2 (Optional character), 4/4.1/4.2 (Character set for journal tape, basic and optional sets). Two figures (Figure 2, Figure 3) cropped and embedded — both marked `<!-- VERIFY: figure, source p.6 -->` for the same reason as Figure 1. Running page number "- 2 -" dropped per instructions.
- Page 7: blank inside-back-cover page (light blue stock, no printed content). No VERIFY marks.

No `[illegible]` or `[?]` marks were needed anywhere in this document — all typewritten body text and all character-set labels were read with full confidence at 200 dpi. The three figures are marked VERIFY only because they are photographic facsimiles of glyph shapes (not dimensioned technical drawings), and a second pass against the source PDF is recommended before treating the cropped glyph outlines as authoritative for any downstream OCR-B shape work.
