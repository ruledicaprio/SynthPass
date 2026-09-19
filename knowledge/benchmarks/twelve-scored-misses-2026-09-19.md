# The twelve scored misses, attributed: two detection, nine recognition, and one filed in the wrong bucket

**Date:** 2026-09-19 · **MAIN:** `bb658d6` · **DATA:** `396b22f` · **Evidence:** Observed (committed outcome ledger, reviewed fixtures, and the dated attribution runs cited per row) · **Status:** current

The re-blessed baseline leaves **twelve scored misses over 152 scored documents**. Every one of the
twelve now carries a reviewed ground-truth fixture — coverage went 7/13 to 12/12 across #336, #343
and #348 — so every future flip in either bucket is attributable to a named document.

This entry is the attribution record for all twelve in one place: fixture, field, observed failure
and evidence. It consolidates and cites the dated runs that did the underlying work rather than
re-deriving them, and it corrects three things those runs could not have known.

**No MRZ content, holder name, character value or zone text appears here.** Defects are described by
ICAO field and 0-based position within a line, the convention
[`checksum-failed-miss-mechanisms-2026-09-18.md`](checksum-failed-miss-mechanisms-2026-09-18.md)
established.

## The headline: the outcome buckets and the mechanisms do not agree

The bench reports **2 `no_mrz_found` and 10 `checksum_failed`**. Grouped by *mechanism* rather than
by outcome, the split is **3 detection or segmentation failures and 9 recognition failures**:

| | By outcome bucket | By mechanism |
| :-- | --: | --: |
| Detection / segmentation | 2 | **3** |
| Recognition | 10 | **9** |

The document that moves is the Russian Federation 2019 passport. It is recorded as
`checksum_failed` because the parser did return a zone and its check digits did not validate — but
the zone it returned is the wrong shape for the document, so those digits were computed over
something that does not correspond to what is printed. See [§4](#4-the-one-in-the-wrong-bucket).

**This matters before any grouping.** Nine of the twelve are reachable by recogniser work. Three are
not, and no recogniser improvement of any quality would move them.

## 1. Detection misses — 2 by outcome, both TD1 card backs

Both carry a fixture created in #348, both fixtures are checksum-valid, and both are the corpus's
first MRZ-only fixtures (`visual_zone_present: false`, #346's key).

| document | truth format | fixture geometry | observed failure | evidence |
| :--- | :--- | :--- | :--- | :--- |
| `France_ID_Specimen_2020_back_mrz` | TD1 | 3 × 30 | no MRZ returned; pass list exhausted at 62 441 ms | [detection-misses-2026-09-18](detection-misses-2026-09-18.md) |
| `Italy_ID_Specimen_2022_back_mrz` | TD1 | 3 × 30 | no MRZ returned; pass list exhausted at 8 123 ms | [detection-misses-2026-09-18](detection-misses-2026-09-18.md) |

**France — a preprocessing variant gap, and the most actionable document in the twelve.** All four
printed check digits were independently verified against ICAO 7-3-1 weights, and the document number
is corroborated by a second printing on the card's right edge. The band is detectable; dense
micro-text and graphics run through it. The browser stack (`tesseract.js`) produced a checksum-valid
read of the same bytes on its **third** pass, a contrast stretch, in 1 305 ms. The native variant
chain never tries that transform. Same bytes, opposite outcome.

**Italy — a magenta FACSIMILE watermark** running diagonally across the whole card and crossing the
band. Check digits verified the same way. This one closes by explanation: it is not a pipeline
defect a variant would reach, and the browser misses it too.

### A correction to the France attribution

The 2026-09-18 entry attributes France partly to a **budget exhaustion** — 66 705 ms against a 52 s
budget with `retry_budget_hit: true`, and Phase D recorded 72 227 ms. **The committed ledger at
`bb658d6` no longer says that:** France records `ocr_ms: 62441`, `retry_budget_hit: false`,
`retry_stop: "exhausted"`.

The budget-stopped case has left the population. **All twelve scored misses now stop by exhausting
the pass list, none by hitting the clock.** Raising `MAX_SECONDS` buys nothing anywhere in this set
— a conclusion Phase D reached for thirteen of fourteen, now unqualified. Do not cite France as
budget-limited.

## 2. Recognition misses with a positional attribution — 6

Derived in [`checksum-failed-miss-mechanisms-2026-09-18.md`](checksum-failed-miss-mechanisms-2026-09-18.md)
by aligning each read against its reviewed fixture position by position.

| document | truth format | failing check digit(s) | differing positions, digit-bearing line | mechanism |
| :--- | :--- | :--- | :--- | :--- |
| `Afghanistan_Passport_Specimen_P0_AFG_2016_mrz` | TD3 | document_number, composite | **1** (position 0) | single-glyph substitution in the document number's first cell |
| `Czechia_Passport_Specimen_P0_CZE_2005_mrz` | TD3 | personal_number, composite | **2** (33, 41) | two isolated substitutions inside the personal number; no shift — 34–40 are correct |
| `Belgium_ID_Specimen_2021_back_mrz` | TD1 | composite **only** | **8** (7 inside optional data 2, plus the composite digit) | error confined to the one TD1 line-2 field with no check digit of its own |
| `Romania_Passport_Specimen_PE_ROU_2024_mrz` | TD3 | personal_number, composite | **1** (position 41) | single substitution at the end of the personal number |
| `Sweden_ID_Specimen_2022_back_mrz` | TD1 | date_of_expiry, composite | **5** (13–17, contiguous) | a five-cell run spanning the expiry/nationality boundary, on the lowest band score in the set (0.399) |
| `Russian_Federation_Passport_Specimen_P0_RUS_2019_mrz` | TD3 | — see §4 | 112 (whole zone) | not a recognition miss — format misdetection |

**Two structural facts do most of the work here, and both are properties of ICAO 9303, not of this
pipeline:**

- **One wrong character can fail two checks.** Afghanistan's single miscoded cell fails
  `document_number` *and* `composite`, because the composite's input includes the document number.
  The co-occurrence is one defect, not two — so "six documents fail the document-number check" is
  not the same claim as "six documents have six document-number defects".
- **`failing_checks == ["composite"]` alone is a positional fingerprint.** TD1's optional data
  carries no check digit of its own, so only the composite can observe an error there. Belgium's
  signature localised the defect before a single character was examined.

### The damage no check digit can see is larger than the damage that fails the checks

Differing positions on the check-digit-bearing line across the five genuine recognition misses
above: **1, 2, 8, 1, 5**. Name-field differences in the same reads: **22 (Romania), 22 (Sweden),
9 (Belgium), 1 (Czechia)**, plus Sweden's three nationality characters.

Nationality and sex are excluded from the composite on **both** TD1 and TD3. Repairing Sweden's
expiry alone would therefore turn it into a clean Tier-1 *hit* still carrying a wrong nationality,
and describing Romania as "a personal-number miss" accounts for 1/23rd of what is wrong with the
read. This is [ADR-0013](../decisions/ADR-0013-names-are-scored-against-mrz-form-truth.md) measured
on named documents.

## 3. Recognition misses with no positional attribution yet — 4

| document | truth format | fixture geometry | failing check digit(s) | what is known |
| :--- | :--- | :--- | :--- | :--- |
| `Croatia_ID_Specimen_2021_back_mrz` | TD1 | 3 × 30 | document_number | browser read **byte-matches the fixture** (`mrz_line_matches_fixture: true`) |
| `Germany_Passport_Specimen_P0_D00_2024_mrz` | TD3 | 2 × 44 | document_number, composite | browser produced a checksum-valid read; fixture now exists (#336) |
| `Hong_Kong_Passport_Specimen_P0_HKG_2007_mrz` | TD3 | 2 × 44 | document_number | **both stacks miss it** |
| `Hong_Kong_Passport_Specimen_P0_HKG_2019_mrz` | TD3 | 2 × 44 | document_number, composite | **both stacks miss it** |

These four have ground truth and a failing-check set but have never been aligned position by
position. **They are the gap in this record**, and closing them is one `--dump-ocr` run plus the
same alignment the six above received — no new tooling.

Note the shape: all four fail the **document number**, and the two that no stack can read are the
two Hong Kong passports, which share an issuer and are twelve years apart.

## 4. The one in the wrong bucket

`Russian_Federation_Passport_Specimen_P0_RUS_2019_mrz` is recorded as `checksum_failed`, format
`TD1`, failing `document_number`, `date_of_birth` and `composite`.

**The reviewed fixture is 2 × 44 — TD3.** The `TD1` in the ledger is the *reader's* output, not the
document's format. Verified at `bb658d6` directly from `samples/ocr_fixtures/`: `document_type` `P`,
two lines of 44, `mrz_checksums_valid: true`, hand-transcribed.

The reader returned three lines of 30 with `mrz_band_score = null` — the band was never scored, the
wrong format was selected, and the check digits were then computed over a zone that does not
correspond to the printed one. Whole-zone mismatch: **112 characters**, from 63 raw OCR lines.

**Consequences, in order of how much they change:**

1. **Its failing-field list carries no information.** A wrong format makes every check-digit
   position wrong. `document_number, date_of_birth, composite` describes an artefact of parsing a
   TD1 out of a TD3; it does **not** say those three fields were misread. Do not group this document
   by its failing checks, and do not count it toward the document-number lead in §6.
2. **It belongs with the detection misses**, in
   [ADR-0015](../decisions/ADR-0015-geometric-mrz-band-location.md)'s territory.
   Reclassification is **recommended and still not applied** — it moves a baseline classification
   and needs its own approval.
3. **It is not mislabelled data.** The row is correct about what the reader did. The open question
   recorded before this run — "is the corpus row wrong?" — is answered: it is not.

## 5. An independent axis that partitions the twelve exactly 8 / 4

[Phase D](phase-d-native-vs-browser-2026-09-18.md) ran the browser stack over the same bytes and the
same population with detection held constant. Intersecting its head-to-head classes with the twelve
covers them with no remainder:

| class | n | documents |
| :--- | --: | :--- |
| **Browser reads it, native scores a miss** | **8** | Afghanistan, Belgium, Croatia, Czechia, France, Germany, Romania, Sweden |
| **Both stacks miss it** | **4** | Italy, Hong Kong 2007, Hong Kong 2019, Russian Federation |

This is the most useful grouping in the record, because it is measured rather than inferred and it
separates two genuinely different questions:

- **The eight are demonstrably readable.** Something in the native pipeline, not the document, is
  the cause. Five of the eight are corroborated against hand transcription rather than merely being
  self-consistent — Croatia, Sweden, Afghanistan, Czechia and Romania match the reviewed fixture's
  lines byte for byte, and Belgium matches all nine reviewed fields.
- **The four are a different problem.** Italy has a watermark across the band; Russia is a format
  misdetection; the two Hong Kong passports are unexplained by any run so far and are the only
  documents in the set that nothing has read.

### A caveat in Phase D that this record closes

Phase D notes that two of the browser's wins — **France and Germany** — "carry no fixture and are
**unverified** beyond their own check digits". Both now have one: France from #348, Germany from
#336. Those two browser reads can now be checked against ground truth rather than taken on their own
checksums, which is a cheap way to confirm the "demonstrably readable" claim for the two documents
where it was weakest.

## 6. The failing check digits

Across the ten `checksum_failed` documents, counting each document's set once:

```
document_number   6      Afghanistan, Croatia, Germany, HKG 2007, HKG 2019, Russia*
personal_number   2      Czechia, Romania
date_of_expiry    1      Sweden
date_of_birth     1      Russia*
composite only    1      Belgium
```

**`document_number` is the dominant failing check — 6 of 10.** The honest count is **5 of 9**:
Russia's failing set is an artefact (§4) and should not be counted. The lead survives the correction
and remains the strongest single signal in the set.

Two things make it worth acting on first. It is the narrowest defect measured — Afghanistan's is
**1 cell of 88**. And it is the same field the ADR-0016 work found card backs print least often
(2 of 10), so the document number is where the MRZ and the visual zone are weakest at once.

## 7. Corrections this record makes to earlier ones

1. **France is no longer budget-limited** in the committed ledger (§1). All twelve stop by
   exhausting the pass list.
2. **Russia's `TD1` is the reader's output, not the document's format** (§4). The fixture is TD3.
3. **Phase D's "France and Germany carry no fixture"** is now stale; both do (§5).

## 8. What this record does not do

It proposes no fix. Four of the ten recognition misses still have no positional attribution (§3),
and no group here has a reproducible fixture-level case yet. Under the standing rule, **no broad OCR
change is justified until each group does.**

The two cheapest next measurements, each one run and no new tooling:

- **Align Croatia, Germany and the two Hong Kong passports** against their fixtures with
  `--dump-ocr`, closing §3 and completing the positional record for all ten.
- **Try a contrast-stretch preprocessing variant on France**, the one document where a named,
  untested transform is known to work on the same bytes in another stack.

## Scope

Documentation only. No code, no data, no manifest, no `samples-data`, and no baseline number moves.
