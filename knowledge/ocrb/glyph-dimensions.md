# OCR-B glyph dimensions: sizes, heights, stroke, reference drawings

Sources: ISO 1073-2 (ISO 1073/II-1976, corrected reprint 1979) and its ECMA twin ECMA-11 (3rd ed.,
1976). The citation keys and verification tags are explained in [`README.md`](README.md#sources).
Doc 9303 selects **size I, constant-strokewidth**
([Part 3 §4.4](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#44-print-specifications)).
Everything below that is specific to another size or style is included only so it is not confused
with that selection.

## Two styles, one definitive centreline

- OCR-B comes in two styles.
  - **Constant-strokewidth:** the stroke is the same width everywhere, and the reference grid
    shows each character's centreline.
  - **Letterpress:** the stroke width varies on purpose, and the stroke endings are specially
    designed.
- **The centrelines are the same for both styles. The centreline is the definitive part of the
  standard** (ISO 1073-2 §2, p. 1 [R]; ECMA-11 §2, p. 1 [R]).
- Letterpress is specified in size I only, with an option for variable pitch. Constant-strokewidth
  is specified in sizes I, III and IV and "will usually" keep a fixed pitch (ISO 1073-2 §3.2-3.3,
  pp. 1-2 [R]).
- The styles differ in their stroke ends.
  - ISO 1073-2 §4.2 says the letterpress shapes are similar "except that the stroke ends are not
    rounded" (p. 2 [R]).
  - ECMA-11 §4.2 says it from the other side: the constant-strokewidth characters "have rounded
    stroke ends" (p. 3 [R]).
- **Bears on SynthPass:** Doc 9303 asks for the constant-strokewidth style, whose stroke ends are
  nominally rounded. The vendored font and the §13 illustrations in both standards have
  square-cut arm ends; see [`filler-and-symbols.md`](filler-and-symbols.md). ISO 1831 accepts both,
  because the maximum outline is squared off at free stroke ends (see
  [`print-quality-iso1831.md`](print-quality-iso1831.md#character-outline-limits-col)). A shape
  prior should therefore not rely on how stroke ends look.

## Sizes and scale factors

| Quantity | Size I | Size III | Size IV | Source |
| --- | --- | --- | --- | --- |
| Styles available | letterpress + constant-strokewidth | constant-strokewidth; numeric sub-set and GROUP ERASE only | constant-strokewidth; all except VERTICAL LINE | ISO 1073-2 §3.2-3.3, §6.1, pp. 1-2, 6 [R] |
| Centreline scale vs size I (vertical / horizontal) | 1 / 1 | 1.333 / 1.086 | 1.500 / 1.500 | ISO 1073-2 §3.5, p. 2 [R]; ECMA-11 §3.5, p. 2 [R] |
| Centreline height of EIGHT (tallest character) | 2.40 mm | 3.20 mm | 3.60 mm | ISO 1073-2 §3.6, p. 2 [R]; ECMA-11 §3.6 [R] |
| Centreline width of ZERO (widest character) | 1.40 mm | 1.52 mm | 2.10 mm | ISO 1073-2 §3.7, p. 2 [R]; ECMA-11 §3.7 [R] |
| Minimum nominal pitch, constant-pitch printing | 2.54 mm | 2.54 mm | 3.63 mm | ISO 1073-2 §3.8, p. 2 [R]; ECMA-11 §3.8 [R] |
| Nominal stroke width (most characters) | 0.35 mm | 0.38 mm | 0.50 mm | ISO 1073-2 §11.4.1, §11.5, §11.6, pp. 19-20 [R] |
| Nominal stroke width (small letters, `#`, `%`, `@`) | 0.31 mm | — | 0.44 mm | same [R] |

- Size II appeared in ISO/R 1073-1969. It was deleted, and size IV replaced it (ISO 1073-2 §3.4
  [R]; ECMA-11 §3.4 and Brief History [R]).
- The scale factors apply to centrelines only, not to outlines, because the stroke width does not
  scale in proportion (ISO 1073-2 §3.5 [R]).
- Size IV is exactly 1.5× the size I centreline. Its outline is rebuilt at the size IV stroke
  width (§11.6 [R]).
- **Bears on SynthPass:** ICAO's "size 1" plus a 2.54 mm pitch is exactly the size I minimum
  pitch. The MRZ is therefore printed at the tightest pitch the typeface allows. The height of
  2.40 mm that [ISO 1831 Table 2](print-quality-iso1831.md#stroke-width-and-print-tolerance-ranges)
  gives is **this centreline height of EIGHT**, not a cap height and not an outline height.
  [`ocrb-filler-geometry-2026-09-23.md`](../benchmarks/ocrb-filler-geometry-2026-09-23.md) §5 lists
  it next to the outline heights as "Capital / digit height... 2.40 mm". That mixes the two
  quantities.

## Typical outline heights above and below the baseline (size I)

The two standards print the same four-column table, labelled A-D. **Each labels it as a
different style, and the digit height differs.**

| Dimension | ISO 1073-2 Table 1, **constant-strokewidth** | ECMA-11 §4.1 table, **letterpress** |
| --- | --- | --- |
| A: digit height above baseline | **2.66 mm** | **2.60 mm** |
| B: capital height | 2.46 mm | 2.46 mm |
| C: small-letter (x) height | 1.83 mm | 1.83 mm |
| D: descender below baseline | 0.60 mm | 0.60 mm |
| Digit / capital (A/B) | **1.081** | **1.057** |

Sources: ISO 1073-2 §4.1, Table 1 and Figure 1, p. 2 [R] (Figure 1 shows A as the top of `3`
and `0`); ECMA-11 §4.1, p. 3 [R]. Both standards call these values "for general information only".
Exact per-character values come from the reference drawings.

**Measured [M] on ISO 1073-2's own §13 illustration** (size I at 4:1, p. 21; PDF page 23 rendered
at 600 dpi from a scan of about 157 dpi, ink threshold 96/128/160, cap height 232-235 px):

- **Heights.** The flat-topped digits `1`, `4` and `7` stand at **1.082 cap**, which is ISO's
  2.66/2.46. The round digits `0`, `6` and `9` are 1.116 (overshoot), and `8` is 1.134. The
  flat capitals are 1.000-1.017, and `O` is 1.047.
- **Pitch.** The letter centres are **1.032 cap** apart. At the nominal cap of 2.46 mm that is
  **2.540 mm**, so the illustration is set at the MRZ pitch.
- **Stroke.** Vertical stems run 31-34 px, which is **0.33-0.36 mm** at scale. The nominal
  stroke is 0.35 mm.
- **Stroke ends** are square-cut in this illustration, even though its digit heights follow the
  constant-strokewidth table.
- The earlier measurement of ECMA-11's §13 illustration found the digit `1` at 1.063 cap, which
  matches ECMA's letterpress 1.057
  ([`ocrb-filler-geometry-2026-09-23.md`](../benchmarks/ocrb-filler-geometry-2026-09-23.md) [P]).

Each illustration therefore agrees with its own document's table. The 2.3% difference between
them is real, not a scan artefact. It is **not resolved** here whether the difference comes from
the style or from an editorial change between 1976 editions.

**Bears on SynthPass:**

- ICAO specifies constant-strokewidth, so the applicable digit/capital gap is **about 8%** of cap
  height (0.20 mm), not the 5.7% that
  [`mrz-geometric-elimination.md`](../research/mrz-geometric-elimination.md) quotes from ECMA-11.
- All the OCR-B font files measured sit at 1.077-1.089 on flat digits
  ([`fonts-vs-standard.md`](fonts-vs-standard.md)).
- The larger gap favours the height signal (top edge separates digit from letter). At a 20 px cap
  it is about 1.6 px for flat tops, before overshoot on round letters eats into it.

## Widths and centring

- ZERO is the widest character. Its centreline width is 1.40 mm, which puts the outline at about
  1.75 mm (§3.7 [R]). Measured on the illustration [M], `0` spans 0.724 cap, i.e. 1.78 mm.
- `1` is narrowest at 0.414 cap. `I` is 0.478, `F` 0.491, `J` 0.496, `K` 0.741 and `Q` 0.759
  (illustration [M]).
- Each reference drawing carries pointers. They fix the baseline, the orientation and, for
  fixed-pitch printing, the horizontal position (ISO 1073-2 §11.4.1, p. 19 [R]).
- A second pointer gives "the most aesthetic spacing". On printers with a large horizontal
  tolerance, the geometric centreline is recommended instead (§11.4.3 [R]).
- ISO 1831 Annex D.8 gives an example: for OCR-B `J` (size I), the centreline sits up to
  **0.18 mm** from the vertical reference line (ISO 1831 p. 41 [R]). That is 7% of the pitch.
- **Bears on SynthPass:** glyphs are placed in their cell by a reference line, not by the centre
  of their ink. A per-cell classifier or `chargrid` fit
  ([`crates/synthpass-ocr/src/chargrid.rs`](../../crates/synthpass-ocr/src/chargrid.rs)) should
  expect glyph-dependent horizontal offsets of up to about 0.07 pitch.
  `synthpass-gen`'s ink-centring
  ([`crates/synthpass-gen/src/render.rs`](../../crates/synthpass-gen/src/render.rs)) removes those
  offsets from the synthetic corpus. The vendored font's own left-edge spread is 0.19 pitch, as
  [`tools/ocrb_metrics.py`](../../tools/ocrb_metrics.py) reports.

## Reference drawings and what the standard reproduces

- **Drawing grid.** The shapes are defined by original drawings for sizes I and III, drawn at
  **100:1 on a 2 mm grid**. The whole grid is 280 mm × 380 mm (ISO 1073-2 §11.1, p. 19 [R]).
  Points can be read to half a square, which is 10 µm at full size, or to a quarter square with
  care (§11.1 [R]).
- **Paper copies are not good enough.** Readings must come from drawings on stable material,
  because paper reproductions are not dimensionally stable (§11.1 [R]).
- **Who holds them.** Duplicates at exact 100:1 were available on request from ECMA, Geneva, and
  NBS, Washington, as sets OD 1-OD 4 (§11.2 [R]). The drawings later moved from ECMA and NIST to
  JISC ([`history-and-governance.md`](history-and-governance.md) [H]).
- **Type is not cut to these outlines.** The standard fixes the *printed image*. Type
  dimensions are to be derived from it after correcting for the printing process's systematic
  effects (§11.3 [R]).
- **Corners.** A special effort should be made to print the given line endings and corners,
  "especially important" for the square corners of B and D (§11.4.2 [R]; §12, p. 21 [R]).
  Corners with no specified radius should be as sharp as practicable, but radii below 0.08 mm are
  not needed for OCR (§1.3 note 3, p. 1 [R]).
- **Strokes sit on the centreline.** Printed strokes must be symmetric about the centrelines
  (§12 [R]).
- **What the document itself reproduces:**
  - the complete set in size I at 4:1 and at 1:1;
  - drawings of ONE, E, PARAGRAPH (§) and YEN in size I, as letterpress and as
    constant-strokewidth;
  - ONE and E in size III.

  The drawings are reproduced at about 70:1 (§13, p. 21 [R]; pp. 23-41 skimmed [R]). ECMA-11
  reproduces the same four at about 62:1 (ECMA-11 §11.1 [R]).
- **No drawing of `<` or of any MRZ letter other than E is reproduced in either standard.**
- **Annex A** (informative) is the old design of digit ZERO, with outline and centreline
  coordinate tables in µm (pp. 43-45 [R]). Reference points 1 and 3 sit at y = 2000 µm, and the
  pitch reference line at x = 2000 µm.
- **Annex B** (informative) is the deleted size-II ten digits, with centreline coordinates and a
  0.35 mm stroke (pp. 47-53 [R]).
- **Annex C** (informative) covers typewriter implementation. It recommends a tool whose diameter
  equals the stroke width, which yields constant stroke; half that width gives letterpress-like
  type (p. 54 [R]).
- **Bears on SynthPass:**
  - The annex coordinate tables are low priority for transcription. They describe glyphs no
    current MRZ uses, except where an issuer still prints the pre-1976 ZERO: ISO 1073-2 §5.1
    note 1 tolerates it only in numeric applications implemented before 1976.
  - The normative shape of `<` is available only from the original drawing 79 (C and III), which
    is not in any document here.
