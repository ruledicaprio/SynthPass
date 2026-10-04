# Fonts for `synthpass-gen`

Six OFL (SIL Open Font License)-licensed TrueType fonts are vendored here, for rendering real
glyphs (instead of placeholder bars) on the VIZ fields and the MRZ band:

- **`ocr-b.ttf`** — an OCR-B-style monospaced font for the MRZ band. Sourced from
  [jaycee723/ocr-b](https://github.com/jaycee723/ocr-b) (`dist/OCR-B.ttf`), © 2019 Raisty,
  Reserved Font Name "OCR-B", SIL OFL 1.1. Full license text: [`OFL-ocr-b.txt`](OFL-ocr-b.txt).
- **`sans.ttf`** — PT Sans Regular, a proportional sans-serif font for the human-readable VIZ
  fields. Sourced from Google Fonts' canonical repository,
  [google/fonts `ofl/ptsans`](https://github.com/google/fonts/tree/main/ofl/ptsans)
  (`PT_Sans-Web-Regular.ttf`), © 2010 ParaType Ltd., Reserved Font Names "PT Sans"/"ParaType",
  SIL OFL 1.1. Full license text: [`OFL-sans.txt`](OFL-sans.txt).

All are static (non-variable) TrueType files, chosen deliberately over variable-font releases
of the same families since `ab_glyph` (the rasterizer used in `src/fonts.rs`) targets classic
outline fonts, not `fvar` variation axes.

The `embedded-fonts` feature bakes them into the binary via `include_bytes!`. It is **on by
default** (all fonts are vendored, so there is nothing to supply); turn it off to build without
them:

```sh
cargo build -p synthpass-gen --no-default-features
```

Without the feature, `fonts::load_fonts` returns `FontError::NotEmbedded` and the
renderer draws placeholder bars in the exact layout rectangles instead — bounding boxes in
`Labels` stay meaningful either way. The unconditional "SYNTHETIC / SPECIMEN" watermark and the
generic, non-country template render regardless of this feature; they do not depend on any TTF.

The fonts' OFL licenses are also summarized in the root [`THIRD_PARTY_NOTICES.md`](../../../THIRD_PARTY_NOTICES.md).

## Release provenance and byte pins

The four additional VIZ Regular files are unmodified upstream releases, not subsets.
They are accessible through `Fonts::viz_font`; rendering still uses PT Sans. No CLI,
configuration, label, golden hash or synthetic baseline changes in ADR-0030 PR 1.
Admission follows [ADR-0030 Decision 5](../../../knowledge/decisions/ADR-0030-viz-fonts-are-a-closed-embedded-set.md),
as amended in [#704](https://github.com/ruledicaprio/SynthPass/pull/704).

| File | Family / version | Release asset or original source | File SHA-256 | Bytes | Licence |
|---|---|---|---|---:|---|
| `sans.ttf` | PT Sans Regular (existing pin) | [Original source](https://github.com/google/fonts/tree/main/ofl/ptsans) | `9cc831490532009bae2b3ce0d39c62adfc889060beb421593bfd9d2396d0f10a` | 442960 | [OFL-sans.txt](OFL-sans.txt) |
| `ocr-b.ttf` | OCR-B (existing pin) | [Original source](https://github.com/jaycee723/ocr-b) | `367d876cca948ecd4900851f6e85687cbb6e71de9d0d2f36348edec5655526af` | 36780 | [OFL-ocr-b.txt](OFL-ocr-b.txt) |
| `liberation-sans.ttf` | Liberation Sans 2.1.5 | [Liberation TTF archive](https://github.com/liberationfonts/liberation-fonts/files/7261482/liberation-fonts-ttf-2.1.5.tar.gz) | `76d04c18ea243f426b7de1f3ad208e927008f961dc5945e5aad352d0dfde8ee8` | 410712 | [OFL-liberation.txt](OFL-liberation.txt) |
| `liberation-serif.ttf` | Liberation Serif 2.1.5 | [Liberation TTF archive](https://github.com/liberationfonts/liberation-fonts/files/7261482/liberation-fonts-ttf-2.1.5.tar.gz) | `058ea80864aef09a23f45cbec2bb5400bc3dfbdea01c3f10538a21fcb497fb74` | 393576 | [OFL-liberation.txt](OFL-liberation.txt) |
| `liberation-mono.ttf` | Liberation Mono 2.1.5 | [Liberation TTF archive](https://github.com/liberationfonts/liberation-fonts/files/7261482/liberation-fonts-ttf-2.1.5.tar.gz) | `f2b83c763e8afd21709333370bed4774337fae82267937e2b5aea7e2fbd922c1` | 319508 | [OFL-liberation.txt](OFL-liberation.txt) |
| `source-sans-3.ttf` | Source Sans 3.052R | [Static TTF ZIP](https://github.com/adobe-fonts/source-sans/releases/download/3.052R/TTF-source-sans-3.052R.zip) | `4644c81b86ec9caaa76b634889968ed3c4f4f52f054855933acc7c2b21e53b0f` | 431196 | [OFL-source-sans-3.txt](OFL-source-sans-3.txt) |

Release archive SHA-256s (shared by each corresponding file's entry above):

- Liberation TTF archive: `7191c669bf38899f73a2094ed00f7b800553364f90e2637010a69c0e268f25d0`.
  The licence is its unmodified `LICENSE` file.
- Source Sans static TTF ZIP: `1b0dd1ec44b39f1dd98bbd153a1a3815f083639874ddee02c842bd601bad3d21`.
  This asset contains no licence; `OFL-source-sans-3.txt` is the unmodified `LICENSE.md`
  from the same release's [source ZIP](https://github.com/adobe-fonts/source-sans/archive/refs/tags/3.052R.zip),
  SHA-256 `8740ef8bee3a3144b3dbb8b673264bfc685dab944610b884b14e6146ded9a50a`.

The four additional files total 1,554,992 bytes. Each family is embedded only under
`embedded-fonts`; no runtime font files or network downloads are used.

## Per-field admission scales

ADR-0030 Decision 5, as amended in #708, computes one factor per field, rather
than one pin per font. The factor multiplies 70% of that rectangle's height.
Fixed-step bisection bounds right/bottom by PT Sans's domain ink; bounded fixed
step-down checks literal top/left pixel rounding. PT Sans remains exactly 1.0.
The renderer reuses the computed f32 pixel size, cached per validated layout and
font; a refused field names the font, format and field, and `random` filters it out.

The 250-row snapshot is in `../tests/fixtures/viz_field_scales.tsv` relative to
the crate source tree (`crates/synthpass-gen/tests/fixtures/viz_field_scales.tsv`).

| Font | Minimum k | Median k | Maximum k |
|---|---:|---:|---:|
| PT Sans | 1.000000 | 1.000000 | 1.000000 |
| Liberation Sans | 0.751090 | 0.779110 | 0.991678 |
| Source Sans 3 | 0.996573 | 1.000000 | 1.000000 |
| Liberation Serif | 0.569087 | 0.754073 | 0.940534 |
| Liberation Mono | 0.346441 | 0.969438 | 1.000000 |

Only the one-letter document code constrains Mono and Serif to small factors.
The test checks all four literal edges and that k + 0.001 fails right or bottom
unless capped at 1. No tolerance or baseline change is applied. Alternative
faces are parsed lazily through named `OnceLock` caches, independent of PT Sans.
