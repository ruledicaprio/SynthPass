# The observed OCR-B confusion matrix is a filler problem and a zero problem, and it is asymmetric

**Date:** 2026-09-21 · **MAIN:** `8480772` · **DATA:** `396b22f` · **Evidence:** Observed (`provider-bench --real-specimens --mrz-only --dump-ocr`, release, all `SYNTHPASS_OCR_*` arms cleared, class-sweep arm `off`; 261 documents, 31 dumped misses) · **Status:** current

This is the **empirical** half of a two-sided comparison: which glyph the recognizer actually
returns when a given glyph was printed. The other half — a *predicted* confusion matrix from
normalized cross-correlation over the 37 OCR-B templates — is a separate workbench and is not
measured here. [§6](#6-a-falsifiable-prediction-for-the-template-similarity-side) commits to what
that side should and should not find, before seeing it.

**No MRZ content, holder name, document number or zone fragment appears here, and no document is
named next to a character value.** The unit of evidence is an ordered glyph pair and a count,
aggregated across the corpus — a property of the recognizer, not of any specimen. Documents are
referred to only by format and count. The dump itself carries `raw_ocr_text` and
`ground_truth_mrz` and stays in the gitignored scratchpad.

## 1. Provenance and the alignment rule

### The run

One release run at MAIN `8480772`, the deterministic `mrz` provider only. **It reproduced the
committed baseline exactly**: `documents` 261, `scored` 152, `tier1_hits` 140 = 92.1%,
`strict_tier1_hit_rate` 12 / 45, and every `by_miss_kind` count equal to
[`real-specimen-mrz-baseline.json`](real-specimen-mrz-baseline.json) (CI, 2026-09-19,
`185e530`) — `checksum_failed` 10, `no_mrz_found` 2, `checksum_failed_specimen` 19,
`false_positive_mrz` 0. The 31 dump records are exactly those three miss buckets (10 + 2 + 19),
so the population reconciles with the baseline by construction. Timings from this run are not
quoted: a short compile overlapped it.

The dump spans **21 TD3, 9 TD1 and 1 MRV-A** records. Ground truth is `labels.mrz_line` from the
hand-transcribed `samples/ocr_fixtures/*.json`.

**Population note.** 19 of the 31 are `checksum_failed_specimen` — documents whose *printed* zone
fails its own ICAO check digits. They are correctly outside the Tier-1 denominator, but their
hand transcription is still the authority on what is printed, so read-versus-print on them is a
valid measurement *of the recognizer*. They are included in the main matrix and every table below
carries the scored-only split as well, because the two populations turn out to have different
shapes ([§5](#5-confusions-by-check-digit-coverage-class)).

### The alignment rule

Three gates, in order. A truth cell and a read cell are compared **only** if all three hold.

1. **Record gate.** Truth and read must have the same line count, and the truth must have the
   shape its resolved format declares (`mrz_zone_matches_layout`, the same guard
   `mrz_field_mismatch` applies). A truth of 2×44 read back as 3×30 is a format misdetection, not
   a set of character errors.
2. **Line-assignment gate.** For read line *i*, compute the Levenshtein distance to **every**
   truth line. The pairing (truth *i*, read *i*) is accepted only if read line *i*'s unique
   nearest truth line is truth line *i*. This is the gate that matters most and the one the
   dump's own `field_mismatch_positions` does not have.
3. **Shift gate.** On an accepted pairing, positional (Hamming) comparison is used only when
   Hamming equals Levenshtein — that is, when no insert/delete alignment explains the line more
   cheaply. If an indel alignment is strictly better, the line has shifted and per-column pairs
   would be fiction.

**Why gate 2 exists, and what it caught.** `provider-bench`'s `field_mismatch_positions` aligns
line *i* of the read against line *i* of the truth, positionally, over the longer of the two. On
**10 of the 57 line pairs** in this dump the read's line *i* is not the printed line *i* at all —
it is a second, independent re-read of a *different* printed line (most often line 1 returned
twice), or on one TD1 a line of ordinary printed card text that is not MRZ. Comparing those
positionally manufactures a full line of confident, well-formed, entirely fictional glyph pairs.
Left in, they were the single largest contributor to the `0 → <` and `< → C` counts in a first
pass of this analysis; `0 → <` fell from 41 to 3 once the gate was applied. A confusion matrix
built on that alignment is worse than none, which is why the rule is stated before the numbers.

Gate 3 removed a further 12 lines where an indel alignment beats positional — horizontal shift of
a filler run, not per-cell substitution.

### What that leaves

| | records | lines | cells |
| :--- | ---: | ---: | ---: |
| Dumped | 31 | — | — |
| Excluded at the record gate | 5 | — | — |
| — nothing recovered (band scores 0.361, 0.623, 0.677) | 3 | | |
| — 2-line truth read back as a 3-line zone (format misdetection) | 2 | | |
| Eligible records | 26 | 57 | |
| Lines dropped, wrong-line assignment (gate 2) | | 10 | |
| Lines dropped, shift (gate 3) | | 12 | |
| **Lines aligned** | **25 records** | **35** | **1 428 compared** |

- **Aligned mismatched cells: 126** of 1 428 compared → per-cell accuracy **91.2%** on the
  aligned subset.
- **Mismatched cells dropped as unalignable: 587** inside eligible records, plus **221** in the
  two format-misdetected records — **808 dropped in total**.
- 18 of the 25 records contributed at least one mismatched cell; 7 contributed aligned lines with
  zero mismatches (one of those is a byte-perfect read of a non-conforming printed zone).

**Reconciliation.** The dump's own naive positional `zone_mismatch`, summed over every record
with a read, is **934**. That is exactly 126 + 587 + 221. Nothing is lost or invented by this
rule; 86.5% of the naive character-mismatch mass is reclassified from "confusion" to
"unalignable", which is the finding as much as the matrix is.

**Six records carry no `field_mismatch_positions`**, as expected, for four distinct reasons:
two `no_mrz_found` and one `checksum_failed_specimen` recovered nothing (`null`); one recovered
the printed zone byte-for-byte, so the map is `{}` and not `null` — absent versus measured-zero,
correctly distinguished; and two hit the layout guard because the reader resolved a format the
truth's own shape does not corroborate (`null`).

## 2. The confusion pairs, ranked

35 distinct ordered pairs over 126 aligned mismatched cells. `docs` is how many distinct
documents contribute; `max doc` is the largest single-document share, because a pair carried by
one specimen is a different claim from one carried by six.

| printed → returned | n | scored / specimen | docs | max doc |
| :--- | ---: | :--- | ---: | ---: |
| `<` → `C` | 29 | 4 / 25 | 3 | 24 (83%) |
| `0` → `O` | 26 | 12 / 14 | 6 | 10 (38%) |
| `<` → `E` | 10 | 7 / 3 | 4 | 7 (70%) |
| `<` → `Z` | 9 | 2 / 7 | 4 | 5 (56%) |
| `<` → `S` | 8 | 0 / 8 | 3 | 4 (50%) |
| `<` → `3` | 7 | 7 / 0 | 2 | 6 (86%) |
| `0` → `D` | 4 | 2 / 2 | 2 | 2 (50%) |
| `0` → `<` | 3 | 0 / 3 | 2 | 2 (67%) |
| `7` → `<` | 2 | 2 / 0 | 1 | 2 (100%) |
| `M` → `N` | 2 | 0 / 2 | 2 | 1 (50%) |
| `<` → `I` | 2 | 1 / 1 | 2 | 1 (50%) |

The remaining 24 pairs occur **once** each: `2`→`0`, `0`→`2`, `9`→`4`, `9`→`<`, `8`→`<`,
`4`→`<`, `9`→`O`, `2`→`<`, `1`→`<`, `O`→`0`, `A`→`0`, `M`→`0`, `4`→`0`, `7`→`4`, `9`→`2`,
`Z`→`7`, `W`→`M`, `N`→`R`, `0`→`G`, `H`→`4`, `F`→`1`, `5`→`S`, `Z`→`2`, `<`→`4`.

**Two pairs are 44% of everything.** `<` → `C` and `0` → `O` together are 55 of 126 cells. The
whole `<`-as-printed row is **66 cells, 52% of the matrix**; the whole `0`-as-printed row is 35
cells, 28%. Everything else is a long tail of singletons.

### Per-glyph read rate

Printed-glyph occurrences on aligned lines, and how often each was returned wrong. Only glyphs
printed ≥ 20 times are listed; below that the rate is not interpretable (`W` reads 1 wrong of 2,
`Z` 2 of 7 — noise, not signal).

| printed | occurrences | wrong | rate |
| :--- | ---: | ---: | ---: |
| `0` | 195 | 35 | **17.9%** |
| `<` | 537 | 66 | **12.3%** |
| `7` | 24 | 3 | 12.5% |
| `9` | 35 | 4 | 11.4% |
| `M` | 24 | 3 | 12.5% |
| `2` | 44 | 2 | 4.5% |
| `4` | 35 | 2 | 5.7% |
| `A` | 34 | 1 | 2.9% |
| `5` | 30 | 1 | 3.3% |
| `8` | 52 | 1 | 1.9% |
| `1` | 60 | 1 | 1.7% |
| `O` | 23 | 1 | 4.3% |
| `3` | 38 | 0 | 0% |
| `E` | 27 | 0 | 0% |
| `R` | 24 | 0 | 0% |
| `P` | 21 | 0 | 0% |

`0` is the worst-read high-frequency glyph, and `<` the second. The classic OCR shape confusions
are conspicuously *absent*: `1` is misread once in 60, `8` once in 52, `3` never in 38, `E` never
in 27. The recognizer is not failing at glyph discrimination in general. It is failing at two
specific glyphs.

## 3. Directionality

**The matrix is strongly asymmetric.** Every one of the eight pairs with n ≥ 3 has a reverse
count of 0 or 1. Raw counts are confounded by base rate (there are 537 printed `<` and 19 printed
`C`), so the rate normalization is given too — it does not rescue the symmetry.

| pair | A→B | B→A | rate A→B | rate B→A |
| :--- | ---: | ---: | ---: | ---: |
| `<` ↔ `C` | 29 | **0** | 5.40% (of 537 `<`) | 0% (of 19 `C`) |
| `0` ↔ `O` | 26 | 1 | 13.3% (of 195 `0`) | 4.3% (of 23 `O`) |
| `<` ↔ `E` | 10 | **0** | 1.86% | 0% (of 27 `E`) |
| `<` ↔ `Z` | 9 | **0** | 1.68% | 0% (of 7 `Z`) |
| `<` ↔ `S` | 8 | **0** | 1.49% | 0% (of 11 `S`) |
| `<` ↔ `3` | 7 | **0** | 1.30% | 0% (of 38 `3`) |
| `0` ↔ `D` | 4 | **0** | 2.05% | 0% (of 12 `D`) |
| `0` ↔ `<` | 3 | **0** | 1.54% | 0% |

This says something a symmetric similarity model structurally cannot say. Normalized
cross-correlation is symmetric by construction — `corr(a, b) = corr(b, a)` — so a template model
can rank a *pair* but cannot, on its own, predict that `<` is read as `C` twenty-nine times while
`C` is read as `<` never. Whatever produces this asymmetry is not shape distance: it is a bias
toward returning the **denser** glyph. Of the 66 cells where `<` was printed, all 66 returned a
full-cell glyph (`C E Z S 3 I 4`); of the 10 cells where `<` was returned, 9 had a **digit**
printed. The recognizer trades ink in one direction and, less often, loses it in the other — but
the two are not the same mechanism and are not the same size (66 versus 10).

## 4. The filler glyph `<`, both directions

`<` is 537 of the 1 428 aligned cells — 37.6% of everything printed. It is also the largest
single source of error.

**`<` printed, something else returned — 66 cells (52.4% of the matrix):**

| returned | n |
| :--- | ---: |
| `C` | 29 |
| `E` | 10 |
| `Z` | 9 |
| `S` | 8 |
| `3` | 7 |
| `I` | 2 |
| `4` | 1 |

**Something else printed, `<` returned — 10 cells (7.9%):** `0`×3, `7`×2, `9`, `8`, `4`, `2`,
`1`. Nine of the ten are digits; the tenth is also a digit. No letter was ever read as `<` in
this corpus.

**But the substitution matrix understates the filler problem by a factor of four**, and this is
the most important sentence in this report. The cells where the filler goes wrong are
overwhelmingly *not* substitutions — they are shifts and whole-line losses, which the alignment
rule (correctly) refuses to score as confusions:

- Of the 587 mismatched cells dropped inside eligible records, **299 (51%) are on the
  name-bearing line** — 252 from shift, 47 from wrong-line assignment.
- **13 of the 26 name-bearing lines were dropped**, against 9 of 31 data lines. Half the name
  lines in this dump cannot be aligned cell-to-cell at all.
- The name line is a long uniform filler run. A recognizer that under-collapses or
  over-collapses a run of identical glyphs produces a *shift*, not a substitution, and every cell
  downstream of it is wrong for one reason rather than many.

So the honest statement of the filler result is two-part: **within alignable lines, `<` accounts
for 52% of character confusions; and the filler run is also the reason half the name lines are
not alignable in the first place.** Both point at ADR-0014 `mrz-cell`'s fixed-grid, per-cell
premise rather than at a better generic recognizer, and the second is the larger effect.

## 5. Confusions by check-digit coverage class

Coverage class is taken from the dump's own `field_mismatch_coverage` vocabulary (`own_check_digit`
/ `composite_only` / `none`), re-derived per position from the same static `*_FIELDS` layout
`mrz_field_mismatch` uses, so the two cannot disagree.

**Whole dump (126 aligned mismatched cells):**

| coverage | wrong | cells | per-cell rate | share of errors |
| :--- | ---: | ---: | ---: | ---: |
| none (no check digit reaches it) | 68 | 681 | 10.0% | **54.0%** |
| own check digit | 48 | 623 | 7.7% | 38.1% |
| composite only | 10 | 124 | 8.1% | 7.9% |

**Scored misses only (52 aligned mismatched cells over 9 documents):**

| coverage | wrong | cells | per-cell rate | share of errors |
| :--- | ---: | ---: | ---: | ---: |
| none | 21 | 179 | 11.7% | 40.4% |
| own check digit | 23 | 292 | 7.9% | 44.2% |
| composite only | 8 | 75 | 10.7% | 15.4% |

**By field, whole dump** — every field that differed at all, with its own denominator:

| field | wrong | cells | rate | coverage |
| :--- | ---: | ---: | ---: | :--- |
| `document_number` | 37 | 171 | **21.6%** | own |
| `name` | 61 | 507 | 12.0% | none |
| `optional_data_2` | 7 | 33 | 21.2% | composite |
| `document_number_cd` | 3 | 19 | 15.8% | own |
| `sex` | 3 | 17 | 17.6% | none |
| `document_code` | 3 | 36 | 8.3% | none |
| `personal_number` | 6 | 182 | 3.3% | own |
| `composite_cd` | 2 | 16 | 12.5% | composite |
| `date_of_birth` | 1 | 102 | 1.0% | own |
| `date_of_birth_cd` | 1 | 17 | 5.9% | own |
| `optional_data_1` | 1 | 75 | 1.3% | composite |
| `optional_data` | 1 | 16 | 6.3% | none |
| `issuing_country` | 0 | 54 | 0% | none |
| `nationality` | 0 | 51 | 0% | none |
| `date_of_expiry` | 0 | 102 | 0% | own |
| `date_of_expiry_cd` | 0 | 17 | 0% | own |
| `personal_number_cd` | 0 | 13 | 0% | own |

**Answer to the question that matters: no — not in the way the framing expects, and the reason is
interesting.**

Errors do *not* concentrate per-cell in uncovered fields. The uncovered per-cell rate is 10.0%
against 7.7% covered — a 1.3× ratio, not a regime difference. Uncovered fields hold 54% of the
errors because the name span is 507 of 1 428 cells (35%) and is mostly filler, not because a cell
inside it is much more dangerous than a cell elsewhere.

What *is* sharp is the opposite of the expected result: **`document_number` is the worst-read
field in the corpus at 21.6%**, and it is fully covered by its own check digit. **All 26 `0` → `O`
confusions land in the document number or its check digit — 24 and 2, zero elsewhere.** That is
why they surface as `checksum_failed` at all: the check digit sees them, and the document is
filed as a scored miss rather than as a silent wrong read. Restricted to the scored population
the covered share rises to 59.6%, which is the same fact seen from the other side — the gate's
own misses are selected for being the errors a check digit can catch.

**This does not contradict
[`twelve-scored-misses-2026-09-19.md`](twelve-scored-misses-2026-09-19.md)'s "the check digits
are blind to 82% of the damage" (33 covered against 154 uncovered).** That count is taken under
naive positional alignment, which attributes a shifted or wrong-line name read as ~40 individual
uncovered character errors. Both are true of the same run and they measure different things: 82%
is the share of *damage* the check digits cannot see, and it is right; 40% is the share of
verified *per-cell substitutions* in the scored set that are uncovered. The gap between them —
130 of the 135 dropped scored cells are on the name-bearing line — **is** the uncovered damage,
and it is structural loss rather than glyph confusion. Cite the 82% for "what the check digits
miss"; do not cite it as a confusion count.

## 6. A falsifiable prediction for the template-similarity side

Committed before seeing the NCC matrix. Each item is scoreable.

**P1 — it will find `0` ↔ `O`.** I expect `{0, O}` in the NCC top-5 most-similar pairs. This is
the one observed confusion I expect a pure shape model to reproduce. *(High confidence.)*

**P2 — it will miss the entire `<` family.** I expect `{<, C}`, `{<, E}`, `{<, Z}`, `{<, S}`,
`{<, 3}` **all outside the NCC top-20**. Those five pairs are 63 of 126 aligned mismatched cells
— **50% of the observed matrix that a template model does not explain.** A chevron occupying the
left half of the cell has low pixel overlap with any full-cell letter; if NCC ranks these highly
anyway, my mechanism claim is wrong. *(High confidence.)*

**P3 — NCC's nearest neighbour to `<` will not be `C`.** I expect its top-3 neighbours of `<` to
come from `{K, X, Y, V, 7, L, J}` — angular, partially-inked glyphs. Observed, the nearest
neighbour of `<` is `C` by a factor of three over the next. *(Moderate confidence; this is the
single crispest disagreement to check.)*

**P4 — it will produce high-similarity pairs with near-zero observed support.** Specifically
`{8, B}`, `{1, I}`, `{1, 7}`, `{6, G}`, `{2, Z}`, `{D, O}`, `{U, V}`, `{E, F}`, `{C, G}`,
`{5, S}`. Observed: `8` is wrong once in 52, `1` once in 60, `6` never in 19, `E` never in 27,
`5` → `S` once in 30, `D` → `0` never in 12. These are the model's expected false alarms and they
are the cheapest way to score it. *(High confidence.)*

**P5 — top-10 overlap ≤ 3.** Intersecting the observed top-10 ordered pairs with the NCC top-10
(read as unordered pairs), I predict **at most 3 in common**, and I predict `{0, O}` is one of
them.

**P6 — the structural one.** NCC is symmetric; the observed matrix is not (§3: eight of eight
pairs with n ≥ 3 have a reverse count of 0 or 1). A similarity-only model therefore cannot
explain the *direction* of any confusion in this corpus, only the pairing. If the predicted side
reproduces directionality, it is carrying a prior — a per-glyph frequency or ink-density term —
and that term, not the correlation, is doing the work. **What would change my mind:** an NCC
matrix that ranks `<`→`C` highly *and* whose `<` row is dominated by the observed letters would
mean the filler failure is shape-driven after all, and ADR-0014's fixed-grid framing would be
solving the wrong half of it.

## What this does not claim

- **Not a full confusion matrix for the recognizer.** It is built from 31 *miss* records. The 140
  Tier-1 hits are not dumped, so every cell the recognizer got right on a successful document is
  invisible. Per-glyph rates in §2 are conditional on the document having missed, and are upper
  bounds on the recognizer's true error rate by a large and unmeasured factor.
- **Not a claim about how much the corpus is damaged.** 808 of 934 naively-mismatched cells are
  dropped. The matrix describes the 126 that survive a defensible alignment, not the whole miss.
- **Not statistically strong below n = 7.** 24 of the 35 pairs occur once. `<` → `C` is 83% one
  document and `<` → `3` is 86% one document; only `0` → `O` (6 documents, max share 38%) is
  carried by a broad set.
- **Not a per-format result.** 21 of 31 records are TD3 and one is MRV-A. Splitting 126 cells
  five ways would be arithmetic, not measurement.
- **Not independent of the ground truth.** Every pair is `fixture` → `read`. A transcription
  error in a fixture appears here as a recognizer confusion. Nothing in this analysis can tell
  the two apart; §2's singletons are the likeliest place for one to hide.
- **No timing claim.** A compile overlapped the run.

## Candidates rejected on the way

- **Naive positional alignment, the dump's own rule** (`field_mismatch_positions`). Rejected: it
  scored 41 `0` → `<` where 3 survive, and made `< → C` and `0 → <` look like a single
  filler-collapse phenomenon. 10 of 57 line pairs compare a read line against a printed line it
  is not. This is the rule already in shipped instrumentation and it is right for its own purpose
  (*how many* characters differ, for a coverage histogram); it is wrong for *which glyph became
  which*.
- **A similarity threshold instead of the line-assignment gate** — drop any line above X%
  mismatched. Rejected: at any threshold that removes the duplicate-line records it also removes
  the one line where every printed `<` was returned as `C`, which is the strongest observation in
  the corpus. Mismatch fraction does not distinguish "same line, catastrophically misread" from
  "different line entirely"; nearest-line assignment does, mechanically.
- **Minimum-cost one-to-one assignment over the line matrix.** Rejected: a permutation-based
  assignment is forced to give every read line *some* truth line, so a read that returns printed
  line 1 twice still has its duplicate assigned to truth line 2. It cannot express "this read
  line is not any line's partner", which is exactly the case that needed excluding.
- **Excluding the 19 `checksum_failed_specimen` records** to match the Tier-1 denominator.
  Rejected: their hand transcription is still the authority on what is printed, and dropping them
  would leave 52 cells over 9 documents. They are included, and split out in every table, because
  the scored and non-scored populations have materially different coverage shapes (§5) and the
  reader should see both.
- **Reporting Levenshtein-alignment pairs for the 12 shifted lines** rather than dropping them.
  Rejected: an optimal edit script is not unique when a run of identical glyphs shifts, so the
  substitutions it reports at the seam are an artifact of tie-breaking. Dropping 587 cells and
  saying so beats inventing 587 defensible-looking pairs.

## Unresolved

Not empty. Three things this run cannot settle.

1. **Whether `<` → `C` is a property of the recognizer or of one specimen.** 24 of 29 come from a
   single TD3 document. The direction and the density bias are corroborated across 3–4 documents
   for `E`/`Z`/`S`; the specific letter `C` is not. A second measurement over the 140 hits, or
   over a synthetic corpus with a matched filler profile, would settle it.
2. **The base rate is unknown.** Without dumping the hits, there is no denominator for "how often
   does the recognizer read a filler cell correctly across the whole corpus", only across misses.
   Every rate in §2 is conditional on failure.
3. **Fixture fidelity for the three newest recognition misses.** The README records that three
   `checksum_failed` documents from the c03/c07/c09 ingest had no hand transcription when they
   entered; they do now (#336), but whether their printed zones conform is recorded as unmeasured.
   If one of those fixtures is a transcription of what the *specimen* prints incorrectly, its
   cells are in this matrix as confusions.
