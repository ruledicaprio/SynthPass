# Glyph atlas, first sweep: `P` reads as itself on 100 / 100 clean renders, the filler after it on 4 / 100, with `S` the winner in 96

**Date:** 2026-09-30 · **MAIN:** `5860cb9` (the run's commit; its header says `dirty: true` for three untracked files, with no tracked file modified) · **DATA:** none (generated TD3 renders, seeds 0-99; the vendored font `crates/synthpass-gen/fonts/ocr-b.ttf`, sha256 `367d876cca94`) · **Evidence:** Observed on synthetic renders, vendored font, oracle crop (one `glyph_atlas` run, the 118 aggregate rows of its `summary.json`) plus Derived (counts over those rows, and the margin's scale from the tool's code) plus Hypothesized (the readings marked so) · **Status:** current

**2026-09-30.** This is the dated finding that
[#433](https://github.com/ruledicaprio/SynthPass/issues/433) asks for. It covers the first full
sweep of the [`glyph_atlas`](../../crates/synthpass-ocr/examples/glyph_atlas.rs) example (merged
in [#603](https://github.com/ruledicaprio/SynthPass/pull/603)). For 100 seeds the tool renders a
clean TD3 page with the vendored OCR-B font. It degrades that page along one axis at a time
(resolution, JPEG, rotation, blur, noise, contrast), and for the first two cells of MRZ line one
(`P`, then the filler `<`) it reads the detection map and the raw CTC matrix of the *oracle* line
crop. **Evidence label: Observed on synthetic renders, vendored font, oracle crop.** The atlas
bypasses imageprep, the retry loop and the grid fit. The tool records no decoded text, so nothing
here says what the pipeline's decode returns, and nothing here is about real documents. No code,
ADR, baseline or headline changes with this note, and
[ADR-0021](../decisions/ADR-0021-fixed-grid-mrz-strips.md) stays Proposed.

**What "read as itself" means here.** It is the tool's `accuracy`. The CTC distribution is averaged
over the cell's timestep range. A render counts as correct when that averaged distribution's winner
equals the truth glyph, with blank excluded from the winner. The winner is taken over every label
the recognition model has, not over the MRZ charset, which is why `?`, `"`, `e` and `r` can appear
as wrong winners. `p_true_peak` is the largest per-timestep probability of the truth glyph inside
the range. `margin` is the truth glyph's mean probability over the range minus the best other
glyph's mean, so it is negative when another glyph beat the truth. Probabilities are summed over
labels that decode to the same character (labels 44 and 49 are both `E`). Ties go to the lower
label.

## The answer

Every rate below is over n = 100 seeds per step, with no missing recognition (`rec_missing` = 0
on all 118 rows). The six axes have 59 steps between them. Resolution 22, rotation 0, blur 0,
noise 0 and contrast 1 are the same clean render, evaluated once per seed and reused, so there are
55 distinct steps. JPEG 100 is not one of them: it is a real encode and decode, and its header
marks no identity step.

- **`P` (cell 0) is read as itself in 100 / 100 seeds (Wilson 95% 0.963-1.000) at 50 of the 55
  distinct steps** (Observed). The five exceptions are rotation 3, 4 and 5 degrees (30, 33 and
  35 / 100) and blur sigma 5 and 6 (43 and 38 / 100). At 5 and 4 px per cell the winner is still
  `P` in 100 / 100, but `p_true_peak` collapses to a median of 0.081 and 0.013, and the median
  margin to +0.013 and +0.002. At that size `P` wins by almost nothing.
- **The filler `<` (cell 1) is read as itself in 4 / 100 clean renders (Wilson 95%
  0.016-0.098)** (Observed). `S` is the winner in the other 96. The median `p_true_peak` is 0.067
  (p10-p90 0.005-0.331), and the median margin is −0.102.
- **Under degradation, the filler's rate is not monotonic** (Observed). At its best it is read
  as itself in 75 / 100 seeds (JPEG quality 20, Wilson 95% 0.657-0.825). It is read as itself in
  0 / 100 at 16 of the 55 distinct steps. Some steps read it more often than the clean render:
  JPEG 20 at 0.75, contrast 0.15 at 0.55, resolution 18 at 0.34, and rotation 0.5 at 0.35.
  Others read it less or no more often: resolution 15 at 0.04, rotation 0.25 at 0.01, and JPEG 30
  at 0.04.
- **The wrong winner is `S` at most steps. It changes character under heavy blur and at small
  cells** (Observed). Under blur it goes to `C` at sigma 3 (90 / 100), then `E` at sigma 4
  (76), `E`, `e` and `O` at sigma 5, and `U`, `O` and `E` at sigma 6. At small cells it goes to
  `R` at 8 px (39), `C`, `S` and `R` at 5 px, and `E` at 4 px (72). From rotation 1.5 degrees
  on, the filler's wrong winners spread over 8 to 30 different glyphs.
- **Detection did not gate recognition in this sweep** (Observed). A detected word box covered
  the cell centre in 100% of renders at every step, except on resolution: `P` at 9, 7, 6 and 4 px
  (0.93, 0.95, 0.99, 0.68), and `<` at 4 px (0.82). Recognition reads the oracle crop, not a
  detected box, so none of the recognition rates above depends on detection.

## The clean render (identity step)

Resolution 22 px per cell = rotation 0 = blur 0 = noise 0 = contrast 1: the same 100 renders,
reported once here. The five identity rows, one per axis, are identical in `summary.json`
(Observed).

| Cell | Truth | Read as itself, of 100 [Wilson 95%] | `p_true_peak` median (p10–p90) | Median margin | Winner when wrong | Median detection fraction ≥ 0.2 | Word box covers centre |
| ---: | --- | --- | --- | ---: | --- | ---: | ---: |
| 0 | `P` | 100 [0.963, 1.000] | 0.999 (0.997–1.000) | +0.146 | — | 0.255 | 1.00 |
| 1 | `<` | 4 [0.016, 0.098] | 0.067 (0.005–0.331) | −0.102 | `S` 96 | 0.360 | 1.00 |

"Median detection fraction ≥ 0.2" is `median_det_frac_above_threshold`: the share of the cell
box's pixels at or above the detection threshold of 0.2, as a median over seeds. Over the whole
sweep it ranges 0.020-0.456 for `P` and 0.140-0.460 for `<` (Observed).

**The margin's scale** (Derived, from `cell_stats` in the tool). The margin is taken on mean
probabilities over the cell's timesteps, while `p_true_peak` is one timestep's maximum. The other
timesteps' mass dilutes the mean (on blank, in a CTC model: Hypothesized, since `blank_mean` is
not in `summary.json`). So a `P` read at a peak of 0.999 carries a median margin of only +0.146
on the clean render. A margin near +0.15 to +0.25 is what a confident read looks like in this
instrument. It is not a weak one.

## By axis

Each table gives, per cell: the seeds read as itself out of 100 with the Wilson 95% interval;
`p_true_peak` as median (p10-p90); the median margin; `winner_when_wrong` (with the top three
named when there are more than four, and the rest summed); and the word-box cover rate. The
identity step of each axis is the clean-render table above.

### Resolution (px per cell; native 22; page downscaled with `FilterType::Triangle`)

| Step | `P`: of 100 [Wilson 95%] | `P`: `p_true_peak` median (p10–p90) | `P`: margin | `P`: winner when wrong | `<`: of 100 [Wilson 95%] | `<`: `p_true_peak` median (p10–p90) | `<`: margin | `<`: winner when wrong | Word box covers centre, `P` / `<` |
| ---: | --- | --- | ---: | --- | --- | --- | ---: | --- | --- |
| 22 | clean render, above | | | | clean render, above | | | | |
| 18 | 100 [0.963, 1.000] | 0.999 (0.999–1.000) | +0.150 | — | 34 [0.255, 0.437] | 0.156 (0.011–0.805) | −0.058 | `S` 66 | 1.00 / 1.00 |
| 15 | 100 [0.963, 1.000] | 1.000 (0.999–1.000) | +0.221 | — | 4 [0.016, 0.098] | 0.029 (0.003–0.225) | −0.122 | `S` 96 | 1.00 / 1.00 |
| 13 | 100 [0.963, 1.000] | 1.000 (0.999–1.000) | +0.222 | — | 6 [0.028, 0.125] | 0.029 (0.004–0.224) | −0.084 | `S` 94 | 1.00 / 1.00 |
| 11 | 100 [0.963, 1.000] | 0.999 (0.999–1.000) | +0.155 | — | 0 [0.000, 0.037] | 0.007 (0.001–0.082) | −0.121 | `S` 100 | 1.00 / 1.00 |
| 10 | 100 [0.963, 1.000] | 0.999 (0.998–1.000) | +0.164 | — | 1 [0.002, 0.054] | 0.011 (0.001–0.112) | −0.124 | `S` 99 | 1.00 / 1.00 |
| 9 | 100 [0.963, 1.000] | 1.000 (0.999–1.000) | +0.245 | — | 13 [0.078, 0.210] | 0.025 (0.002–0.519) | −0.054 | `S` 72, `K` 11, `R` 4 | 0.93 / 1.00 |
| 8 | 100 [0.963, 1.000] | 0.769 (0.413–0.927) | +0.128 | — | 15 [0.093, 0.233] | 0.046 (0.003–0.362) | −0.043 | `R` 39, `S` 30, `K` 13, `C` 3 | 1.00 / 1.00 |
| 7 | 100 [0.963, 1.000] | 0.998 (0.995–0.999) | +0.151 | — | 15 [0.093, 0.233] | 0.035 (0.003–0.344) | −0.052 | `S` 55, `C` 24, `R` 4, `E` 2 | 0.95 / 1.00 |
| 6 | 100 [0.963, 1.000] | 1.000 (0.999–1.000) | +0.193 | — | 0 [0.000, 0.037] | 0.001 (0.000–0.010) | −0.081 | `S` 66, `C` 34 | 0.99 / 1.00 |
| 5 | 100 [0.963, 1.000] | 0.081 (0.034–0.178) | +0.013 | — | 0 [0.000, 0.037] | 0.000 (0.000–0.006) | −0.065 | `C` 37, `S` 36, `R` 25; 2 over 2 others | 1.00 / 1.00 |
| 4 | 100 [0.963, 1.000] | 0.013 (0.005–0.026) | +0.002 | — | 0 [0.000, 0.037] | 0.000 (0.000–0.000) | −0.095 | `E` 72, `C` 23, `K` 4, `U` 1 | 0.68 / 0.82 |

At 8 px, `P` dips: `p_true_peak` median 0.769 (p10 0.413). This is between 9 and 7 px, where it
is 1.000 and 0.998. It is the only `P` dip on this axis above 5 px, and every seed still reads
`P` (Observed).

### JPEG (quality; a real `image` encode and decode; no identity step)

| Step | `P`: of 100 [Wilson 95%] | `P`: `p_true_peak` median (p10–p90) | `P`: margin | `P`: winner when wrong | `<`: of 100 [Wilson 95%] | `<`: `p_true_peak` median (p10–p90) | `<`: margin | `<`: winner when wrong | Word box covers centre, `P` / `<` |
| ---: | --- | --- | ---: | --- | --- | --- | ---: | --- | --- |
| 100 | 100 [0.963, 1.000] | 0.999 (0.997–1.000) | +0.146 | — | 5 [0.022, 0.112] | 0.076 (0.006–0.360) | −0.097 | `S` 95 | 1.00 / 1.00 |
| 90 | 100 [0.963, 1.000] | 0.999 (0.997–1.000) | +0.146 | — | 12 [0.070, 0.198] | 0.117 (0.008–0.464) | −0.091 | `S` 88 | 1.00 / 1.00 |
| 75 | 100 [0.963, 1.000] | 0.999 (0.997–1.000) | +0.147 | — | 4 [0.016, 0.098] | 0.084 (0.010–0.311) | −0.102 | `S` 96 | 1.00 / 1.00 |
| 60 | 100 [0.963, 1.000] | 0.999 (0.997–1.000) | +0.147 | — | 10 [0.055, 0.174] | 0.132 (0.021–0.441) | −0.088 | `S` 90 | 1.00 / 1.00 |
| 50 | 100 [0.963, 1.000] | 0.999 (0.997–0.999) | +0.148 | — | 19 [0.125, 0.278] | 0.125 (0.016–0.577) | −0.088 | `S` 81 | 1.00 / 1.00 |
| 40 | 100 [0.963, 1.000] | 0.999 (0.996–1.000) | +0.146 | — | 29 [0.210, 0.385] | 0.216 (0.040–0.638) | −0.060 | `S` 71 | 1.00 / 1.00 |
| 30 | 100 [0.963, 1.000] | 0.999 (0.997–1.000) | +0.148 | — | 4 [0.016, 0.098] | 0.071 (0.008–0.250) | −0.102 | `S` 96 | 1.00 / 1.00 |
| 20 | 100 [0.963, 1.000] | 0.998 (0.995–0.999) | +0.147 | — | 75 [0.657, 0.825] | 0.560 (0.172–0.923) | +0.054 | `S` 25 | 1.00 / 1.00 |
| 15 | 100 [0.963, 1.000] | 0.999 (0.997–1.000) | +0.148 | — | 46 [0.366, 0.557] | 0.327 (0.065–0.741) | −0.008 | `S` 54 | 1.00 / 1.00 |
| 10 | 100 [0.963, 1.000] | 0.999 (0.997–1.000) | +0.144 | — | 48 [0.385, 0.577] | 0.204 (0.014–0.733) | −0.005 | `C` 25, `E` 12, `S` 11; 4 over 2 others | 1.00 / 1.00 |
| 5 | 100 [0.963, 1.000] | 0.999 (0.999–1.000) | +0.159 | — | 0 [0.000, 0.037] | 0.001 (0.000–0.002) | −0.107 | `S` 92, `?` 8 | 1.00 / 1.00 |

### Rotation (degrees clockwise, about the line centre; ISO 1831's skew tolerance is 3 degrees)

| Step | `P`: of 100 [Wilson 95%] | `P`: `p_true_peak` median (p10–p90) | `P`: margin | `P`: winner when wrong | `<`: of 100 [Wilson 95%] | `<`: `p_true_peak` median (p10–p90) | `<`: margin | `<`: winner when wrong | Word box covers centre, `P` / `<` |
| ---: | --- | --- | ---: | --- | --- | --- | ---: | --- | --- |
| 0 | clean render, above | | | | clean render, above | | | | |
| 0.25 | 100 [0.963, 1.000] | 1.000 (0.999–1.000) | +0.167 | — | 1 [0.002, 0.054] | 0.003 (0.000–0.057) | −0.160 | `S` 99 | 1.00 / 1.00 |
| 0.5 | 100 [0.963, 1.000] | 1.000 (0.999–1.000) | +0.190 | — | 35 [0.264, 0.447] | 0.121 (0.007–0.754) | −0.026 | `S` 40, `K` 11, `Y` 8; 6 over 3 others | 1.00 / 1.00 |
| 1 | 100 [0.963, 1.000] | 1.000 (0.996–1.000) | +0.227 | — | 0 [0.000, 0.037] | 0.011 (0.001–0.074) | −0.089 | `S` 79, `K` 12, `C` 8, `?` 1 | 1.00 / 1.00 |
| 1.5 | 100 [0.963, 1.000] | 0.998 (0.993–1.000) | +0.206 | — | 0 [0.000, 0.037] | 0.000 (0.000–0.007) | −0.095 | `S` 42, `K` 34, `?` 12; 12 over 5 others | 1.00 / 1.00 |
| 2 | 100 [0.963, 1.000] | 0.989 (0.757–0.999) | +0.251 | — | 0 [0.000, 0.037] | 0.000 (0.000–0.001) | −0.067 | `S` 21, `K` 20, `R` 11; 48 over 11 others | 1.00 / 1.00 |
| 3 | 30 [0.219, 0.396] | 0.061 (0.005–0.834) | −0.051 | `?` 27, `F` 9, `E` 6; 28 over 13 others | 0 [0.000, 0.037] | 0.000 (0.000–0.000) | −0.186 | `?` 11, `R` 10, `S` 10; 69 over 21 others | 1.00 / 1.00 |
| 4 | 33 [0.246, 0.427] | 0.096 (0.003–0.968) | −0.091 | `?` 17, `S` 14, `R` 8; 28 over 13 others | 0 [0.000, 0.037] | 0.000 (0.000–0.000) | −0.210 | `R` 17, `O` 10, `S` 8; 65 over 27 others | 1.00 / 1.00 |
| 5 | 35 [0.264, 0.447] | 0.145 (0.001–0.853) | −0.057 | `B` 19, `A` 9, `E` 6; 31 over 12 others | 0 [0.000, 0.037] | 0.000 (0.000–0.000) | −0.256 | `S` 19, `E` 17, `R` 6; 58 over 24 others | 1.00 / 1.00 |

### Blur (Gaussian sigma, native px; `synthpass_gen::degrade` `GaussianBlur`)

| Step | `P`: of 100 [Wilson 95%] | `P`: `p_true_peak` median (p10–p90) | `P`: margin | `P`: winner when wrong | `<`: of 100 [Wilson 95%] | `<`: `p_true_peak` median (p10–p90) | `<`: margin | `<`: winner when wrong | Word box covers centre, `P` / `<` |
| ---: | --- | --- | ---: | --- | --- | --- | ---: | --- | --- |
| 0 | clean render, above | | | | clean render, above | | | | |
| 0.5 | 100 [0.963, 1.000] | 0.999 (0.996–0.999) | +0.146 | — | 1 [0.002, 0.054] | 0.030 (0.002–0.178) | −0.116 | `S` 99 | 1.00 / 1.00 |
| 1 | 100 [0.963, 1.000] | 0.999 (0.997–1.000) | +0.154 | — | 6 [0.028, 0.125] | 0.023 (0.003–0.250) | −0.109 | `S` 94 | 1.00 / 1.00 |
| 1.5 | 100 [0.963, 1.000] | 0.999 (0.998–1.000) | +0.163 | — | 1 [0.002, 0.054] | 0.012 (0.002–0.163) | −0.109 | `S` 99 | 1.00 / 1.00 |
| 2 | 100 [0.963, 1.000] | 0.999 (0.997–0.999) | +0.163 | — | 2 [0.006, 0.070] | 0.005 (0.001–0.124) | −0.114 | `S` 97, `C` 1 | 1.00 / 1.00 |
| 2.5 | 100 [0.963, 1.000] | 0.999 (0.998–0.999) | +0.163 | — | 0 [0.000, 0.037] | 0.002 (0.000–0.032) | −0.101 | `S` 97, `C` 3 | 1.00 / 1.00 |
| 3 | 100 [0.963, 1.000] | 0.999 (0.998–0.999) | +0.165 | — | 0 [0.000, 0.037] | 0.000 (0.000–0.001) | −0.094 | `C` 90, `S` 10 | 1.00 / 1.00 |
| 4 | 100 [0.963, 1.000] | 0.947 (0.885–0.979) | +0.134 | — | 0 [0.000, 0.037] | 0.000 (0.000–0.000) | −0.097 | `E` 76, `O` 11, `C` 10, `U` 3 | 1.00 / 1.00 |
| 5 | 43 [0.337, 0.528] | 0.207 (0.140–0.375) | −0.002 | `S` 57 | 0 [0.000, 0.037] | 0.000 (0.000–0.000) | −0.027 | `E` 41, `e` 33, `O` 21, `U` 5 | 1.00 / 1.00 |
| 6 | 38 [0.291, 0.478] | 0.151 (0.107–0.205) | −0.003 | `M` 45, `?` 8, `N` 5, `S` 4 | 0 [0.000, 0.037] | 0.000 (0.000–0.000) | −0.012 | `U` 35, `O` 27, `E` 23; 15 over 2 others | 1.00 / 1.00 |

### Noise (additive Gaussian, sigma in grey levels; seeded from seed, axis and step)

| Step | `P`: of 100 [Wilson 95%] | `P`: `p_true_peak` median (p10–p90) | `P`: margin | `P`: winner when wrong | `<`: of 100 [Wilson 95%] | `<`: `p_true_peak` median (p10–p90) | `<`: margin | `<`: winner when wrong | Word box covers centre, `P` / `<` |
| ---: | --- | --- | ---: | --- | --- | --- | ---: | --- | --- |
| 0 | clean render, above | | | | clean render, above | | | | |
| 2 | 100 [0.963, 1.000] | 0.999 (0.996–0.999) | +0.146 | — | 8 [0.041, 0.150] | 0.074 (0.007–0.366) | −0.107 | `S` 92 | 1.00 / 1.00 |
| 4 | 100 [0.963, 1.000] | 0.998 (0.996–0.999) | +0.148 | — | 6 [0.028, 0.125] | 0.060 (0.007–0.375) | −0.109 | `S` 94 | 1.00 / 1.00 |
| 8 | 100 [0.963, 1.000] | 0.999 (0.996–0.999) | +0.146 | — | 12 [0.070, 0.198] | 0.076 (0.007–0.509) | −0.103 | `S` 88 | 1.00 / 1.00 |
| 12 | 100 [0.963, 1.000] | 0.998 (0.995–0.999) | +0.144 | — | 9 [0.048, 0.162] | 0.074 (0.007–0.397) | −0.095 | `S` 91 | 1.00 / 1.00 |
| 16 | 100 [0.963, 1.000] | 0.999 (0.995–0.999) | +0.144 | — | 18 [0.117, 0.267] | 0.074 (0.007–0.592) | −0.097 | `S` 82 | 1.00 / 1.00 |
| 24 | 100 [0.963, 1.000] | 0.998 (0.994–0.999) | +0.143 | — | 15 [0.093, 0.233] | 0.075 (0.004–0.542) | −0.084 | `S` 80, `C` 2, `Q` 2, `E` 1 | 1.00 / 1.00 |
| 32 | 100 [0.963, 1.000] | 0.998 (0.990–0.999) | +0.142 | — | 17 [0.109, 0.255] | 0.073 (0.003–0.525) | −0.065 | `S` 74, `R` 3, `C` 2; 4 over 3 others | 1.00 / 1.00 |

### Contrast (ink scale `c`: `p' = paper - c * (paper - p)`)

| Step | `P`: of 100 [Wilson 95%] | `P`: `p_true_peak` median (p10–p90) | `P`: margin | `P`: winner when wrong | `<`: of 100 [Wilson 95%] | `<`: `p_true_peak` median (p10–p90) | `<`: margin | `<`: winner when wrong | Word box covers centre, `P` / `<` |
| ---: | --- | --- | ---: | --- | --- | --- | ---: | --- | --- |
| 1 | clean render, above | | | | clean render, above | | | | |
| 0.8 | 100 [0.963, 1.000] | 0.998 (0.996–0.999) | +0.144 | — | 23 [0.158, 0.322] | 0.114 (0.007–0.581) | −0.083 | `S` 76, `?` 1 | 1.00 / 1.00 |
| 0.6 | 100 [0.963, 1.000] | 0.998 (0.993–0.999) | +0.143 | — | 36 [0.273, 0.458] | 0.191 (0.009–0.813) | −0.042 | `S` 63, `?` 1 | 1.00 / 1.00 |
| 0.5 | 100 [0.963, 1.000] | 0.998 (0.989–0.999) | +0.143 | — | 40 [0.309, 0.498] | 0.199 (0.013–0.880) | −0.026 | `S` 59, `?` 1 | 1.00 / 1.00 |
| 0.4 | 100 [0.963, 1.000] | 0.998 (0.988–0.999) | +0.142 | — | 39 [0.300, 0.488] | 0.169 (0.014–0.839) | −0.022 | `S` 60, `?` 1 | 1.00 / 1.00 |
| 0.3 | 100 [0.963, 1.000] | 0.997 (0.983–0.999) | +0.141 | — | 42 [0.328, 0.518] | 0.189 (0.016–0.832) | −0.019 | `S` 56, `?` 2 | 1.00 / 1.00 |
| 0.2 | 100 [0.963, 1.000] | 0.996 (0.978–0.999) | +0.142 | — | 50 [0.404, 0.596] | 0.272 (0.020–0.858) | +0.003 | `S` 48, `?` 2 | 1.00 / 1.00 |
| 0.15 | 100 [0.963, 1.000] | 0.996 (0.980–0.999) | +0.144 | — | 55 [0.452, 0.644] | 0.348 (0.017–0.906) | +0.010 | `S` 45 | 1.00 / 1.00 |
| 0.1 | 100 [0.963, 1.000] | 0.994 (0.974–0.998) | +0.149 | — | 37 [0.282, 0.468] | 0.192 (0.017–0.769) | −0.023 | `S` 63 | 1.00 / 1.00 |

## Readings (Hypothesized)

These are candidate explanations. None was tested by this run.

- **The filler sits close to a `<` / `S` decision boundary on the clean render.** The clean-render
  margin is −0.102 and `p_true_peak`'s p90 is 0.331. Perturbations that change stroke edges
  (JPEG 20, lower contrast, noise 16-32) tip some seeds back to `<`, and others (resolution 11
  and 10, rotation 0.25, blur 0.5) tip them further to `S`. That fits a knife edge better than a
  degradation curve.
- **The run cannot say where `S` comes from.** It could be the filler glyph itself: its shape, or
  the vertical position the [filler geometry note](ocrb-filler-geometry-2026-09-23.md) measured
  0.03 cap low against real print. It could be the cell-to-timestep mapping picking up a
  neighbour's mass. It could be the label set. This run does not separate them.
- **The rotation cliff between 2 and 3 degrees may be the instrument as much as the model.** The
  oracle crop under rotation is the rotated line's axis-aligned bounding box. Across 44 cells of
  22 px, the line rises by about 34 px at 2 degrees, 51 px at 3 degrees and 84 px at 5 degrees
  (Derived: 968 px × sin θ). The glyphs stay one line tall. The tool's module doc also calls its
  rotated timestep extent an approximation. Both would hit `P` and `<` together, which is what the
  3-degree step shows for `P`.

## Evidence and limits

- **Run** (from the run header in `summary.json`, and the laptop's report for the facts the header
  does not carry): `glyph_atlas` at `5860cb9d43803d036613ece9795391d1a38713cb`, schema 1, on a
  Windows x86_64 laptop with 12 logical CPUs and `RTEN_NUM_THREADS=4`. The CPU was a 12th Gen
  Intel Core i7-1255U (the header records `cpu_model: null`). It ran on AC, through the CPU lock,
  with nothing else running. The stack was ocrs 0.13.1, rten 0.26.0, beam width 24, crop
  `oracle_line`, and detection threshold 0.2. The pinned models were detection sha256
  `f15cfb56bd02…` and recognition `e484866d4cce…`; the model pin and the alphabet self-check
  passed. The setup was TD3 line 0, cells 0 and 1, seeds 0-99, and all six axes with the
  module doc's steps. These parameters equal the tool's defaults:

  ```
  RTEN_NUM_THREADS=4 cargo run -p synthpass-ocr --release --example glyph_atlas -- \
    --seeds 100 --seed-start 0 --cells 0,1
  ```

  The exact command line is not recorded in `summary.json`. The laptop's report gives a wall time
  of 2,988.6 s for 5,500 computed renders (55 distinct steps × 100 seeds), and exit 0.
- **Output:** `summary.json` holds aggregates only: 118 rows, one per cell × axis step. Its sha256
  is `47021589397d…`. Neither it nor the per-seed `records.jsonl` is committed. `records.jsonl`
  stayed on the laptop, and this note uses nothing from it.
- **Determinism:** the degradations are pure functions of the clean render and (seed, axis, step).
  Inference is floating-point. Compare runs on one machine with pinned models and a pinned
  `RTEN_NUM_THREADS`, never across machines (the tool's module doc).
- **The tool's Windows test failure is not in scope.** The unit test
  `dot_dot_cannot_smuggle_an_in_tree_path_out_of_the_check` failed on Windows before this run.
  It was a POSIX path literal in the test, fixed in PR #623, and it tests the `--out` guard
  only.

## What this does not say

- **Nothing about the pipeline's decoded text.** "Read as itself" is the winner of an averaged
  distribution over an oracle crop, not a decode. A cell that loses here may still decode
  correctly, and a cell that wins may not. The atlas also bypasses imageprep, the retry loop and
  the grid fit.
- **Nothing about real documents.** Synthetic results have not transferred before. The chargrid
  A/B netted +30 strict names on 500 synthetic documents and 0 on 257 real specimens, fixing one
  and breaking one
  ([reconciliation](chargrid-ab-reconciliation-2026-09-24.md)). A 4 / 100 filler rate on
  synthetic renders is not a rate on print.
- **Nothing to adopt.** No rank, threshold or constant is proposed. ADR-0021 stays Proposed, and
  no ADR, code, baseline or headline changes.
- **Nothing about PR [#604](https://github.com/ruledicaprio/SynthPass/pull/604)** (the frozen
  filler raise) or its effect. See the next measurements for what could be run.
- **No composed degradations.** The axes never compose, and real images degrade on several at once.

## Next measurements

Each would be one run of the same tool, on the same laptop with `RTEN_NUM_THREADS=4`, compared
row by row with this `summary.json`.

- **The same sweep on #604's branch, as an A/B.** It would show whether the averaged-distribution
  winner at cell 1 moves on synthetic renders when the filler is drawn at the height the
  [filler geometry note](ocrb-filler-geometry-2026-09-23.md) measured. This is a follow-up
  measurement only, and it would carry the same evidence label and the same limits.
- **Other cells.** `--cells` accepts any of TD3 line one's 44 cells, for example a cell inside
  the trailing filler run, whose neighbours are also `<`. That would bear on the neighbour
  question above. `I<`, `ID` and `V<` are document codes of other formats (identity cards and
  visas). The merged tool renders TD3 only and checks that line one starts with `P<`, so
  measuring them needs the tool extended first.
- **The same sweep after any OCR-stack bump** (ocrs, rten, or the model pin), as a fingerprint.
  Any row that moves is a behaviour change to explain before the bump lands.
- **The decoded-text comparison, which this tool does not make.** `records.jsonl` carries two
  beam-control booleans per render (does the beam read of the oracle crop start with `P<`,
  strictly and ignoring fillers). `summary.json` does not aggregate them, and this note has not
  seen them. What the pipeline itself decodes needs the pipeline, not the atlas.

## Rejected on the way

Nothing was proposed, so nothing was rejected. This is a first measurement.
