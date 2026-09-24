# Print quality: ISO 1831-1980, and what Doc 9303 adopts from it

Source: ISO 1831-1980 (E), 1st edition. Every number below was read on a page render, because
the OCR text layer of the local scan is not trustworthy for numbers. Printed page N is PDF page
N+4. Tags are defined in [`README.md`](README.md#sources). The ancestor standard, ECMA-15 (1968),
has the same structure, but its values were not compared.

## What Doc 9303 adopts

[Doc 9303-3 §4.10-§4.11](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#411-quality-specifications-of-the-mrz)
applies ISO 1831 to the MRZ with these choices:

| MRZ requirement | ISO 1831 clause it points to | Value |
| --- | --- | --- |
| General print quality | §5.2 | **Range X** (tight) |
| Substrate quality | §4.3-§4.3.2 (dirt) | reference only |
| Substrate opacity | §4.4.1, §4.4.3 | at least **medium** opacity (> 70%) |
| Spectral band | §3.2 | visibly black (B425 to B680), and absorbing in **B900** |
| Print contrast | §5.4 | **PCS/min ≥ 0.6 in B900** (ICAO's own value) |
| Stroke width | §5.3.1 | Range X: **0.35 ± 0.08 mm** at size I |
| CVR | §5.4.5.8 | **< 1.50** (the Range X value) |
| Spots and extraneous marks | §5.4.4.6, §5.4.5.12; Annex B.6, C.5.10 | apply at the reading surface |
| Voids | §5.4.5.9 | **d = 0.4** at the reading surface |
| Line separation and spacing | — | per format, in Doc 9303 Parts 4-7, not ISO 1831 |
| Skew | §6.4 | ≤ 3° for lines and characters, and nothing may leave the printing zone |

[§4.5](../docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md#45-machine-reading-requirements-and-the-effective-reading-zone)
allows security features inside the MRZ, provided they do not interfere with reading in B900.
The characters must be machine-readable at least in B900.

**Bears on SynthPass (inferred):**

- The whole print-quality contract for the MRZ is written for the **near-infrared B900 band**.
  In visible light ICAO asks only that the print be "black". There is no visible-band PCS minimum
  and no bound on background security print under the zone.
- A phone photo or a published specimen scan is a visible-light image. So no ISO or ICAO number
  bounds the background contrast `synthpass-ocr` meets.
- The guilloche and watermark failures recorded in
  [`detection-misses-2026-09-18.md`](../benchmarks/detection-misses-2026-09-18.md) are therefore
  conforming documents, not defective ones.

## Spectral bands (§3.2, Table 1, p. 2 [R])

| Band | Peak (nm) | Bandwidth at 50% (nm) |
| --- | --- | --- |
| B425 / B460 / B490 / B530 | 425 / 460 / 490 / 530, ± 5 | ≤ 50 / ≤ 60 / ≤ 60 / ≤ 60 |
| B570 / B620 / B680 | 570 / 620 / 680, ± 10 | ≤ 100 / ≤ 100 / ≤ 120 |
| **B900** | **900 ± 50** | **≤ 400** |

The bands describe the response of the complete instrument: source, filter and detector.
Illumination energy below 400 nm should not exceed 5% of the band's energy (§3.2 [R]).

Annex B.4 adds that print in high-carbon-black ink on white paper generally meets all three
spectral regions (p. 29 [R]). Dyes tend to absorb mainly in the visible.

## Paper (§4, pp. 2-4 [R])

- **Luminous reflectance R₀** is measured on a single sheet over a black backing of ≤ 0.5%
  (§4.2.1).
  - Visible: R₀ > 60% at 425-500 nm and > 70% at 500-700 nm. For medium-opacity paper these
    drop to 50% and 60% (§4.2.3).
  - Near-IR: R₀ ≥ 70% at 900 nm, or 60% for medium opacity (§4.2.4).
- **Opacity** is R₀ with black backing divided by the intrinsic reflectance R∞ (§4.4.1).
  - High opacity: > 85%.
  - **Medium opacity: 70-85%** (§4.4.3). This is the ICAO minimum.
- **Reflectance variation** is measured with a 0.2 mm aperture. The standard deviation must be
  < 3.5% of the mean for high-opacity paper and < 5% for medium, in B425, in B530/B570 and in
  B900. The ratio of highest to lowest value must be ≤ 1.2 (§4.5).
- **Dirt:**
  - Method A (visual grid): at most 200 dirty squares per 6 m².
  - Method B (count): a mean of < 250 particles per m² above 0.1 mm, and at most 25 per m² above
    0.2 mm in 19 of 20 samples (§4.3).

## Stroke width and print tolerance ranges

- Three ranges exist.
  - **X** is tight. Doc 9303 requires X.
  - **Y** is medium.
  - **Z** is wide: at the limit of good print, likely to raise rejects, and measurable only by
    the computer-aided method (§5.2, pp. 4-5 [R]).
- **Table 2** gives nominal stroke widths (§5.3.1, p. 5 [R]).
  - The heights in the table are exact for OCR-A. For OCR-B they are only indicative, and the
    exact values must come from the ISO 1073 drawings.
  - For OCR-B, small letters and `#`, `%`, `@` take 0.31 mm at size I and 0.44 mm at size IV.

| Size | Height (OCR-B, see note) | Nominal stroke | Range X tolerance | Ranges Y, Z tolerance |
| --- | --- | --- | --- | --- |
| **I** | 2.40 mm | **0.35 mm** | **± 0.08 mm** (0.27-0.43) | ± 0.15 mm (0.20-0.50) |
| III | 3.20 mm | 0.38 mm | ± 0.08 mm | ± 0.18 mm |
| IV | 3.60 mm | 0.50 mm | ± 0.13 mm | ± 0.25 mm |

Note: the "height" column is the centreline height of EIGHT from ISO 1073-2 §3.6 (see
[`glyph-dimensions.md`](glyph-dimensions.md#sizes-and-scale-factors)). It is not an outline height.

**Bears on SynthPass:**

- At the nominal capital height of 2.46 mm, Range X puts the stroke at **0.110-0.175 cap**
  (nominal 0.142).
- The vendored font's stems are 0.135-0.138 cap, inside that band
  ([`fonts-vs-standard.md`](fonts-vs-standard.md)).
- A synthetic stroke-width sweep in `synthpass-gen`'s degradation stage
  ([`crates/synthpass-gen/src/degrade.rs`](../../crates/synthpass-gen/src/degrade.rs)) that stays
  within 0.08-0.20 cap covers ranges X to Z.

## Character outline limits (COL)

- **Definition.** The minimum COL and the maximum COL are the envelopes swept by a circle of the
  minimum and maximum stroke width moving along the character centreline. A COL gauge is a
  transparent overlay of both COLs and the centreline (§5.3, §5.3.2, pp. 5-6 [R]).
- **Fairing radii.** At size I, R₁ = R₂ = 0.10 mm (Table 3, p. 6 [R]).
  - An internal corner of the minimum COL with radius ≤ R₁ is drawn sharp.
  - A sharp internal corner of the maximum COL is faired to R₂.
  - A sharp centreline corner gives a sharp external maximum COL, except above 305°
    (§5.3.4-§5.3.5.2).
- **Free stroke ends.** Here the maximum COL is **squared off** (§5.3.5.3 [R]). A square-cut end
  and a round end both lie inside it, so both conform.
- **Letterpress.** The letterpress font is checked with the same size I, range X gauges. Its
  stroke deviates 5-10% from nominal, which is neglected. Sharp corners of well under 90° may poke
  outside the COLs (§5.3.6 [R]).
- **Range Z cut-off.** Range Z allows asymmetric "cut-off" of numeric-subset characters. The
  cut-off rectangle for OCR-B size I is **2.40 mm × 1.40 mm**, the centreline height of EIGHT by
  the centreline width of ZERO (Table 4, p. 7 [R]).
- **Rectangle offset d_v.** The rectangle sits d_v = **0.13 mm** above the horizontal character
  reference line. It is centred on the vertical reference line for OCR-B (Table 5, p. 7 [R]).

## Print contrast, voids, spots, edges (§5.4, pp. 10-19 [R])

**Definitions** (§5.4.3; §5.4.5.2-§5.4.5.3):

- PCS = (R_w − R_p) / R_w.
  - R_w is the maximum paper reflectance in the area of interest.
  - R_p is the reflectance at the point measured.
  - Both are measured through a 0.2 mm circular aperture, or a 0.15 mm square.
- The **area of interest** is about twice the nominal character height by twice its width,
  centred on the character (§5.4.5.2; Annex B.8).
- A **stroke edge** is where reflectance is about halfway between the stroke and the background
  (§5.4.3.9).
- **Best fit** is the gauge position where the character fills the minimum COL as much as
  possible while extending as little as possible past the maximum COL (§5.4.3.3, §5.4.5.4).

**Instrumented values** (§5.4.5):

- **Sampling.** Basic PCS values are taken with a 0.2 mm aperture moved along the centreline in
  0.1 mm steps. The step is 0.05 mm when the centreline is shorter than 2 mm (§5.4.5.5.1).
- **PCS₈₀%**, the smallest of the highest 80% of basic values, must be **> 0.60 in range X** and
  > 0.50 in range Y (§5.4.5.5.2). The CAM method adds > 0.35 for range Z (§5.4.6.5).
- **PCS_max and PCS_min** are the highest and lowest averages of 3 consecutive basic values, or 5
  when the centreline is under 2 mm (§5.4.5.6-§5.4.5.7).
- **CVR** = PCS_max / PCS_min. It must be **< 1.50 (X)** and < 1.75 (Y) (§5.4.5.8), and < 2.0
  for Z under CAM (§5.4.6.8).
- **Voids:** a point is a void where PCS < d, with **d = 0.40 (X)** and 0.35 (Y) (§5.4.5.9).
  - Centreline longer than 2 mm: an isolated void point is allowed. A pair of void points is
    allowed if the next pair is at least 11 steps away. **Three or more consecutive void points
    make the character non-conforming.**
  - Centreline under 2 mm: one or two void points are allowed. Three or four are allowed only if
    the next such group is at least 21 steps away. Five or more make the character
    non-conforming.
  - The CAM variant: PCS_min > 0.40 / 0.35 / 0.30 for X / Y / Z (§5.4.6.9).
- **Stroke edges:** sample in 0.2 mm steps.
  - Along the minimum COL every value must exceed 0.5·PCS_avg.
  - Along the maximum COL every value must be below 0.5·PCS_avg.
  - If 0.5·PCS_avg < 0.3, the fixed value 0.3 is used instead (§5.4.5.10.2).
  - PCS_avg is the mean of the highest 80% of basic values (§5.4.5.10.1). A quick approximation
    is (PCS_max + PCS₈₀%)/2 (Annex B.5, p. 29).
  - An edge irregularity must be at least 1 mm from the next one (§5.4.5.11).
- **Spots:**
  - The aperture is centred on the spot's darkest point, plus the eight positions around it at
    0.1 mm steps. The threshold is e = 0.65·PCS_min (X) or 0.70·PCS_min (Y). Three or more
    positions above e make the spot non-allowable; two require a second test (§5.4.5.12).
  - Spots remote from the character are not PCS-limited. In the clear area they must stay within
    0.2 mm diameter (§5.4.5.12).
  - Annex B.8 warns that a spot over 0.2 mm can trigger a reader's "first black point" start of
    recognition (p. 30).

**Visual method** (§5.4.4, pp. 10-12):

- **Voids and spots** are allowed when they fit a **0.2 mm inspection circle** and cover less
  than 1/3 of it. Larger ones, up to the full circle, are allowed if the next such one is at
  least **1 mm** away (§5.4.4.4, §5.4.4.6).
- **Edge irregularity:** a bump outside the maximum COL, or a bite inside the minimum COL, is
  allowed if it is **≤ 0.3 mm long, measured along the COL** (it is a length, not a depth) and
  at least 1 mm from the next one (§5.4.4.5, Figure 10).

**Computer-aided method** (§5.4.6, pp. 16-19; Annex C):

- **Scanner:** 25 µm resolution and aperture, at least 32 grey levels. Parameters integrate over
  a 0.2 mm circle (§5.4.6.1).
- **Rectangle Q:** R_w is taken in a rectangle Q centred on the character. For OCR-B size I,
  sub-sets 1 and 2 (which hold every MRZ character), Q is **4.90 mm high × 2.50 mm wide**. For
  sub-sets 3 and 4 it is 3.30 × 2.50 mm (Table 6, p. 17).
- **Edge threshold:** stroke edges are set at PCS₄ = 0.5·PCS₃, or at 0.3 if PCS₃ < 0.6
  (§5.4.6.10.1). PCS₃ is the mean of PCS values ≥ PCS₈₀% along the centreline.
- **Irregularities:** violations up to 0.3 mm along a limit line are allowed, if 0.7 mm of clean
  line separates them (§5.4.6.10.3).
- **Spots inside Q:** allowed if they never cover more than **10% of any 1 mm circle** centred
  in Q (§5.4.6.11).

**Bears on SynthPass:**

- **Adaptive threshold (inferred).** ISO defines ink **relative to the character's own
  contrast**: the stroke edge sits at half the stroke's PCS. `chargrid::is_ink` uses a fixed luma
  cut of `< 110`
  ([`crates/synthpass-ocr/src/chargrid.rs`](../../crates/synthpass-ocr/src/chargrid.rs)). A
  standard-shaped threshold would be adaptive: half-way between the local paper maximum and the
  stroke level, within a Q-sized window of about 2 pitches by 2 line heights.
- **Where the fixed cut bites (modelled).** Suppose paper reads sRGB 230, which is linear 0.79. A
  stroke that just meets PCS 0.6 has linear reflectance 0.32, which is sRGB **≈ 152**. That is
  above 110, so the fixed cut would count a conforming but light stroke as paper. The PCS
  requirement is set in B900 and camera responses are not linear reflectance, so this is a bound
  on direction, not a prediction.
- **Voids and the ink floor (inferred).** The rule of "3 consecutive void points" on a 0.1 mm
  step means a conforming stroke can lose up to about 0.2 mm of contrast in a row. A fine
  binarised filler arm can therefore legitimately break. This bears on `DEFAULT_INK_FLOOR` and on
  any template matching of `<`
  ([`ocrb-filler-geometry-2026-09-23.md`](../benchmarks/ocrb-filler-geometry-2026-09-23.md) §4).
- **Spots (inferred).** The 10%-of-a-1-mm-circle spot allowance is the most stray ink a
  conforming print may carry next to a character. It is a B900 figure; in the visible band the
  background print is unbounded, as above.
