# Two of the seven `no_mrz_found` documents have no ICAO zone to find

**Date:** 2026-09-13 · **MAIN:** `715538a` · **DATA:** `f906d1f` · **Evidence:** unlabelled (pre-standard) · **Status:** superseded by [README.md#current-headline-numbers](README.md#current-headline-numbers)

**Superseded by:** the live headline in
[`README.md#current-headline-numbers`](README.md#current-headline-numbers).

**2026-09-13.** The manifest review
[`orientation-fix-2026-09-12.md`](orientation-fix-2026-09-12.md) asked for, before its four
unattributed detection failures count as ADR-0008 targets. Read-only on the corpus; no
benchmark was run and no extraction code was touched.

Four documents were named because `samples/corpus.jsonl` records no document code and no
issuing state for them, so nothing verified said what their zone should read. Opening the four
images splits them **two and two**: France and Italy print fully conforming TD1 zones and are
genuine detection targets; **Sweden 2027 is a card front with no zone at all, and the
Netherlands licence carries a single 30-character line that is not an ICAO 9303 zone in any
format.** Both of the latter are in the scored denominator today, and one of them is there
because OCR put it there.

## Per document

| Document | Printed zone | Conforms? | Manifest says | Coverage doc says | Recommended bucket |
| :-- | :-- | :-- | :-- | :-- | :-- |
| France ID 2020 back | **TD1, 3 × 30** | yes — all four check digits validate | `present: true`, code/state `null` | FRA: "Passport, ID card — HIT (+ 2 checksum-failed)" | **scored detection target** (no change) |
| Italy CIE 2022 back | **TD1, 3 × 30** | yes — all four check digits validate | `present: true`, code/state `null` | ITA: "Passport, ID card — HIT (+ 1 checksum-failed)" | **scored detection target** (no change) |
| Sweden ID 2027 | **none** — card front | n/a | `present: true` (from the `_mrz` in the filename) | SWE: "**No specimen yet**" | **`no_mrz_expected`** |
| Netherlands driving licence | one line, 30 chars, no `<` fillers | **not ICAO** — no 9303 layout is one line | `present: true` (**from OCR**, see below) | NLD: "Driving license (**no MRZ**)" | **`no_mrz_expected`** |

None of the four has a hand-transcribed `samples/ocr_fixtures/<stem>.json`. The only Swedish
fixture in the corpus is `Sweden_ID_Specimen_2022_back_mrz.json`, a different document (the
`checksum_failed` ID-card back).

### France ID 2020 back — genuine target, and now transcribed

`samples/id_cards/France_ID_Specimen_2020_back_mrz.png`, 4584 × 2903:

```text
IDFRAX4RTBPFW46<<<<<<<<<<<<<<<
9007138F3002119FRA<<<<<<<<<<<6
MARTIN<<MAELYS<GAELLE<MARIE<<<
```

Three lines of exactly 30 — ICAO 9303 Part 5 TD1. Document number `X4RTBPFW4` check `6`,
DOB `900713` check `8`, expiry `300211` check `9`, composite check `6`: **all four computed
values match the print.** The document number is independently corroborated by the card's own
right-edge vertical printing, `X4RTBPFW4`. The zone is large, unoccluded and high contrast.
This is a detection failure on a conforming zone and belongs exactly where it is.

It is also the document the orientation writeup's untested band-squeeze hypothesis is about:
at 4584 px it is the only `no_mrz_found` from a scan wider than 1600 px. That hypothesis is
untouched here.

### Italy CIE 2022 back — genuine target, despite the overprint

`samples/id_cards/Italy_ID_Specimen_2022_back_mrz.jpg`, 637 × 403:

```text
C<ITACA00000AA4<<<<<<<<<<<<<<<
6412308F2212304ITA<<<<<<<<<<<0
ROSSI<<BIANCA<<<<<<<<<<<<<<<<<
```

TD1 again, all four check digits validate. A diagonal magenta `FACSIMILE` overprint crosses the
left third of the zone; the glyphs stay legible through it, so this is **not** `redacted_mrz`.

The check digit also settles a transcription ambiguity worth recording, because it is the same
O-versus-zero confusable that costs this corpus documents elsewhere. Read as the letter O,
`CAOOOOOAA` computes a check digit of **0**; read as zeros, `CA00000AA` computes **4**, which is
what is printed. The arithmetic decides it, not the glyph shapes.

This specimen is one of the four the [README](README.md) has flagged since 2026-08-16 as
resolving to TD2/MRV-B rather than the ID-card-shaped TD1. The printed zone is unambiguously
TD1, so that cross-format confusion is the pipeline's, not the specimen's.

### Sweden ID 2027 — a card front, mislabelled `_mrz`

`samples/id_cards/Sweden_ID_Specimen_2027_mrz.png`, 1024 × 622, is the **front** of the Swedish
national identity card: EU flag, portrait, `SPECIMEN` / `SVEA`, personal number `820821-2384`,
card number `XA0000002`, expiry `01 JAN/JAN 27`, CAN `123456`, signature line. The bottom strip
below the signature is guilloche and card edge. **There is no machine-readable zone on this
image.** The Swedish ID card's TD1 zone is on the back — `Sweden_ID_Specimen_2022_back_mrz.jpg`
is the corpus's example of one.

The manifest agrees with the image on what OCR saw (`read_by_ocr: false`) and disagrees with it
on what the document is (`present: true`). `present` is true because the **filename** says
`_mrz`, and per [`samples/README.md`](../../samples/README.md) that tag "is not optional" — a
human wrote it and wrote it wrong. The correct name is
`Sweden_ID_Specimen_2027_front_no_mrz.png`, matching the `_front_no_mrz` / `_back_mrz` pairs
France and Italy already carry.

### Netherlands driving licence — a real machine-readable line, not an ICAO zone

`samples/driving_licenses/Netherlands_Driving_License_Specimen.jpg`, 1024 × 670. The bottom edge
carries one line:

```text
D1NLD362361864332VD9S5D36V7R25
```

Thirty characters, one line, **no `<` fillers anywhere**. No ICAO 9303 layout is one line: TD1
is 3 × 30, TD2 and MRV-B are 2 × 36, TD3 and MRV-A are 2 × 44. Reading it as a TD1 line 1
anyway — document code `D1`, state `NLD`, document number at positions 6–14 — gives a computed
check digit of 9 against a printed 3; the 10-digit variant gives 0 against 3. Nothing in it
satisfies the 7-3-1 rule at any ICAO position tested. The structure (`D1` + `NLD` + the licence
number, which the card's field 5 prints as `6236186433`) is the EU/ISO-18013 driving-licence
line, *inferred* from its shape; what is *measured* is only that it is not ICAO.

So `CORPUS_COVERAGE.md` is right — "Driving license (no MRZ)" — and `samples/corpus.jsonl` is
wrong. The contradiction the orientation writeup flagged resolves in the coverage doc's favour.

## Why the Netherlands row is wrong, which matters more than that it is

`mrz.present` is not a human record for this file. `corpus_manifest.rs` derives it as:

```rust
"present": claims.mrz_present.unwrap_or(observed.found),
```

`claims.mrz_present` comes from the `_mrz` / `_no_mrz` token in the filename.
`Netherlands_Driving_License_Specimen.jpg` carries neither, so the field falls through to
`observed.found` — **what OCR returned on this image**, which was a non-validating TD1 with
document code `A<` and issuing state `ADW`. `synthpass_bench::MrzExpectations::get` reads
`mrz.present` before it reads the filename, so that hallucination is what put this document in
the Tier-1 denominator.

Three lines above that expression sits the comment *"The manifest asserts only what a human
verified… There is deliberately no OCR fallback here."* It is accurate about
`document_code` and `issuing_state` and not about the field immediately above them.

A census of the manifest finds **exactly three rows whose filename carries neither tag**, and
therefore exactly three whose denominator membership OCR decided:

| File | `observed.read_by_ocr` | `present` | Right? |
| :-- | :-- | :-- | :-- |
| `Netherlands_Driving_License_Specimen.jpg` | true | true | **no** |
| `Bosnia_Herzegovina_Driving_License_Specimen_face.gif` | false | false | yes, by luck |
| `Bosnia_Herzegovina_Driving_License_Specimen_front.gif` | false | false | yes, by luck |

`synthpass-bench/src/lib.rs`'s own doc comment cites the two Bosnian licences as the case where
"the manifest records them correctly" and the filename tag does not. It records them correctly
because `ocrs` happened to return nothing on a GIF — the identical mechanism that got the
Netherlands wrong. One in three is not a fallback working.

This is the same class as the 2026-09-09 redaction and non-conformance bugs: **a property of
the document decided by what the run returned.** It is one document wide today because the
naming convention covers the rest.

## What the two reclassifications do to the numbers

Baseline `real-specimen-mrz-baseline.json`, CI sha `715538a`, measured 2026-09-11, `samples_data_sha`
`f906d1f`. The "after" column is arithmetic on that baseline, not a measurement:

| | baseline (measured) | after reclassification (modelled) |
| :-- | --: | --: |
| `documents` | 254 | 254 |
| `scored` | 157 | **155** |
| Tier-1 HIT | 143 | 143 |
| **hit rate, scored** | **91.1%** | **92.3%** |
| **hit rate, whole corpus** | **56.3%** | **56.3%** |
| `no_mrz_found` | 7 | **5** |
| `no_mrz_expected` | 43 | **45** |
| `no_mrz_found` : `checksum_failed` | 7 : 7 | **5 : 7** |

Both denominators are quoted because only one of them moves. The corpus-wide rate is unchanged
at 143 / 254: a drawer of documents containing this Swedish card front and this Dutch licence
still yields the same 143 records. The scored rate rises 1.2 pp with zero extraction change,
for the same reason the 2026-09-09 correction did — the denominator stops counting documents
that cannot produce a hit.

The consequence for ADR-0008 is the ratio, not the rate. Detection has not been level with
character accuracy since 2026-09-11; it has been **behind it, 5 to 7**. The five genuine
detection targets are:

```text
France ID 2020 back        Argentina P0_ARG_2021_mrz_child    Moldova PA_MDA_2014
Italy ID 2022 back         Egypt P0_EGY_2012
```

## Recommended changes

Both fixes are **renames on `samples-data`**, not manifest edits, and that is not a style
preference. `mrz.present` is re-derived from the filename on every
`cargo run -p synthpass-ocr --release --example corpus_manifest` and is not among the fields
carried forward from the previous row (`notes`, `year.kind`, `provenance` are). A hand-edited
`present: false` would survive until the next regeneration and then silently revert.

1. `samples/id_cards/Sweden_ID_Specimen_2027_mrz.png`
   → `Sweden_ID_Specimen_2027_front_no_mrz.png`.
2. `samples/driving_licenses/Netherlands_Driving_License_Specimen.jpg`
   → `Netherlands_Driving_License_Specimen_no_mrz.jpg`.
3. Regenerate `samples/corpus.jsonl`; both rows should land `present: false`, and the basename
   keys in `ground_truth_stem` / `expected_document_number` are `null` for both, so nothing else
   re-links.
4. `knowledge/CORPUS_COVERAGE.md`: the **SWE row says "No specimen yet"** and the corpus holds
   two Swedish ID specimens (the 2022 back, a named `checksum_failed` with a fixture, and this
   2027 front). Same staleness as the RUS row corrected on 2026-09-12. The NLD row is correct
   and needs no change.
5. Optional, and a separate PR: promote the France and Italy transcriptions above to
   `samples/ocr_fixtures/<stem>.json`. Both are check-digit-verified, which is the bar the
   2026-09-08 step-3 fixtures were held to. That touches a gate path and gives each document an
   `expected_document_number` for the day it reads.

**The reclassification owes a CI re-bless in the same PR, and the gate will not ask for one.**
`tier1_hits` is unchanged at 143, `no_mrz_found` falls 7 → 5 and the gate fails only on an
*increase*, `documents` stays 254 so the corpus-changed warning does not fire either. The change
is completely silent to `real-specimen-gate.yml`. Merging it without re-blessing leaves a
`no_mrz_found` ceiling of 7 standing over a corpus whose true value is 5 — two regressions of
headroom, on a `tolerance: 0` gate, indefinitely. This is the deferred-re-bless failure from
2026-09-12 in its quieter form: there, the gate was disarmed by fourteen hits and everyone knew
the re-bless was pending.

Fix the manifest generator separately, and do not bundle it: `present` should record the
filename claim or nothing, with `observed.found` left in `observed` where the rest of the
machine's output lives, and the three untagged names brought into the convention so the
fallback has nothing to decide.

## What this does not claim

**No measurement was taken.** The "after" column is arithmetic applied to the 2026-09-11 CI
baseline, and only CI can produce the real one. Nothing here says France or Italy will read
after any particular change; they remain unread detection failures, and this finding only
establishes that they are worth aiming at.

**It does not claim the Netherlands line is unreadable** — only that it is not an ICAO 9303 MRZ
and that SynthPass's reader has no business validating it. Whether the EU driving-licence line
is worth supporting is a product question for `VISION.md`, not a denominator question. Nor does
it claim the line is *harmless* where it is: the reader currently returns a non-validating TD1
from that page, and a non-validating read on a `no_mrz_expected` document is scored
`no_mrz_expected`. Should a future parser ever validate that line, the same document becomes
`false_positive_mrz` and fails the build — which is the correct and intended outcome, and worth
knowing about before it happens.

**The `present` census covers this corpus, not the mechanism.** Three untagged filenames today
is a fact about 254 files. The generator will fall back to OCR for the next untagged name too.

**Sweden 2027's reclassification rests on the image alone.** No back-side scan of this specimen
exists in the corpus to pair it with; the claim is that *this file* carries no zone, not that
this Swedish card design lacks one — it does not, and the 2022 back proves it.

## Addendum, later on 2026-09-13 — two of the "five genuine targets" are not

The five detection targets listed above included Egypt `P0_EGY_2012` and Argentina
`P0_ARG_2021_mrz_child`. This review never opened either: both rows record a code and a state, so
neither was among the four it set out to check. Opened since, neither can yield a hit. Egypt's two
zone lines are pixel-masked by the publisher — only the `P<EGY` and `A…EGY` prefixes and the `<`
fillers survive — so it belongs in `redacted_mrz`. Argentina's printed zone, transcribed character
for character, fails four of its five ICAO check digits (only the document number validates), so a
byte-perfect read still fails: `checksum_failed_specimen`. Both moved in the same PR as the two
renames above, and CI measured the four together. The detection targets are **France, Italy and
Moldova**; see [`README.md`](README.md#current-headline-numbers) for the live counts.

## Candidates considered and rejected

- **Scoring the Netherlands licence as `checksum_failed_specimen`.** Tempting, because a
  30-character line that fails ICAO check digits is literally a printed zone failing its own
  checks. Rejected: that bucket means "an ICAO zone whose check digits are wrong", and reaching
  it requires `mrz_expected: true`, which is the error being corrected. A non-ICAO line is not a
  non-conforming ICAO line.
- **`redacted_mrz` for Italy.** The `FACSIMILE` overprint crosses the zone. Rejected on the
  image: every glyph under it is legible and the transcription check-digit-validates, so a
  correct read is reachable and the redaction rungs do not apply.
- **Treating the France and Italy `document_code` / `issuing_state` nulls as a manifest defect
  to fill in.** They are null because the `id_cards/` naming form predates the `Code`/`State`
  slots ([`samples/README.md`](../../samples/README.md)), not because anyone failed to record
  something. `source: "unrecorded"` is the generator saying so out loud, which is the design.
  The fix for these two is a fixture file, not a filename.
- **Hand-editing `mrz.present` in `samples/corpus.jsonl`.** The one-line change that reverts on
  the next manifest regeneration; see above.

## Exact invocations

No benchmark was run. Everything above is from the images, the manifest and the source:

```bash
# the four images, read directly, plus Pillow crops at 3-4x for the two small MRZ bands
python -c "from PIL import Image; ..."   # artifacts/manifest-review-2026-09-13/*.png

# check digits, ICAO 7-3-1, TD1 composite over L1[6:30] + L2[1:7] + L2[9:15] + L2[19:29]
python -                                  # France and Italy: 4 of 4 each, both OK

# the untagged-filename census
python -c "import json; rows=[json.loads(l) for l in open('samples/corpus.jsonl')]; ..."
```

Source read, not run: `crates/synthpass-ocr/examples/corpus_manifest.rs` (the `present`
derivation, line ~545, and `FilenameClaims::parse`), `crates/synthpass-bench/src/lib.rs`
(`MrzExpectations::get`), `crates/synthpass-bench/src/provider_bench.rs` (the outcome rungs).
