# ADR-0021 Phase 0: class (C) meets K1 only in the line-1 prefix, and two fixtures are wrong

**Date:** 2026-09-24 · **MAIN:** `738cd16` · **DATA:** not recorded by the run manifest (corpus manifest SHA-256 `88b31aff…253c`; outcome ledger identical to the CI baseline at DATA `396b22f`) · **Evidence:** Observed (release `provider-bench --real-specimens --mrz-only --dump-ocr --dump-ocr-hits`, all OCR arms at their defaults; 261 documents, 171 dumped) plus Derived (tail-normalized string alignment against fixtures, hand adjudication) · **Status:** current

**2026-09-24.** Phase 0 of the proposed ADR-0021 (fixed-grid MRZ strips; review on
[#430](https://github.com/ruledicaprio/SynthPass/pull/430), instruments on
[#432](https://github.com/ruledicaprio/SynthPass/pull/432)). One instrumented OCR pass, then
[`classify_mrz_mechanisms.py`](../../tools/classify_mrz_mechanisms.py), then the recomputation and
hand adjudication below. Method: [`mrz-strip-phase0-method.md`](mrz-strip-phase0-method.md).

**No MRZ content, holder name, document number or zone fragment appears here.** Evidence is given
as asset IDs, counts, 0-based cell positions and ICAO field regions. The dump, the ledger and the
per-asset analysis stay in gitignored `artifacts/phase0/`.

**Every label and relation below is a string-level candidate explanation ranked by edit cost. None
is an observed mechanism.** A string comparison cannot show that a glyph physically sits in
another cell. That needs per-character boxes or a fitted cell grid, which this run does not
collect.

## The answer

- **The instrumentation changed no outcome.** All 261 assets have the same outcome as the committed
  [`real-specimen-outcomes.jsonl`](real-specimen-outcomes.jsonl) (**Observed**). Tier-1 is
  **140 / 152 = 92.1%** on documents that can yield a hit. Corpus-wide it is **140 / 261 = 53.6%**.
  Both match [the CI baseline](real-specimen-mrz-baseline.json) (2026-09-19, `185e530`).
- **Trailing fillers inflate the classifier's `indel` label.** When the only length mismatch is the
  `<` tail, which the parser pads anyway, the `indel` relation still fires. With the tail set aside,
  the label covers **6 of 12** scored misses rather than 9, and **13 of 25** hit targets rather than
  16 (**Derived**, Table 3).
- **In the zone the parser returned, 18 review targets have an interior indel: 6 misses and 12
  hits.** For 13 of them the indel lies only in the names, which no check digit covers and which
  ADR-0021 Layer 2 leaves alone (**Derived**, Table 5).
- **Three assets meet K1 at k ≤ 3, and all three are in the line-1 prefix (cells 0–4):**
  - `passports/Cetis_Sample_Passport_Specimen_P0_TRC_2022_mrz_inner_page.jpg` at k = 1;
  - `passports/Germany_Passport_Specimen_P0_D00_2024_mrz.jpg` at k = 2;
  - `passports/Djibouti_Passport_Specimen_P0_DJI_2017_mrz_partly_censored.jpg` at k = 3.

  Each is a filler relocation that needs no invented character, and each is fixture-confirmed.
  Only one asset reaches class (C) in check-digit-covered data: `id_cards/Sweden_ID_Specimen_2022_back_mrz.jpg`
  (k = 2). Its dropped digit is absent from the returned line.
- **K1 does not fire, but only just.** Three is exactly the threshold, and all three assets are in a
  region that no check digit arbitrates. Amendment 1 turns a prefix disagreement into a refusal.
  Restricted to check-digit-covered data, K1 would fire: the count is 0, or 1 counting Sweden.
  **K2 and K3 cannot be assessed** without the P0.3 enumerator and an aligner.
- **Two fixtures disagree with their printed zone.** The Belgium 2021 card back's fixture fails its
  own TD1 composite digit, while the print and the OCR pass it. On the Somaliland book, the fixture
  and the OCR differ by a three-cell shift over a run of zeros, and both pass all five digits.

## Table 0: run identity

| Item | Value | Source |
| --- | --- | --- |
| Instrument commit | `738cd16481a8c72109882f39c65ce6cce693adaa` | manifest `git_commit` |
| Working tree dirty | `false` | `working_tree_dirty` |
| Flags | `--real-specimens --mrz-only --dump-ocr --dump-ocr-hits --progress --out artifacts/phase0/provider-bench-report.json` | `flags` |
| Century pivot | 26 | `pivot_yy` |
| OCR arms | chargrid `off`, order/rotate/skew `default`, texture `on`. All five are the defaults: texture defaults to `on` in `synthpass-ocr` | `ocr_arms` |
| Corpus manifest SHA-256 | `88b31aff74378b4f5c58b14fb9708c021b304bd6b9203f7b6998a90a79cd253c` (matches the on-disk `samples/corpus.jsonl`) | `corpus_manifest_sha256` |
| Documents / labelled loaded | 261 / 64 | `documents_loaded`, `labelled_loaded` |
| `samples-data` commit | **not recorded**. `origin/samples-data` was `396b22f` at analysis time, the baseline's DATA pin. The local image set's sync state is not recorded | by hand |
| Model hashes, OCR backend, machine | **not recorded**. Release build (build 11 min 54 s), desktop host, one run | by hand |
| Started | `1790266917` = 2026-09-24 16:21:57 UTC; the OCR pass took about 50 min | `started_unix_seconds`, file times |

## Table 1: populations, both denominators

| Population | Count | From |
| --- | --- | --- |
| Assets in the run's ledger (corpus-wide) | 261 | `summary.total_documents` |
| Scored assets (Tier-1 denominator) | 152 | `summary.scored_documents` |
| Scored: hit / `checksum_failed` / `no_mrz_found` / other | 140 / 10 / 2 / 0 | records, `outcome` |
| Off-denominator: `checksum_failed_specimen` / `redacted_mrz` / `no_mrz_expected` | 19 / 39 / 51 | records, `outcome` |
| Assets with an `mrz` dump row | 171 (140 hits, 19 + 10 + 2 others) | `summary.dumped_mrz_documents` |
| Review targets | 37 | `summary.review_targets` |
| scored misses | 12 (7 TD3, 5 TD1) | records |
| hits with a name error or a line-1 difference | 25 (24 TD3, 1 TD1) | records |
| Review targets with no dump row | 0 | `summary.review_targets_with_missing_dump` |
| Hits without fixture truth (`unlabelled`, never counted correct) | 107 of 140 | `summary.unlabelled_hits` |
| Review targets with a distance or candidate tie | 2: Belgium 2021 card back (line 2, a tie) and Romania PE 2024 (line 1, a candidate tie). 10 records overall; 8 of them are `checksum_failed_specimen` | `summary.distance_ties` |

No outcome differs from the committed ledger, so no moved asset needs listing.

## Table 2: mechanism labels, as the classifier emitted them

An asset may carry several labels, so rows do not sum.

| Label | Scored-miss targets (12) | Hit targets (25) | `checksum_failed_specimen` (19; not targets) |
| --- | --- | --- | --- |
| band location | 0 | 0 | 0 |
| wrong line/attempt association | 0 | 0 | 2 |
| format/window selection | 1 | 0 | 4 |
| indel | 9 | 16 | 12 |
| substitution | 6 | 14 | 2 |
| inherited repair | 9 | 22 | 15 |
| printed non-conformance | 0 | 0 | 19 |
| indel only / substitution only / both | 6 / 3 / 3 | 11 / 9 / 5 | 12 / 2 / 0 (5 neither) |

**`inherited repair` is a weak signal.** It fires whenever a returned line is not verbatim in the
provider input, so padding alone triggers it. On 7 of the 22 hit targets and 5 of the 15
`checksum_failed_specimen` assets that carry it, every returned line matches a provider-input line
once trailing `<` are ignored. Nothing was repaired beyond the tail. For the 9 misses and the other
15 hits, the label shows only that the returned line differs from the provider input, not that a
repair happened or that it helped.

**`band location` is 0 by construction here.** It fires only when a `no_mrz_found` asset has no band
score. Both `no_mrz_found` assets have one.

## Table 3: string relation per truth line (labelled review targets)

### 3a: the classifier as run (nearest provider-input line, raw)

Cells read scored misses / hits.

| Truth line | exact | substitution | indel | tie | candidate tie | no candidate |
| --- | --- | --- | --- | --- | --- | --- |
| line 1 | 3 / 2 | 0 / 8 | 8 / 15 | 0 / 0 | 1 / 0 | 0 / 0 |
| line 2 | 1 / 15 | 5 / 9 | 5 / 1 | 1 / 0 | 0 / 0 | 0 / 0 |
| line 3 (TD1) | 0 / 0 | 1 / 0 | 4 / 1 | 0 / 0 | 0 / 0 | 0 / 0 |

A tie means no indel script is strictly better. It does not mean "no shift".

### 3b: the same lines with trailing fillers set aside

Method: take the line the classifier chose, and strip trailing `<` from the candidate and from the
truth. Then re-pad the candidate with `<` to the truth's length, recompute Levenshtein, Hamming and
the best indel-containing script, and apply the classifier's rules.

Cells read scored misses / hits.

| Truth line | exact | tail fillers only | substitution, same length | substitution + tail difference | interior indel | tie | candidate tie |
| --- | --- | --- | --- | --- | --- | --- | --- |
| line 1 | 3 / 2 | 0 / 0 | 0 / 8 | 5 / 3 | 3 / 12 | 0 / 0 | 1 / 0 |
| line 2 | 1 / 15 | 0 / 0 | 5 / 9 | 0 / 0 | 5 / 0 | 1 / 1 | 0 / 0 |
| line 3 (TD1) | 0 / 0 | 0 / 0 | 1 / 0 | 1 / 0 | 2 / 1 | 1 / 0 | 0 / 0 |
| **all non-exact lines** | | 0 / 0 | 6 / 17 | 6 / 3 | **10 / 13** | 2 / 1 | 1 / 0 |

The classifier's `indel` lines split as follows:

- **scored misses:** 17 lines, of which 10 are interior indels, 6 are a substitution plus a tail
  difference, and 1 is a tie;
- **hits:** 17 lines, of which 13 are interior indels, 3 are a substitution plus a tail difference,
  and 1 is a tie.

Every classifier `substitution` line stays a same-length substitution. At the asset level, the
`indel` label drops from 9 to 6 misses. Belgium 2021 card back, Czechia 2005 and Hong Kong 2007
leave the set. It drops from 16 to 13 hit targets: Canada PP 2023 `_highlight`, Slovakia PS 2014
and Spain 2015 `_highlight` leave it.

**Sensitivity.** Normalizing before choosing the nearest line, instead of after, gives these
counts:

- **scored misses:** 12 interior indels, 5 substitutions plus a tail difference, 5 same-length
  substitutions, 2 ties and 1 tail-only line;
- **hits:** 12 interior indels, 4 substitutions plus a tail difference, 17 same-length
  substitutions and 1 tie.

A quicker hand count made earlier in the session gave, for misses and hits respectively:

- 9 / 13 interior indels;
- 9 / 4 substitutions plus a tail difference;
- 7 / 17 same-length substitutions or ties.

Both variants and the quick count agree to within three lines per cell. The conclusion holds either
way: of the classifier's 34 `indel` lines, 9 (26%) become substitutions once the tail is set aside,
and 2 more become ties.

### 3c: the zone the parser actually returned, tail-normalized

The nearest provider-input line is not what the parser returned. For K1, the returned zone decides.

| Truth line | exact | substitution, same length | interior indel | tie | no zone / line count differs |
| --- | --- | --- | --- | --- | --- |
| line 1 | 3 / 0 | 3 / 13 | 3 / 11 | 0 / 1 | 3 / 0 |
| line 2 | 1 / 23 | 6 / 2 | 1 / 0 | 1 / 0 | 3 / 0 |
| line 3 (TD1) | 0 / 0 | 0 / 0 | 2 / 1 | 1 / 0 | 2 / 0 |

Three scored misses have no comparable zone. `id_cards/France_ID_Specimen_2020_back_mrz.png` and
`id_cards/Italy_ID_Specimen_2022_back_mrz.jpg` returned none (`no_mrz_found`).
`passports/Russian_Federation_Passport_Specimen_P0_RUS_2019_mrz.jpg` returned a three-line TD1
zone for a two-line TD3 truth. It is the one `format/window selection` label, and no provider-input
line is within 44 edits of either truth line.

## Table 4: hits, line 1

| Labelled, line 1 exact | Labelled, line 1 differs | Labelled, line counts differ | Unlabelled |
| --- | --- | --- | --- |
| 8 | 25 | 0 | 107 |

All 25 hit targets differ on line 1. For 21 of them the name is also wrong (`names_exact` false).
Four have exact names but a line-1 difference: India 2023 and Serbia 2012 (a filler run in the name
region), and Somaliland and Uzbekistan 2013 (document-code cell 1). No hit has a line-count
mismatch, so the classifier's `line1_error = false`-when-counts-differ gap did not bite on this run.

## Table 5: class (C), adjudicated by hand in the returned zone

Class (C) means an interior indel beats a positional reading, or a line is assigned to the wrong
printed line. **k** counts cell edits in the admissible region. Re-padding the filler tail is not
charged, matching today's `fit_length`. "Reachable at k" columns are cumulative.

| Region of the misalignment | within one line | crossing a line break | reachable at k = 1 / 2 / 3 | fixture-confirmed better value |
| --- | --- | --- | --- | --- |
| checked data (local or composite) | 1 | 0 | 0 / 1 / 1 | 1, in position only (see Sweden below) |
| prefix, cells 0–4 | 4 | 1 | 1 / 2 / 3 | 5 |
| sex, nationality (constrained, unchecked) | 0 on their own; Sweden's shift runs from the expiry date into nationality | 0 | — | — |
| names | 18 (1 of them a tie) | 2 | not admissible from text | — |
| optional data with no local digit | 0 | 0 | — | — |
| **Silent wrong field on a hit, any region** | 11 (3 with a non-name field) | 1 | — | 12 |

The named assets, row by row:

- **Checked data.** `id_cards/Sweden_ID_Specimen_2022_back_mrz.jpg` is a scored miss that fails the
  expiry and composite digits. The returned line 2 has lost one cell inside the expiry date, and
  cells 13–16 sit one to the left. A spurious cell at 17 compensates.

  Realigning it is one insertion plus one deletion (k = 2), all within line 2. But the inserted
  cell's value is not in the returned line. An aligner could only get it by solving the expiry
  check digit, which would spend the anchor that is meant to verify it. Another attempt in the same
  provider input already carries line 2 aligned, with one substitution at cell 17. This is an
  **attempt-selection** case at least as much as an alignment case.
- **Prefix, k = 1.** `passports/Cetis_Sample_Passport_Specimen_P0_TRC_2022_mrz_inner_page.jpg` is a
  hit. The filler at cell 1 is lost, so the document code and the issuer are both returned wrong.
- **Prefix, k = 2.** `passports/Germany_Passport_Specimen_P0_D00_2024_mrz.jpg` is a scored miss. Two
  filler cells in cells 1–4 are lost, so the document code and the issuer are wrong. The miss
  itself is a separate substitution at document-number cell 1 (class B), so realignment would not
  make it a hit.
- **Prefix, k = 3.** `passports/Djibouti_Passport_Specimen_P0_DJI_2017_mrz_partly_censored.jpg` is a
  hit. Three fillers are inserted at cell 2, and the issuer is returned as fillers.
- **Prefix, beyond k = 3.** `passports/Canada_Passport_Specimen_P0_CAN_2013_mrz.jpg` is a hit. Seven
  fillers are inserted after cell 1, and the issuer is returned as fillers. A provider-input line-1
  attempt within two edits of the fixture exists.
- **Prefix, crossing a line break.** `passports/Slovakia_Passport_Specimen_P0_SVK_2005_mrz.png` is a
  hit. The returned line 1 is a line-2 reading with the line-1 letter repairs applied. Document code,
  issuer and names are all wrong. A provider-input line-1 attempt within one edit of the fixture
  exists. This is line association, not a k-bounded edit.
- **Names, within one line.** 13 hit targets: Slovenia ID 2022 (line 3), Angola PN 2026, Bangladesh
  2023, Canada 2013, Canada PP 2023 `_wide` (a tie), Cetis TRC 2022, Djibouti 2017, India 2023,
  Indonesia 2024, Nigeria 2022, Serbia 2012, Spain 2013 and United Arab Emirates 2011. 5 scored
  misses: Belgium 2021 card back, Croatia 2021 card back, Germany 2024, Hong Kong 2019 and Romania
  PE 2024. No check digit covers the name region, so ADR-0021 Layer 2 region (c) excludes it.
- **Names, crossing a line break.** Sweden 2022 card back: the name slot holds a line-1-like
  reading (a tie at string level), and no provider-input line is near the true name line.
  Slovakia 2005: see above.
- **Silent wrong field on a hit, class (C).**
  - Within one line: 3 with a non-name field wrong (Canada 2013 issuer; Cetis document code and
    issuer; Djibouti 2017 issuer) and 8 with only the name wrong. Those eight are Angola,
    Bangladesh, Canada `_wide`, Indonesia 2024, Nigeria, Spain 2013, United Arab Emirates 2011 and
    Slovenia ID.
  - Crossing a line break: Slovakia 2005.
  - India 2023 and Serbia 2012 carry a name-region indel but return exact names.

Adjudicated as **not** class (C):

| Asset | Scored outcome | Why not (C) |
| --- | --- | --- |
| `id_cards/Belgium_ID_Specimen_2021_back_mrz.png` | `checksum_failed` | The fixture is wrong at line-2 cells 20–21 (next section). The returned line 2 is missing its final five cells, which is truncation, not misalignment. |
| `id_cards/Croatia_ID_Specimen_2021_back_mrz.jpg` | `checksum_failed` | B: every document-number digit is returned as its confusable letter, as it was in the provider input |
| `id_cards/France_ID_Specimen_2020_back_mrz.png` | `no_mrz_found` | A: no line-1 candidate near the truth. The line-2 attempt has a spurious leading cell and a filler run three cells short: 4 edits, beyond k = 3 |
| `id_cards/Italy_ID_Specimen_2022_back_mrz.jpg` | `no_mrz_found` | A/B: line 1 is 11 edits from the truth over 30 cells, mostly substitutions. The prefix deletion the classifier found is one of several equal-cost scripts |
| `passports/Afghanistan_Passport_Specimen_P0_AFG_2016_mrz.webp` | `checksum_failed` | B: document-number cell 0 |
| `passports/Czechia_Passport_Specimen_P0_CZE_2005_mrz.jpg` | `checksum_failed` | B: personal-number cells 33 and 41 |
| `passports/Hong_Kong_Passport_Specimen_P0_HKG_2007_mrz.png` | `checksum_failed` | B: four document-number cells and the check digit |
| `passports/Hong_Kong_Passport_Specimen_P0_HKG_2019_mrz.png` | `checksum_failed` | B in line 2 (document-number cell 0, sex cell 20). One provider-input attempt dropped the sex cell, but the returned line is aligned. Line-1 names are (C), which is inadmissible |
| `passports/Romania_Passport_Specimen_PE_ROU_2024_mrz.jpg` | `checksum_failed` | B: personal-number cell 41. Line-1 names are (C), which is inadmissible |
| `passports/Russian_Federation_Passport_Specimen_P0_RUS_2019_mrz.jpg` | `checksum_failed` | A/format: nothing in the provider input resembles either truth line |

## Verdict

| Criterion | Holds? | Deciding row |
| --- | --- | --- |
| K1 | **No, at exactly the threshold.** Three review-target assets reach class (C) at k ≤ 3 with a fixture-confirmed better value: Cetis TRC 2022, Germany 2024 and Djibouti 2017. All three are in the line-1 prefix. Sweden 2022 card back (k = 2, checked data, value not in the returned line) makes four on a loose reading. Canada 2013 (k = 7) makes five if k is unbounded | Table 5, prefix row (checked-data row: 0 strict, 1 loose) |
| K2 | **Not assessable.** It needs the P0.3 refusal rates and an aligner's recoveries on the same population | — |
| K3 | **Not assessable.** It needs the P0.3 enumerator over a declared grammar. One real-corpus observation bears on it (Somaliland, below) | — |

K1 passes without a margin. Three facts qualify that pass:

- **No check digit covers any of the three.** Amendment 1, item 4, turns a prefix disagreement
  into a refusal in v1. So a v1 aligner repairs these only when exactly one prefix reading survives
  the registry and the Part 4 §4.4 prior, and that is K3 territory. For Cetis, whose issuer is a
  sample code outside the registry and whose misread second letter is a legal §4.4 document-code
  letter, a refusal is the likely result (**Hypothesized**).
- **Realignment turns none of the three into a new hit.** Germany 2024 stays a miss. The other two
  are hits already, where the gain is a corrected or refused silent issuer.
- **Region (a) contributes no strict case.** Region (a) is where the check digits arbitrate, and it
  is the core of ADR-0021's claim.

"Three" is the maintainer's effort threshold, not a truth criterion (Amendment 1). Under the
ADR's own rule, the evidence favours the **line-based** variant if anything is built. No
admissible class-(C) case crosses a line break, and the two crossings are line association, not
edits.

## Two fixtures disagree with their printed zone

- **`id_cards/Belgium_ID_Specimen_2021_back_mrz.png`, line 2, cells 20–21.**
  - The fixture's TD1 composite digit does not verify (the document number is checked with the
    TD1 overflow rule).
  - The provider input's complete line-2 attempt verifies both date digits and the composite as
    read. Its only other difference from the print is nationality cell 17, which no digit covers.
  - That attempt differs from the fixture only by transposing cells 20–21. The printed zone, read
    by eye from the specimen image, agrees with the OCR.
  - **Observed.** The fixture carries a transposition inside `optional_data_2`. This is why the
    classifier reports a tie on this line, and it inflates earlier per-field damage counts for this
    asset by two cells (below).
- **`passports/Somaliland_Passport_Specimen_P0_RSL_2023_mrz_non_ISO.jpg`, line 2, personal
  number.**
  - The fixture has three more leading zeros than the returned zone. The two non-zero cells sit
    three positions later, and correspondingly fewer fillers follow.
  - Both readings satisfy all five TD3 digits (**Observed**, computed).
  - This is arithmetic, not luck. Zeros and fillers both carry value 0, and the 7-3-1 weights repeat
    every three cells. So shifting a segment by a multiple of three across cells whose value is 0
    leaves every weighted sum unchanged (**Derived**).
  - A by-eye read of the low-resolution image supports the OCR's count, not the fixture's
    (**Hypothesized** until checked at full resolution).
  - The Tier-1 outcome is unaffected, because the document number matches.
  - Either way, this is a real-corpus pair of checksum-consistent readings that differ in a checked
    field. That is the phenomenon Amendment 1, item 2 constructs synthetically, and it bears on K3.
    A grammar that may insert or delete `0` or `<` at k ≥ 3 can produce such pairs on real zones.

## What this does not claim

- **No label here is a mechanism.** The string relations are ranked by edit cost. Glyph positions
  were not measured.
- **No aligner exists or was simulated.** "Reachable at k" is hand arithmetic on the returned line.
  It is not a search result, and it says nothing about how many *other* readings the same k would
  admit (K3).
- **Nothing here says that realigning any asset would verify.** Replaying the dump through
  `find_and_parse_with` is P0.4 and the A/B's first stage, and it has not been built.
- **The DATA pin is inferred.** It rests on the identical outcome ledger and on the
  `origin/samples-data` tip. The run manifest does not record it. Model hashes and the OCR backend
  are not recorded either.
- **This is one run.** `rten` run-to-run noise flips about three documents, and no outcome moved
  here. This note makes no speed claim.
- **107 of the 140 hits have no fixture.** Nothing here says they are correct.

## Rejected on the way

- **The classifier's `indel` count as the size of the alignment population.** 9 misses and 16 hits
  shrink to 6 and 13 once trailing fillers are set aside (Table 3b).
- **The nearest provider-input line as the basis for adjudication.** The parser returned a line
  farther from the truth than the best attempt on 18 lines across 16 review targets (below). K1 is
  about what was returned.
- **`inherited repair` as evidence that a repair happened.** 7 of the 22 hits and 5 of the 15
  `checksum_failed_specimen` assets that carry it are explained by padding alone.
- **Belgium 2021 line 2 as a class-(C) tie in check-digit-covered optional data.** It is a fixture
  transposition.
- **Italy 2022 line 1 as a prefix deletion (the classifier's `del @ 0–1`).** Equal-cost scripts
  disagree, and 11 edits over 30 cells is recognition, not alignment.
- **France 2020 line 2 as a K1 case.** It needs 4 edits, and line 1 is absent from the provider
  input.
- **Somaliland's personal number as a silent wrong read on a hit.** The fixture is the likelier
  error; see above.

## Also found

- **Better attempts in the provider input than in the returned zone.** For 10 of the 25 hit
  targets, the provider input already holds a line-1 attempt strictly closer to the fixture than the
  returned line 1. Two of them are exact: Djibouti 2019 and Somalia 2023. The other eight are Canada
  2013, Canada `_wide`, Cyprus 2010, Djibouti 2017, Dominican Republic 2020, India 2022, Slovakia
  2005 and Spain 2013.

  Scored misses show the same thing on line 2 for Belgium 2021 and Sweden 2022, and on line 1 for
  Croatia 2021, Hong Kong 2007 and Hong Kong 2019. Line 3 adds Slovenia ID 2022 and Belgium and
  Sweden again. Line 1 carries no check digit, so today nothing can rank its attempts. Choosing
  among them without truth is an open problem (structural anchors and cross-attempt consensus are
  candidates), and it is P0.4's question (**Observed**, string distances to fixtures alone; Croatia
  2021's closer line-1 attempt is itself a line-2 reading).
- **The line-1 rule misses TD1 name errors.** `id_cards/Serbia_ID_Specimen_2008_back_with_mrz.png`
  is a labelled hit whose returned line 3 (the TD1 name line) has an interior indel. It is not a
  review target: its names compare exact, and the classifier's line-difference rule checks only
  line 1, which on TD1 holds no name.
- **Silent wrong non-name fields on hits that are class (B), not (C).** Six hits return a wrong
  document code at cell 1: Cyprus 2010, Dominican Republic 2020, India 2022, Somaliland, Spain 2015
  `_highlight` and Uzbekistan 2013. Dominican Republic 2020 also returns a wrong issuer (cell 4) and
  wrong nationality (cells 10 and 12). Slovenia ID 2022 returns wrong `optional_data_1` (cell 28).
  Under Amendment 1's K1 note, each is a narrow Alternative A fix, whatever K1 says. Only Somaliland's
  cell-1 value was checked against the image, and it agrees with the fixture.
- **A stale figure.** [`twelve-scored-misses-2026-09-19.md`](twelve-scored-misses-2026-09-19.md)
  lists Belgium 2021's line-2 damage as cells 20–21 and 24–29, eight cells in `optional_data_2`.
  Cells 20–21 are the fixture's transposition, not OCR damage.

## Invocation

```text
# OCR pass (run by the calling session, release build at 738cd16, clean tree)
provider-bench --real-specimens --mrz-only --dump-ocr --dump-ocr-hits --progress \
  --out artifacts/phase0/provider-bench-report.json
# classifier
python tools/classify_mrz_mechanisms.py --dump artifacts/phase0/provider-bench-miss-ocr-dump.jsonl \
  --ledger artifacts/phase0/provider-bench-ocr-outcomes.jsonl --out artifacts/phase0/mechanisms.json
# tail-normalized tables (PII-free output; scripts kept beside the artifacts)
cd artifacts/phase0/analysis && python phase0_tables.py > ../phase0-tables.json
```

Hand adjudication (Table 5, the fixture checks) used shape-masked alignments. Each character was
printed as letter, digit or filler class plus a match mask, so no zone text left the local
artifacts. The two images were viewed from `samples/`.
