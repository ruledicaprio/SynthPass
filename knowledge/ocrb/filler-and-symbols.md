# The filler `<`: what the standards say, and what their illustration measures

Doc 9303 uses LESS-THAN SIGN as the MRZ filler. It fills unused positions, separates name
components, and counts as zero in the check digit
([Part 3 §4.6](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#46-convention-for-writing-the-name-of-the-holder),
[§4.9](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#49-check-digits-in-the-mrz),
[§4.10](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#410-characteristics-of-the-mrz)).
None of the OCR standards knows the character as a "filler". To them it is an ordinary graphic
character. Tags: [`README.md`](README.md#sources).

## Identity and membership

| Fact | Source |
| --- | --- |
| OCR-B reference no. **79**, LESS THAN SIGN. Drawings exist for **L** (letterpress size I), **C** (constant-strokewidth size I) and **III** (size III) | ISO 1073-2 index table, p. 14 [R]; ECMA-11 index table, p. 14 (PDF p. 19) [R] |
| Member of sub-sets **1, 2 and 3**: numeric, initial alphanumeric and extended | same [R]; [`character-subsets.md`](character-subsets.md) |
| ISO 1073-2 gives no remark for ref. 79. ECMA-11 says only "Character in numeric sub-set. See ECMA-30" | ISO 1073-2 p. 14 [R]; ECMA-11 p. 14 [R] |
| ECMA-30 lists LESS THAN in both numeric sub-sets (single-line documents and journal tape) with no geometry, and leaves the printed image to ECMA-11 | ECMA-30 2nd ed., §2, §3.1, §4.1 [T] |
| Coded at **3/12** in 7-bit and 03/12 in 8-bit (= ASCII 0x3C), the same position as ISO 646's LESS-THAN SIGN | ISO 2033 Table 8, p. 8 [R]; [`code-positions.md`](code-positions.md) |
| Size III is one of the 22 characters with a size-III drawing, together with the digits, `+`, `>`, the long vertical mark, `C E N S T X Z` and GROUP ERASE | ISO 1073-2 §11.5, p. 19 [R] |

**No standard read here states the vertical position, height, width or angle of `<` in words or
numbers.** The normative shape is drawing 79 C. It is not reproduced in ISO 1073-2 or ECMA-11
([`glyph-dimensions.md`](glyph-dimensions.md#reference-drawings-and-what-the-standard-reproduces)).
Both documents carry `<` only in their §13 illustration of the complete set.

## Measured on ISO 1073-2's §13 illustration (new, [M])

**Method.**

- **Source.** ISO 1073-2 p. 21 (PDF page 23), size I at 4:1. The page was rendered at 600 dpi
  with PyMuPDF. The underlying scan is about 157 dpi, so one scan pixel is about 3.8 render px,
  about 0.016 cap.
- **Segmentation.** Connected components at ink thresholds 96, 128 and 160. All three gave the
  same edges for `<` to within 1 px.
- **Cap height.** The median of the flat capitals `E F H K L M` in row `A`-`M`: **234 px**.
- **Baseline of the `?!()<>` row.** The median bottom of the `?` dot, the `!` dot, `%` and `#`:
  4011, 4014, 4014 and 4018, giving 4014.
- Units are line-cap heights (baseline 0, cap line 1), the same unit
  [`ocrb-filler-geometry-2026-09-23.md`](../benchmarks/ocrb-filler-geometry-2026-09-23.md) uses.

| Quantity | ISO 1073-2 illustration [M] | ECMA-11 illustration [P] | 16 real TD3 specimens, median [P] | Vendored `ocr-b.ttf` [P] |
| --- | ---: | ---: | ---: | ---: |
| Bottom above baseline | **+0.047** | +0.050 | +0.061 | +0.010 |
| Top | 1.017 | 1.046 | 0.997 | 1.006 |
| Height | 0.970 | 0.996 | 0.924 (blur-compressed) | 0.995 |
| **Centre** | **0.532** | 0.548 | **0.528** | 0.508 |
| Width | 0.684 cap (1.68 mm at size I) | — | — | 0.714 |

Other measurements from the same crop:

- The arm stroke measures 39 px vertically, about 32 px (0.34 mm) perpendicular to the arm.
- Each arm rises at about 34° from the horizontal, so the opening angle is about 68°.
- The arm ends are square-cut, not rounded.
- Ink area is **1.22 mm²** at size I, 0.202 cap². In a pitch × cap cell (2.54 × 2.46 mm) that is
  **19.6%**.

**Uncertainty.** About ±0.02 cap on each edge. The scan resolution dominates, and so does the
fact that the baseline comes from other glyphs in the row. The centre is less affected, about
±0.015, because the two edge errors partly cancel.

**What it shows:**

- The document ICAO actually cites, ISO 1073-2, draws the filler centred at **0.53 cap**, with
  its bottom about 0.05 cap above the baseline. This matches the **real-specimen median (0.528)**
  and Barcodesoft `ocrbI` (0.534) almost exactly.
- ECMA-11's reproduction sits about 0.016 cap higher, and it draws `<` taller. The two
  illustrations of one design disagree by about as much as the effect the earlier note measured.
  So the ECMA illustration alone was not a sharp reference.
- Both illustrations put the vendored font's filler about **0.025-0.04 cap too low**.

**Bears on SynthPass:**

- [`crates/synthpass-gen/fonts/ocr-b.ttf`](../../crates/synthpass-gen/fonts/ocr-b.ttf) is the
  vendored font. The correction proposed in
  [`ocrb-filler-geometry-2026-09-23.md`](../benchmarks/ocrb-filler-geometry-2026-09-23.md) §3 is
  to raise the glyph by 0.03-0.04 cap. The ISO illustration supports the lower end of that range
  (about +0.025-0.03). It also supports a height of about 0.97 rather than 1.0.
- Any `<` template for [ADR-0014](../decisions/ADR-0014-per-cell-ocrb-classification.md) should be
  centred at 0.53 cap, not at the vendored 0.51.

## Tolerances that bound where `<` may print

The vertical placement tolerances come from ISO 1831. Details and page citations are in
[`line-and-pitch.md`](line-and-pitch.md#alignment-and-skew).

- **Adjacent-character alignment:** at most 0.65 mm at size I (0.26 cap).
- **Alignment of any two characters in a line:** at most 1.30 mm (0.53 cap).
- **Character skew:** at most 3°.

These are measured against each character's nominal position from the drawings (ISO 1831 §6.8
and Figure 17, p. 21 [R]). Symbols such as `+` carry a nominal vertical offset against the
digits, which the definition corrects for. The measured filler offsets between fonts and prints
(0.02-0.09 cap) are an order of magnitude inside that tolerance.

**Bears on SynthPass:** a vertical-position test that separates `<` from a letter
([`mrz-geometric-elimination.md`](../research/mrz-geometric-elimination.md)) can use a band only
as tight as the conforming print allows. A conforming issuer may misalign one character against
its neighbour by up to 0.26 cap. That swamps a 0.03 cap filler offset. So the filler-versus-letter
discriminator must be **mass and shape**, not vertical position. The research note already
reached that conclusion.

## The other symbols in the neighbourhood

- `>` (ref. 80) and `+` (ref. 64) are the other two symbols in numeric sub-set 1 (ISO 1073-2
  §5.1 [R]). Neither is in the MRZ set.
- A misread of `<` as `>` or `+` is out-of-alphabet. [`crates/mrz`](../../crates/mrz/src/repair.rs)
  can reject it outright, and the standards give no reason to expect such confusions. A misread
  of `<` as `K`, the case the research note flags, is inside the alphabet.
