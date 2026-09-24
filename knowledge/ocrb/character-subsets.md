# Character sub-sets: ISO 1073-2, ECMA-30, and the MRZ's 37

The tags are defined in [`README.md`](README.md#sources).

## ISO 1073-2 sub-sets (§5, pp. 3-5 [R])

The full OCR-B set has **121 characters**. ISO 1073-2 names five sub-sets:

| Sub-set | Size | Contents |
| --- | --- | --- |
| 1 Numeric | 22 | the digits 0-9; `<` `+` `>`; `C E N S T X Z`; LONG VERTICAL MARK; SPACE |
| 2 Initial alphanumeric | 47 | the digits 0-9; A-Z; `< + > * - = / . ,`; LONG VERTICAL MARK; SPACE |
| 3 Extended alphanumeric | 98 | essentially the ISO 646-1973 7-bit graphic set, lower case included |
| 4 Options | 21 | 8 national capitals, 5 national small letters, 4 diacritical signs, and 4 further characters (§ and ¥ among them) |
| 5 Erase | 2 | CHARACTER ERASE and GROUP ERASE |

- **The MRZ set is a strict subset of sub-set 2.** Doc 9303's Figure 4 allows 0-9, A-Z and `<`
  only ([Part 3 §4.3-§4.4](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#43-constraints-of-the-mrz)).
  Every MRZ character is in the "initial alphanumeric" repertoire. This matters for print
  quality, because ISO 1831 sizes its measurement rectangle Q by sub-set, and sub-sets 1 and 2
  share the larger Q ([`print-quality-iso1831.md`](print-quality-iso1831.md)).
- **§5.1 note 2** recommends that `C E N S T X Z` preferably not be used in document-reading
  applications (p. 3 [R]). They are in the numeric sub-set only as journal-tape symbols. Doc 9303
  uses all seven freely.
- **§5.1 note 1:** ZERO is the only digit whose design changed from ISO/R 1073-1969. The old
  design is tolerated only in numeric applications implemented before 1976, and reading it needs
  a special agreement. After 1976 only the new design is standard (p. 3 [R]). The old design's
  coordinates are in informative Annex A.
- **Erase characters.** Table 2 gives CHARACTER ERASE at size I as a solid block 2.4-2.9 mm high
  and 1.4-1.9 mm wide, seated D = 0.13 mm below the baseline. GROUP ERASE is a bar at least
  7.6 mm long and at least 0.2 mm thick (§5.5, p. 5 [R]). Neither can appear in an MRZ.
  - **Bears on SynthPass (inferred):** CHARACTER ERASE matches ISO 1831's cut-off rectangle and
    its 0.13 mm offset ([`print-quality-iso1831.md`](print-quality-iso1831.md#character-outline-limits-col)).
    That offset is the only per-character vertical offset the documents give as a number.

## ECMA-30 numeric sub-sets (2nd ed., 1976 [T])

| Application | Basic set | Optional |
| --- | --- | --- |
| Single-line documents | 14 characters: the digits 0-9, `+`, `>`, `<`, PRE-PRINTED LONG VERTICAL MARK | SPACE |
| Journal tape | 17 characters: the digits 0-9, `+`, `>`, `<`, `C`, `E`, `N`, `X` | `S`, `T`, `Z`, which may not share columns with digits `1`, `2` or `5` |

- ECMA-30 fixes the repertoire only. The image comes from ECMA-11 and the print quality from
  ECMA-15 (§2).
- The 1st edition (1971) had `V` where the 2nd has `Z`, and a larger single-line set.
- **The rule on S/T/Z is the standard's own confusability statement**: `S`~`5`, `T`~`1` and
  `Z`~`2` are close enough that a numeric reader must not face both members of a pair in one
  column. The same three pairs appear in `mrz::CONFUSABLES`: `5`/S, `1`/T, `2`/Z
  ([`crates/mrz/src/repair.rs`](../../crates/mrz/src/repair.rs)).
- **Bears on SynthPass:** Doc 9303 does in effect what ECMA-30 asks. The field layout decides by
  position whether a column holds a digit or a letter. That is why the per-position class table
  in `repair_td3_line2` and the height-template idea in
  [`mrz-geometric-elimination.md`](../research/mrz-geometric-elimination.md) are consistent with
  the standards. ECMA-30 solved the same problem with a column rule.
- **The Barcodesoft `ocrbIII.ttf` font** holds exactly the digits, `< >`, `+`, `C E N S T X Z` and
  `|`. That is ISO 1073-2 sub-set 1, the characters ISO provides drawings for in size III.
  So "III" in its name means **size III**, not a version number
  ([`fonts-vs-standard.md`](fonts-vs-standard.md)).

## Confusable pairs, in the standards' own words

- **TC304 N982 (2001)** tested OCR-B with a general-purpose engine. Most errors were `O`/`0` and
  `I`/`1`, "or vice versa". The report's explanation: OCR-B designs those glyphs "to have
  differing distinguishing features", which a reader not tuned to OCR-B does not use [H].
- **ISO 1073-2's index table** recommends writing Ä, Ö and Ü (refs. 94, 97, 99) as the two
  capitals AE, OE and UE where possible, for OCR (p. 15 [R]). This is the origin of the "AE / OE /
  UE" choice in Doc 9303's transliteration table
  ([Part 3 §6 A](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#a-transliteration-of-multinational-latin-based-characters)).
- **Bears on SynthPass:** `0`/`O` and `1`/`I` are the pairs the typeface was designed to separate,
  and a model not trained on OCR-B confuses them anyway. That is consistent with the observed
  zero problem in
  [`observed-ocrb-confusions-2026-09-21.md`](../benchmarks/observed-ocrb-confusions-2026-09-21.md),
  and it argues for OCR-B-specific features, above all the digit/letter height gap
  ([`glyph-dimensions.md`](glyph-dimensions.md#typical-outline-heights-above-and-below-the-baseline-size-i)),
  rather than general recognition confidence. On ISO's illustration ZERO is wider than `O`
  (0.724 against 0.69-0.71 cap [M]) and taller (1.116 against 1.047). OCR-B's `0` is not a
  "narrow oval".
