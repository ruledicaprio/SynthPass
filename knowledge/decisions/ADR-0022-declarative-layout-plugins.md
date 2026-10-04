# ADR-0022 — Declarative layout plugins: a closed, geometry-only schema, checked at load time

**Status:** Proposed
**Date:** 2026-09-27

## Context

M8's first Definition-of-Done criterion is *"A third-party layout definition drives generation
without a code change"* ([`ROADMAP.md`](../ROADMAP.md), milestone table, M8 row).
[`ADR-0002`](ADR-0002-provider-model-before-layout-plugins.md) narrowed the old "plugin
architecture" to *"declarative layout plugins — data-driven document definitions"* and moved the
stable-interface criterion to M7. A layout plugin is therefore data, not code. Four facts shape
the design.

**The ethics guardrails hold today because nothing can be supplied.** `README.md` promises that
every generated document carries *"a mandatory synthetic watermark and a generic, non-country
template, enforced in code"*; [`VISION.md`](../VISION.md) §4 calls these ethics guardrails, and
[`BRANDING.md` §4](../BRANDING.md#4-messaging) says the generator exists never to imitate genuine
credentials. In code, `render.rs` draws the watermark last and unconditionally, and
`tests/watermark.rs` checks that its band has ink. Nothing tests "non-country": it holds because
`layout::for_format` returns one of five fixed constants and offers no way to pass another. The
template is generic; the data is not. Issuing states are drawn from a pool that includes real
codes (`data.rs`, `COUNTRY_CODES`).

**A test cannot check a file it never sees.** CI runs on files in the repository. A third-party
layout is loaded later, on another machine. Every rule a test would check has to run at load
time, on every file, and fail closed; the tests then check the checker.

**Country likeness cannot be decided from a file.** ICAO 9303 fixes the arrangement of a data
page — portrait, data zones, machine-readable zone at the bottom — so box positions do not
separate a national document from the generic one. Country identity lives in the art: emblem,
national title, security background, fonts, colours. [`ADR-0009`](ADR-0009-generator-as-a-service.md)
§2 records that country-accurate templates in a public MIT repository are a decision only the
user can take. A schema able to express art would take that decision by implication.

**The fixed geometry is what the benchmarks measure.** `Labels` are accurate by construction
because the generator knows each box before drawing it. The canvas, the watermark band and the
MRZ lines are the values behind the synthetic Tier-1 rates and the M4 gate, and the TD1
watermark-to-MRZ spacing was widened after a closer band broke MRZ detection (`layout.rs`,
`td1_layout`). `PageLayout` exposes the watermark as a plain `Rect`, so a layout file carrying it
could shrink it or move it off the canvas.

## Decision

**1. A layout file states the geometry of a closed set of fields, and nothing else.** It names
one ICAO format (`td1`, `td2`, `td3`, `mrva`, `mrvb`) and gives one rectangle each for the
portrait and the ten visual-zone fields the generator draws today. All eleven are required. It
also carries `schema_version`, a `name` and an optional `description`; neither string is ever
drawn. Coordinates are absolute integers. There are no expressions, includes, references, paths
or variables.

**2. The schema cannot express art or content.** It has no key for an image, emblem, background,
font, colour, caption, title or any text that reaches the page, and none for a field's value.
Issuing state and every value come from the seeded generator. Captions or a title, if ever
added, come from a closed vocabulary the generator owns, by amendment to this ADR. None is
planned for M8.

**3. The engine owns the canvas, frame, watermark and machine-readable zone, per format.** They
are exactly what the built-ins define today. A layout cannot move or resize zone VII and has no
key for the watermark, which is drawn last. Every layout rectangle must lie inside the frame and
above the watermark band, so nothing can sit on the watermark or the MRZ, and no layout brings
the visual zone closer to the MRZ than a built-in does.

**4. Checks run at load time and fail closed.** Parsing rejects unknown and duplicate keys at
every level, a `schema_version` other than 1, a non-integer or negative coordinate, and input
over 64 KiB. Validation rejects an empty rectangle, arithmetic overflow, a rectangle outside the
permitted area, overlapping rectangles, a visual-zone rectangle shorter than the shortest
built-in row, and a visual-zone rectangle too small for the widest value the generator can draw
into that field with the embedded font. The last rule keeps every value's ink inside its
labelled box, which is what keeps `Labels` accurate by construction.

**5. Validation lives in `synthpass-gen`; parsing lives in a new `synthpass-layout` crate.** The
renderer accepts only a `ValidatedLayout`, whose one constructor runs the checks above, so a
library caller cannot skip them either. `synthpass-gen` stays free of `serde`
([`ADR-0007`](ADR-0007-dataset-export-format.md)). `synthpass-layout` depends on `synthpass-gen`,
`serde`, `serde_json` and `sha2`, all already in `Cargo.lock`. The format is JSON.

**6. The five built-ins go through the same path.** Each is committed as JSON in
`synthpass-layout`, and the CLI loads built-ins through the same parser and checks as any other
file. Tests pin that each parsed built-in equals its Rust definition, that the committed JSON
equals the emitter's output, and that renders are byte-identical to golden hashes recorded
before the change.

**7. A layout has an identity**: SHA-256 over a canonical, versioned encoding of the validated
geometry, not of the file's bytes. It is written to the `generate` sidecar and the `export`
manifest with the layout's name. The same seed, layout identity and generator version give
byte-identical pixels. A layout does not change which identity a seed produces.

**8. First-cut surfaces:** `synthpass generate --layout FILE`, then `synthpass export --layout
FILE`, local files only. `synthpass-serve` takes no layouts. `synthpass-bench` and the M4 gate
stay on the built-ins. For M8's criterion, a third-party layout is a file written from the schema
reference alone, with no Rust change, and committed as an example.

**9. Distance to real specimens is a CI tripwire, not enforcement.** In its own advisory
workflow, which fetches the pinned corpus and runs on generator and layout changes, a test
compares renders of the built-ins, the example layouts and random accepted layouts with each
specimen of the same format, using a margin calibrated from distances between editions of the
same state. It ships only if that calibration shows it separates the two; otherwise the
calibration is recorded and the check is rejected.

## Alternatives rejected

**A watermark rectangle in the file, with minimum bounds.** It reproduces the built-ins, but
turns the guardrail into limits a file negotiates with. Engine ownership is simpler, and no
built-in needs anything else.

**A movable MRZ.** Zone VII is what Tier-1 grading reads. A plugin-placed band puts an unmeasured
variable into every number computed on plugin output.

**Captions, titles, colours, fonts or images in the first cut.** Each carries national identity,
and fonts and images add untrusted binary parsing. Loosening a schema later breaks no file;
tightening it after third-party files exist breaks them.

**A load-time likeness check using fingerprints of specimens.** Geometry does not discriminate,
the corpus covers only the states it holds, and the binary would ship data derived from
specimens whose reuse terms differ by source (ADR-0009 §2). Making art unexpressible is stronger
than detecting it.

**Parsing in `synthpass-gen`, `synthpass-cli` or `synthpass-export`.** The first reverses the
posture ADR-0007 records; the second strands logic in a thin binary that the exporter cannot
reuse; the third puts two concerns in one crate.

**JSON-only built-ins with no Rust definition.** Every consumer of `synthpass-gen` would need the
parsing crate, or the generator would need `serde`. The pinned equality gives the same guarantee.

**TOML, YAML or RON.** None is in `Cargo.lock`. TOML's comments are a real loss.

**Scripting, WASM or dynamic libraries.** Out of scope under ADR-0002.

## Consequences

**Positive**

- Both guardrails hold for any file the tool accepts, because the schema has no way to break them.
- ADR-0009's country-template decision stays with the user.
- The built-ins do not move, and hashes prove it without re-measuring any benchmark.
- The validated layout is the geometry COCO and YOLO need ([`EXPORTS.md`](../EXPORTS.md),
  "Deferred"), and its closed field set is most of a class taxonomy. That work stays a separate
  M8 item.

**Negative**

- A plugin can rearrange only the visual zone of five fixed canvases.
- The built-ins exist in two encodings, held equal by tests.
- The workspace gains a crate to maintain.
- Widening a generator value pool can make a previously accepted layout fail the fit check. It
  fails closed, naming the field.
- Box positions alone can echo a real document's arrangement. Without an emblem, national text, a
  security background or a removable watermark, the output stays unmistakably synthetic. What
  happens to an image after the tool writes it is beyond any generator's control, and a fork can
  delete any check (ADR-0009 §2).

**Explicitly not licensed by this decision:** art or free text on the page, layouts supplied over
a network, a movable MRZ or watermark, executable plugins of any kind, country-accurate
templates, and any crate not already in `Cargo.lock`.

## Amendment 1 (2026-10-03) — visual-zone text starts inside its box; M8 PR 1 re-blesses the changed goldens

**Status of this amendment:** Accepted (maintainer, 2026-10-03).

Decision 4's fit check found ink one pixel left of the labelled box wherever a value starts with a
left-hanging glyph, on all five formats. The maintainer chose to start the pen that many whole pixels
further right (`render::pen_start`), in drawing and measuring alike, so the ink lands inside the box
and the labels stay accurate by construction. The glyphs `J V Ж Л Ъ` hang one pixel left at both row
heights. Over seeds 0..1999, 470 of 2,000 renders change on each of TD1, TD2 and TD3 (23.5 %), and all
2,000 on MRV-A and MRV-B, whose document code is `V`. By field: surname 135, document number 118,
personal number 112, issuing country 107, nationality 107, given names 49. The 13 TD goldens recorded
by PR 0 are unchanged, because their seeds carry none of these glyphs. M8 PR 1
([#697](https://github.com/ruledicaprio/SynthPass/pull/697)) therefore re-blesses 8 of PR 0's 21
golden hashes and the seed-565 PNG pins in `synthpass-cli`; the labels hashes do not move. Decision 6's
byte-identity holds from #697's goldens on.
