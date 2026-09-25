# The twelve scored misses, attributed: the check digits see a sixth of the damage

**Date:** 2026-09-19 · **MAIN:** `bb658d6` · **DATA:** `396b22f` · **Evidence:** Observed (`provider-bench --real-specimens --mrz-only --dump-ocr`, all `SYNTHPASS_OCR_*` arms cleared; 261 documents, 31 dumped misses) · **Status:** current

The re-blessed baseline leaves **twelve scored misses over 152 scored documents**. Every one now
carries a reviewed ground-truth fixture — coverage went 7/13 to 12/12 across #336, #343 and #348 —
so every future flip in either bucket is attributable to a named document.

All twelve are attributed here from a single instrumented run at `bb658d6`: fixture, field, observed
failure, evidence. The instrumentation (`mrz_field_mismatch`, #344/#347) reports which ICAO field
each differing position falls in and whether any check digit covers it, so the hand alignment of
2026-09-18 is now reproducible by command.

**No MRZ content, holder name, character value or zone text appears here.** Defects are described by
ICAO field and 0-based position within a line, the convention
[`checksum-failed-miss-mechanisms-2026-09-18.md`](checksum-failed-miss-mechanisms-2026-09-18.md)
established. The dump itself carries `raw_ocr_text` and `ground_truth_mrz` and stays in gitignored
`artifacts/`.

## The headline

**The check digits are blind to 82% of the damage.** Across the nine misses that carry a field
attribution:

```
characters wrong in a checksum-covered field       33
characters wrong where no check digit reaches     154      4.7x
```

Nearly five times more wrong characters sit outside every ICAO check digit than inside one —
overwhelmingly the name field, plus Germany's issuing country, Sweden's nationality and Hong Kong
2019's sex. [`checksum-failed-miss-mechanisms-2026-09-18.md`](checksum-failed-miss-mechanisms-2026-09-18.md)
saw this shape on six documents by hand; it holds at scale, on named documents, from instrumentation.

A second consequence: the outcome buckets and the mechanisms disagree. The bench reports **2
`no_mrz_found` and 10 `checksum_failed`**; by mechanism the split is **3 detection or segmentation
and 9 recognition** (see [§4](#4-the-one-in-the-wrong-bucket)).

## 1. Detection misses — 2, both TD1 card backs

Both carry a fixture created in #348, both fixtures are checksum-valid, and both are the corpus's
first MRZ-only fixtures (`visual_zone_present: false`, #346's key).

| document | truth format | band score | observed failure | evidence |
| :--- | :--- | ---: | :--- | :--- |
| `France_ID_Specimen_2020_back_mrz` | TD1 | 0.361 | band located, no MRZ parsed; pass list exhausted at 62 441 ms | [detection-misses-2026-09-18](detection-misses-2026-09-18.md) |
| `Italy_ID_Specimen_2022_back_mrz` | TD1 | 0.677 | band located, no MRZ parsed; exhausted at 8 123 ms | [detection-misses-2026-09-18](detection-misses-2026-09-18.md) |

**France — a preprocessing variant gap, and the most actionable document in the twelve.** All four
printed check digits were independently verified against ICAO 7-3-1 weights, and the document number
is corroborated by a second printing on the card's right edge. The browser stack (`tesseract.js`)
produced a checksum-valid read of the same bytes on its **third** pass, a contrast stretch, in
1 305 ms. The native variant chain never tries that transform.

**Italy — a magenta FACSIMILE watermark** crossing the band. It closes by explanation: no variant
reaches it, and the browser misses it too.

**Both now carry a band score**, where the 2026-09-18 entry describes France as finding nothing. The
band is being *located* and not *parsed* — which is a different defect from the one recorded, and a
narrower one.

### Two corrections to the France attribution

The 2026-09-18 entry attributes France partly to a **budget exhaustion** — 66 705 ms against a 52 s
budget, `retry_budget_hit: true`; Phase D recorded 72 227 ms. **The committed ledger at `bb658d6`
no longer says that:** `ocr_ms: 62441`, `retry_budget_hit: false`, `retry_stop: "exhausted"`.

So the budget-stopped case has left the population. **All twelve now stop by exhausting the pass
list, none by hitting the clock.** Raising `MAX_SECONDS` buys nothing anywhere in this set. Do not
cite France as budget-limited.

## 2. Recognition misses — 9, all attributed

Positions are 0-based columns; line numbers are 1-based and match `field_mismatch_positions`. The
last two columns split the damage by whether any check digit can see it.

| document | fmt | band | differing positions | covered | uncovered |
| :--- | :--- | ---: | :--- | ---: | ---: |
| `Afghanistan_…_AFG_2016` | TD3 | 0.898 | `L2:0` | 1 | 0 |
| `Czechia_…_CZE_2005` | TD3 | 0.772 | `L1:29` `L2:33,41` | 2 | 1 |
| `Romania_…_ROU_2024` | TD3 | 0.525 | `L1:12-25,27-29,34-38` `L2:41` | 1 | 22 |
| `Germany_…_D00_2024` | TD3 | **null** | `L1:1-21,42` `L2:1` | 1 | 22 |
| `Hong_Kong_…_HKG_2019` | TD3 | 0.765 | `L1:1,9,11-43` `L2:0,20` | 1 | 36 |
| `Sweden_ID_…_2022_back` | TD1 | 0.399 | `L2:13-17` `L3:0-21` | 2 | 25 |
| `Belgium_ID_…_2021_back` | TD1 | 0.794 | `L2:20-21,24-29` `L3:9-17` | 8 | 9 |
| `Hong_Kong_…_HKG_2007` | TD3 | 0.849 | `L1:1,13,16,19-21,23-24,30,32-40,42` `L2:1-3,8-9,37-38` | 7 | 19 |
| `Croatia_ID_…_2021_back` | TD1 | **0.315** | `L1:5-14` `L3:0-1,3,5-21` | 10 | 20 |
| | | | **total** | **33** | **154** |

**All six documents attributed by hand on 2026-09-18 reproduce exactly** — Afghanistan 1 cell at
position 0, Czechia 2 at (33, 41), Romania 1 at 41, Sweden 5 at 13–17, Belgium 8, and Russia's
112-character whole-zone mismatch with no attribution. Same figures, different commit, different
corpus pin, instrumentation instead of hand alignment. The alignment is deterministic.

**Two structural facts about ICAO 9303 do most of the interpretive work, and neither is a property
of this pipeline:**

- **One wrong character can fail two checks.** Afghanistan's single miscoded cell fails
  `document_number` *and* `composite`, because the composite's input includes the document number.
  So "five documents fail the document-number check" is not "five documents have five
  document-number defects".
- **A `check_states` map whose only `false` entry is `composite` is a positional fingerprint.**
  TD1's optional data carries no check digit of its own, so only the composite can observe an
  error there. Belgium's historical failed-only signature localised its defect before a character
  was examined.

### What causes each checksum failure

Grouped by the shape of the damage **on the check-digit-bearing line** — the only damage that makes
these documents misses at all:

| shape | n | documents |
| :--- | ---: | :--- |
| **One or two isolated cells** | 5 | Afghanistan `L2:0` · Romania `L2:41` · Germany `L2:1` · Czechia `L2:33,41` · Hong Kong 2019 `L2:0,20` |
| **One contiguous run** | 2 | Sweden `L2:13-17` (5) · Croatia `L1:5-14` (10) |
| **Several runs in one field** | 1 | Belgium `L2:20-21,24-29` (8, all in `optional_data_2`) |
| **Scattered across both lines** | 1 | Hong Kong 2007 (7 cells, 3 runs on line 2) |

**The majority of these misses are one or two cells.** Afghanistan's is 1 of 88 — the narrowest
recognition miss the format permits.

**Croatia is a single glyph confusion applied ten times, and run length hides that.** `L1:5-14` is
the *entire* document number plus its check digit, ten contiguous cells, on the lowest band score in
the set (0.315) — while line 2 is character-for-character correct. Ten contiguous wrong cells looks
like a line recovered at the wrong offset. It is not: see [§3a](#3a-croatia-one-confusion-class-ten-cells-and-a-repair-that-is-capped-at-one).

**So run shape is descriptive, not diagnostic.** The table above groups the damage; it does not name
the mechanism. Croatia and Sweden are both "one contiguous run" and have nothing else in common.
Naming a mechanism takes the glyphs, and three of the nine are now named — Croatia here, the two
Hong Kong passports in [§3](#3-a-named-failure-mode-filler-read-as-glyphs).

### Repairing the check digit does not repair the document

Sweden's five covered cells straddle the expiry/nationality boundary: positions 13 and 14 are the
expiry's last digit and its check digit, and **15–17 are all three nationality characters**.
Nationality and sex are excluded from the composite on **both** TD1 and TD3. Repairing the expiry
alone would turn Sweden into a clean Tier-1 *hit* still carrying a wrong nationality.

Romania is the same lesson in the other direction: 1 wrong character on line 2 and 22 on line 1.
Calling it "a personal-number miss" describes 1/23rd of what is wrong with the read. This is
[ADR-0013](../decisions/ADR-0013-names-are-scored-against-mrz-form-truth.md) measured on named
documents.

## 3. A named failure mode: filler read as glyphs

Splitting each name line at the last non-filler character separates two different failures — a name
misread, and filler misread as characters. The second is measurable: count the truth-filler cells
the read filled with a non-`<` glyph.

| document | name-line content | name-line filler tail | filler cells given a glyph |
| :--- | :--- | :--- | :--- |
| `Hong_Kong_…_HKG_2019` | 9 / 18 differ | **26 / 26 differ** | **26 of 26** |
| `Hong_Kong_…_HKG_2007` | 4 / 20 differ | 15 / 24 differ | **15 of 24** |
| `Sweden_ID_…_2022_back` | 14 / 14 differ | 8 / 16 differ | 8 of 16 |
| `Romania_…_ROU_2024` | 17 / 30 differ | 5 / 14 differ | 5 of 14 |
| `Croatia_ID_…_2021_back` | 16 / 18 differ | 4 / 12 differ | 4 of 12 |
| `Germany_…_D00_2024` | **21 / 22 differ** | 1 / 22 differ | 1 of 22 |
| `Czechia_…_CZE_2005` | 0 / 19 differ | 1 / 25 differ | 1 of 25 |
| `Afghanistan_…_AFG_2016` | 0 / 25 differ | 0 / 19 differ | 0 of 19 |
| `Belgium_ID_…_2021_back` | 9 / 18 differ | 0 / 12 differ | 0 of 12 |

**The two Hong Kong passports are a distinct failure mode.** Hong Kong 2019 put a glyph in *every
one* of its 26 filler cells; Hong Kong 2007 in 15 of 24. They are also the only two documents in the
twelve that **neither** the native nor the browser stack has ever read
([Phase D](phase-d-native-vs-browser-2026-09-18.md)), and they share an issuer twelve years apart.

### CORRECTED 2026-09-20 — this is **not** the ADR-0014 effect

This entry originally read that the fabrication is *"a real-document instance of what
[ADR-0014](../decisions/ADR-0014-per-cell-ocrb-classification.md)'s premise-2 correction measured
synthetically"* — that the isolated OCR-B `<` is present in the model but suppressed by context.
**That claim was wrong, and it was mine.** The measurement above stands; the mechanism attached to
it does not.

ADR-0014 predicts that an **isolated** filler is suppressed while an **in-run** filler is
comparatively unaffected. The fabricated cells are the opposite distribution
([hong-kong-filler-context-2026-09-19](hong-kong-filler-context-2026-09-19.md)):

| specimen | trailing filler run | fabricated | isolated | boundary | interior (run ≥ 3) |
| :--- | ---: | ---: | ---: | ---: | ---: |
| Hong Kong 2007 | 24 | 15 | **0** | 1 | 14 |
| Hong Kong 2019 | 26 | 26 | **0** | 2 | 24 |

**Zero fabricated isolated fillers in either document**, and both are dominated by interior in-run
cells — including a complete 26-cell tail. That is the case ADR-0014 says is *not* suppressed, so
these documents cannot be evidence for the mechanism. The one-character-short `restored()` path is
ruled out too: both recovered name lines already arrive at the target width, so nothing in that
path could have manufactured a tail.

**So the twelve give ADR-0014 no real-document support.** That does not weaken the ADR — its
synthetic measurement is unaffected — but it removes a prop this entry wrongly attached to it, and
the track still has no named document behind its premise. The fabrication itself remains unexplained
and is worth its own mechanism.

**This also explains Hong Kong 2019's 33-cell "contiguous run".** `L1:11-43` is not one long misread
of the name; it is the name's short content region plus an entirely fabricated filler tail. Run
length on a name line is confounded by filler and should not be read as evidence of a shift.

**And it shows the converse trap.** Afghanistan and Czechia have clean name-line content (0/25 and
0/19), but a naive contiguity reading of Germany's `L1:1-21` would call positions 22–41 "correct"
when both sides are filler there — matching vacuously. Germany's name content is **21 of 22 cells
wrong**. Only the content/filler split separates the two cases.

## 3a. Croatia: one confusion class, ten cells, and a repair that is capped at one

The document number on `Croatia_ID_Specimen_2021_back_mrz` is ten cells of a single character. The
read is ten cells of a single *different* character — the digit/letter pair that heads
[`CONFUSABLES`](../../crates/mrz/src/repair.rs) (`('0', "ODQ")`). Not ten errors: **one confusion
class, applied uniformly, including the check-digit cell.**

Three facts make this the best-characterised miss in the set.

**The same glyph reads correctly sixteen times on the line below.** Line 2 of this specimen contains
16 cells of the digit and every one is read correctly; line 1's ten are all read as the letter. Same
card, same band, same pass. So this is not a legibility failure — it is a **field-alphabet** effect.
TD1 line 2's positions there are numeric-only, while the line 1 document number is alphanumeric, so
the letter is *legal* where it was emitted and nothing downstream rejects it.

**The check digit is the only thing that could have rejected it, and it was misread too.** The two
characters are not check-digit equivalent — ICAO values 0 and 24, giving check digits 0 and 2 over
this field — so the arithmetic does catch the substitution. But the printed check-digit cell is
inside the same ten-cell run and was misread identically, so the comparison runs against a cell that
is not a digit at all.

**The existing repair cannot fix it, by design.** `MAX_SUBSTITUTIONS = 1`
(`crates/mrz/src/repair.rs:343`) is fixed at one deliberately: letting positions vary independently
multiplies the candidate count and re-admits the coincidental agreement `CONFUSABLES` exists to keep
out. That rationale is sound and this case does not contradict it — **a uniform sweep of one class
across one field is a single decision, not ten independent ones**, so it does not widen the search
the cap guards against. Sweeping this field's ten cells and its check digit together yields a zone
whose check digit validates.

### A controlled pair is already in the corpus

| document | outcome |
| :--- | :--- |
| `Croatia_ID_Specimen_2002_back_mrz` | **hit** |
| `Croatia_ID_Specimen_2021_back_mrz` | `checksum_failed` (document_number) |

Same issuer, same document class, both TD1 card backs, one read cleanly and one not. The 2021 card
carries a dense guilloche and wave pattern through the MRZ band where the 2002 card's band sits on a
plainer ground — consistent with its 0.315 band score, the lowest in the twelve. No new data is
needed to use this pair.

**This is the first reproducible fixture-level case in the set**, which is the precondition [§8](#8-what-this-record-does-not-do)
sets before any repair is proposed. It is one document, so it sizes nothing on its own.

## 4. The one in the wrong bucket

`Russian_Federation_Passport_Specimen_P0_RUS_2019_mrz` is recorded as `checksum_failed`, format
`TD1`, failing `document_number`, `date_of_birth` and `composite`.

**The reviewed fixture is 2 × 44 — TD3.** The `TD1` in the ledger is the reader's output, not the
document's format. The reader returned **3 × 30** with `mrz_band_score = null`: the band was never
located, the wrong format was selected, and the check digits were computed over a zone that does not
correspond to the printed one. Whole-zone mismatch **112 characters**. It is the only one of the
twelve whose read and truth disagree on shape.

1. **Its failing-field list carries no information.** A wrong format makes every check-digit position
   wrong. Do not group this document by its failing checks, and do not count it toward the
   document-number tally in §5.
2. **It belongs with the detection misses**, in
   [ADR-0015](../decisions/ADR-0015-geometric-mrz-band-location.md)'s territory. Reclassification is
   **recommended and still not applied** — it moves a baseline classification and needs approval.
3. **It is not mislabelled data.** `mrz_field_mismatch` correctly withholds attribution when the
   truth's shape contradicts the declared format (#347), which is why this row has null field
   mismatch while carrying failing checks. The guard works.

### Germany has the same null band and is *not* the same case

Germany also reports `mrz_band_score = null`, so no band was geometrically located
(`detect_mrz_band_scored` returned `None`, `synthpass-ocr/src/lib.rs:410`) and the zone was parsed
out of general page text. But **its read and its truth agree on shape** (2 × 44), so it is not a
format misdetection, and the mechanism split stays 3 / 9 rather than moving to 4 / 8.

What it is instead is **two separable defects in one document**:

- **Line 2 is 43 of 44 characters correct.** The single wrong cell at position 1 is what fails
  `document_number` and `composite` — a genuine, narrow recognition miss that belongs with
  Afghanistan and Romania.
- **Line 1's content is 21 of 22 characters wrong**, with the band never located. That is a
  detection failure, and no recogniser improvement reaches it.

So Germany's *checksum* failure is one cell; its *name* failure is downstream of the missing band.
A document can be in both buckets at once, and this one is.

## 5. The failing check digits, and why the lead they suggest is wrong

Across the ten `checksum_failed` documents, counting each document's set once:

```
document_number   6      Afghanistan, Croatia, Germany, HKG 2007, HKG 2019, Russia*
personal_number   2      Czechia, Romania
date_of_expiry    1      Sweden
date_of_birth     1      Russia*
composite only    1      Belgium
```

The honest count is **5 of 9** — Russia's failing set is an artefact (§4).

**`document_number` looks like the dominant repair class and is not one.** The five documents that
fail it do so by opposite mechanisms:

- **Afghanistan, Germany, Hong Kong 2019 — one cell each.**
- **Hong Kong 2007 — 4 cells plus its check digit,** scattered, with filler misread elsewhere.
- **Croatia — the entire ten-cell field,** one confusion class applied uniformly ([§3a](#3a-croatia-one-confusion-class-ten-cells-and-a-repair-that-is-capped-at-one)).

A repair aimed at "the document number" would be aimed at five different problems. **What the field
name predicts is nothing; what the glyphs predict is the mechanism** — §3a and the filler split in
§3 name three of the nine, and the damage-shape table in §2 only sorts the rest for inspection.

## 6. An independent axis that partitions the twelve exactly 8 / 4

[Phase D](phase-d-native-vs-browser-2026-09-18.md) ran the browser stack over the same bytes and
population with detection held constant. Its head-to-head classes cover the twelve with no remainder:

| class | n | documents |
| :--- | --: | :--- |
| **Browser reads it, native scores a miss** | **8** | Afghanistan, Belgium, Croatia, Czechia, France, Germany, Romania, Sweden |
| **Both stacks miss it** | **4** | Italy, Hong Kong 2007, Hong Kong 2019, Russian Federation |

- **The eight are demonstrably readable**, so the cause is in the native pipeline, not the document.
  Five are corroborated against hand transcription rather than being merely self-consistent —
  Croatia, Sweden, Afghanistan, Czechia and Romania match the fixture's lines byte for byte, and
  Belgium matches all nine reviewed fields.
- **The four are a different problem**, and §3 and §4 now name three of them: Italy's watermark,
  Russia's format misdetection, and the two Hong Kong passports' filler fabrication.

Phase D notes that two of the browser's wins — **France and Germany** — "carry no fixture and are
unverified beyond their own check digits". Both now have one (#348, #336), so those reads can be
checked against ground truth rather than taken on their own checksums.

## 7. Corrections this record makes to earlier ones

1. **France is no longer budget-limited** (§1). All twelve stop by exhausting the pass list.
2. **France and Italy both carry band scores** (§1) — located, not parsed, where the earlier entry
   describes France as finding nothing.
3. **Russia's `TD1` is the reader's output, not the document's format** (§4). The fixture is TD3, and
   it is the only one of the twelve whose read and truth disagree on shape.
4. **Phase D's "France and Germany carry no fixture"** is stale; both do (§6).

## 8. What this record does not do

It proposes no fix. **Croatia is the first document here with a reproducible fixture-level case**
([§3a](#3a-croatia-one-confusion-class-ten-cells-and-a-repair-that-is-capped-at-one)), and it is one
document — enough to justify a measurement, not a default-on change. The standing rule holds: **no
broad OCR change until each group has such a case.**

The four measurements the data now points at, each one run and no new tooling:

- **A uniform single-class confusable sweep over one field and its check digit**, measured against
  the whole corpus rather than Croatia alone. The question is not whether it fixes Croatia — it
  does — but how many documents it silently *breaks*. That is exactly what the chargrid A/B's
  per-document 2×2 caught: on 500 synthetic documents it fixed 46 names and broke 13 (net `+33`
  `names_exact`; strict hits net `+30`), and on 257 real specimens it fixed one and broke one (net
  `0`) ([reconciliation](chargrid-ab-reconciliation-2026-09-24.md)). Three-arm, same-binary, real arm
  required.
- **A contrast-stretch preprocessing variant on France** — the one document where a named, untested
  transform is known to work on the same bytes in another stack.
- **Why the band is not located on Germany and Russia**, which is
  [ADR-0015](../decisions/ADR-0015-geometric-mrz-band-location.md)'s question with two named
  documents attached.
- **Whether the filler fabrication on the two Hong Kong passports is the effect ADR-0014 measured**
  — and if so, they become the first real-document evidence that track has.

## Scope

Documentation only. No code, no data, no manifest, no `samples-data`, and no baseline number moves.
