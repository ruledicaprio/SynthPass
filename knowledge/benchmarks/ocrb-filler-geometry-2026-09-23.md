# Synthetic MRZ geometry against ECMA-11 and real print: the filler sits 0.03 cap low, cells are 9% too wide

**Date:** 2026-09-23 · **MAIN:** `66b50eb` · **DATA:** `396b22f` · **Evidence:** Observed (glyph geometry measured on 16 public TD3 specimens from `samples-data`, on the ECMA-11 §13 4:1 illustration, and on four OCR-B font files; positive-control renderings; band geometry from the same crops and from the generator's layout constants; no cargo, no benchmark run) · **Status:** current

`synthpass-gen` draws every MRZ glyph from the vendored `crates/synthpass-gen/fonts/ocr-b.ttf`
(Raisty 2019, OFL). The project has measured that the synthetic corpus does not predict real
results for a repair class (chargrid: +30 names on synthetic, 0 on real), so a filler that is
shaped or placed differently from the printed one is a candidate explanation. This note measures
where the filler `<` sits relative to the capitals on the same line, in the standard, in three
independent OCR-B cuts, and on real specimens.

**Unit throughout: the cap height of the line's own capitals** (baseline = 0, cap line = 1).
*Centre* is the midpoint of the filler's bottom and top ink edges. No MRZ content, name or number
appears here; the unit of evidence is a glyph's vertical extent.

## 1. What the references say

| Source | Filler bottom | Filler top | Height | Centre | How it was measured |
| --- | ---: | ---: | ---: | ---: | --- |
| ECMA-11 3rd ed. (1976), §13 illustration, scale 4:1 | **+0.050** | 1.046 | 0.996 | **0.548** | scan, deskewed, bbox at 3 thresholds |
| Vendored `ocr-b.ttf` (Raisty 2019) | +0.010 | 1.006 | 0.996 | 0.508 | outline bounds |
| Barcodesoft `ocrbI.ttf` / `ocrbIV.ttf` | +0.074 | 0.995 | 0.921 | 0.535 | outline bounds |
| Barcodesoft `OCRB.TTF` | +0.162 | 1.037 | 0.875 | 0.600 | outline bounds |

Font cap height is the median of the flat capitals `EFHIKLMNTXZ`. The Barcodesoft files are
commercial and local only; they are named, not vendored.

**ECMA-11 does not state the filler's position in words.** The index table (page 19, ref. 79,
LESS THAN SIGN, sets 1-3) gives only "Character in numeric sub-set. See ECMA-30", and ECMA-30
(2nd ed., 1976) lists `<` in the numeric sub-sets without geometry. The normative shapes are the
original drawings (§11.1-11.2: drawings 79 L, C and III exist, supplied on request at 100:1). They
are **not** among the reference drawings reproduced in the standard, which are `1`, `E`, `§` and
`¥` only (pages 30-39). The only in-document evidence is therefore the §13 illustration (page 29,
"complete character set in Size I at scales 4:1 and 1:1").

On that illustration, embedded as a 300 dpi scan and deskewed by 0.70°, the cap height is
118-121 px (rows `ABC…M` and `NOP…Z`) and the digit `1` is 127 px: 1.063 cap against the 1.057
that §4.1 specifies (2.60 / 2.46 mm), so the vertical scale holds to 0.6%. In the `?!()<>[]%#&`
row the baseline, taken from the bottoms of `?`, `!`, `%` and `#`, is at 1904-1906. The filler's
last ink row is 1899, so it sits **6 ± 1.5 px = 0.050 cap above the baseline**. Its top edge lands
on the same row as the top of `?`. The result does not move by more than 1 px across ink thresholds
of 96, 128 and 160. The arms end in vertical cuts, as in the vendored font; the Barcodesoft
`ocrbI` cut has rounded (constant-strokewidth) ends.

**ISO 1831-1980** defines character alignment against each character's *nominal* position from
the drawings (§6.8, figure 17: "shift of the nominal position of plus with respect to the
digits"). It confirms that symbols carry a nominal vertical offset but does not give one for `<`.
It allows adjacent-character misalignment of up to **0.65 mm at size I** (§6.8.1) and 1.30 mm
within a line (§6.8.2). At size I that is 0.26 and 0.53 cap. The offsets this note measures are
about 0.03-0.04 cap (≈0.07-0.10 mm), well inside that tolerance.

## 2. What real specimens print

Sixteen TD3 books, line 1 (letters and fillers only, so no digit can pollute the cap line). All
are public specimens extracted from `origin/samples-data` at `396b22f`, and every file's sha256
matched `samples/corpus.jsonl`. `observed.checksums_valid` and `observed.agrees` are true for
each. Private samples were not used.

| Specimen | Cap px | Letters | Fillers | Bottom | Top | Height | Centre ± SEM |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Netherlands P0 NLD 2014 | 43.7 | 28 | 16 | +0.055 | 0.998 | 0.944 | 0.527 ± 0.002 |
| Belgium P0 BEL 2018 child | 30.3 | 16 | 28 | +0.060 | 0.975 | 0.914 | 0.512 ± 0.005 |
| Norway P0 NOR 2020 | 29.9 | 30 | 14 | +0.032 | 1.031 | 1.000 | 0.531 ± 0.002 |
| Bahrain PB BHR 2023 | 24.6 | 33 | 11 | +0.066 | 0.996 | 0.927 | 0.531 ± 0.001 |
| Türkiye P0 TUR 2019 | 21.9 | 20 | 24 | +0.035 | 0.935 | 0.905 | 0.485 ± 0.010 |
| Monaco P0 MCO 2021 | 21.8 | 20 | 24 | +0.085 | 1.035 | 0.948 | 0.559 ± 0.002 |
| Brazil P0 BRA 2015 | 21.7 | 26 | 18 | +0.090 | 1.001 | 0.911 | 0.544 ± 0.003 |
| United Arab Emirates P0 ARE 2011 | 21.4 | 37 | 7 | +0.138 | 0.944 | 0.803 | 0.539 ± 0.004 |
| Saudi Arabia P0 SAU 2021 | 20.8 | 28 | 16 | +0.047 | 0.961 | 0.918 | 0.504 ± 0.005 |
| Azerbaijan PC AZE 2013 | 20.8 | 19 | 25 | +0.050 | 1.019 | 0.964 | 0.534 ± 0.004 |
| Serbia P0 SRB 2012 | 20.6 | 14 | 30 | +0.063 | 0.993 | 0.927 | 0.528 ± 0.001 |
| Seychelles P0 SYC 2022 | 20.4 | 18 | 26 | +0.012 | 1.023 | 1.013 | 0.516 ± 0.005 |
| Austria PP AUT 2023 | 20.2 | 18 | 26 | +0.044 | 1.000 | 0.957 | 0.522 ± 0.003 |
| Nicaragua P0 NIC 2015 | 18.2 | 29 | 15 | +0.071 | 0.976 | 0.907 | 0.524 ± 0.005 |
| United States P0 USA 2020 | 18.0 | 17 | 27 | +0.068 | 0.987 | 0.918 | 0.528 ± 0.001 |
| Japan PP JPN 2025 | 15.8 | 16 | 28 | +0.083 | 1.005 | 0.920 | 0.542 ± 0.001 |
| **Median (IQR)** | | | **335** | **+0.061** (0.046-0.074) | 0.997 | 0.924 | **0.528** (0.520-0.535) |

Every value is a median over that line's fillers. The SEM is the within-line spread of the
centre, divided by √fillers.

### Method

1. **Locate.** A local-threshold connected-component pass groups glyph-sized components into
   rows. The MRZ is the lowest pair of adjacent rows holding 40-46 components each, with heights
   within 25% of each other. Line 1 is the upper row, cropped to half the line gap above and
   below.
2. **Segment.** An Otsu threshold is applied inside the crop. A line is accepted only when it
   yields exactly 44 glyphs.
3. **Classify.** Every glyph is resized to a 16×16 bitmap. The filler is the medoid of the
   largest cluster of near-identical shapes (IoU > 0.6), because the trailing run guarantees a
   dozen or more copies of one glyph. Every recovered letter/filler pattern was checked against
   TD3 line-1 structure (document code, `<` or type letter, three-letter issuer), and
   Netherlands 2014 was checked cell by cell against the printed line.
4. **Edges.** Top and bottom are sub-pixel. In each column the ink edge is where darkness crosses
   half-way between paper and ink (ISO 1831 §5.4.3.9's stroke-edge definition), linearly
   interpolated, and the glyph takes the outermost column.
5. **Reference lines.** Baseline and cap line are Theil-Sen fits of the letters' bottoms and tops
   against x, which absorbs residual skew. The cap height is their local difference.

**Positive control: the instrument recovers known geometry.** The same code was run on lines
rendered from the fonts at cap heights of 16-45 px, with Gaussian blur σ 0, 0.8 and 1.5 px, and
JPEG q85:

| Font (true centre) | Measured centre, cap ≥ 20 px, all σ | Measured bottom | Measured height |
| --- | --- | --- | --- |
| Vendored (0.508) | 0.479-0.505 | +0.022 to +0.051 | 0.885-0.960 |
| Barcodesoft `ocrbI` (0.535) | 0.513-0.534 | +0.066 to +0.084 | 0.897-0.919 |
| Barcodesoft `OCRB.TTF` (0.600) | 0.578-0.599 | +0.170 to +0.190 | 0.811-0.853 |

The control carries three lessons. **The centre is recovered to within −0.01 of truth** and
separates the fonts. **The bottom edge is biased upward by 0.01-0.04** on the vendored font's
sharp arm corners, which erode first under blur, so bottoms alone cannot separate vendored from
`ocrbI` at these resolutions. **Blur compresses every font's height to about 0.90**, so the data
cannot resolve height.

**Error.** Edge quantisation is sub-pixel after interpolation and averaged over 7-30 fillers per
line. On the letters, the residual scatter about the fitted baseline is 0.002-0.014 cap
(0.05-0.3 px). The dominant uncertainty is the systematic, shape-dependent bias above: ±0.01 cap
on the centre, ±0.03 on the bottom.

## 3. Verdict

**The vendored filler's height matches ECMA-11; its vertical position does not.** On the standard's
own illustration the filler is centred at 0.548 cap with its bottom 0.050 cap above the baseline.
The vendored glyph is centred at 0.508 with its bottom at 0.010. On real print, **14 of 16
specimens measure a centre above the highest value the vendored control produced (0.505)**. Their
median, 0.528, is about +0.03 cap above the vendored control's typical 0.50. That is **≈0.6 px at
a 20 px cap** (median across specimens +0.62 px; 1.2 px on the 44 px Netherlands scan). Corrected
for the instrument's −0.005 centre bias, real print sits at ≈0.533, which is `ocrbI`'s geometry
(0.535) and slightly below ECMA-11's illustration (0.548).

Real print is not one geometry:

- **Close to the vendored cut:** Türkiye 2019 (0.485) and Saudi Arabia 2021 (0.504) are centred
  on the band. Seychelles 2022 is a full-height filler sitting on the baseline (bottom +0.012,
  height 1.01), centred at 0.516.
- **A short, raised filler:** United Arab Emirates 2011 (bottom +0.138, height 0.80) matches the
  `OCRB.TTF` family (0.17 / 0.82 under the same instrument), not ECMA-11 and not the vendored
  font.
- **The remaining eleven** sit between `ocrbI` and the ECMA illustration.

**What a corrected synthetic `<` needs** (no Rust edited here):

- **Position:** raise the glyph by about 0.03-0.04 cap, which is 27-35 units on the vendored
  font's 885-unit cap height. Its bottom lands at about +0.04 to +0.05 cap and its centre at
  0.53-0.55, matching ECMA-11's illustration and the specimen median.
- **Height:** anything from 0.92 to 1.0 cap is consistent with the evidence. ECMA-11 draws it at
  1.0 and `ocrbI` at 0.92; the specimens cannot decide.
- **Variants:** because real print shows at least two families, a generator that must cover UAE
  2011-style print would also need a short raised variant (bottom ≈ +0.14 to +0.16, height
  ≈ 0.80-0.88).

**Whether it matters (Hypothesized).** The mismatch is real but small: sub-pixel at the
resolutions the corpus is read at, and one-sixth to one-ninth of the misalignment ISO 1831 §6.8.1
permits between adjacent characters. Nothing here shows that it drives the synthetic-to-real gap.
A 0.6 px vertical offset is unlikely to be the main reason a CRNN or chargrid behaves differently
on real print; texture, blur and background, which vary far more between synthetic and real, are
better candidates. The measurement that would settle it is a same-binary A/B: generate the
synthetic corpus with the filler raised by 0.035 cap, arm A against arm B, and read the name
repair outcome. It is recorded here and not run.

## 4. Print-quality limits relevant to chargrid's `DEFAULT_INK_FLOOR = 0.05`

Numbers and section references only (ISO 1831-1980 unless stated):

- **Stroke width**, size I: nominal 0.35 mm, tolerance ±0.08 mm in range X and ±0.15 mm in ranges
  Y and Z (§5.3.1, table 2). ECMA-11 §11.4-11.5 gives the same 0.35 mm (0.31 mm for small letters
  and `#`, `%`, `@`).
- **Print contrast:** PCS80% > 0.60 in range X and > 0.50 in range Y (§5.4.5.5.2). The CVR (the
  ratio of a character's darkest to lightest contrast) is < 1.50 in range X and < 1.75 in range Y
  (§5.4.5.8).
- **Voids:** a void is a point with PCS < d, where d = 0.40 in range X and 0.35 in range Y. Three
  or more consecutive void points on a centreline longer than 2 mm make the character
  non-conforming (§5.4.5.9). The visual method allows voids within a 0.2 mm circle covering less
  than 1/3 of its area (§5.4.4.4).
- **Edges and spots:** the stroke edge is at half-reflectance (§5.4.3.9). An edge irregularity
  may be at most 0.3 mm (§5.4.4.5). Spots are allowed within a 0.2 mm circle covering less than
  1/3 of its area (§5.4.4.6).
- **Skew:** at most 3° per character (§6.4).
- **What ICAO Doc 9303-3 §4.4 adopts from ISO 1831 for the MRZ:** PCS_min ≥ 0.6 in the B900
  band, range-X stroke width (§5.3.1), CVR < 1.50, and void d = 0.4 at the reading surface. Lines
  must be readable in B900 (near infrared) even when security print runs under the zone (§4.5).

**Modelled ink fraction of an ideal filler.** Two arms of about 2.11 mm each, taken from the
ECMA-11 illustration's filler (0.70 cap wide, 1.0 cap tall at size I), give 1.36 mm² at 0.35 mm
stroke. At the range-X minimum of 0.27 mm this falls to 1.07 mm², and at the range-Y/Z minimum
of 0.20 mm to 0.80 mm². In a 2.54 × 2.60 mm cell that is **20.6% / 16.2% / 12.1%**. If the band
is 1.5× taller than the glyph, it is 13.7% / 10.8% / 8.1%. Every case clears the 0.05 floor, by
at least 1.6×. This is a model of a conforming print, not a measurement of `cell_ink` on a real
band, and it leaves out blur, which thins a 0.20 mm stroke first.

## 5. Band geometry: standard, real and synthetic

Added the same day, from the same 16 crops. Units are line-1 cap heights unless stated.

### What the standards fix

| Quantity | Value | Source |
| --- | --- | --- |
| Typeface and pitch | OCR-B size 1, constant stroke width, 2.54 mm fixed pitch (10 per 25.4 mm) | Doc 9303-3 §4.4 |
| Capital / digit height, size I | 2.46 / 2.60 mm (ISO 1831 table 2: 2.40 mm, "indicative" for OCR-B) | ECMA-11 §4.1; ISO 1831 §5.3.1 |
| TD3 and MRV line pitch (reference centre lines) | 6.35 mm; lower line 9.40 mm and upper line 15.75 mm above the bottom edge | Doc 9303-4 §4.2.1.3, figure 3; Doc 9303-7 (same values) |
| TD3 printing zone per line | 4.3 mm tall (upper zone 13.6-17.9 mm, lower 7.25-11.55 mm from the bottom edge) | Doc 9303-4 figure 3 |
| TD3 MRZ | 23.2 ± 1.0 mm tall; first character's left edge 6.0 ± 1.0 mm from the document edge; 114.0 mm reading width | Doc 9303-4 figure 3, §4.2.1.3 |
| TD1 line pitch / printing zone | 4.23 mm / 2.95 mm; first character 5.0 ± 1.0 mm from the edge | Doc 9303-5 figure 6, §4.2.1.3 |
| TD2 | first character 4.0 ± 1.0 mm from the edge; the figure 6 dimensions are not transcribed in the local copy | Doc 9303-6 §4.2.1.3 |
| Margins and clear area | 2.0 mm page margins with no print except background security print; security print is allowed inside the MRZ provided B900 reading works; ERZ 17.0 × 118.0 mm | Doc 9303-4 figure 3 note 3; Doc 9303-3 §4.5 |
| Clear area (generic OCR) | at least 2.5 mm around the printing area | ISO 1831 §6.10 (superseded for MRTDs by the security-print allowance above) |
| Line skew | at most 3° | Doc 9303-3 §4.4; ISO 1831 §6.4 |

Nominal ratios, taking cap = 2.46 mm: TD3 line pitch / cap = **2.58**; character pitch / cap =
**1.033**; TD1 line pitch / cap = **1.72**. Each TD3 printing zone is 4.3 mm tall around a 2.6 mm
glyph, so each line may drift ±0.85 mm, and the pitch actually printed can lie anywhere in
4.65-8.05 mm, which is **1.89-3.27 cap**.

### What real specimens print (TD3, N = 16, measured)

| Quantity | Median | IQR | Range | Nominal |
| --- | ---: | ---: | ---: | ---: |
| Character pitch / cap | **1.024** | 1.004-1.037 | 0.968-1.056 | 1.033 |
| Line pitch (baseline to baseline) / cap | **2.54** | 2.47-2.58 | 1.65-2.99 | 2.58 |
| Inter-line gap (line-1 baseline to line-2 ink top) / cap | 1.48 | 1.42-1.50 | 0.60-1.92 | 1.50 |
| Line-2 glyph height / line-1 glyph height | 1.08 | — | 1.05-1.10 | ≈1.03-1.06 |
| Grid residual (glyph centres about a linear fit) / pitch | 2.1% | — | max 3.2% | 0 |

- **Character pitch** is a linear fit of the 44 line-1 glyph centres against cell index.
- **Line-2 height** is the median non-filler glyph height. Line 2 carries digits, so it is taller.
- **Outliers:** Brazil 2015 prints its lines at **1.65 cap** (it would be about 4.1 mm if the cap is
  nominal), below the 1.89 cap the printing zones permit. The United States 2020 prints at
  **2.99 cap**, inside the permitted range. Nicaragua 2015 prints at 2.08 cap. Thirteen of 16 fall
  within ±10% of the nominal 2.58.
- **Error bars** are about ±0.5 px per baseline fit, which is ±0.03 cap at a 20 px cap. That is
  comfortably tighter than the spread above.
- The absolute scale in mm is unknown for every specimen, so these are ratios only.

### What the generator lays out (derived from code, no build)

`layout.rs` gives every format `MRZ_LINE_HEIGHT` = 50 px and `MRZ_LINE_SPACING` = 5 px, so the
line pitch is 55 px. `mrz_char_rect_for_line` makes the cell width `line.width / mrz_chars`, with
integer division. `render.rs` `draw_mrz_glyphs` sets the `ab_glyph` px scale to 0.8 × 50 = 40.
`ab_glyph` divides that by `ascender − descender`, and `ttf-parser` takes those from `hhea`
because the font's `USE_TYPO_METRICS` bit is clear: 1319 − (−332) = 1651 units. So 1 unit =
0.02423 px, the **cap height is 21.44 px**, the digits are 23.8 px, the baseline sits 32.0 px
below the top of the line rect, and glyph ink occupies rows 8.2-32.2 of the 50-row rect.

| Format | Cell px | Cell / cap | Line pitch / cap | Nominal cell / cap | Nominal pitch / cap |
| --- | ---: | ---: | ---: | ---: | ---: |
| TD3 (1080 / 44) | 24 | **1.119** | 2.565 | 1.033 | 2.58 |
| TD2, MRV-B (908 / 36) | 25 | **1.166** | 2.565 | 1.033 | (figure not transcribed) |
| TD1 (742 / 30) | 24 | **1.119** | **2.565** | 1.033 | **1.72** |
| MRV-A (1052 / 44) | 23 | 1.073 | 2.565 | 1.033 | 2.58 |

The synthetic inter-line gap (line-1 baseline to line-2 ink top) is 31.2 px = 1.46 cap, against
1.48 real.

**Synthetic against real.** Line pitch matches for TD3 (2.565 against 2.54 real and 2.58
nominal). Two things do not:

- **Cells are too wide.** Synthetic TD3 cells are **1.119 cap wide against 1.024 real** (+9%;
  every one of the 16 specimens is below 1.06). TD2 and MRV-B are 13% wide. The glyph's ink is
  centred in a cell 9-13% wider than print, so a synthetic filler run is sparser than a real one,
  and a synthetic line is proportionally longer.
- **TD1 lines are too far apart.** They are spaced at 2.565 cap against ICAO's 1.72 cap (4.23 mm),
  which is **49% too far apart**. This comes from the standard, not from a measurement; no TD1
  specimen was measured here.

### Chargrid consequence (measured on the 16 crops, luma < 110 as in `chargrid::is_ink`)

`cell_ink` averages over rows `top..bottom` of the band that `ocrs` returned
(`lib.rs`: `matched.top`, `matched.bottom`). `tools/ocrb_metrics.py` assumes that band is exactly
the union of glyph ink, about 1.13 cap. How tall `ocrs`'s line box really is was **not measured**,
because that needs cargo. So the filler's share of a pitch-wide cell is bracketed three ways:

| Band | Filler cell ink, median (range) | Letter cell ink, median | Fillers below 0.05 |
| --- | --- | ---: | --- |
| Cap line to baseline (1.0 cap) | **0.179** (0.134-0.241) | — | 0 of 16 specimens |
| Same ± 0.25 cap (1.5 cap) | 0.119 (0.085-0.161) | — | 0 |
| One full line pitch (≈2.5 cap) | **0.070** (0.057-0.110) | 0.109 | 1 of 16 (Japan 2025: 4% of its fillers; minimum 0.049) |

- **Where the floor stands.** `DEFAULT_INK_FLOOR = 0.05` clears a real filler by **3.6×** if the
  band is the ink extent, but only by **1.4×** (median) if the band is a full line pitch. At that
  height it is already touching the lowest real filler.
- **Why the band height matters so much.** Going from the ink-extent band to the pitch band divides
  the fraction by 2.5.
- **Filler against letter.** A filler carries 0.67× a letter's cell ink (range 0.60-0.72). That is a
  real contrast, but not a gap.
- **What the tool's "ideal" means.** The idealised shares `tools/ocrb_metrics.py` prints (16.7-22.5%
  across the four fonts) match the real ink-extent row, so they are a ceiling in exactly the sense
  the tool says.
- **What would settle it.** The number that decides the floor's margin is `ocrs`'s line-box
  height over ink height. Log it in one release run (a `--dump-ocr` field) before treating 0.05 as
  safe.

### ADR-0015's assumptions against the measurements

- **"Two or three long horizontal bars of near-equal height": holds only loosely.** Line 2 prints
  **5-10% taller** than line 1 (digits, measured). A near-equal test needs at least ±10%.
- **"Vertically adjacent at a known spacing": contradicted as a tight prior.**
  - The spacing ranges from 1.65 to 2.99 cap on 16 conforming-read specimens.
  - Brazil 2015 falls below even the printing-zone limit.
  - The standard itself allows 1.89-3.27 cap.
  - A locator should use a band of spacings, not 6.35 mm, and must not assume that 2-line spacing
    equals 3-line spacing: TD1 is nominally 1.72 cap.
- **"A fixed pitch, equal-width cells": holds.** The grid residual is ≤ 3.2% of pitch.
- **"A known aspect ratio per format": holds only to ±5% on the line** (character pitch 0.968-1.056
  cap). It does not hold on a two- or three-line band, whose height follows the spacing spread
  above.
- **"Periodic low-ink gaps where filler runs sit": overstated.** Filler cells carry 0.6-0.7× a
  letter's ink, so they are low-ink cells, not gaps. Whether they read as a gap depends on the band
  height, as in the table above.

The ADR's "a correct fit makes filler runs land on cell boundaries" is at odds with fillers
being centred in their cells. Either wording is a guess until `mrz-locate` exists; flagged for the
architect, not changed here.

## Invocation

Throwaway Python (numpy, scipy, Pillow, PyMuPDF, fontTools) in
`C:\Users\rusmirs\AppData\Local\Temp\claude\ocrb-analyst\`, not in the repo:

- `ecma_meas.py`: extracts the page-29 image (xref 144) of the local ECMA-11 PDF, rotates it
  upright, deskews by −0.70° and takes bboxes at thresholds 96/128/160.
- `fontcmp.py`: outline bounds in cap units for the four fonts.
- `control.py`: renders the ICAO example line with each font at caps of 16-45 px, blur
  σ ∈ {0, 0.8, 1.5}, JPEG q85, then measures.
- `run_specs.py` (using `findmrz.py` and `measure.py`): the 20 candidates, each extracted with
  `git show origin/samples-data:samples/passports/<file>` and verified against the manifest's
  sha256.
- `band.py`: the §5 band geometry and the `cell_ink` brackets, on the same 16 crops. Line 2 is
  segmented the same way, and ink is `luma < 110`.

## What this does not claim

- **Nothing about accuracy.** No reader was run, and no synthetic corpus was generated with a
  corrected glyph. The "whether it matters" paragraph is a hypothesis.
- **Not the normative ECMA-11 geometry.** The normative shape is drawing 79, which is not
  reproduced in the standard. The illustration is a reproduction of it and is the best evidence
  the document itself carries.
- **Not "physical print".** Published specimen images may be scans of printed books or issuer
  artwork set in some OCR-B font. What they show is what a reader sees on specimen images, which
  is the corpus this project is scored on.
- **Not the filler's height** (§2 control: blur compresses it), **and not TD1 or TD2** (only TD3
  line 1 was measured).

## Rejected on the way

- **Filler bottom as the discriminator.** On the vendored font's sharp arm corners the instrument
  reads bottoms 0.01-0.04 high. That bias is as large as the difference being measured, so the
  centre is reported instead.
- **A column-run shape heuristic for "is this `<`".** It misclassified 3 of 28 letters on
  Netherlands 2014 (25 letters / 19 fillers against the printed 28 / 16). It was replaced by the
  shape-cluster medoid.
- **Four candidates excluded after inspection.** The inclusion rule, set after inspection and
  stated here for that reason, is a cap height of at least 15 px and a letter-baseline residual
  SD of at most 0.015 cap.
  - **Bulgaria 2010:** 41 glyphs segmented instead of 44.
  - **Finland 2017:** visibly blurred; residual SD 0.021; centre 0.444 ± 0.018.
  - **Nicaragua 2001:** residual SD 0.033; top 0.81, implausible.
  - **India 2023:** background texture broke the filler clustering; residual SD 0.19.

  Including Finland and Nicaragua 2001 would add two low centres (0.444, 0.408) whose own quality
  checks fail.
