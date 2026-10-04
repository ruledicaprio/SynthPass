# LAYOUTS.md — the layout file format

**Status:** the format and the `synthpass-layout` crate that reads it. The `--layout` flag on
`synthpass generate` and `synthpass export` arrives in a later M8 PR; until then a layout file
loads through the crate's API only. The decisions behind the format are
[`ADR-0022`](decisions/ADR-0022-declarative-layout-plugins.md) (Decisions 1 to 8); the milestone is
M8 in [`ROADMAP.md`](ROADMAP.md).

A layout file moves the portrait and the ten visual-zone text fields of one ICAO 9303 format
(TD1, TD2, TD3, MRV-A or MRV-B) to other places on that format's canvas. That is everything it can
do. This page is meant to be enough to write one without reading any code.

## 1. A file, whole

```json
{
  "schema_version": 1,
  "name": "icao-td3-builtin",
  "description": "Optional. Never drawn.",
  "format": "td3",
  "portrait": {"x": 860, "y": 100, "width": 260, "height": 340},
  "document_type": {"x": 60, "y": 60, "width": 120, "height": 34},
  "issuing_country": {"x": 220, "y": 60, "width": 120, "height": 34},
  "surname": {"x": 60, "y": 120, "width": 700, "height": 34},
  "given_names": {"x": 60, "y": 170, "width": 700, "height": 34},
  "document_number": {"x": 60, "y": 220, "width": 300, "height": 34},
  "nationality": {"x": 60, "y": 270, "width": 200, "height": 34},
  "date_of_birth": {"x": 280, "y": 270, "width": 240, "height": 34},
  "sex": {"x": 540, "y": 270, "width": 80, "height": 34},
  "date_of_expiry": {"x": 60, "y": 320, "width": 240, "height": 34},
  "personal_number": {"x": 60, "y": 370, "width": 400, "height": 34}
}
```

That is the TD3 built-in, with `description` added. The file is one UTF-8 JSON object of at most 65,536 bytes. Whitespace and key order do not matter.

The five built-ins are such files: [`layouts/td1.json` … `layouts/mrvb.json`](../crates/synthpass-layout/layouts)
in the `synthpass-layout` crate, written by the crate's emitter. They are the best starting point:
copy one and move rectangles. The committed
[`examples/`](../crates/synthpass-layout/examples) are layouts written from this page.

## 2. Keys

The schema is closed. A key not listed here, at either level, is an error, and so is a key given
twice.

### Top level

| Key | Type | Required | Rule |
| --- | --- | --- | --- |
| `schema_version` | integer | yes | exactly `1` |
| `name` | string | yes | 1 to 64 characters, each in `[a-z0-9._/-]` |
| `description` | string | no | at most 1,024 bytes; `null` is not "absent" and is refused |
| `format` | string | yes | one of `td1`, `td2`, `td3`, `mrva`, `mrvb` |
| `portrait` | rectangle | yes | where the portrait photo is drawn |
| `document_type` | rectangle | yes | the document code |
| `issuing_country` | rectangle | yes | the issuing-state code |
| `surname` | rectangle | yes | |
| `given_names` | rectangle | yes | |
| `document_number` | rectangle | yes | |
| `nationality` | rectangle | yes | |
| `date_of_birth` | rectangle | yes | |
| `sex` | rectangle | yes | |
| `date_of_expiry` | rectangle | yes | |
| `personal_number` | rectangle | yes | required on every format |

`name` and `description` are never drawn and are not part of a layout's identity (section 6).

### A rectangle

| Key | Type | Required |
| --- | --- | --- |
| `x` | integer | yes |
| `y` | integer | yes |
| `width` | integer | yes |
| `height` | integer | yes |

Each is an integer from 0 to 4,294,967,295. A negative number, a fraction (`1.5`, and also `1.0`),
a string or a number past that range is an error.

## 3. Coordinates and the permitted area

Coordinates are absolute pixels on the format's canvas. The origin `(0, 0)` is the top-left
pixel, `x` grows rightwards and `y` downwards. A rectangle covers `x .. x + width` by
`y .. y + height`, the end exclusive, so two rectangles that touch edge to edge do not overlap.

The engine owns the canvas, the frame drawn around it, the watermark band and the
machine-readable zone. A layout cannot move or resize them, and has no key for them. A rectangle
must lie inside the **permitted area**: inside the frame, and no lower than the lowest bottom edge
among that format's eleven built-in rectangles (the portrait included). That edge sits above the
watermark band, so nothing can land on the watermark or the MRZ, and no layout brings the visual
zone closer to the MRZ than a built-in does.

| `format` | Canvas (w × h) | Frame | Permitted `x` | Permitted `y` | Watermark band at (x, y), w × h |
| --- | --- | --- | --- | --- | --- |
| td1 | 822 × 518 | 6 | 6..816 | 6..272 | (40, 315) 742 × 32 |
| td2 | 1008 × 710 | 6 | 6..1002 | 6..334 | (50, 360) 908 × 50 |
| td3 | 1200 × 840 | 6 | 6..1194 | 6..440 | (60, 470) 1080 × 60 |
| mrva | 1152 × 768 | 6 | 6..1146 | 6..370 | (50, 372) 1052 × 50 |
| mrvb | 1008 × 710 | 6 | 6..1002 | 6..334 | (50, 360) 908 × 50 |

The ranges are end-exclusive: a rectangle with `x + width = 1194` fits TD3, one with `1195` does
not. The table is checked against `synthpass_gen::layout::frame_for` by
`crates/synthpass-layout/tests/layouts_doc.rs`, so it cannot drift from the code.

The shortest visual-zone row among the five built-ins is **28 px** (TD1). No visual-zone rectangle
may be shorter than that, on any format. The portrait has no minimum.

## 4. The checks, in order

Each check fails closed, and the load stops at the first failure. Messages name the key or field
first, then the rule.

**The file** (these run in this order, before any geometry):

| Check | Message begins |
| --- | --- |
| input over 65,536 bytes, refused before parsing | `input: over 65536 bytes, which is the limit` |
| not UTF-8 | `input: not valid UTF-8 (first bad byte at offset N)` |
| not JSON, an unknown, duplicate or missing key, or a value that is not a `u32` or a known `format` | `layout JSON: …` with serde_json's line and column, and the text next to the error, which carries the key |
| `schema_version` other than 1 | `schema_version: N is not supported` |
| `name` out of rule | `name: must be 1 to 64 characters, found N`, or `name: character 'C' at index I is not in [a-z0-9._/-]` |
| `description` over 1,024 bytes | `description: N bytes exceeds the 1024-byte limit` |

**The geometry** (the rules of `ValidatedLayout::try_from_spec` in `synthpass-gen`). Rule ids appear
in brackets at the end of the message, e.g. `surname: overlaps given_names [rule overlap]`.

| Order | Rule id | Message | What it means |
| --- | --- | --- | --- |
| 1 | `empty-rect` | `F: empty rectangle (zero width or height)` | `width` or `height` is 0 |
| 1 | `overflow` | `F: arithmetic overflow (x + width or y + height exceeds u32)` | `x + width` or `y + height` is past 4,294,967,295 |
| 1 | `outside-permitted-area` | `F: outside the permitted area (x A..B, y C..D)` | the rectangle leaves the permitted area of section 3 |
| 1 | `too-short` | `F: height H is below the shortest built-in visual-zone row (28)` | a text field under 28 px |
| 2 | `overlap` | `F: overlaps G` | two rectangles share a pixel; `F` is the earlier in the table of section 2 |
| 3 | `text-fit` | `F: text does not fit (the widest value inks x …, y …; the rectangle spans x …, y …)` | see below |

Rule 1 runs per rectangle in the order of section 2 (portrait first), all four checks on one
rectangle before the next. Rule 2 runs over all pairs, and rule 3 over the text fields.

**Text fit.** The generator knows the widest value each field can hold. The loader shapes that
value with embedded PT Sans at the size the default renderer uses, places it at the rectangle's left edge
and requires all its ink to land inside the rectangle on all four sides. This is what keeps the exported labels
accurate by construction. **Text is drawn at 70 % of the rectangle's height**: a 34 px row draws
23.8 px text, a 28 px row 19.6 px. So height and width trade off: making a rectangle taller
makes its text larger, and a larger text needs more width. A built-in's width is known to fit at
that built-in's height only; at a greater height it may not. Narrowing a rectangle is the usual
way to fail, and the message gives the ink span to widen it by. Widening the generator's value pools
can make a previously accepted layout fail this check later, and it will say which field.

The opt-in VIZ fonts in [ADR-0030](decisions/ADR-0030-viz-fonts-are-a-closed-embedded-set.md)
are drawn at a per-field scale at or below the PT Sans size (0 < k ≤ 1), using their own
ascent and left-side bearing. ADR-0030 Decision 5, as amended in #708, computes the largest
scale inside the rectangle's top-left and PT Sans's domain-ink bottom-right, stepping down
for f32 edge rounding. Results are cached once per validated layout and font, including
refusals. A fixed font that cannot draw a field is refused with the font, format and field
in the error; `random` picks only among fonts admitted by that layout. The built-in table
is held in `crates/synthpass-gen/tests/fixtures/viz_field_scales.tsv`.
The fit checker continues measuring PT Sans only; MRZ text remains OCR-B. Fonts cannot be
selected by a layout file.

## 5. What a layout cannot express

There is no key for any of these, and a key of that name is refused as unknown:

- **The watermark** ("SYNTHETIC / SPECIMEN"): drawn last on every page, not movable or removable.
- **The machine-readable zone**: position, size, font and line pitch belong to the engine.
- **Captions, titles or any text that reaches the page**: no field labels, no national title.
  Values come from the seeded generator, never from the file.
- **Fonts, colours, images, emblems and backgrounds**: the page art is fixed and generic.
- **The canvas size and the frame.**
- **Expressions, includes, references, paths and variables.** Coordinates are literal integers.

Country identity lives in art, so a layout that cannot carry art cannot carry a country. That is
the point of the closed schema ([`ADR-0022`](decisions/ADR-0022-declarative-layout-plugins.md)
Decision 2).

## 6. Identity

A layout's identity is the SHA-256 of `ValidatedLayout::canonical_bytes()`, 197 bytes: the 16-byte
tag `synthpass-layout`, `schema_version` as a `u32` little-endian, one byte for the format (`1`
TD1, `2` TD2, `3` TD3, `4` MRV-A, `5` MRV-B), then for each of the eleven rectangles in the order of
section 2 its `x`, `y`, `width` and `height`, each a `u32` little-endian. It is shown as 64
lowercase hex digits.

Whitespace, key order, `name` and `description` are not in it, so two files with the same
geometry have one identity, and moving any rectangle by a pixel changes it. A built-in's identity is
what that format has always drawn.

## 7. Names of the built-ins

The five built-ins are named `icao-<format>-builtin`: `icao-td1-builtin`, `icao-td2-builtin`,
`icao-td3-builtin`, `icao-mrva-builtin` and `icao-mrvb-builtin`. `icao-` marks the generic ICAO
9303 arrangement (no country), the middle is the file's `format`, and `-builtin` marks the layouts
the engine ships. Layouts you write should not use the `-builtin` suffix. Names are labels for
sidecars and manifests, not identities: two files may share a name, and the identity tells them
apart.

## 8. Writing one

1. Pick a `format` and read its canvas and permitted area in section 3.
2. Copy that format's `layouts/<format>.json`, set a new `name`, and move rectangles.
3. Keep every rectangle in the permitted area, no text field under 28 px tall, and no two
   rectangles overlapping. Keep each width at or above the built-in's, and widen it if you make the rectangle taller (text is 70 % of the height, section 4).
4. Load it. The first message names the field and the rule that failed. Fix it and load again.

Do not edit the files in `layouts/`: they are written by the crate's emitter, and a test requires
them to equal its output byte for byte.
