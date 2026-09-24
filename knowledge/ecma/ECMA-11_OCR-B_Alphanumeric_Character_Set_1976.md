<!--
Source: Standard ECMA-11 for the Alphanumeric Character Set OCR-B for Optical Recognition, ECMA-11, 3rd edition (March 1976). © Ecma International. Ecma publishes its standards free of
charge (https://ecma-international.org/publications-and-standards/standards/).
Transcribed for SynthPass from a scan of the original by reading page images. Text is
copied as printed: source typos are kept, uncertain characters are marked [?], and every
'page N' anchor is the PDF page. Figures are not reproduced (20 in this document);
each is replaced by a pointer to its source page. See knowledge/ecma/README.md.
-->

# Standard ECMA-11 — The Alphanumeric Character Set OCR-B for Optical Recognition

<!-- page 1 -->
Cover page (light-blue card stock). Text reads:

ECMA
EUROPEAN COMPUTER MANUFACTURERS ASSOCIATION

STANDARD ECMA-11
FOR
THE ALPHANUMERIC
CHARACTER SET OCR-B
FOR
OPTICAL RECOGNITION

3rd Edition — March 1976

<!-- page 2 -->
Inside front cover, blank except for a boxed notice near the bottom:

Free copies of this document are available from ECMA,
European Computer Manufacturers Association
114 Rue du Rhône – 1204 Geneva (Switzerland)

<!-- page 3 -->
Repeat of the title page (same text as page 1, printed on white paper).

<!-- page 4 -->
## BRIEF HISTORY

ECMA TC4 started their standardization work in the field of Optical Character Recognition in June 1961. In the course of the work, several character sets already in existence, were evaluated. As a result of this evaluation and under consideration of the widely varying European OCR requirements, it was decided to develop two fonts, a stylized (Class A) numeric character set and a conventional one (Class B).

The work of ECMA on a stylized set is laid down in Standard ECMA-8. In parallel a considerable amount of work was done and a full proposal, entirely developed by ECMA, for an OCR-B alphanumeric character set for optical recognition was submitted in February 1965 to the Expert Group of ISO/TC97/SC3/WG-1. It was recognized that this font fulfills the requirements for a Class B font as outlined in ISO document 97/3/1/N29. The ECMA proposal was eventually accepted by TC97/SC3 and by TC97 at their meetings in October 1965 in Tokyo as the ISO font for OCR-B (ISO Recommendation 1073). The Standard ECMA-11 corresponds to this ISO font. It was adopted by the General Assembly of ECMA in November 1965.

Work on a revision of this Standard was started in September 1969. It was felt that a number of shape modifications would make the font more widely accepted. In addition, some new characters were added to increase the general applicability of OCR-B. The 2nd edition also contained a small number of minor changes to improve the aesthetics of the font. Five character shapes were deleted. Size II was also deleted from the revised Standard because of lack of interest. Instead, size IV was introduced, which has the same aspect ratio as size I.

This 3rd edition of the Standard is technically identical to the 2nd edition but for the following points: the character set has been completed by adjunction of the characters PARAGRAPH, YEN and of two erase characters; the minimum length of the PRE-PRINTED LONG VERTICAL MARK has been modified to make it identical to that chosen in ISO 1073.

THIS 3rd EDITION SUPERSEDES THE VERSION DATED OCTOBER 1971.

<!-- page 5 -->
## CONTENT

1. SCOPE — 1
2. GENERAL CONSIDERATIONS — 1
3. OCR-B SIZES — 1
4. TYPICAL DIMENSIONS OF THE NOMINAL PRINTED IMAGE — 4
5. OCR-B CHARACTER SET — 5
6. INDEX TABLE — 7
7. USE OF DIACRITICAL SIGNS — 18
8. USE OF THE TWO UNDERLINE CHARACTERS — 19
9. SPACE — 19
10. VERTICAL LINE AND PRE-PRINTED VERTICAL MARK — 20
11. CHARACTER SHAPE DEFINITION — 20
12. PRINTING THE LETTERPRESS AND CONSTANT-STROKEWIDTH FONTS — 22
13. ILLUSTRATION OF OCR-B — 23

APPENDIX A
RECOMMENDATION FOR THE IMPLEMENTATION OF OCR-B ON TYPEWRITERS

<!-- page 6 -->
## 1 SCOPE

This Standard ECMA-11 defines the nominal shapes of the printed images of the individual characters of the OCR-B font. It does not define characteristics of the print quality or tolerances in character positioning, which are covered elsewhere (see Standards ECMA-15, ECMA-18, ECMA-21, ECMA-30).

The OCR-B font whilst primarily intended for use in Optical Character Recognition, is also highly suitable for general purposes.

## 2 GENERAL CONSIDERATIONS

Two styles of characters are provided, one entitled 'letterpress font' for use on printing equipment with accurate control of the printed image, and the other 'constant-strokewidth font' for printing equipment without such accurate control. The letterpress font is intended for use with any printing mechanism which can reproduce fine detail with sufficient accuracy. The strokewidths of its characters are varied deliberately for aesthetic reasons. For many classes of printers, however, the strokewidths are less controllable and for these the constant-strokewidth font is more appropriate. The definitive part of the Standard is the set of centreline shapes, since the dimensions have been specified to make these shapes the same for both letterpress and constant-strokewidth fonts.

## 3 OCR-B SIZES

3.1 Three sizes are specified for OCR-B characters in order to provide for use with a wide range of printing equipment processing differing print quality characteristics. Devices such as typewriters, cash registers, numbering machines, high-speed printers, credit card imprinters besides printing processes such as letterpress and offset lithography are all suitable.

3.2 The letterpress font is specified in size I (the smallest) only. It provides the option of a variable pitch between characters as is usual with letterpress.

3.3 The constant strokewidth font is specified in three sizes, I, III and IV. Mechanisms using the constant-strokewidth font will usually maintain a fixed pitch.

3.4 Size II which was in the first edition of the Standard has been deleted in 1971 when the 2nd edition was issued.

3.5 The centreline shapes for the three sizes are simply related by appropriate horizontal and vertical scale factors. The factors for size III and size IV referred to

<!-- page 7 -->
size I are:

For size III  Vertical: 1,333  Horizontal: 1,086
For size IV   Vertical: 1,500  Horizontal: 1,500

<!-- VERIFY: text, source p.7 -->
Note: on the source page, the last digit of each Horizontal value above ("6" in 1,086 and "0" in 1,500) is a hand-written correction in blue ink added over/after the typewritten figure (the typed text alone reads "1,08" and "1,50"). The completed values are transcribed above.

This scale relationship does not apply to the outline shapes, since nominal strokewidth is not strictly proportional to centreline dimensions. The strokewidths for each size are shown in the reference drawings.

3.6 The character with the greatest height in each size is digit EIGHT. It is the character which extends farthest above the base line for capital letters. The longest character is SMALL LETTER j, because of its descender.

The centreline heights of the digit EIGHT are:

for Size I   : 2,40 mm
for Size III : 3,20 mm
for Size IV  : 3,60 mm

3.7 The widest character in each size (except for the alternative SMALL LETTER m in the letterpress font) is digit ZERO. Its centreline widths are:

for Size I   : 1,40 mm
for Size III : 1,52 mm
for Size IV  : 2,10 mm

3.8 Constant-pitch printing

In constant-pitch printing for OCR applications following minimum nominal pitches are appropriate:

Size I   : 2,54 mm minimum
Size III : 2,54 mm minimum
Size IV  : 3,63 mm minimum

<!-- page 8 -->
## 4 TYPICAL DIMENSIONS OF THE NOMINAL PRINTED IMAGE

### 4.1 Letterpress font

Typical dimensions for the nominal printed image of the letterpress font in size I are given below. These dimensions are the heights above and below the horizontal base line of digits, capital and small letters, ascenders and descenders. These dimensions are for general information only. The exact values for all individual characters are obtainable from the original drawings only (see 11.1).

| Size | A (mm) | B (mm) | C (mm) | D (mm) |
| :--- | :--- | :--- | :--- | :--- |
| I | 2,60 | 2,46 | 1,83 | 0,60 |

> *[Figure 1 - Letterpress font: heights A, B, C, D above/below the base line, illustrated on the sample "Hhp40": drawing not reproduced here; see the original, page 8.]*

Caption/labels in the drawing: sample text "Hhp40" set on guide lines, with dimension arrows A (top guide line to base line, full capital+ascender height), B (top of lower-case ascender to base line), C (top of lower-case x-height to base line), and D (base line to descender guide line, below baseline). A row of tick marks below indicates character-cell boundaries (pitch marks).

### 4.2 Constant-strokewidth font

The shapes of the constant-strokewidth characters are similar except that they have rounded stroke ends.

<!-- page 9 -->
## 5 OCR-B CHARACTER SET

The full character set comprises 121 characters. The following sub-sets can be distinguished.

### 5.1 Sub-set 1 — Numeric Sub-set

This sub-set comprises 22 characters.

> *[Figure 2 - Sub-set 1, Numeric Sub-set: the 22 characters: drawing not reproduced here; see the original, page 9.]*

Characters shown: 0 1 2 3 4 5 6 7 8 9 ; < + > ; C E N S T X Z ; and a narrow vertical bar labelled SPACE.

### 5.2 Sub-set 2 — Initial Alphanumeric Sub-set

This sub-set comprises 47 characters.

> *[Figure 3 - Sub-set 2, Initial Alphanumeric Sub-set: the 47 characters: drawing not reproduced here; see the original, page 9.]*

Characters shown: 0 1 2 3 4 5 6 7 8 9 ; A B C D E F G H I J K L M N O P Q R S T U V W X Y Z ; < + > * - = / . , ; and a narrow vertical bar labelled SPACE.

<!-- page 10 -->
### 5.3 Sub-set 3 — Extended Alphanumeric Sub-set

This sub-set comprises 98 characters, in particular those of the ISO 7-Bit Coded Character Set (ECMA-6).

> *[Figure 4 - Sub-set 3, Extended Alphanumeric Sub-set: the 98 characters: drawing not reproduced here; see the original, page 10.]*

Characters shown, row by row:
! " # £ ¤ $ % & ' ( ) * + , - . /
0 1 2 3 4 5 6 7 8 9 : ; < = > ?
@ A B C D E F G H I J K L M N O
P Q R S T U V W X Y Z [ \ ] ^ _
` a b c d e f g h i j k l m n o
p q r s t u v w x y z { | } ~
and a narrow vertical bar labelled SPACE.

### 5.4 Sub-set 4 — Options Sub-set

This sub-set comprises 8 capital national letters, 5 small national letters, 4 accents and 4 further characters.

> *[Figure 5 - Sub-set 4, Options Sub-set: capital/small national letters, accents and further characters: drawing not reproduced here; see the original, page 10.]*
<!-- VERIFY: figure, source p.10 -->

Characters shown, row by row:
- Capital national letters (8): Ä Å Æ Ĳ[?] Ñ Ö Ø Ü — the fourth glyph is a tight capital-I/capital-J ligature; its exact identity is confirmed by the Index Table entry naming it (not reached within pages 1-20).
- Small national letters + 2 further characters: å æ ij[?] ø ß § ¥ (the 5 small national letters appear to be å, æ, ij-ligature, ø, ß; § and ¥ are counted among the "4 further characters").
- Accents (row of 3 marks): ¨ (diaeresis) ' (acute) ^ (circumflex), plus one further isolated mark (small diagonal stroke) lower on the page, for a total of 4 accents.
- Remaining further characters: a character shown as "m" with a mark beneath it (ALTERNATIVE SMALL LETTER m, referenced in 3.7), and an underscore-like mark.

<!-- page 11 -->
### 5.5 Sub-set 5 — Erase Characters

This sub-set comprises 2 characters.

> *[Figure 6 - CHARACTER ERASE and GROUP ERASE sample glyphs: drawing not reproduced here; see the original, page 11.]*

CHARACTER ERASE: a solid filled rectangle.
GROUP ERASE: a solid horizontal bar interrupted by a diagonal double-slash break ( // ).

CHARACTER ERASE is normally printed by printing devices controlled by means of a keyboard. It can be printed over or after an erroneous character. GROUP ERASE is also normally printed by printing devices controlled by means of a keyboard, however, it can also be drawn by hand. It is mandatory that the precise use of these characters be agreed upon by user and manufacturer of the reading equipment.

The dimensions of these two characters are as follows:

> *[Figure 7 - Dimensioned drawings of CHARACTER ERASE (W, H, D, baseline) and GROUP ERASE (a, b, minimum length 7,6 mm, width 0,2 mm): drawing not reproduced here; see the original, page 11.]*

CHARACTER ERASE drawing: a MIN rectangle nested inside a MAX rectangle, centred on a vertical centreline and a horizontal centreline; W = width (MIN rectangle), H = height (MIN rectangle), D = offset of MAX rectangle below the baseline.
GROUP ERASE drawing: a horizontal bar broken by a double diagonal slash; b = distance from baseline to upper limit of upper edge; a = distance from baseline to lower limit of lower edge; minimum length 7,6 mm MIN; width 0,2 mm.

<!-- page 12 -->
| SIZE | I | III | IV |
| :--- | :--- | :--- | :--- |
| CHARACTER ERASE: | | | |
| min H | 2,4 | | 3,8 |
| max H | 2,9 | | 4,6 |
| min W | 1,4 | | 2,0 |
| max W | 1,9 | | 2,8 |
| D | 0,13 | | 0,20 |
| GROUP ERASE: | | | |
| minimum length | 7,6 | 7,6 | 10,9 |
| minimum width | 0,2 | 0,2 | 0,2 |
| a | 0,4 | 0,5 | 0,6 |
| b | 2,0 | 2,7 | 3,0 |

(Size III column for CHARACTER ERASE is blank in the source — CHARACTER ERASE is not specified for Size III, consistent with 6.1: only the Numeric Sub-set and GROUP ERASE are available in Size III.)

## 6 INDEX TABLE

6.1 All characters are available in Size I as letterpress font and as constant-strokewidth font, with the exception of VL which is not in the letterpress font.

Only the characters of the Numeric Sub-set (sub-set 1) and the character GROUP ERASE are available in Size III as constant-strokewidth font.

All characters are available in Size IV as constant-strokewidth font, with the exception of VL.

6.2 In the following Index Table each character is given with the indication of the reference drawing or drawings and the sub-set or sub-sets in which it is comprised.

The drawings are identified as follows:

L — for letterpress font, Size I
C — for the constant-strokewidth font, Size I
III — for the constant-strokewidth font, Size III.

6.3 As stated in Section 11.7 the character shapes for Size IV are derived from those of Size I for the constant-strokewidth font (designated by C).

6.4 Application advice is given in column "Remarks", where it is indicated amongst others which characters are included for general-purpose use only and should not be used for OCR purposes.

It is recommended that prospective users of this Standard consult manufacturers before deciding on a particular character set.

<!-- page 13 -->
<!-- page 14 -->
<!-- page 15 -->
<!-- page 16 -->
<!-- page 17 -->
<!-- page 18 -->
<!-- page 19 -->
<!-- page 20 -->

### INDEX TABLE (Ref. Nos. 1–96, as printed on pages 13–20)

Columns: Ref. No. | Shape | Drawing(s) No. | Name | Sets | Remarks

| Ref. No. | Shape | Drawing(s) No. | Name | Sets | Remarks |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1 | 1 | 1 (L,C,III) | DIGIT ONE | 1,2,3 | |
| 2 | 2 | 2 (L,C,III) | DIGIT TWO | 1,2,3 | |
| 3 | 3 | 3 (L,C,III) | DIGIT THREE | 1,2,3 | |
| 4 | 4 | 4 (L,C,III) | DIGIT FOUR | 1,2,3 | |
| 5 | 5 | 5 (L,C,III) | DIGIT FIVE | 1,2,3 | |
| 6 | 6 | 6 (L,C,III) | DIGIT SIX | 1,2,3 | |
| 7 | 7 | 7 (L,C,III) | DIGIT SEVEN | 1,2,3 | |
| 8 | 8 | 8 (L,C,III) | DIGIT EIGHT | 1,2,3 | |
| 9 | 9 | 9 (L,C,III) | DIGIT NINE | 1,2,3 | |
| 10 | 0 | 10 (L,C,III) | DIGIT ZERO | 1,2,3 | |
| 11 | A | 11 (L,C) | CAPITAL LETTER A | 2,3 | |
| 12 | B | 12 (L,C) | CAPITAL LETTER B | 2,3 | |
| 13 | C | 13 (L,C,III) | CAPITAL LETTER C | 1,2,3 | Character in numeric sub-set. See ECMA-30 |
| 14 | D | 14 (L,C) | CAPITAL LETTER D | 2,3 | |
| 15 | E | 15 (L,C,III) | CAPITAL LETTER E | 1,2,3 | Character in numeric sub-set. See ECMA-30 |
| 16 | F | 16 (L,C) | CAPITAL LETTER F | 2,3 | |
| 17 | G | 17 (L,C) | CAPITAL LETTER G | 2,3 | |
| 18 | H | 18 (L,C) | CAPITAL LETTER H | 2,3 | |
| 19 | I | 19 (L,C) | CAPITAL LETTER I | 2,3 | |
| 20 | J | 20 (L,C) | CAPITAL LETTER J | 2,3 | |
| 21 | K | 21 (L,C) | CAPITAL LETTER K | 2,3 | |
| 22 | L | 22 (L,C) | CAPITAL LETTER L | 2,3 | |
| 23 | M | 23 (L,C) | CAPITAL LETTER M | 2,3 | |
| 24 | N | 24 (L,C,III) | CAPITAL LETTER N | 1,2,3 | Character in numeric sub-set. See ECMA-30 |
| 25 | O | 25 (L,C) | CAPITAL LETTER O | 2,3 | |
| 26 | P | 26 (L,C) | CAPITAL LETTER P | 2,3 | |
| 27 | Q | 27 (L,C) | CAPITAL LETTER Q | 2,3 | |
| 28 | R | 28 (L,C) | CAPITAL LETTER R | 2,3 | |
| 29 | S | 29 (L,C,III) | CAPITAL LETTER S | 1,2,3 | Character in numeric sub-set. See ECMA-30 |
| 30 | T | 30 (L,C,III) | CAPITAL LETTER T | 1,2,3 | Character in numeric sub-set. See ECMA-30 |
| 31 | U | 31 (L,C) | CAPITAL LETTER U | 2,3 | |
| 32 | V | 32 (L,C) | CAPITAL LETTER V | 1,2,3 | <!-- VERIFY: table, source p.15 --> (Drawing column shows only L,C, without III, yet Sets is printed as 1,2,3 — no remark given; V is not among the numeric sub-set letters listed in 5.1. Transcribed exactly as printed; likely a printing inconsistency in the source.) |
| 33 | W | 33 (L,C) | CAPITAL LETTER W | 2,3 | |
| 34 | X | 34 (L,C,III) | CAPITAL LETTER X | 1,2,3 | Character in numeric sub-set. See ECMA-30 |
| 35 | Y | 35 (L,C) | CAPITAL LETTER Y | 2,3 | |
| 36 | Z | 36 (L,C,III) | CAPITAL LETTER Z | 1,2,3 | Character in numeric sub-set. See ECMA-30 |
| 37 | a | 37 (L,C) | SMALL LETTER a | 3 | Smaller strokewidth, see Sec. 11. |
| 38 | b | 38 (L,C) | SMALL LETTER b | 3 | Smaller strokewidth, see Sec. 11. |
| 39 | c | 39 (L,C) | SMALL LETTER c | 3 | Smaller strokewidth, see Sec. 11. |
| 40 | d | 40 (L,C) | SMALL LETTER d | 3 | Smaller strokewidth, see Sec. 11. |
| 41 | e | 41 (L,C) | SMALL LETTER e | 3 | Smaller strokewidth, see Sec. 11. |
| 42 | f | 42 (L,C) | SMALL LETTER f | 3 | Smaller strokewidth, see Sec. 11. |
| 43 | g | 43 (L,C) | SMALL LETTER g | 3 | Smaller strokewidth, see Sec. 11. |
| 44 | h | 44 (L,C) | SMALL LETTER h | 3 | Smaller strokewidth, see Sec. 11. |
| 45 | i | 45 (L,C) | SMALL LETTER i | 3 | Smaller strokewidth, see Sec. 11. |
| 46 | j | 46 (L,C) | SMALL LETTER j | 3 | Smaller strokewidth, see Sec. 11. |
| 47 | k | 47 (L,C) | SMALL LETTER k | 3 | Smaller strokewidth, see Sec. 11. |
| 48 | l | 48 (L,C) | SMALL LETTER l | 3 | Smaller strokewidth, see Sec. 11. |
| 49 | m | 49 (L,C) | SMALL LETTER m | 3 | Smaller strokewidth, see Sec. 11. |
| 50 | n | 50 (L,C) | SMALL LETTER n | 3 | Smaller strokewidth, see Sec. 11. |
| 51 | o | 51 (L,C) | SMALL LETTER o | 3 | Smaller strokewidth, see Sec. 11. |
| 52 | p | 52 (L,C) | SMALL LETTER p | 3 | Smaller strokewidth, see Sec. 11. |
| 53 | q | 53 (L,C) | SMALL LETTER q | 3 | Smaller strokewidth, see Sec. 11. |
| 54 | r | 54 (L,C) | SMALL LETTER r | 3 | Smaller strokewidth, see Sec. 11. |
| 55 | s | 55 (L,C) | SMALL LETTER s | 3 | Smaller strokewidth, see Sec. 11. |
| 56 | t | 56 (L,C) | SMALL LETTER t | 3 | Smaller strokewidth, see Sec. 11. |
| 57 | u | 57 (L,C) | SMALL LETTER u | 3 | Smaller strokewidth, see Sec. 11. |
| 58 | v | 58 (L,C) | SMALL LETTER v | 3 | Smaller strokewidth, see Sec. 11. |
| 59 | w | 59 (L,C) | SMALL LETTER w | 3 | Smaller strokewidth, see Sec. 11. |
| 60 | x | 60 (L,C) | SMALL LETTER x | 3 | Smaller strokewidth, see Sec. 11. |
| 61 | y | 61 (L,C) | SMALL LETTER y | 3 | Smaller strokewidth, see Sec. 11. |
| 62 | z | 62 (L,C) | SMALL LETTER z | 3 | Smaller strokewidth, see Sec. 11. |
| 63 | * | 63 (L,C) | ASTERISK | 2,3 | |
| 64 | + | 64 (L,C,III) | PLUS SIGN | 1,2,3 | Character in numeric sub-set. See ECMA-30 |
| 65 | - | 65 (L,C) | HYPHEN (MINUS SIGN) | 2,3 | |
| 66 | = | 66 (L,C) | EQUALS SIGN | 2,3 | |
| 67 | / | 67 (L,C) | SOLIDUS | 2,3 | |
| 68 | . | 68 (L,C) | FULL STOP | 2,3 | |
| 69 | , | 69 (L,C) | COMMA | 2,3 | Two vertical locations are specified, one of which projects below the base line for capital letters (see Sec. 11.). |
| 70 | : | 70 (L,C) | COLON | 3 | |
| 71 | ; | 71 (L,C) | SEMI-COLON | 3 | Two vertical locations are specified, one of which projects below the base line for capital letters (see Sec. 11.). |
| 72 | " | 72 (L,C) | QUOTATION MARK | 3 | Can be replaced by DIAERESIS (Ref. 107) in non-OCR applications, if it is required to print QUOTATION MARK and DIAERESIS with the same type-face (see Sec. 7.2). |
| 73 | ' | 73 (L,C) | APOSTROPHE | 3 | Can be replaced by ACUTE ACCENT (Ref. 108) in non-OCR applications, if it is required to print APOSTROPHE and ACUTE ACCENT with the same type-face (see Sec. 7.2). |
| 74 | _ | 74 (L,C) | DISCONTINUOUS UNDERLINE | 3 | |
| 75 | ? | 75 (L,C) | QUESTION MARK | 3 | |
| 76 | ! | 76 (L,C) | EXCLAMATION MARK | 3 | |
| 77 | ( | 77 (L,C) | LEFT PARENTHESIS | 3 | |
| 78 | ) | 78 (L,C) | RIGHT PARENTHESIS | 3 | |
| 79 | < | 79 (L,C,III) | LESS THAN SIGN | 1,2,3 | Character in numeric sub-set. See ECMA-30 |
| 80 | > | 80 (L,C,III) | GREATER THAN SIGN | 1,2,3 | Character in numeric sub-set. See ECMA-30 |
| 81 | [ | 81 (L,C) | LEFT SQUARE BRACKET | 3 | |
| 82 | ] | 82 (L,C) | RIGHT SQUARE BRACKET | 3 | |
| 83 | % | 83 (L,C) | PERCENT SIGN | 3 | Smaller strokewidth, see Sec. 11. |
| 84 | # | 84 (L,C) | NUMBER SIGN | 3 | Smaller strokewidth, see Sec. 11. |
| 85 | & | 85 (L,C) | AMPERSAND | 3 | |
| 86 | @ | 86 (L,C) | COMMERCIAL AT | 3 | Smaller strokewidth, see Sec. 11. |
| 87 | ^ | 87 (L,C) | UPWARD ARROW HEAD | 3 | Can be replaced by CIRCUMFLEX ACCENT (Ref. 110) in non-OCR applications, if it is required to print UPWARD ARROW HEAD and CIRCUMFLEX ACCENT with the same type-face (see Sec. 7.2). |
| 88 | ¤ | 88 (L,C) | CURRENCY SIGN | 3 | |
| 89 | £ | 89 (L,C) | POUND SIGN | 3 | |
| 90 | $ | 90 (L,C) | DOLLAR SIGN | 3 | |
| 91 | \| | 91 (C) | VERTICAL LINE | 3 | Not to be confused with PLVM (Ref. 92). See Sec. 10. |
| 92 | ¦ | 92 (L,C,III) | PRE-PRINTED LONG VERTICAL MARK | 1,2,3 | Is intended to be pre-printed only. See Sec. 10. Character in numeric sub-set. See ECMA-30 |
| 93 | \\ | 93 (L,C) | REVERSE SOLIDUS | 3 | |
| 94 | Ä | 94 (L,C) | CAPITAL LETTER Ä | 4 | Where possible, substitution by the two capital letters A (Ref. 11) and E (Ref. 15) is recommended for OCR. |
| 95 | Å | 95 (L,C) | CAPITAL LETTER Å | 4 | |
| 96 | Æ | 96 (L,C) | CAPITAL LETTER Æ | 4 | |

<!-- VERIFY: table, source pp.13-20 --> The "Shape" column above reproduces the nominal character identity only (for readability); it does not reproduce the exact OCR-B centreline/outline geometry shown in the original per-character reference drawings. Ref. 92's shape is rendered here as "¦" (broken bar) purely to distinguish it visually from Ref. 91 "|" in this transcription; the source drawing shows both as vertical-bar marks of differing dimensions (see Sec. 10, not yet reached in pages 1–20).

## Transcription notes (pages 1-20)

- p.7: The last digit of the two "Horizontal" scale-factor values in 3.5 (1,086 and 1,500) is a hand-written correction in blue ink over/after the typewritten text, which otherwise reads "1,08" and "1,50". Transcribed with the handwritten digit included. See `<!-- VERIFY: text, source p.7 -->` in the body.
- p.10: Sub-set 4 (Options Sub-set) figure — the identity of the 4th capital national letter (an I/J-like ligature), the small-letter "ij" ligature, and the exact identity of the 4th accent mark and the two remaining "further characters" (an "m"-based glyph and an underscore-like mark) could not be confirmed from the figure alone within pages 1–20; marked `[?]` in the body and flagged `<!-- VERIFY: figure, source p.10 -->`. Ref. 94–96 in the Index Table (reached on page 20) confirm the first three capital national letters as Ä, Å, Æ; the remaining sub-set 4 characters (Ĳ or similar, ij, §, ¥, the 4th accent, alternative m, etc.) fall past Ref. 96 and are not yet reached in the Index Table within this page range.
- p.12: The CHARACTER ERASE dimension table has no entries in the "Size III" column, consistent with 6.1 (CHARACTER ERASE is not available in Size III).
- p.15 (Index Table Ref. 32, CAPITAL LETTER V): printed Drawing(s) column shows only "L,C" (no "III") but the Sets column shows "1,2,3" with no Remarks entry, unlike every other row bearing Sets "1" which carries an "III" drawing reference and a "Character in numeric sub-set. See ECMA-30" remark. This appears to be an inconsistency in the original document; transcribed exactly as printed. See `<!-- VERIFY: table, source p.15 -->`.
- p.17 (Index Table Ref. 49–60, remarks column): the repeated remark "Smaller strokewidth, see Sec. 11." is printed with a faded/worn typewriter ribbon, making several individual letters faint (e.g. "s" appears in place of some letters at a glance). Verified by a 450-dpi crop: all rows read identically to the clearer instances elsewhere ("Smaller strokewidth, see Sec. 11."); no uncertain characters.
- Pages 13, 15, 17, 19: small hand-drawn curved tick marks appear in the left margin next to a few Ref. Nos. (e.g. 28, 32, 52, 56, 76, 80). These appear to be non-textual proofreading/checking marks and are not part of the printed content; not transcribed as text.
- Illegible count: 0. Uncertain-character `[?]` count: 2 (both in the Sub-set 4 figure discussion on page 10, as noted above; the ligature glyphs are visible but their precise Unicode/name identity is unconfirmed within this page range).


<!-- page 21 -->

## INDEX TABLE (cont.)

| Ref. No. | Shape | Drawing(s) No. | Name | Sets | Remarks |
| --- | --- | --- | --- | --- | --- |
| 97 | Ö | 97 L,C | CAPITAL LETTER ö | 4 | where possible, substitution by the two capital letters O (Ref. 25) and E (Ref. 15) is recommended for OCR. |
| 98 | Ø | 98 L,C | CAPITAL LETTER Ø | 4 | |
| 99 | Ü | 99 L,C | CAPITAL LETTER Ü | 4 | where possible, substitution by the two capital letters U (Ref. 31) and E (Ref. 15) is recommended for OCR. |
| 100 | IJ | 100 L,C | CAPITAL LETTER DUTCH IJ | 4 | For OCR purpose separate capital letters I (Ref. 19) and J (Ref. 20) should be used. |
| 101 | Ñ | 101 L,C | CAPITAL LETTER Ñ | 4 | |
| 102 | a | 102 L,C | SMALL LETTER a | 4 | Smaller strokewidth, see Sec. 11. |
| 103 | æ | 103 L,C | SMALL LETTER æ | 4 | Smaller strokewidth, see Sec. 11. |
| 104 | ø | 104 L,C | SMALL LETTER ø | 4 | Smaller strokewidth, see Sec. 11. |
| 105 | ij | 105 L,C | SMALL LETTER DUTCH ij | 4 | Smaller strokewidth, see Sec. 11. |
| 106 | ß | 106 L,C | SMALL LETTER GERMAN DOUBLE s | 4 | Smaller strokewidth, see Sec. 11. |
| 107 | " (two dots) | 107 L,C | DIAERESIS | 4 | For use see Sec. 7. |
| 108 | ' | 108 L,C | ACUTE ACCENT | 4 | For use see Sec. 7. |

<!-- page 22 -->

## INDEX TABLE (cont.)

| Ref. No. | Shape | Drawing(s) No. | Name | Sets | Remarks |
| --- | --- | --- | --- | --- | --- |
| 109 | ` | 109 L,C | GRAVE ACCENT | 4 | For use see Sec. 7. |
| 110 | ^ | 110 L,C | CIRCUMFLEX ACCENT | 4 | For use see Sec. 7. |
| 111 | ~ | 111 L,C | TILDE | 4 | For use see Sec. 7. |
| 112 | ¸ | 112 L,C | CEDILLA | 4 | For use see Sec. 7. |
| 113 | { | 113 L,C | LEFT CURLY BRACKET | 3 | Use is not recommended for OCR. |
| 114 | } | 114 L,C | RIGHT CURLY BRACKET | 3 | Use is not recommended for OCR. |
| 115 | m | 115 L,C | ALTERNATIVE SMALL LETTER m | 4 | May be used in variable pitch printing as a substitute for Ref. 49. |
| 116 | (bar) | 116 L,C | CONTINUOUS UNDERLINE | 4 | CONTINUOUS UNDERLINE is not intended for OCR use. Its width must be such that adjacent CONTINUOUS UNDERLINES have no gap between them. See Sec. 8. |
| 117 | (blank) | No Drawing | SPACE | 1 2 3 | SPACE is a non-printing character. For definition, see Sec. 9. Not all readers will necessarily recognize SPACE. Character in numeric sub-set, see ECMA-30. |
| 118 | § | 118 L,C | PARAGRAPH | 4 | |
| 119 | ¥ | 119 L,C | YEN | 4 | |
| 120 | (solid block) | No Drawing | CHARACTER ERASE | 5 | See Sec. 5.5 |

<!-- page 23 -->

## INDEX TABLE (cont.)

| Ref. No. | Shape | Drawing(s) No. | Name | Sets | Remarks |
| --- | --- | --- | --- | --- | --- |
| 121 | (bar with slash) | No Drawing | GROUP ERASE | 5 | See Sec. 5.5 |

## 7 USE OF DIACRITICAL SIGNS

### 7.1

Besides the specially designed National Letters (Sub-set 4) a number of diacritical marks are provided which have been designed and positioned in such a way that they can be combined with small letters in order to modify or stress their meaning. These are:

- CIRCUMFLEX ACCENT (ref. 110) — shape: ^
- GRAVE ACCENT (ref. 109) — shape: `
- ACUTE ACCENT (ref. 108) — shape: '
- DIAERESIS (ref. 107) — shape: " (two dots)
- TILDE (ref. 111) — shape: ~
- CEDILLA (ref. 112) — shape: ¸

The relative position of the accent and of the letter is obtained by super-imposing the horizontal and vertical axes of the two graphics concerned. Accented letters can be obtained as shown below. For OCR purposes, the super-position of an accent on a character shape must be done very accurately. The composed character must meet the centreline tolerance specified in ECMA-15. If this tolerance cannot be met in a given printing device by printing in two operations, the composite character must be printed in a single operation. Prospective users should consult with manufacturers before planning inclusion of accented letters in OCR character sets.

> *[Figure - composed accented letters ä, ô, é: drawing not reproduced here; see the original, page 23.]*
<!-- VERIFY: figure, source p.23 -->

Caption: none printed (illustrative figure below the section 7.1 text). Shows three composed characters built by super-imposing a diacritical mark on a small letter, each with a vertical tick mark below indicating the shared centreline:
- DIAERESIS (two short vertical bars) over small letter a → ä
- CIRCUMFLEX ACCENT (^) over small letter o → ô
- ACUTE ACCENT (/) over small letter e → é

<!-- page 24 -->

### 7.2

In non-OCR applications the DIAERESIS, ACUTE ACCENT and CIRCUMFLEX ACCENT may be used standing alone, to mean QUOTATION MARKS, APOSTROPHE and UPWARD ARROW HEAD respectively, thereby reducing the total number of characters required. For OCR, however, this practice is not recommended and the proper designs must be used (ref. 72, 73 and 87) for these three characters.

## 8 USE OF THE TWO UNDERLINE CHARACTERS

Two characters are provided for underlining:

- DISCONTINUOUS UNDERLINE (ref. 74)
- CONTINUOUS UNDERLINE (ref. 116)

The latter, CONTINUOUS UNDERLINE, is not intended for use in OCR applications. The character DISCONTINUOUS UNDERLINE shall be used in OCR applications as a free-standing character only and shall not be printed under another character.

> *[Figure - underline usage example "DH_1925": drawing not reproduced here; see the original, page 24.]*
<!-- VERIFY: figure, source p.24 -->

Caption: none printed. Shows the example string "DH_1925" (capital D, capital H, DISCONTINUOUS UNDERLINE character, digits 1 9 2 5), illustrating the DISCONTINUOUS UNDERLINE printed as a free-standing character between "H" and "1", not under another character.

## 9 SPACE (no reference drawing)

The character SPACE is an intentionally blank position in a line of printing.

With constant pitch printing its nominal width is equal to the printing pitch (for example 2,54 mm if the characters are printed 10 per 25,4 mm). With variable pitch printing its nominal width is equal to the largest character pitch used (i.e. the nominal distance between the vertical centrelines of two successive SMALL LETTERS m).

The vertical centrelines (for definition see ECMA-15, par. 5.11.2) of bounding characters and the intended character pitch shall be used for the purpose of determining the number of SPACE characters between printed characters.

NOTE: The possibility of counting either a single SPACE character or the number of SPACE characters in a given blank area is dependent on the OCR reader, the print location tolerances and other factors requiring prior agreement between user and manufacturers involved.

<!-- page 25 -->

## 10 VERTICAL LINE and PRE-PRINTED VERTICAL MARK

Both the VERTICAL LINE (ref. 91) and the PRE-PRINTED LONG VERTICAL MARK (ref. 92) are vertical lines but they differ in minimum height, as indicated below.

| | millimetre — Size I | millimetre — Size III | millimetre — Size IV |
| --- | --- | --- | --- |
| Height VL | 3,20 | * | * |
| Minimum height of PLVM | 3,7 | 5,0 | 5,6 |
| Nominal strokewidth (both VL and PLVM) | 0,35 | 0,38 | 0,50 |

\* The character VL is available in Size I only.

As its name indicates, the PRE-PRINTED LONG VERTICAL MARK is normally to be pre-printed. However, it can also be generated by other means. It should not be confused with VERTICAL LINE, which is a standard graphic character of the International Reference Version of the 7-bit coded character set (Table 2 in Standard ECMA-6). The precise use of both characters should be agreed upon by user and manufacturer of the reading equipment.

For purpose of Character Spacing (see ECMA-15) both characters are to be considered as a full width character.

## 11 CHARACTER SHAPE DEFINITION

### 11.1

The shapes and dimensions of the OCR-B character for both the letterpress and the constant-strokewidth font are specified by original drawings for size I and III.

The characters are drawn at scale 100 : 1 on a 2 mm square grid. The total grid measures 280 mm x 380 mm. For the purpose of illustration in this Standard, some of these original drawings have been reduced to approximately 62 x full size.

Grid readings should be made for the establishment of work drawings on stable material. Photographic reproductions of drawings printed on paper are not satisfactory for this purpose - the dimensional stability of paper is not sufficient.

Points on the reference drawing can certainly be determined with an accuracy of half a square (10 micrometres at full size) and if desired one quarter of a square (5 micrometres at full size) should be possible. The number of readings taken on a character further determines the accuracy of the work drawing.

<!-- page 26 -->

### 11.2

Duplicates of the original drawings on a stable base at exact 100 : 1 scale with the 280 mm x 380 mm grid can be obtained upon request. Reproduction and mailing costs only will be charged.

Following sets of drawings are available:

- OD 1. Letterpress font, Size I.
- OD 2. Letterpress font, Size I with the grid removed over approximately 2mm around the character outline. This set is particularly suitable for photographic reduction.
- OD 3. Constant-strokewidth font, Size I.
- OD 4. Constant-strokewidth font, Size III.

Requests with precise indication of the set(s) desired should be addressed to:

The Secretary General
ECMA
114, Rue du Rhône
CH-1204 GENEVA
Switzerland

### 11.3

Attention is called to the fact that since this Standard specifies the nominal printed images, the type should not necessarily be cut to these dimensions. Type dimensions should be deduced from the nominal printed images after due correction for the systematic effects occuring in the printing process.

### 11.4 Letterpress font, Size I

The nominal printed image of each character is drawn on a reference grid to allow readings with any desired accuracy from drawings marked "L". Pointers establish the vertical position (base line), the orientation and for letterpress type the body width. A pointer establishes the horizontal position for fixed-pitch printing.

Reference drawings 69 L and 71 L contain also pointers, to indicate alternative positions.

The characters of the letterpress font are designed with minor strokewidth variations. However, strokewidths are always close to the nominal value of 0,35 mm for numerals and capital letters, and of 0,31 mm for small letters, #, % and a.

### 11.5 Constant-Strokewidth font, Size I

#### 11.5.1

The nominal printed image of each character is defined by its centreline and by its nominal strokewidth. The nominal strokewidth is:

<!-- page 27 -->

0,35 mm for most of the characters,

0,31 mm for all small letters, #, % and a.

The centreline and preferred line endings and corners are given in drawings marked "C". Pointers are as in paragraph 11.4.

#### 11.5.2

A special effort should be made in type design and manufacturing to arrive at actual print that conforms as closely as possible to the given line endings and corners. This is especially important for the square corners of capital letters CAPITAL LETTER B and CAPITAL LETTER D.

#### 11.5.3

A pointer is provided to produce the most aesthetic spacing of characters in a line of printing. However, on printers having a significant horizontal spacing tolerance it is recommended to use the geometric character centreline instead of the line defined by the pointer where necessary to achieve an acceptable Character Separation (see ECMA-15, par. 5.13).

### 11.6 Constant-Strokewidth font, Size III

The nominal printed image of each character is given by its centreline and by its nominal strokewidth. The nominal strokewidth is 0,38 mm. The 21 reference drawings for 0 1 2 3 4 5 6 7 8 9 < + > PLVM C E N S T X and Z are marked "III" and include pointers. Paragraphs 11.5.2 and 11.5.3 also apply.

### 11.7 Constant-Strokewidth font, Size IV

The nominal printed image of each character is given by its centreline and by its nominal strokewidth. The size IV centreline is derived from the corresponding size I centreline (see par. 11.5, reference drawings marked "C") by a linear magnification of exactly 1,5. For example, a character centreline width of 2,40 mm becomes 1,5 x 2,40 = 3,60 mm in size IV, and so on. The nominal strokewidth is:

0,50 mm for most of the characters,

0,44 mm for all small letters, #, % and a.

Preferred line endings and corners cannot be accurately arrived at by a 1,5 magnification since the ratio of nominal strokewidths for size IV and I is not exactly 1,5. However, given a 1,5 magnification of the size I drawing, the nominal size IV constant strokewidth image can easily be constructed.

## 2 PRINTING THE LETTERPRESS AND CONSTANT-STROKEWIDTH FONTS

<!-- VERIFY: the standard's own section number is printed exactly "2." here, following section 11.7; it is transcribed as printed and not corrected to "12." — see Transcription notes. -->

In order to print the letterpress font and to achieve the most

<!-- page 28 -->

satisfactory appearance, the printing device should be able to print sharp corners and to keep the strokewidth variations under close control. These features are not required for printing the constant-strokewidth fonts, although a special effort should be made to produce sharp corners in the CAPITAL LETTERS B and D. There may well be printing equipment in which the accuracy of strokewidth control is intermediate between that required in letterpress quality and that provided by, for example, high speed printers. It is at the discretion of the manufacturers of such printing equipment to design their type so that the printed images incorporate as many as practicable of the strokewidth variations which contribute to the aesthetically satisfactory appearance of the letterpress character shapes.

Care should be taken that the printed image strokes are symmetrically distributed around the centrelines as specified in this document.

## 13 ILLUSTRATION OF OCR-B

The enclosed drawings show:

- the complete character set in Size I at scales 4 : 1 and 1 : 1,
- digit ONE, CAPITAL LETTER E, PARAGRAPH and YEN in Size as letterpress font and as constant-strokewidth font,
- digit ONE and CAPITAL LETTER E in Size III as constant-strokewidth font.

These reproductions of the original drawings are approximately at scale 62 : 1.

<!-- page 29 -->

> *[Figure - complete OCR-B character set, scales 4:1 and 1:1: drawing not reproduced here; see the original, page 29.]*
<!-- VERIFY: figure, source p.29 -->

Caption (printed above the large block): "SCALE 4:1"; caption above the small block (lower right): "SCALE 1:1".

Character set shown, in reading order (identical content in both the 4:1 and 1:1 blocks):

- Row 1: 0 1 2 3 4 5 6 7 8 9
- Row 2: A B C D E F G H I J K L M
- Row 3: N O P Q R S T U V W X Y Z
- Row 4: a b c d e f g h i j k l m
- Row 5: n o p q r s t u v w x y z
- Row 6: * + - = / . , : ; " ' _
- Row 7: ? ! ( ) < > [ ] % # & a ^
- Row 8: ¤ £ $ | ¦ \ (a plain vertical bar, likely VERTICAL LINE ref.91, followed by a bar with a small gap near the top, likely PRE-PRINTED LONG VERTICAL MARK ref.92 — see Sec. 10) [?]
- Row 9: Ä ß Æ IJ Ñ Ö Ø Ü
- Row 10: a æ ij ø ß § ¥
- Row 11: " ' ` \ ^ ~
- Row 12: ¸
- Row 13: { } m _ (underline), then a CHARACTER ERASE solid block and a GROUP ERASE bar-with-slash glyph, labelled "SPACE" to its right

<!-- page 30 -->

> *[Figure - Reference Drawing Nr. 1, digit ONE, Size I, letterpress font: drawing not reproduced here; see the original, page 30.]*
<!-- VERIFY: figure, source p.30 -->

Caption (printed in box at bottom of page): "REF DRAWING NR. 1" / "SIZE I"

Shows the digit ONE as a solid (letterpress, filled-outline) glyph on the 2 mm reference grid, with small pointer tick marks at the left and right margins marking the vertical base-line/orientation reference and a triangular pointer mark below the character on the centreline. No numeric dimension labels are printed on the drawing itself; the grid squares are the measurement reference (100:1 scale, each square = 2 mm at full size, per Sec. 11.1).

<!-- page 31 -->

> *[Figure - Reference Drawing Nr. 15, CAPITAL LETTER E, Size I, letterpress font: drawing not reproduced here; see the original, page 31.]*
<!-- VERIFY: figure, source p.31 -->

Caption (printed in box at bottom of page): "REF. DRAWING NR. 15" / "SIZE I"

Shows CAPITAL LETTER E as a solid (letterpress) glyph on the reference grid, with pointer tick marks at left and right margins and a triangular pointer below the character on the centreline. No numeric dimension labels printed; grid squares are the measurement reference.

<!-- page 32 -->

> *[Figure - Reference Drawing Nr. 118, PARAGRAPH, Size I, letterpress font: drawing not reproduced here; see the original, page 32.]*
<!-- VERIFY: figure, source p.32 -->

Caption (printed in box at bottom of page): "REF. DRAWING NR. 118" / "SIZE I"

Shows the PARAGRAPH (§) character as a solid (letterpress) glyph on the reference grid, with a triangular pointer below the character on the centreline. No numeric dimension labels printed; grid squares are the measurement reference.

<!-- page 33 -->

> *[Figure - Reference Drawing Nr. 119, YEN, Size I, letterpress font: drawing not reproduced here; see the original, page 33.]*
<!-- VERIFY: figure, source p.33 -->

Caption (printed in box at bottom of page): "REF. DRAWING NR. 119" / "SIZE I"

Shows the YEN (¥) character as a solid (letterpress) glyph on the reference grid, with a triangular pointer below the character on the centreline. No numeric dimension labels printed; grid squares are the measurement reference.

<!-- page 34 -->

> *[Figure - Reference Drawing Nr. 1, digit ONE, Size I, constant-strokewidth (centreline) font: drawing not reproduced here; see the original, page 34.]*
<!-- VERIFY: figure, source p.34 -->

Caption (printed in box at bottom of page): "REF DRAWING NR. 1" / "SIZE I"

Shows the digit ONE as an outline/centreline-only drawing (constant-strokewidth font, corresponding to the "C" drawings described in Sec. 11.5) on the reference grid, with a horizontal and a vertical centreline crossing at the character's geometric centre and a triangular pointer below on the centreline. No numeric dimension labels printed; grid squares are the measurement reference.

<!-- page 35 -->

> *[Figure - Reference Drawing Nr. 15, CAPITAL LETTER E, Size I, constant-strokewidth (centreline) font: drawing not reproduced here; see the original, page 35.]*
<!-- VERIFY: figure, source p.35 -->

Caption (printed in box at bottom of page): "REF. DRAWING NR. 15" / "SIZE I"

Shows CAPITAL LETTER E as an outline/centreline-only drawing (constant-strokewidth font) on the reference grid, with a triangular pointer below the character on the centreline. No numeric dimension labels printed; grid squares are the measurement reference.

<!-- page 36 -->

> *[Figure - Reference Drawing Nr. 118, PARAGRAPH, Size I, constant-strokewidth (centreline) font: drawing not reproduced here; see the original, page 36.]*
<!-- VERIFY: figure, source p.36 -->

Caption (printed in box at bottom of page): "REF. DRAWING NR. 118" / "SIZE I"

Shows the PARAGRAPH (§) character as an outline/centreline-only drawing (constant-strokewidth font) on the reference grid, with a triangular pointer below the character on the centreline. No numeric dimension labels printed; grid squares are the measurement reference.

<!-- page 37 -->

> *[Figure - Reference Drawing Nr. 119, YEN, Size I, constant-strokewidth (centreline) font: drawing not reproduced here; see the original, page 37.]*
<!-- VERIFY: figure, source p.37 -->

Caption (printed in box at bottom of page): "REF. DRAWING NR. 119" / "SIZE I"

Shows the YEN (¥) character as an outline/centreline-only drawing (constant-strokewidth font) on the reference grid, with a triangular pointer below the character on the centreline. No numeric dimension labels printed; grid squares are the measurement reference.

<!-- page 38 -->

> *[Figure - Reference Drawing Nr. 1, digit ONE, Size III, constant-strokewidth (centreline) font: drawing not reproduced here; see the original, page 38.]*
<!-- VERIFY: figure, source p.38 -->

Caption (printed in box at bottom of page): "REF. DRAWING NR. 1" / "SIZE III"

Shows the digit ONE as an outline/centreline-only drawing for Size III (per Sec. 11.6) on the reference grid, with a triangular pointer below the character on the centreline. No numeric dimension labels printed; grid squares are the measurement reference.

<!-- page 39 -->

> *[Figure - Reference Drawing Nr. 15, CAPITAL LETTER E, Size III, constant-strokewidth (centreline) font: drawing not reproduced here; see the original, page 39.]*
<!-- VERIFY: figure, source p.39 -->

Caption (printed in box at bottom of page): "REF. DRAWING NR. 15" / "SIZE III"

Shows CAPITAL LETTER E as an outline/centreline-only drawing for Size III on the reference grid, with a triangular pointer below the character on the centreline. No numeric dimension labels printed; grid squares are the measurement reference.

<!-- page 40 -->

## APPENDIX A

### RECOMMENDATION FOR THE IMPLEMENTATION OF OCR-B ON TYPEWRITERS

The design of OCR-B is based on fundamental aesthetic laws which, as far as feasible, correspond to the criteria emerging from the long development of our classic typography. One of the essential principles prescribes that in a letter design all vertical parts must be heavier than the horizontal parts. This is also true for so-called sans serif characters, that is for a design, which at first sight has a thread-like appearance. This is precisely the case for OCR-B.

The OCR-B character set can be implemented in two clearly different forms. It can be used as a font with constant-strokewidth as well as a letterpress font. Type engraving can be based on either implementation.

For printing devices like high speed printers and similar machines, the centreline is the skeleton along which a stroke of prescribed width is placed. It is recommended to use a tool the diameter of which is equal to the strokewidth. The resulting engraving is completely thread-like, all strokes have an equal width. The aesthetic appearance as well as readability are partly diminished by this process.

In spite of strong technical limitations and difficulties, there is a tendency to design type fonts for typewriters which, as close as possible, look like letterpress fonts. For this type of application it is therefore strongly recommended to use a finer tool and to base the design on the OCR-B letterpress font used as basic pattern. Using a tool with a diameter equal to half the strokewidth it should be possible to engrave types presenting most of the intended variations of the strokewidth. Furthermore, the ends of the strokes, instead of being rounded, would then have a more rectangular appearance. Also, the internal angles would remain more open. The whole character set then looks less mechanical and bears more ressemblance to the typographic forms to which the human eye is accustomed for centuries.

Each manufacturer is, of course, free to take advantage of the aesthetic features of the letterpress font, depending on the technical means at disposal and on his desire to achieve a more typographic appearance of the characters.

## Transcription notes (pages 21-40)

- p.23: figure (composed accented letters ä, ô, é) — VERIFY. No printed caption; description of the three composites and the centreline tick marks is my own reading of the image, not printed text.
- p.24: figure (underline example "DH_1925") — VERIFY. No printed caption; description is my own reading of the image.
- p.27: heading numbering anomaly — the section following 11.7 is printed in the original as "2. PRINTING THE LETTERPRESS AND CONSTANT-STROKEWIDTH FONTS" (a single digit "2", not "12"). Confirmed at 500 dpi against the page's left margin (no clipped/faded "1"): the source document itself prints "2." at this point, immediately before Section 13. Transcribed exactly as printed, not corrected.
- p.29: figure (complete character set, scales 4:1 and 1:1) — VERIFY. Row-by-row character listing is my own reading of the grid of glyphs. In row 8, the identification of the plain vertical bar as VERTICAL LINE (ref. 91) and the bar with a small gap near the top as PRE-PRINTED LONG VERTICAL MARK (ref. 92) is inferred from Sec. 10 and not from any printed label on the figure itself [?].
- p.30–p.39 (10 reference-drawing figures: Ref. Drawing Nr. 1, 15, 118, 119, each in Size I letterpress, Size I constant-strokewidth, and — for Nr. 1 and 15 only — Size III constant-strokewidth): all marked VERIFY. These are large-format grid drawings (100:1 scale, 2 mm squares per Sec. 11.1) with no numeric dimension labels printed on the pages themselves; only the "REF[.] DRAWING NR. n" / "SIZE n" caption box is printed text. The pointer marks (small tick marks and triangular arrows) are described from visual inspection, not from printed labels.
- No [illegible] text was encountered in this page range.
- No inline [?] uncertain characters were needed within body text (all running text on pages 21-28 and 40 was clearly legible at 200 dpi); uncertainty is confined to the figure-content descriptions listed above.
