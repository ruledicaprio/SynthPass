# ADR-0030 — The VIZ draws from a closed set of embedded open-licence fonts, opt-in, and no font needs more room than PT Sans

**Status:** Accepted (owner, 2026-10-04). Decision 5 was amended twice the same day; see the amendments at the end.
**Date:** 2026-10-04

## Context

- **One font today.** The generator draws every human-readable (VIZ) field in one font and the MRZ in another.
  - VIZ: PT Sans Regular (`crates/synthpass-gen/fonts/sans.ttf`, SIL OFL 1.1).
  - MRZ: OCR-B.
  - `Fonts` in `crates/synthpass-gen/src/fonts.rs` holds exactly that pair.
- **Real documents vary.** ICAO Doc 9303 fixes a typeface only for the MRZ (OCR-B). The VIZ typeface is the issuing
  state's choice, within Latin letters and Arabic numerals, with diacritics permitted. In practice four styles dominate:
  - neo-grotesque sans (Helvetica, Arial, Univers);
  - humanist sans (Frutiger, Myriad);
  - Times-style serif;
  - monospace, for some numbers.

  A reader measured, or a model trained, on one font sees none of that variety.
- **What SynthPass may not ship.**
  - The originals are proprietary and cannot be redistributed.
  - Some states commission their own typefaces. Shipping one would tie a synthetic document to a real issuer, against
    the generic, non-country template ([VISION](../VISION.md); the exclusions in
    [ADR-0022](ADR-0022-declarative-layout-plugins.md)).
- **What depends on the one font.**
  - The layout fit check (`crates/synthpass-gen/src/layout/validated/fit.rs`) proves that each field's widest value
    fits its rectangle. It measures in PT Sans, at the size the renderer draws: 70 % of the rectangle's height
    (`viz_px_scale` in `crates/synthpass-gen/src/render.rs`).
  - The golden render hashes are renders in PT Sans, and so is every synthetic baseline: the README headline numbers
    and `bench-ab.yml`'s synthetic arms.
- **Layout files.** ADR-0022 keeps fonts out of layout files: a font file is untrusted binary input and carries
  national identity. Nothing here changes that.

## Decision

1. **A closed, embedded set.** The VIZ font set is PT Sans, the default, plus four families, Regular weight only:

   | `--viz-font` name | Family | Stands in for | Licence |
   |---|---|---|---|
   | `pt-sans` (default) | PT Sans | today's font | SIL OFL 1.1 |
   | `liberation-sans` | Liberation Sans | Helvetica, Arial, Univers | SIL OFL 1.1 |
   | `source-sans-3` | Source Sans 3 | Frutiger, Myriad | SIL OFL 1.1 |
   | `liberation-serif` | Liberation Serif | Times New Roman | SIL OFL 1.1 |
   | `liberation-mono` | Liberation Mono | Courier | SIL OFL 1.1 |

   The fonts are compiled in under the existing `embedded-fonts` feature. No font is ever loaded at run time, whether
   from a file, a layout or the network.
2. **Licence and provenance.**
   - **Unmodified files.** Each file is the upstream release, unmodified. There is no subsetting, because under the OFL
     a subset is a Modified Version and raises Reserved Font Name questions.
   - **Records.**
     - [`crates/synthpass-gen/fonts/README.md`](../../crates/synthpass-gen/fonts/README.md) records each file's
       family, version, source URL and SHA-256.
     - Each family's licence text sits beside its file.
     - [`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md) gets a row per family.
   - **Future additions.**
     - **Allowed licences:** SIL OFL 1.1, Apache-2.0 or the Bitstream Vera licence.
     - **Never:** GPL or AGPL terms, a proprietary font, or a typeface commissioned for, or identified with, an
       issuing state.
     - **Process:** adding a family amends this ADR.
3. **Opt-in, with the default unchanged.**
   - **Interfaces.** `synthpass generate` and `synthpass export` take `--viz-font <name|random>`. The library takes the
     same choice in `GeneratorConfig`.
   - **Default output.** Without the option, the output is byte-identical to today's: the golden hashes, the labels
     and every synthetic baseline stay put.
   - **Variety as the default** is a separate decision, taken after the measurement below.
4. **Deterministic, and independent of the content stream.**
   - `random` picks one font per document from a stream derived from the document seed and a fixed domain tag. It
     never draws from the `ChaCha8Rng` that draws the content.
   - A document under `random` therefore has the same names, dates, numbers and layout as its default render; only
     the glyphs differ.
   - The same seed always gives the same font.
5. **No font may need more room than PT Sans (the admission rule).** Amended twice on 2026-10-04. The amendments at the
   end of this ADR record the earlier versions and why they changed.
   - **Where F may ink.**
     - **Placement.** The renderer anchors every font at the rectangle's top-left corner:
       - the baseline sits at the rectangle's top plus the font's own ascent;
       - the pen starts at the left edge, moved right past any negative left-side bearing.

       This is `flow_glyphs` and `pen_start` in `crates/synthpass-gen/src/render.rs`.
     - **The area.** Drawn that way, F's ink must lie inside the area from the rectangle's top-left corner to the
       bottom-right corner of PT Sans's ink.
     - **PT Sans's ink** is the union over the field's whole domain at 70 %, as the fit check measures it.
   - **The scale, per field.** In each field, F is drawn at k × 70 % of the rectangle's height. k is the largest value
     in (0, 1] that keeps F's ink inside the area over the field's whole domain.
     - **Right and bottom.** These edges grow with k, and they set it. Bisection finds it.
     - **Top and left.** The placement keeps both inside the rectangle unless a glyph rises above its font's ascent.
       One f32 rounding of the scaled size can still move ink by a pixel. So k steps down from the bisection's value
       until all four edges are inside.
   - **Computed once, held in a snapshot.**
     - k depends only on the field's rectangle, its domain and the font.
     - The generator computes it once per layout and font, and reuses it.
     - A snapshot test holds the built-in layouts' table, so a change to a font, a layout or the rule shows up in
       review.
   - **A field no scale fits.**
     - If no k in (0, 1] keeps F inside the area in some field, F cannot draw that layout.
     - Asking for F with that layout is an error that names the field, and `random` picks only among the fonts that
       can draw it.
     - Every font fits every field of the built-in layouts.
   - **Why that suffices.**
     - The fit check proves that PT Sans's ink lies inside the rectangle.
     - At its scale, F's top and left edges are inside the rectangle, and its right and bottom edges are no further out
       than PT Sans's.
     - So every rectangle that fits PT Sans fits F, in every layout, the custom ones included. This holds by
       construction, at each field's own size.
   - **What stays the same.** The fit check keeps measuring PT Sans only. No layout that validates today stops
     validating, the committed examples included.
   - **Tests and docs.**
     - A test checks the rule at the computed scale in every built-in field, and holds the snapshot.
     - [LAYOUTS.md](../LAYOUTS.md) says that the other fonts are drawn at or below the PT Sans size.
6. **Coverage.** A test checks that every character any field's domain can produce maps to a real glyph, not
   `.notdef`, in every font of the set.
7. **Recorded in the labels.** When `--viz-font` is given, the labels file records the font's name as `viz_font`.
   Without the option the field is absent, and the labels are byte-identical to today's.
8. **Unchanged.**
   - The MRZ stays OCR-B.
   - Layout files still cannot name a font (ADR-0022).
   - The service ([ADR-0009](ADR-0009-generator-as-a-service.md)) keeps PT Sans until a later change asks for the
     option.

## Alternatives rejected

- **Variety on by default.** It would move every golden hash and every synthetic number at once, with no measurement to
  say whether the change helps. Opt-in comes first, and a default change can follow the evidence.
- **Checking every layout against every font.** This is stricter, but layouts that validate today could start failing,
  the committed examples among them, and every new font would reopen every layout. Instead, validation keeps measuring
  PT Sans only, and each font fits itself to each field (Decision 5).
- **The proprietary originals, or URW's Nimbus clones.** The originals cannot be redistributed, and the clones carry
  (A)GPL terms the project does not take.
- **Subset fonts.** They are smaller, but a subset is a Modified Version under the OFL. Unmodified files keep the
  licence question closed.
- **Fonts in layout files.** ADR-0022 excludes them, for reasons that still hold.

## Consequences

- **Size.** The repository and the `embedded-fonts` build grow by the four Regular files, on the order of 1.5 MB. PR 1
  reports the exact sizes.
- **Two PRs implement this ADR.**
  - **PR 1** adds the files, the provenance records, the font set, and the admission and coverage tests. It changes no
    output.
  - **PR 2** adds the option and the separate stream. It also adds the labels field, the docs, a changelog fragment,
    and one golden render per format under one non-default font.
- **Measurement follows PR 2.**
  - **The run.** A synthetic A/B compares the default against `random`, on five formats × 100 documents.
  - **What to expect.** The MRZ reads should not move, because the MRZ font is unchanged. Any VIZ-dependent figure
    that moves is the finding.
  - **The default.** Whether variety becomes the default is decided on that note.
- **A wider font.** A font later found wider than PT Sans in some field is not an exception to the rule: its scale
  shrinks in that field.

## Amendment 1 (2026-10-04): the admission rule checks the rectangle's top and left, and PT Sans's right and bottom

**What changed.** As first accepted, Decision 5 required F's ink to stay inside PT Sans's ink in every built-in
rectangle. No k_F satisfies that for the three Liberation fonts:
- The renderer hangs each font from the rectangle's top by its own ascent.
- PT Sans's ascent is taller, so Liberation's capitals sit higher in the rectangle than PT Sans's do.
- Shrinking F raises its baseline with it, so its top moves further out, not in.

This came up while PR 1 was being implemented, on TD1's letter `I`.

**What replaced it.** The rule now protects what the fit check protects: the rectangle.
- **Top and left.** The renderer anchors these two edges to the rectangle, so they are checked against the rectangle.
- **Right and bottom.** These two edges grow with k, and they may not pass PT Sans's.
- **The guarantee is unchanged.** A rectangle that fits PT Sans fits every admitted font.
- **The title changes with the rule**, from "no font out-inks PT Sans" to "no font needs more room than PT Sans".

**Rejected: drawing every font on PT Sans's baseline.** It fixes the top edge, but a narrow glyph such as `I` then
fails the same way at the left edge: shrinking F moves its left ink toward the pen, out of PT Sans's.

**Decided by** the owner on 2026-10-04.

## Amendment 2 (2026-10-04): one scale per field, computed by the rule, instead of one per font

**What changed.** After amendment 1, each font had one constant k_F: the smallest value any of its built-in fields
allowed.
- **One field set it.** For Liberation Serif and Liberation Mono, that value came from TD1's one-letter document code
  `I`, whose slab serifs reach further right than PT Sans's bare stem.
- **The result.** Drawn at that one value everywhere, Mono's capitals came out at 37 % of PT Sans's height and Serif's at
  62 %.
- **The risk.** A measurement of the fonts would then have measured small text as much as typefaces.

**What replaced it.** The same rule, applied to each field: every field gets the largest scale that keeps the font
inside its area. This was measured on PR 1's code over the five built-in layouts, 50 fields per font. PR 2's snapshot
test holds the exact table.

| Font | k per field: min / median / max | Capital height at that k, median |
|---|---|---:|
| Liberation Sans | 0.751 / 0.779 / 0.992 | 89 % |
| Source Sans 3 | 0.997 / 1.0 / 1.0 | 91 % |
| Liberation Serif | 0.569 / 0.754 / 0.941 | 82 % |
| Liberation Mono | 0.346 / 0.969 / 1.0 | 104 % |

Capital height is a share of PT Sans's in the same field. Only the one-letter document codes stay small.

- **The guarantee is unchanged.** It now holds by construction for every layout, rather than up to a pixel of rounding.
- **The cost.** Computing every built-in layout for all four fonts takes about 2 s, measured. It is paid once per
  layout and font.
- **Review.** A snapshot test keeps the built-in table reviewable.

**Rejected: a pinned table of 200 constants.** It would be as reviewable as the snapshot, but it covers only the
built-in layouts, and a custom layout needs the computation anyway.

**Decided by** the owner on 2026-10-04.
