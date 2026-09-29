# Pitch, spacing, alignment and line geometry

The OCR standards set **minimums and tolerances** for any OCR document (ISO 1831 §6). Doc 9303
then fixes the **nominal** layout for each MRZ format (Parts 4-7). Where the two overlap, Doc 9303
is the tighter one. The tags are defined in [`README.md`](README.md#sources). ISO 1831 printed
page N is PDF page N+4.

## Horizontal: pitch, spacing, separation

| Quantity | Value (size I) | Source |
| --- | --- | --- |
| MRZ pitch | **2.54 mm fixed** (10 per 25.4 mm) | [Doc 9303-3 §4.4](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#44-print-specifications) |
| Minimum nominal pitch, OCR-B size I, constant pitch | 2.54 mm | ISO 1073-2 §3.8, p. 2 [R] |
| **Character spacing**: distance between the centrelines of adjacent character boundaries, corrected for each pair's nominal offsets | **≥ 2.30 mm** | ISO 1831 §6.7.2, p. 20 [R] |
| Characters count as "adjacent" if their spacing is below | 4.60 mm | same [R] |
| **Character separation**: the clear gap between adjacent boundaries | **≥ nominal stroke width (0.35 mm)** | ISO 1831 §6.7.1, p. 20 [R]; Annex D.8, p. 41 [R] |
| Nominal centreline offset of a glyph from its reference line (example: `J`) | up to 0.18 mm | ISO 1831 Annex D.8 [R] |

- Pitch may therefore shrink by up to **0.24 mm (9.4%)** between any two characters and still
  conform to ISO 1831.
- Doc 9303 gives no pitch tolerance of its own. Only the positions of the first character are
  toleranced: TD3 6.0 ± 1.0 mm, TD1 5.0 ± 1.0 mm, TD2 4.0 ± 1.0 mm (Parts 4-6 §4.2.1.3).
- ISO 1831 §6.7.2 says letterpress, variable-pitch and some journal-tape printers may break the
  spacing rule. Some scanners can tolerate this as long as the separation rule holds.

**Bears on SynthPass:**

- Measured on real specimens, TD3 character pitch is **0.968-1.056 cap**, with a median of 1.024
  ([`ocrb-filler-geometry-2026-09-23.md`](../benchmarks/ocrb-filler-geometry-2026-09-23.md) §5 [P]).
- The nominal pitch is 2.54 / 2.46 = **1.033 cap**. The ISO floor is 2.30 / 2.46 = **0.935 cap**.
  Every specimen sits inside that envelope.
- A `chargrid` pitch fit
  ([`crates/synthpass-ocr/src/chargrid.rs`](../../crates/synthpass-ocr/src/chargrid.rs)) can
  therefore bound its pitch search at **0.93-1.07 cap** without excluding conforming print. The
  upper bound is a measurement, not a standard limit, because nothing in the standard caps pitch
  from above.
- *Corrected 2026-09-28 ([#411](https://github.com/ruledicaprio/SynthPass/issues/411)):* the
  generator used to draw a **1.119 cap** cell (TD3) and 1.166 (TD2/MRV-B), both **outside every
  real measurement and above the nominal value**. It now draws a fixed 22 px `MRZ_CELL_WIDTH`
  ([`crates/synthpass-gen/src/layout.rs`](../../crates/synthpass-gen/src/layout.rs)) in every
  format — **1.026 cap**, inside the ISO 1831 floor and the measured real-TD3 range. See
  [`../benchmarks/ocrb-standards-vs-priors-2026-09-24.md`](../benchmarks/ocrb-standards-vs-priors-2026-09-24.md).

## Alignment and skew

Alignment is measured between the bottoms of two character boundaries in one line. It is
**corrected** for the vertical offset the two characters would have if both were printed in their
nominal positions (ISO 1831 §6.8, Figure 17, p. 21 [R]).

| Quantity | Size I | Source |
| --- | --- | --- |
| Adjacent-character misalignment | ≤ **0.65 mm** (0.26 cap) | ISO 1831 §6.8.1 [R] |
| Misalignment of any two characters in the line | ≤ **1.30 mm** (0.53 cap) | ISO 1831 §6.8.2 [R] |
| Character skew | ≤ **3°** | ISO 1831 §6.4, p. 20 [R]; [Doc 9303-3 §4.11](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#411-quality-specifications-of-the-mrz) (line and character) |

- Annex D.9 names the causes: individual type faces out of line, the document misaligned in the
  printer, and local distortion or folding. It also warns about fields printed at different times
  on different devices (pp. 41-42 [R]).
- **Bears on SynthPass:**
  - A conforming MRZ line need not be straight to within 0.26 cap.
  - Baseline fits in `chargrid` or a future `mrz-locate`
    ([ADR-0015](../decisions/ADR-0015-geometric-mrz-band-location.md)) should be robust fits, as
    the Theil-Sen fit in the 2026-09-23 instrument is, not per-glyph thresholds.
  - An MRZ personalised in two passes, for example a laser-engraved number field, can
    legitimately step by up to 0.53 cap within one line.

## Lines: spacing, separation, clear area

| Quantity | Value | Source |
| --- | --- | --- |
| **Line spacing**: distance between the average horizontal centrelines of two lines | ≥ **4.20 mm** at size I | ISO 1831 §6.13, p. 22 [R] |
| **Line separation**: gap between the line boundaries | ≥ **0.65 mm** at size I | ISO 1831 §6.12, p. 22 [R] |
| Recommended line separation | 2.5 mm | ISO 1831 Annex D.5, p. 41 [R] |
| Maximum line-packing density, size I | about 6 lines per 25.4 mm (4.23 mm) | ISO 1831 Annex D.4, Table 8, p. 40 [R] |
| Clear area around a line's printing area | ≥ **2.5 mm** each side; clear areas of successive lines may overlap | ISO 1831 §6.10, Figure 18, pp. 21-22 [R] |
| Margin from printing area to paper edge | normally ≥ 6.35 mm | ISO 1831 §6.11, p. 22 [R] |
| Single-line documents: line centre height from bottom edge / printing-area height, size I | 9.6 mm / 5.8 mm | ECMA-18 2nd ed., §5-§6, p. 1 [R] |

- Annex D.4 explains why a line-spacing rule exists alongside the separation rule. A line may hold
  only short symbols (its example is minus), so spacing must still leave room for full-height
  characters (p. 40 [R]).
- ECMA-18's printing-area height of 5.8 mm is derived from the vertical misalignment and
  stroke-width tolerances, plus 1.6 mm for guillotining (ECMA-18 §6 [R]). It concerns cheques
  rather than MRTDs.

### Against Doc 9303's per-format nominal values

Doc 9303 overrides the generic clear area. It allows background security print inside the MRZ,
provided the zone still reads in B900
([§4.5](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#45-machine-reading-requirements-and-the-effective-reading-zone)).

| Format | Nominal line pitch (reference centre lines) | Printing zone per line | vs ISO 1831 minimum spacing 4.20 mm | Source |
| --- | --- | --- | --- | --- |
| TD3 / MRV | **6.35 mm**; line centres at 9.40 and 15.75 mm from the bottom edge | 4.3 mm | 1.5× the minimum | [Doc 9303-4 Figure 3](../docs9303/Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md) [R on the figure render] |
| TD1 | **4.23 mm** (0.167 in) | 2.95 mm | **at the ISO minimum**: 6 lines per inch, Annex D.4's maximum packing | [Doc 9303-5 Figure 6](../docs9303/Doc_9303_Part5_Specs_for_TD1_MROTDs.md#421-data-position-data-elements-and-print-position-in-the-mrz) [R on the figure render] |

**Bears on SynthPass:**

- **TD1 is packed as tightly as the OCR standard permits.** Its lines sit 4.23 mm apart, which is
  1.72 cap at the nominal capital height and 1.59 cap if the constant-strokewidth digit height
  (2.66 mm, [`glyph-dimensions.md`](glyph-dimensions.md)) is taken as the line height. Each
  printing zone is only 2.95 mm tall, 0.29 mm more than a digit.
- A TD1 prior therefore has almost no vertical slack: each line may drift about ±0.15 mm inside
  its zone, so a conforming TD1 prints its lines 4.20-4.52 mm apart (1.71-1.84 cap).
- The TD3 printing zones (4.3 mm around a 2.66 mm digit) permit a line pitch of 4.71-7.99 mm
  (1.91-3.25 cap). Under ISO 1831 the pitch can never fall below 4.20 mm.
- *Corrected 2026-09-28:* these bullets first used ECMA-11's letterpress digit (2.60 mm), which
  gave 1.63 cap, 0.35 mm, ±0.17 mm and 4.65-8.05 mm.
- **Neither format's spacing prior can be one number.** TD3 has a wide band. TD1 is a different,
  much tighter value (see the findings file).
- **What the generator draws ([#411](https://github.com/ruledicaprio/SynthPass/issues/411)):**
  TD1's MRZ lines sit [`TD1_MRZ_LINE_PITCH`](../../crates/synthpass-gen/src/layout.rs) = 37 px =
  **1.726 cap** apart, inside the 1.71-1.84 cap band above. Before #411, TD1 used the pitch every
  format shares, `MRZ_LINE_HEIGHT` + `MRZ_LINE_SPACING` = 55 px = 2.565 cap. The other four formats
  keep 55 px; TD3's nominal 6.35 mm is 2.58 cap.

## Doc 9303 transcription defects found while checking these figures

Checked against ICAO's own PDFs on 2026-09-24, **neither table exists in Doc 9303**. On Part 4
p. 12 and Part 5 p. 14, the values are only labels on the figure. The tables were invented when
the PDFs were converted, and some of their values (Part 5's "23.3" and "2.54") appear nowhere
in that Part. Both were removed by the PDF audit of `knowledge/docs9303/` (see its README,
"PDF audit, 2026-09-24"), and the figures' real labels are now listed in their place.

- The **Part 4 MRZ dimension table** below Figure 3 labels 7.25 mm the "upper reference line
  position" and 6.35 mm the "lower reference line position". On the figure itself, 6.35 mm is the
  **distance between** the two reference centre lines. The line centres sit at 9.40 and 15.75 mm
  from the bottom edge, and 7.25 mm is the **bottom of the lower printing zone**.
- The **Part 5 table** below Figure 6 gives line spacing as **4.0 mm**. The figure shows
  **4.23 mm (0.167 in)**, and the ISO 1831 minimum of 4.20 mm rules out 4.0 mm for size I.
