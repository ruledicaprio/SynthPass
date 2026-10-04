# ADR-0030 — The VIZ draws from a closed set of embedded open-licence fonts, opt-in, and no font out-inks PT Sans

**Status:** Accepted (owner, 2026-10-04)
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
5. **No font may out-ink PT Sans (the admission rule).**
   - **The scale.** Every other font F has a constant k_F in (0, 1], pinned in code. F is drawn at k_F × 70 % of the
     rectangle's height.
   - **Choosing k_F.** It is the largest value for which F's ink stays inside PT Sans's ink in every built-in
     rectangle, comparing the widest ink each font produces over the field's whole domain. Both are placed exactly as
     the renderer places them.
   - **Why that suffices.** Outlines are unhinted and scale linearly with size. A rectangle that fits PT Sans therefore
     fits every admitted font, up to a pixel of rounding, which the test absorbs by checking every built-in rectangle
     at its own size.
   - **What stays the same.** The fit check keeps measuring PT Sans only. No layout that validates today stops
     validating, the committed examples included.
   - **Tests and docs.** A test recomputes every k_F and fails if a pinned value is too large.
     [LAYOUTS.md](../LAYOUTS.md) says that the other fonts are drawn at or below the PT Sans size.
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
  the committed examples among them, and every new font would reopen every layout. The admission rule moves that cost
  onto the font, and pays it once.
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
- **A wider font.** A font later found wider than PT Sans in some field is not an exception to the rule: its k_F
  shrinks.
