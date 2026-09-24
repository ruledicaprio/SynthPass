# The OCR-B standards against our priors: TD1 spacing and the cell width are confirmed wrong, the filler offset is real but ≈0.025 cap, and the ink floor is thin only on a pitch-tall band

**Date:** 2026-09-24 · **MAIN:** `5adde9f` · **DATA:** n/a (no corpus read) · **Evidence:** Observed (ISO 1073-2 §13 illustration measured on a 600 dpi render; ISO 1831, ECMA-11 and ECMA-18 clauses read on page renders; six OCR-B font files measured by outline) plus Modelled (ink fractions) · **Status:** current

Context: Doc 9303-3 §4.4 and §4.11 delegate the MRZ's glyphs to ISO 1073-2 and its print to
ISO 1831. Those standards are now distilled, with per-fact citations, in
[`../ocrb/README.md`](../ocrb/README.md). This note sets their numbers against the priors that
SynthPass's Tier-1 work currently carries:

- the synthetic generator's layout;
- [`chargrid`](../../crates/synthpass-ocr/src/chargrid.rs)'s `DEFAULT_INK_FLOOR`;
- [ADR-0015](../decisions/ADR-0015-geometric-mrz-band-location.md)'s band prior;
- [`mrz-geometric-elimination.md`](../research/mrz-geometric-elimination.md)'s height signal.

Most of these priors were measured on real print in
[`ocrb-filler-geometry-2026-09-23.md`](ocrb-filler-geometry-2026-09-23.md). This note adds the
normative side.

No accuracy denominator applies. Nothing was read by a provider, so neither the scored count nor
the whole-corpus count moves.

Unit: the cap height of the line's own capitals, nominally **2.46 mm** at size I (ISO 1073-2
Table 1).

## Verdicts

| Prior | Our value | What the standard says | Verdict |
| --- | --- | --- | --- |
| The filler `<` sits low in the vendored font | vendored centre 0.508; proposed raise +0.03-0.04 | ISO 1073-2 §13 illustration: centre **0.532**, bottom +0.047, height 0.970 [M]. Real-print median 0.528; `ocrbI` 0.534 | **Right direction, slightly large.** Raise by **≈0.025 cap** (≈22 font units), not 0.035. ECMA-11's illustration (0.548) overstates the offset |
| Synthetic cells are 9-13% too wide | TD3 1.119 cap; TD2/MRV-B 1.166 | Pitch is fixed at 2.54 mm = **1.033 cap** (Doc 9303-3 §4.4). ISO 1831's floor is 2.30 mm = 0.935 cap; it sets no ceiling | **Confirmed wrong.** TD3 is 8.3% and TD2 12.9% above nominal. Real print sits at 1.024 |
| TD1 lines are 49% too far apart | synthetic 2.565 cap | Doc 9303-5 Figure 6: **4.23 mm = 1.72 cap**, which is 6 lines per inch, the **densest packing ISO 1831 allows** (§6.13: ≥ 4.20 mm; Annex D.4) | **Confirmed wrong, and tighter than stated.** A conforming TD1 can only print at 4.20-4.52 mm (1.71-1.84 cap) |
| Digit/letter height gap | 4.6% (vendored font); 5.7% cited from ECMA-11 | ISO 1073-2 Table 1, **constant-strokewidth** (the style ICAO requires): digits 2.66 mm, capitals 2.46 mm, a gap of **8.1%**. ECMA-11's 5.7% comes from its **letterpress** table | **Our prior understates the gap.** The ISO illustration measures 1.082 on flat digits [M] |
| `DEFAULT_INK_FLOOR = 0.05` has margin | 3.6× the ideal filler on an ink-extent band; 1.4× on a line-pitch band (measured) | Model below | **Safe on any band up to the printing zone. Thin (1.17×) on a pitch-tall TD3 band at the thinnest range-X stroke. Fails at range Y/Z strokes, which ICAO does not permit** |
| ADR-0015: "near-equal bar heights" | — | Line 2 carries digits, which are 8.1% (flat) to 13% (round `8`) taller than capitals | **Holds only as a ±15% band.** The 2026-09-23 measurement (5-10%) falls inside it |
| ADR-0015: "vertically adjacent at a known spacing" | one spacing | TD3: nominal 6.35 mm, but its printing zones allow **4.65-8.05 mm** (1.89-3.27 cap). TD1: **4.20-4.52 mm** (1.71-1.84 cap). ISO's absolute floor is 4.20 mm | **Wrong for TD3**, where the allowed band spans 1.7×. **Right for TD1**, which is nearly fixed. Two formats need two priors |
| ADR-0015: "filler runs land on cell boundaries" | — | Glyphs are placed by a reference line within the pitch (ISO 1073-2 §11.4.1). Adjacent characters are separated by a clear gap of at least one stroke width (ISO 1831 §6.7.1). The filler is centred in its cell | **Wrong wording.** Fillers land at cell **centres**. The cell **boundaries** are the clear gaps between every pair of glyphs |
| Brazil 2015 prints TD3 lines 1.65 cap apart | measured | That is below ISO 1831's minimum line spacing (4.20 mm = 1.71 cap), if its cap height is nominal | **Non-conforming print** (inferred). A locator must not reject it, so the locator's floor cannot be the ISO value |

## The ink-floor model (modelled)

**Inputs.** The ISO illustration's filler has **1.22 mm²** of ink at size I, at its drawn stroke of
0.34-0.36 mm [M]. That area is scaled linearly with stroke width to the ISO 1831 Table 2 limits.
The cell is one pitch (2.54 mm) wide. The band height is the variable.

| Band height (what `ocrs` might return as the line box) | Nominal 0.35 mm | Range X minimum, 0.27 mm (ICAO's limit) | Range Y/Z minimum, 0.20 mm |
| --- | ---: | ---: | ---: |
| Cap height, 2.46 mm | 19.5% (3.9×) | 15.1% (3.0×) | 11.2% (2.2×) |
| Ink extent = digit height, 2.66 mm | 18.1% (3.6×) | 13.9% (2.8×) | 10.3% (2.1×) |
| TD1 printing zone, 2.95 mm | 16.3% (3.3×) | 12.6% (2.5×) | 9.3% (1.9×) |
| TD1 line pitch 4.23 mm / TD3 printing zone 4.3 mm | 11.3% (2.3×) | 8.7% (1.7×) | 6.4% (1.3×) |
| TD3 line pitch, 6.35 mm | 7.6% (1.5×) | **5.8% (1.17×)** | **4.3% (0.86×)** |

(× = multiple of the 0.05 floor.)

- **Consistency.** This agrees with the 2026-09-23 real-print measurement (filler median 0.070 at
  a full line pitch) and with that note's own model (20.6/16.2/12.1% at the cap band).
- **What ISO adds that the model leaves out:**
  - **Voids.** A conforming stroke may carry up to two consecutive sub-threshold samples at a
    0.1 mm step; a third makes the character non-conforming (ISO 1831 §5.4.5.9). That can lose
    about 0.2 mm of an arm.
  - **The ink threshold.** `is_ink` is a fixed luma cut (< 110). ISO puts the stroke edge at half
    the character's own contrast (§5.4.5.10). A light but conforming stroke (PCS 0.6 on paper at
    sRGB 230) images near sRGB 152, which a fixed cut at 110 does not count as ink.
- **Conclusion.** The floor's safety is decided by **the band height `ocrs` returns**, not by the
  font or the standard.
  - Up to the printing zone, the floor clears a conforming filler by ≥ 1.7×.
  - At a full TD3 pitch it clears by only 1.17×, and only at nominal contrast. A light print
    fails it.
  - The 2026-09-23 note already asked for the measurement that settles this: log
    `matched.bottom - matched.top` over ink height in one `--dump-ocr` run. It is still
    unmeasured.

## What moved

Nothing. No provider was run, and both accuracy denominators are untouched.

## Invocations

All scratch work stayed out of the repository, in the session scratchpad's `ocrb-analyst/`.

- **Renders.** PyMuPDF `get_pixmap` of the local ISO 1073-2, ISO 1831, ECMA-11, ECMA-15, ECMA-18,
  ECMA-19, ECMA-21 and ISO 2033 PDFs, at 110-300 dpi. ISO 1073-2 PDF page 23 was also rendered at
  600 dpi.
- **Illustration measurement** (`meas_iso2.py`, `meas_iso3.py`). Connected components with
  `scipy.ndimage.label`, at ink thresholds 96, 128 and 160.
  - Cap height = the median height of `E F H K L M`: 232-235 px.
  - Pitch = a linear fit of the `A`-`M` row's glyph centres: 239.5 px, or 1.032 cap.
  - `<` = the component at x 1826-1986, y 3776-4003.
  - Baseline = the median bottom of the `?` dot, the `!` dot, `%` and `#`: 4014.
- **Stroke** (`stroke.py`). Horizontal ink runs across the `A`-`M` row at y = 1700: runs of
  27-43 px, with the stems at 31-34 px.
- **Fonts.** `fontTools` `BoundsPen` over `OCR-B.ttf/.otf`, `OCRB.TTF` and `ocrbI/III/IV.ttf`.
  The calling session also ran `python tools/ocrb_metrics.py --font <f> --json`; `ocrbIII` fails
  because it has no `A`.
- **Ink model.** `python -c` arithmetic, reproduced in the table above.

## What this does not claim

- **Nothing about accuracy.** It does not claim that any correction would move a Tier-1 hit. The
  synthetic-to-real gap has not been shown to come from any of these geometry errors.
- **Not the normative `<` geometry.** That is drawing 79 C, which no document here reproduces.
  The ISO illustration is a 4:1 reproduction from a scan of about 157 dpi, so each edge is good
  to about ±0.02 cap.
- **Not a value-by-value comparison of ECMA-15 with ISO 1831.** ECMA-15 was used only for
  lineage.
- **Not a reading of ISO 1831 Annex C** (the computer-aided method) or of the ISO 1073-2 annex
  coordinate tables. Neither contains anything for the MRZ glyphs.

## Rejected on the way

- **ECMA-11's §13 illustration as the filler reference.** It draws `<` taller and higher (centre
  0.548) than ISO 1073-2's illustration of the same design (0.532). Two reproductions of one
  design disagree by 0.016 cap. The ISO illustration is from the document ICAO cites, and it
  agrees with real print.
- **ISO 1831 numbers from the local OCR text layer.** Table 2's heights, Table 4 and Table 5 are
  garbled there. Every value above was read from the rendered page instead.
- **ISO 1831 Table 2's "2.40 mm height" as a capital or digit height.** It is the centreline
  height of EIGHT (ISO 1073-2 §3.6). The 2026-09-23 note's §5 table lists it as "Capital / digit
  height ... 2.40 mm", which mixes the two quantities.

## Also found

- **Doc 9303 transcription defects in this repository (not fixed here).** Either would mislead
  anyone who builds a layout prior from the tables alone.
  - [Part 4](../docs9303/Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md)'s MRZ dimension table,
    below Figure 3, labels 7.25 mm the "upper reference line position" and 6.35 mm the "lower".
    On the figure itself, 6.35 mm is the *distance between* the two reference centre lines. The
    lines sit at 9.40 and 15.75 mm, and 7.25 mm is the bottom of the lower printing zone.
  - [Part 5](../docs9303/Doc_9303_Part5_Specs_for_TD1_MROTDs.md#421-data-position-data-elements-and-print-position-in-the-mrz)'s
    table gives TD1 line spacing as **4.0 mm**. The figure shows **4.23 mm**, and 4.0 mm would
    violate ISO 1831's 4.20 mm floor.
- **A mis-cited clause.** [`ocrb-filler-geometry-2026-09-23.md`](ocrb-filler-geometry-2026-09-23.md)
  §4 heads its list "What ICAO Doc 9303-3 §4.4 adopts". The adoption is in §4.11; §4.4 fixes only
  the font, the size and the pitch.
- **The print-quality contract is near-infrared only.** Doc 9303 sets it in the **B900 band**. It
  sets no visible-band contrast minimum, and nothing bounds the visible-light background under the
  zone. Every image in the corpus is visible-light, so no standard bounds the background clutter
  `ocrs` meets.
