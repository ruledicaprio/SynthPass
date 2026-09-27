# Why `mrz` did not recover six M6 misses: one correct reading refused, three missing pairs, one cap, one date gate

**Date:** 2026-09-27 · **MAIN:** `c8ec825` (OCR run); replay at `01074ba`, whose `crates/mrz` is byte-identical · **DATA:** `396b22f3b97ee36275a5329667ee37cbd0449b40` · **Evidence:** Observed (release `provider-bench --real-specimens --mrz-only --dump-ocr --dump-ocr-hits`, all OCR arms at their defaults, 0 outcome changes against the committed ledger; the recorded provider input of six assets replayed through an instrumented copy of the shipped parser, reproducing the dumped zone on all six) plus Derived (counterfactual replays on the same copy with one gate lifted at a time; check-digit arithmetic) · **Status:** current

**2026-09-27.** This note closes the last gap in M6's frozen Definition of Done. The criterion is in
[ADR-0011](../decisions/ADR-0011-split-m6-packaging-into-m8.md)'s 2026-09-17 amendment: each of
the 14 frozen misses must be a HIT or carry "a dated attribution … naming the mechanism that
defeats it". An architect review on 2026-09-27 counted 8 of the 14 as meeting that standard. This
note does not re-check those 8. For the other six, the record held only a symptom: a cell position
and a failing check digit. The owner chose attribution as the route on 2026-09-27.

The question here is narrower than "why did OCR misread the cell". It asks why the deterministic
layer, `crates/mrz`, did not recover the misread cells. The answer comes from replaying each
document's recorded provider input through the shipped parser, with a trace on every repair
decision.

**No MRZ content, holder name, document number or zone fragment appears here.** Evidence is given
as asset IDs, counts, 0-based cell positions, ICAO field regions and character classes (L letter,
D digit, F filler). Neither [`twelve-scored-misses-2026-09-19.md`](twelve-scored-misses-2026-09-19.md)
nor [`checksum-failed-miss-mechanisms-2026-09-18.md`](checksum-failed-miss-mechanisms-2026-09-18.md)
names a glyph pair for these six, so no pair is named here either. The dump and the replay output
stay in gitignored `artifacts/`.

## The answer

Every one of the six has an observed mechanism. None of them is attempt selection.

| asset | the mechanism that defeats `mrz` | evidence |
| --- | --- | --- |
| `passports/Afghanistan_Passport_Specimen_P0_AFG_2016_mrz.webp` | **Recovered, then refused by the unanimity gate.** The single-substitution repair rebuilt the fixture's line 2 exactly. A second single-`CONFUSABLES` substitution, at document-number cell 6, also verifies every digit. It shares cell 0's weight, and the composite repeats the document number's weights. `single()` refuses to choose between the two | Observed |
| `passports/Czechia_Passport_Specimen_P0_CZE_2005_mrz.jpg` | **The damaged pass's date gate rejects the printed zone itself.** Its date of birth names no calendar day, so `accept_damaged` refuses every repaired reading, including the correct one. Behind that gate, the two misread pairs are not in `CONFUSABLES`, and two cells exceed the one-substitution cap | Observed |
| `passports/Germany_Passport_Specimen_P0_D00_2024_mrz.jpg` | **The misread pair is not in `CONFUSABLES`.** The only line-2 attempt in the provider input never reaches the substitution search anyway: it arrived one cell short, and it does not sit next to a line-1 attempt | Observed |
| `passports/Romania_Passport_Specimen_PE_ROU_2024_mrz.jpg` | **The misread pair is not in `CONFUSABLES`**: a filler read as a digit, and `<` is in no row. The search built 20 checksum-consistent readings, none of them correct, and `single()` refused them | Observed |
| `passports/Hong_Kong_Passport_Specimen_P0_HKG_2007_mrz.png` | **The damage exceeds the one-substitution cap.** Every line-2 attempt misreads at least four document-number cells. One of those pairs is also outside `CONFUSABLES`, and the misread is not one uniform class | Observed |
| `passports/Hong_Kong_Passport_Specimen_P0_HKG_2019_mrz.png` | **The misread pair is not in `CONFUSABLES`.** The two attempts where it is the only check-covered misread arrived one cell short, and the substitution search takes only exact-width lines | Observed |

The four candidate mechanisms examined were:

1. the misread pair is not in `CONFUSABLES`;
2. the damage exceeds the one-substitution cap;
3. the field's check-digit cell is misread too, so the check cannot arbitrate;
4. a verifying attempt existed among the OCR passes but was not the one returned.

Mechanisms 1 and 2 bind as the table says. Mechanisms 3 and 4 bind for none of the six. Two
mechanisms outside the list decide Afghanistan and Czechia: the unanimity gate and the date gate.

**Verdict: all six are attributed to a mechanism, each observed on today's run.** No single
extension of today's repair turns any of the six into a hit on this provider input (**Derived**,
Table 5). The extensions tested were one more `CONFUSABLES` row, width tolerance, a wider pairing
window and the class sweep. Wherever a lifted gate lets the correct reading into the candidate set,
checksum-consistent rivals come with it, and the unanimity gate refuses. That refusal is working
as designed, so this evidence offers no repair lever for these six. Attribution is how they close.

**Hong Kong, line 1.** Only line 2 matters to the outcome. A TD3 line 1 carries no check digit, so
the filler fabrication on both Hong Kong line 1s cannot fail any check. For the measurement and
the mechanisms it rules out, see
[`hong-kong-filler-context-2026-09-19.md`](hong-kong-filler-context-2026-09-19.md) (:30-33). It
costs names, which [ADR-0013](../decisions/ADR-0013-names-are-scored-against-mrz-form-truth.md)
scores separately. It does not cost the hit.

## Table 0: run identity

| Item | Value | Source |
| --- | --- | --- |
| Instrument commit | `c8ec8258e2b5dfa070517b0f65d9692fb4a6488e`, clean tree | run manifest `git_commit`, `working_tree_dirty` |
| Replay commit | `01074ba`. `git diff c8ec825 01074ba` touches only `crates/synthpass-cli/src/generate.rs` | `git diff --stat` |
| Flags | `--real-specimens --mrz-only --dump-ocr --dump-ocr-hits --progress --assert-baseline knowledge/benchmarks/real-specimen-mrz-baseline.json --out artifacts/phase0/provider-bench-report.json` | manifest `flags` |
| OCR arms | chargrid `off`, order/rotate/skew `default`, texture `on`: all defaults. `mrz` class-sweep arm `off` | manifest `ocr_arms`, report `mrz_class_sweep_arm` |
| OCR environment | `SYNTHPASS_OCR_MODEL_DIR` = worktree root, `SYNTHPASS_OCR_AUTO_DOWNLOAD=0`, `SYNTHPASS_OCR_MAX_PASSES` unset (the container sets it to 1) | the calling session |
| OCR models | `text-detection.rten` `f15cfb56…b5ca`, `text-recognition.rten` `e484866d…5a6e`, both equal to `real-specimen-gate.yml`'s pins | `sha256sum` against the workflow |
| DATA | `396b22f`, the committed baseline's pin. The six assets' bytes on disk, and the `source_sha256` the run recorded, equal the blobs at that commit | `git show 396b22f:samples/<asset>` |
| Corpus manifest SHA-256 | `37274376…d17c`, 261 documents, 65 labelled | manifest |
| Pivot | 26 | manifest `pivot_yy` |
| Platform | Linux container, 4 vCPUs, release build, one run | the calling session |

## Table 1: populations, both denominators

| Population | Count |
| --- | --- |
| Assets (corpus-wide) | 261 |
| Scored (Tier-1 denominator) | 151 |
| Tier-1 hits | **139 / 151 = 92.1%** scored, **139 / 261 = 53.3%** corpus-wide |
| Scored misses: `checksum_failed` / `no_mrz_found` | 10 / 2 |
| Off-denominator: `checksum_failed_specimen` / `redacted_mrz` / `no_mrz_expected` | 20 / 39 / 51 |
| Outcome changes against the committed ledger | 0 (**Observed**, `--assert-baseline`). The calling session also matched the summary to CI gate run 36300160981 (`1ef7426`) |
| Assets examined here | 6 of the 10 `checksum_failed`, all TD3 |

These six are the recognition misses that twelve-scored-misses §2 left with only a damage shape.
Their returned-zone damage positions are identical, cell for cell, to that note's at `bb658d6`.
They agree with the phase-0 rows at `738cd16`, and for Afghanistan, Czechia and Romania with
the 2026-09-18 run at `d2d5855` too. The misreads are stable across every run that recorded
them.

## How `mrz` handles a line-2 misread

Every mechanism below is a named branch of this path. All references are to `crates/mrz/src` at
`01074ba`.

1. **Ordinary scan** (`parser.rs:1515`). For each line-1 candidate, the scan pairs each of the next
   three lines as line 2 (`:1643`), in its `variants`. A line 2 that is 1–14 cells short is
   padded (`checksum.rs:237`). Line-2 check-digit cells and dates are digitized
   (`repair_td3_line2`, `parser.rs:1199`); the document number is alphanumeric and is not. The
   first reading that verifies every digit is returned. Otherwise the best partial reading is kept
   as the fallback (`:1537`, ranked by `fallback_rank`, `:2454`).
2. **Damaged pass** (`:2310`). It runs only if nothing verified and a fallback exists (`:1883`). It
   pairs only **adjacent** lines, `lines[i]` with `lines[i + 1]` (`:2413`). Its single-glyph
   shape is `substituted` (`:1997`), which accepts only a line **exactly** the target width
   (`:1999`). It changes **one** cell (`MAX_SUBSTITUTIONS = 1`, `repair.rs:349`), and only to a
   `CONFUSABLES` alternative (`repair.rs:399`). Its width shape, `restored` (`:1964`), takes a line
   exactly one cell short, but it only inserts a cell and never substitutes. The two shapes are
   mutually exclusive by construction.
3. **Acceptance.** A repaired reading must verify every digit *and* carry calendar dates
   (`accept_damaged`, `:2291`). The surviving readings must then agree on every field except the
   raw zone (`single`, `:2486`), or nothing is returned.
4. The class sweep is off by default (`lib.rs:207`).

## Table 2: the returned line 2, cell by cell

Classes read printed → returned. "In `CONFUSABLES`" means the pair appears in either direction.
**Observed** (dump against fixture).

| asset | cell(s) | ICAO field | class | in `CONFUSABLES` | failing checks |
| --- | --- | --- | --- | --- | --- |
| Afghanistan 2016 | 0 | document number | L→D | **yes** | document number, composite |
| Czechia 2005 | 33, 41 | personal number | D→D, F→D | no, no | personal number, composite |
| Germany 2024 | 1 | document number | L→D | no | document number, composite |
| Romania PE 2024 | 41 | personal number | F→D | no | personal number, composite |
| Hong Kong 2007 | 1, 2, 3, 8; 9; 37, 38 | document number; its check digit; personal number | D→L ×5; F→D ×2 | no, yes, yes, yes; yes; no, no | document number |
| Hong Kong 2019 | 0; 20 | document number; sex (uncovered) | L→D; L→D | no; no | document number, composite |

## Table 3: what the damaged pass did, as shipped

This is a trace of the shipped decision path. Every other line of the copy is identical to
`crates/mrz`. The copy only adds a thread-local log and read-only accessors. **Observed:** on all
six, the replay returns exactly the zone the dump recorded.

A **covered-correct** hit agrees with the fixture on every check-covered cell. Nationality (cells
10–12) and sex (cell 20) are uncovered, so a covered-correct hit may still differ from the
fixture there.

| asset | substitution candidates built | readings accepted | covered-correct / wrong | `single()` | attempts used of 200,000 |
| --- | --- | --- | --- | --- | --- |
| Afghanistan 2016 | 58 each, from three adjacent pairs | 6 | **3 / 3** (the 3 are the fixture's line 2, byte for byte) | refused | 364 |
| Czechia 2005 | 149, from one adjacent pair | 0 (4 readings verified, all refused by `accept_damaged`, all wrong at cells 33 and 36 and 41) | 0 / 0 | nothing to decide | 298 |
| Germany 2024 | 0: no adjacent pair of a line-1 and a line-2 attempt exists | 0 | 0 / 0 | nothing to decide | 0 |
| Romania PE 2024 | 166, from one adjacent pair | 20 | **0 / 20** | refused | 664 |
| Hong Kong 2007 | 89 by substitution, plus width restoration on two 43-cell attempts | 0 | 0 / 0 | nothing to decide | 46,596 |
| Hong Kong 2019 | 19, 69 and 108 by substitution, plus width restoration on six one-cell-short lines | 0 | 0 / 0 | nothing to decide | 50,179 |

**The budget never binds.** The largest use is 50,179 of `MAX_DAMAGED_ATTEMPTS = 200_000`.

## Table 4: the line-2 attempts in the provider input

A line-2 attempt is a raw line whose best `variants` reading lies within 12 cells of the fixture's
line 2. "Verifies as read" means some `variants` reading of that attempt satisfies all five TD3
digits. **Observed.**

| asset | attempts | carrying the returned misread | verifying as read | exactly 44 wide | whose substitution set contains the fixture's line 2 |
| --- | --- | --- | --- | --- | --- |
| Afghanistan 2016 | 11 | 11 | 0 | 7 | 6 |
| Czechia 2005 | 8 | 7 carry cell 33 (the eighth is 40 wide and 5 cells off once padded) | 0 | 2 (cells 11+33 and 33+41 wrong) | 0 |
| Germany 2024 | 1 | 1 | 0 | 0 (43 wide) | — |
| Romania PE 2024 | 2 | 2 | 0 | 2 | 0 |
| Hong Kong 2007 | 3 | 3, with 4, 5 and 6 wrong document-number cells | 0 | 1 | 0 |
| Hong Kong 2019 | 7 | 7 | 0 | 2, each with a second covered misread | 0 |

**No attempt verifies as read, on any of the six (0 of 32).** The phase-0 note found an
attempt-selection case for Belgium and Sweden. There is none here: the OCR returned the same
misread in every attempt it made.

## Table 5: which gate binds (counterfactual replays, Derived)

The counterfactuals run on the same copy with one gate lifted at a time:

- **"+ pair"** adds the returned zone's misread pairs on check-covered cells to `CONFUSABLES`.
- **"+ width"** lets `substituted` also sweep `fit_length`'s padded forms of a line one or two
  cells short.
- **"+ window"** lets the damaged pass pair line *i* with any of the next three lines, as the
  ordinary scan does.

Each cell gives covered-correct / covered-wrong accepted readings. None of these arms is shipped
behaviour, and none describes the default arm.

| asset | as shipped | + pair | + pair + width | + pair + window | + pair + width + window | class sweep on (shipped, non-default) |
| --- | --- | --- | --- | --- | --- | --- |
| Afghanistan 2016 | 3 / 3, refused | pair already listed | 3 / 3, refused | 4 / 4, refused | 4 / 4, refused | no change |
| Czechia 2005 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | no change |
| Germany 2024 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | **3 / 6, refused** | no change |
| Romania PE 2024 | 0 / 20, refused | **4 / 20, refused** | 4 / 20, refused | 4 / 20, refused | 4 / 20, refused | no change |
| Hong Kong 2007 | 0 / 0 | 0 / 0 | 0 / 40 | 0 / 0 | 0 / 40 | no change |
| Hong Kong 2019 | 0 / 0 | 0 / 0 | **6 / 12, refused** | 0 / 0 | 6 / 12, refused | no change |

Isolation, Afghanistan (**Observed**): replayed on its own, the adjacent pair of attempts that
produced the exact recovery yields 1 covered-correct and 1 covered-wrong reading, refused. The
other pair yields 2 and 2, refused.

## Per document

**Afghanistan 2016: recovered, then refused.** None of candidate mechanisms 1–4 applies:

- the pair is in `CONFUSABLES`;
- one cell is wrong;
- the check digit is read correctly;
- no attempt verifies as read.

The repair works. Exactly two single-`CONFUSABLES` substitutions of the returned line 2 satisfy all
five digits (**Derived**, recomputed independently of the trace): the true fix at cell 0, and a
digit read as a letter at cell 6. Cells 0 and 6 share the weight 7. So a swap at either cell with
the same value shift mod 10 satisfies the document-number digit equally.

The composite adds nothing here. Its weights over the document number are the field's own weights,
so for an error confined to that field it is the same equation twice
([`checkdigit-blindspots-exact-2026-09-26.md`](checkdigit-blindspots-exact-2026-09-26.md) §1c).

A second line-1 attempt, 42 cells wide and paired with its own line 2, also yields accepted
readings. Their line 1 differs from the fixture in six name-region cells, so `single()` would
refuse on that disagreement too. The refusal is the parser working as designed. Returning either
reading would be a guess, and the rival carries a wrong document number.

**Czechia 2005: the printed zone fails the repair path's own acceptance bar.** The fixture's date
of birth is six digits that name no calendar day (`MrzDate::OutOfCalendar`). The OCR returns the
same six digits, and the date-of-birth check digit verifies them. Parsing the fixture's own zone
gives `valid() = true` and `accept_damaged = false` (**Observed**).

So no repaired reading of Czechia can ever be accepted. Only an unrepaired read that verifies as
read could score. Behind that gate:

- the misread pairs at cells 33 (a digit read as another digit) and 41 (a filler read as a digit)
  are both outside `CONFUSABLES`;
- the attempt the ordinary scan returned needs two substitutions, where the cap allows one;
- the one attempt that reads cell 41 correctly still misreads cell 33, and it is 42 wide.

**Germany 2024: pair not listed, and the only attempt never reaches the search.** 48 provider-input
lines hold exactly one line-2 attempt. It misreads document-number cell 1, a letter read as a
digit, and that pair is not in `CONFUSABLES`. It also arrived 43 cells wide, while `substituted`
takes exact width only. The line before it is a 21-cell fragment, and the nearest line-1 attempt is
two lines up. The ordinary scan's three-line window pairs them; the damaged pass's adjacency does
not.

Adding the pair alone recovers nothing. Lifting all three gates lets the right reading in, alongside
six wrong ones, and it is refused (**Derived**).

**Romania PE 2024: pair not listed.** The filler at personal-number cell 41 is read as a digit. `<`
appears in no `CONFUSABLES` row, so no candidate restores it. Both line-2 attempts carry this
misread.

The substitution search still found 20 readings that satisfy every digit and have calendar dates:

- five distinct values across the check-covered cells;
- doubled by an uncovered nationality variant and by two line-1 readings;
- every one right on the document number and wrong in the personal number.

This is the personal number's composite alignment again (blindspots §1c). `single()` refused all
20. With the pair added, the correct reading joins them and is refused too (**Derived**).

**Hong Kong 2007: over the cap.** The printed document-number cells 1, 2, 3 and 8 and the check
digit hold one repeated digit. Every attempt returns at least four of those cells as letters, three
different ones.

Three of the four document-number pairs are in `CONFUSABLES`; cell 1's is not. The check-digit
cell's pair is listed, and `repair_td3_line2` digitizes it back, so candidate mechanism 3 does not
bind here. The check digit can arbitrate. It just has four or more wrong cells to arbitrate over,
against a cap of one.

The class sweep would need one uniform class and finds three. With every pair added and width
tolerance on, the search accepts 40 readings, none of them correct (**Derived**).

**Hong Kong 2019: pair not listed, and the best attempts are the wrong width.** All seven attempts
misread document-number cell 0 (a letter read as a digit, not in `CONFUSABLES`) and the sex cell
20, which no digit covers.

The two attempts whose only check-covered misread is cell 0 arrived 43 wide, so `substituted` skips
them. The two exact-width attempts each carry a second covered misread, at more than the cap
allows. With the pair added and width tolerance on, the correct reading is accepted alongside 12
wrong ones and refused (**Derived**).

## Where this agrees and disagrees with earlier records

- **[`mrz-strip-phase0-results-2026-09-24.md`](mrz-strip-phase0-results-2026-09-24.md)**
  - :15-18, :319. "No label is a mechanism", because string relations cannot show one. **Agrees.**
    This note supplies the mechanism from a replay of the parser, not from string distance. It is
    the replay :324-325 said had not been built. It covers only these six and only the repair
    path, not an aligner.
  - :252-263. The rows for the six (class B, and the cells named there) **agree cell for cell.**
- **[`twelve-scored-misses-2026-09-19.md`](twelve-scored-misses-2026-09-19.md)**
  - §2 and :308-310. The positions **agree exactly.** The note said run shape "does not name the
    mechanism" (:127-130) and named three of nine; this note names six more.
  - :319-322. It puts Afghanistan, Czechia, Germany and Romania among the eight the browser stack
    reads. That is **consistent**: their obstacle is what `mrz` does with one or two misread cells,
    not legibility.
- **[`checksum-failed-miss-mechanisms-2026-09-18.md`](checksum-failed-miss-mechanisms-2026-09-18.md)**
  (:5, :26-48). Its entries call a single-glyph substitution the "mechanism". **Agrees on the
  damage**, and the damage is a symptom. Its Czechia entry is headed "Czech identity-card
  specimen", but the asset is a passport, TD3.
- **[`mrz-class-sweep-ceiling-2026-09-20.md`](mrz-class-sweep-ceiling-2026-09-20.md)**:
  **Disagrees.** It records the dates as well-formed on every TD3 row and says the date clause
  excludes "one of twelve" scored misses. Czechia 2005 is a second: its fixture fails
  `accept_damaged` on its own date of birth (**Observed**, above). The ceiling it derives for the
  class sweep (one) is unaffected, because Czechia is not sweepable.
- **[`checksum-failed-real-specimens-2026-09-08.md`](checksum-failed-real-specimens-2026-09-08.md)**
  (superseded)
  - **Afghanistan: disagrees in part.** It says "the blocker is line 1". Today the returned line 1
    is exact, and the document-number rival alone is enough to refuse (isolation above). A second
    attempt's line 1 is a second, independent source of refusal, so the earlier reading was half
    right.
  - **Czechia: agrees** that the cap binds, and adds the missing pairs and the date gate.
- **[`hong-kong-filler-context-2026-09-19.md`](hong-kong-filler-context-2026-09-19.md)**
  (:30-33). **Consistent.** The line-1 fabrication is outside every check-digit region, and this
  note does not need to resolve its cause.

## What this does not claim

- **It explains what `mrz` did, not why the recognizer misread the cells.** Print, texture,
  resolution and model behaviour are not measured here.
- **It is one OCR run.** `rten` run-to-run noise flips about three documents. The misread cells
  reproduce in every run that recorded them, but another run's provider input can carry
  different attempts, and the trace describes this run's.
- **Table 5 is not shipped behaviour and not a proposal.** Each arm lifts a gate for diagnosis. No
  arm was run over the corpus, so none says how many *other* documents a lifted gate would break.
  That breakage is exactly what those gates exist to prevent.
- **"Refused" is not "the gate is wrong".** In every refusal here, at least one accepted rival
  disagrees with the fixture on a check-covered cell. Returning it would have been a silent wrong
  read.
- **The other 8 of the 14 frozen misses were not re-examined.** Their status is the architect
  review's.
- **No baseline number moves.** This is documentation only.

## Rejected on the way

- **Attempt selection (candidate 4), for all six.** No line-2 attempt verifies as read: 0 of 32.
- **Check-digit cell misread (candidate 3), for Hong Kong 2007.** The cell is misread in the
  returned zone, but the digitized variants carry the fixture's digit, so the check can arbitrate.
- **Budget exhaustion.** No run came near `MAX_DAMAGED_ATTEMPTS`; the largest use was 25%.
- **Candidate 1 for Afghanistan.** The pair is listed, and the repair found the right reading.
- **"The blocker is line 1" as Afghanistan's sole cause.** Rejected by the isolation replay above.
- **The class sweep as a lever for any of the six.** Replayed with `with_class_sweep(true)`, it
  changes no outcome.
- **String distance from the nearest provider-input line as the basis for a mechanism.** This is
  the phase-0 note's rejection, kept. Every mechanism here is a named branch in a trace of the
  parser.

## Also found

- **Hong Kong 2007's returned zone verifies two digits it should not.** The personal-number digit
  verifies over two fabricated digits in a filler run. The composite verifies over seven wrong
  covered cells.

  `fallback_rank` therefore preferred the un-digitized reading, 4 of 5 digits verified, over the
  digitized one at 3 of 5. The returned zone thus carries a letter in the document-number check
  digit that the repaired variant had already fixed (**Observed**). It is a miss either way.
- **The unanimity gate is load-bearing on Romania.** All 20 readings it refused have the right
  document number and a wrong personal number. Accepting any one of them would have scored a Tier-1
  HIT on a wrong read. This is the same class as the Bosnia 2013 refusal
  ([FINDINGS.md, 2026-09-24](FINDINGS.md#2026-09-24--a-manufactured-hit-refused-bosnia-2013-card-back-leaves-tier-1-140--152--139--151)).

## Invocation

```text
# OCR pass (run by the calling session; release provider-bench built at c8ec825, clean tree)
# SYNTHPASS_OCR_MODEL_DIR=<worktree root>  SYNTHPASS_OCR_AUTO_DOWNLOAD=0  SYNTHPASS_OCR_MAX_PASSES unset
target/release/provider-bench --real-specimens --mrz-only --dump-ocr --dump-ocr-hits --progress \
  --assert-baseline knowledge/benchmarks/real-specimen-mrz-baseline.json \
  --out artifacts/phase0/provider-bench-report.json
# -> 0 outcome changes vs the committed ledger; no regression vs baseline (139 / 151)

# Replay (this note). A copy of crates/mrz at 01074ba under artifacts/m6-six/mrz-instr, with a
# thread-local trace in damaged_pass/find_and_parse_with, read-only accessors, and two
# counterfactual switches that default off. The per-asset provider input and fixture were split
# out of provider-bench-miss-ocr-dump.jsonl into artifacts/m6-six/in/.
cd artifacts/m6-six/mrz-instr && CARGO_TARGET_DIR=../target cargo build --release --offline --bins
../target/release/replay ../in Afghanistan Czechia Germany Romania Hong_Kong_2007 Hong_Kong_2019
../target/release/cf ../in
```

Both binaries print only positions, counts, booleans and character classes.
`tools/classify_mrz_mechanisms.py` was not re-run. This note uses no string-distance label.
