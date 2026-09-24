# OCR-B font files against the standard's nominal values

Six local OCR-B font files were measured on 2026-09-24 in two ways:

- by outline bounds with `fontTools`;
- with [`tools/ocrb_metrics.py`](../../tools/ocrb_metrics.py) `--font <f> --json`, run by the
  calling session. `ocrbIII.ttf` fails there, with "no glyph for 'A'".

Units are the font's own cap height: the median `yMax` of the flat capitals `EFHIKLMNTXZ`, over
a baseline at 0. The standard's values come from
[`glyph-dimensions.md`](glyph-dimensions.md) and [`filler-and-symbols.md`](filler-and-symbols.md).

**Licensing.**

- `OCR-B.ttf` and `OCR-B.otf` are Raisty 2019, SIL OFL. The vendored
  [`crates/synthpass-gen/fonts/ocr-b.ttf`](../../crates/synthpass-gen/fonts/ocr-b.ttf) is the same
  design.
- `OCRB.TTF`, `ocrbI.ttf`, `ocrbIII.ttf` and `ocrbIV.ttf` are Barcodesoft, "all rights reserved".
  They are measured and cited here only. They must never be vendored, and their outlines must
  never be copied.

| Quantity | Standard (constant-strokewidth, size I) | Raisty `OCR-B` (vendored) | Barcodesoft `OCRB` | Barcodesoft `ocrbI` / `ocrbIV` | Barcodesoft `ocrbIII` |
| --- | --- | --- | --- | --- | --- |
| Glyphs (MRZ set) | 37 | 339 (37) | 221 (37) | 125 / 122 (37) | 27 (18: sub-set 1) |
| Flat digit (`1 4 7`) / cap | **1.081** (ISO Table 1; 1.082 on the ISO illustration [M]) | 1.089 | 1.081 | 1.077 | 1.087 |
| `0` / `8` / cap | 1.116 / 1.134 on the ISO illustration [M] | 1.110 / 1.110 | 1.081 / 1.081 (no overshoot) | 1.093 / 1.091 | 1.102 / 1.101 |
| `<` bottom / top / height | +0.047 / 1.017 / 0.970 (ISO illustration [M]) | +0.010 / 1.006 / 0.995 | +0.162 / 1.037 / 0.875 | +0.072 / 0.995 / 0.923 | +0.074 / 1.004 / 0.930 |
| **`<` centre** | **0.532** (ISO [M]); 0.548 (ECMA [P]) | 0.508 | 0.599 | **0.534** | 0.539 |
| `<` width / cap | 0.684 (ISO [M]) | 0.714 | 0.683 | 0.688 | 0.563 |
| Advance (pitch) / cap | **1.033** (2.54 / 2.46) | 0.999; `5` alone 0.88 | 0.980, monospaced | `ocrbI` 0.977, monospaced; `ocrbIV` 5 advance groups, 0.93-1.06 | 0.740 |
| Stem stroke / cap | **0.142** (0.35 / 2.46); range X 0.110-0.175 | 0.138 (0.34 mm) | 0.138 (0.34 mm) | 0.130 (0.32 mm) | — |
| Stroke ends | rounded (constant-strokewidth, ECMA-11 §4.2) | square-cut | — | rounded (per the 2026-09-23 note) | — |

Stems were derived from `H`'s scanline ink runs in the `ocrb_metrics` JSON. The scale in mm
assumes a cap of 2.46 mm.

## Readings

- **Closest to the standard: Barcodesoft `ocrbI`.**
  - Its filler centre (0.534) is ISO's illustration (0.532) and the real-specimen median (0.528).
  - Its stroke ends are rounded, as the constant-strokewidth style requires, and it is monospaced.
  - Its flat-digit ratio of 1.077 is 0.004 from ISO's 1.081.
  - It is lighter than nominal (0.32 mm stroke) and its filler is short (0.92). The short filler
    may be real or may be blur in the specimens, which the 2026-09-23 instrument could not
    resolve.
- **`ocrbIV` has `ocrbI`'s outlines with different spacing.** It has 5 advance groups, so it is a
  proportional variant, not size IV. Size IV is a 1.5× centreline scale with a thicker stroke
  (ISO 1073-2 §3.5, §11.6); in em units that would look identical, so the name cannot be tested
  from outlines alone.
- **`ocrbIII` is ISO sub-set 1**, the characters for which ISO provides size-III drawings (§11.5
  lists the digits, `< + >`, the long vertical mark, `C E N S T X Z` and GROUP ERASE). It is not
  an MRZ font: it has no `A`.
  - Its narrow advance (0.74 cap) and narrow `<` (0.563) fit size III's horizontal scale factor
    of 1.086 against a vertical factor of 1.333. That makes the glyphs relatively narrower, since
    1.086 / 1.333 = 0.81 and 0.684 × 0.81 = 0.557.
  - **This is evidence the Barcodesoft family follows ISO's size tables.**
- **Barcodesoft `OCRB.TTF` is a different design.**
  - Its digits have no overshoot (every digit sits at 1.08).
  - Its filler is short and raised (bottom +0.16). That matches the United Arab Emirates 2011 print
    ([`ocrb-filler-geometry-2026-09-23.md`](../benchmarks/ocrb-filler-geometry-2026-09-23.md) §3),
    not ISO.
- **The vendored Raisty font** matches the standard on stroke width (0.34 mm) and on the digit/cap
  gap (1.089; slightly large). It misses it on four counts:
  - **Filler position:** its centre is about 0.025 cap too low. The bottom is only +0.010, where
    ISO has +0.047.
  - **Stroke ends:** square-cut, not rounded.
  - **Advance of `5`:** narrow (782 units against 884/886).
  - **Advance generally:** about 1.0 cap. That is a font property and harmless, because
    `synthpass-gen` places each glyph in its own cell.

## What it means

- **For the synthetic generator**
  ([`crates/synthpass-gen/src/render.rs`](../../crates/synthpass-gen/src/render.rs)):
  - The glyph shapes are close enough on digits, letters and stroke.
  - The two corrections the standard supports are:
    - raise `<` by about 0.025-0.03 cap, about 22-27 units on the vendored font's 885-unit cap;
    - fix the **cell width**, which is a layout bug, not a font bug. The generator's 1.119 cap
      cell is 8% wider than ISO's pitch (1.033) and 9% wider than real print (1.024).
  - Rounded stroke ends matter far less. ISO 1831's squared-off maximum COL accepts both.
- **For chargrid priors** ([`crates/synthpass-ocr/src/chargrid.rs`](../../crates/synthpass-ocr/src/chargrid.rs)):
  - Every font puts an ideal filler at 17-23% of a cell (`ocrb_metrics` `filler_cell_ink`:
    Raisty 0.185, `OCRB` 0.225, `ocrbI` 0.181, `ocrbIV` 0.167).
  - ISO's own illustration gives 19.6% [M] in a pitch × cap cell.
  - So the font a template is built from moves the ideal filler ink by about ±15%. It does not
    move the ink floor's margin enough to matter, because the band height does
    ([`../benchmarks/ocrb-standards-vs-priors-2026-09-24.md`](../benchmarks/ocrb-standards-vs-priors-2026-09-24.md)).
  - For an [ADR-0014](../decisions/ADR-0014-per-cell-ocrb-classification.md) template bank, a
    Raisty-plus-`ocrbI` pair spans the two filler positions real print shows. The `ocrbI` half
    can be measured locally but cannot ship; a template has to be regenerated from an openly
    licensed font adjusted to the measured geometry.
