# The C3 and C6 refusal switches over one replayed capture: C3 refuses one scored hit (Croatia 2002, a wrong read an earlier note counted as a hit) and re-labels one miss, C6 re-labels one miss, and no synthetic correct read is lost

**Date:** 2026-10-03 · **MAIN:** `c60936d` (`origin/main` when the job started; one release build of `provider-bench` and `synthpass-bench` from it served the capture, every replay and every synthetic arm; the run manifests say `working_tree_dirty: true`, which is the tree's untracked files only) · **DATA:** the local `samples/` mirror of `samples-data` at `65a5040` (the names after [#556](https://github.com/ruledicaprio/SynthPass/pull/556) and [#658](https://github.com/ruledicaprio/SynthPass/pull/658); every image's blob hash equals that revision's), corpus manifest sha256 `002180019f5d`, all 261 documents; synthetic clean, seeds 0-99 of each of the five formats · **Evidence:** Observed on one live capture, replayed, and on synthetic renders; Derived where marked · **Status:** current

**2026-10-03.** This is the A/B that `knowledge/PLAN_2026_Q4.md` lists as "#579(b), (c), (d): the A/B", for the two
opt-in refusal switches already on `main`:

- **C3 refuse**, `SYNTHPASS_MRZ_REFUSE_REPEATED_LINE` ([#633](https://github.com/ruledicaprio/SynthPass/pull/633)):
  a zone whose lines repeat one another is refused;
- **C6**, `SYNTHPASS_MRZ_DATE_DIGITS` ([#631](https://github.com/ruledicaprio/SynthPass/pull/631)): a date field must
  hold digits.

Both are off by default, and since [#649](https://github.com/ruledicaprio/SynthPass/pull/649) a miss reason names the
opt-in rule that refused the read. Both act after the OCR text, so the plan's method is a replay
([README, "Replaying a captured run"](README.md#replaying-a-captured-run)): one live capture is read once, and every
arm replays the same text. The 09-30 capture is retired (#658 changed `samples/corpus.jsonl`, and a replay refuses a
capture whose manifest hash differs), so this is a fresh one. The plan's third rule (d), the filler as check digit 0,
has no switch on `main`: the only `SYNTHPASS_MRZ_*` arms are `CLASS_SWEEP`, `LINE1_SELECT`, `REFUSE_REPEATED_LINE` and
`DATE_DIGITS`. It is not measured here. No code, ADR, baseline or default changes with this note.

**The run.**

| | |
|---|---|
| Machine | win11-i7-1255U, 12 logical CPUs, Windows, AC power; nothing else ran during the capture; every command through `slot.ps1`, one at a time |
| `RTEN_NUM_THREADS` | 4 (the laptop's usual value for slot runs; set explicitly, not unset). `SYNTHPASS_OCR_MODEL_DIR` points at the repo's `models\`; no other `SYNTHPASS_OCR_*` or `SYNTHPASS_MRZ_*` variable is set except the arm's |
| Capture | `provider-bench --real-specimens --mrz-only --dump-ocr --dump-ocr-hits --dump-ocr-passes`, **18:32:44Z to 19:22:05Z, exit 0**; run manifest `provider-bench-ocr-run-654cf3f00829...json`, SHA-256 `654cf3f00829bc540c6e4f9311540a1b6ecc5133fff523f5171cc4d55d9cddbd`; 261 pass rows |
| Synthetic | `synthpass-bench --document-type <f> --profile clean --count 100 --seed 0 --dump-ocr`, each of `td1 td2 td3 mrva mrvb`, **live** in the default arm and in each of the four arms, under the arm's variable |
| Replays | `provider-bench --real-specimens --mrz-only --dump-ocr --dump-ocr-hits --replay-ocr-passes <capture>`, 10 to 14 s each, exit 0; each manifest's `replay_of` names the capture's manifest and hash above |
| Whole job | 18:32:44Z to 21:50:08Z; every capture, replay, synthetic run and diff exited 0 |

The arms, quoted from each run's manifest (`mrz_arms`), not from the variable that was set. Every arm has
`class_sweep` `off` and **`line1_select` `on`**, which is `main`'s default (the variable is unset): the selector of
[#609](https://github.com/ruledicaprio/SynthPass/pull/609) acts in every arm, the capture included.

| Arm | `refuse_repeated_line` | `date_digits` |
|---|---|---|
| default (capture, replay-default) | off | off |
| `c3-control` | **control** | off |
| `c3-on` | **on** | off |
| `c6-control` | off | **control** |
| `c6-on` | off | **on** |

## The answer

1. **Inspect: the replay and both placebos are identical to the capture (Observed).**
   `bench_ab_diff --expect-identical --check-report` returned NEUTRAL, exit 0, for the default replay and for
   `c3-control` and `c6-control` against the default arm. Each reads: real documents 261 / 261, ledger differ 0, dump
   171 / 171 rows differ 0, report rows 261 / 261 differ 0; five synthetic formats, 100 / 100 seeds each, `results[]`
   differ 0 and dump blocks differ 0 (hits 59, 81, 74, 87 and 90); budget stops 2 in both arms (France ID 2020's two
   sides, which hit the 52 s budget in the capture). The runs differ only in `flags`, `replay_of`, `model_paths` and, for
   `c6-control`, the recorded arm. The synthetic arms are live runs under the control variable, so for them this is an
   identity check of two OCR runs, not a copy.
2. **The crosstab by `asset_id`, per rule (Observed).** "Correct" means a scored hit in the default arm. The ledger
   (outcome, miss reason, format and every other column) is compared for all 261 documents; no other document moves.

   | Rule | Document | Before | After | Class |
   |---|---|---|---|---|
   | C3 | `id_cards/Croatia_ID_Specimen_2002_back_mrz.jpg` | `hit` (TD2) | `checksum_failed`, reason "checksum invalid: document_number; rejected by the repeated-line rule: the Td2 zone holds a line twice" | **correct -> refused** |
   | C3 | `id_cards/Switzerland_ID_Specimen_ID_CHE_2003_back_mrz.jpg` | `checksum_failed_specimen`, "checksum invalid" | `checksum_failed_specimen`, "rejected by the repeated-line rule: the MrvB zone holds a line twice (printed zone is non-conforming)" | checksum-failed -> refused (the outcome does not move) |
   | C6 | `id_cards/Russian_Federation_ID_Specimen_2013_back_mrz.jpeg` | `checksum_failed_specimen`, "checksum invalid" | `checksum_failed_specimen`, "rejected by the date-digits rule: date_of_birth not digits (printed zone is non-conforming)" | checksum-failed -> refused (the outcome does not move) |

   | Class | C3 `on` | C6 `on` |
   |---|---:|---:|
   | correct -> refused (the cost) | **1** | **0** |
   | wrong or checksum-failed -> refused | 1 | 1 |
   | miss -> miss, a new reason that is not a refusal | 0 | 0 |
   | anything else | 0 | 0 |

   Under C3, Croatia's scored hit becomes a scored miss, and the TD2 hit count goes from 1 to 0; the zone the dump
   returns for it changes too (a two-line zone before, a three-line zone after). The line-1 selector's `unresolved`
   count falls from 3 to 1 under C3. Switzerland's recorded format column changes (MRV-B before, TD1 after). The names and field-correctness checks are unaffected: 67 documents with truth compared, 0 transitions, 0
   regressions, 0 name changes, in both rules. The placebo arms change nothing (answer 1).
3. **Tier 1 per rule, on both denominators (Observed).** Scored is 149 (the 261 without `no_mrz_expected` 51,
   `redacted_mrz` 39 and `checksum_failed_specimen` 22).

   | Arm | Hits | Of scored 149 | Of the corpus 261 | `checksum_failed` | Strict names |
   |---|---:|---:|---:|---:|---:|
   | default | 137 | 91.9% | 52.5% | 10 | 15 / 45 |
   | C3 `on` | **136** | **91.3%** | **52.1%** | 11 | 15 / 45 |
   | C6 `on` | 137 | 91.9% | 52.5% | 10 | 15 / 45 |

   Hits by format, default: TD3 123, TD1 12, TD2 1, MRV-B 1; C3 `on` has TD2 0. (The 09-29 note's capture had 139
   hits: a different capture, before #630 and #658.)
4. **Synthetic per rule and format (Observed).** Clean, seeds 0-99, 100 documents per format, live OCR in every arm.

   | Format | Hits | Wrong accepts | Correct reads | C3 `on` hits / wrong / correct | C6 `on` hits / wrong / correct | Seeds that moved |
   |---|---:|---:|---:|---|---|---|
   | TD1 | 59 | 28 | 31 | 59 / 28 / 31 | 59 / 28 / 31 | **C3: seed 0** |
   | TD2 | 81 | 33 | 48 | same | same | none |
   | TD3 | 74 | 30 | 44 | same | same | none |
   | MRV-A | 87 | 60 | 27 | same | same | none |
   | MRV-B | 90 | 42 | 48 | same | same | none |

   **C6 moves no seed in any format.** **C3 moves one: TD1 seed 0**, a miss before (`document_number_mismatch`, with a
   wrong accepted read of its first line) and a refused miss after (`checksum_failed`, with the repeated-line rule's
   reason); its hits, wrong accepts and correct reads do not change, so the refusals are 1 under C3 and 0 under C6, and
   **no correct read is lost in either rule** (the veto passes). Neither rule creates a wrong accept.
5. **The expected populations against what was measured.** The 09-28 note
   ([wrong-physical-line1](wrong-physical-line1-2026-09-28.md)) measured, at `c21606b`, before #580, #581, #593 and
   #602: C3 refuses Croatia 2002 and Switzerland 2003 and nothing else; C6 refuses Russia 2013 only (**Derived**: the
   plan's expectation, not a measurement of today). Measured here at `c60936d`:
   - **C3: exactly Croatia 2002 and Switzerland 2003, and no other document** (Observed). The same two, so the
     population did not move across #580, #581, #593 and #602.
   - **C6: exactly Russia 2013, and no other document** (Observed).
   - **What the 09-28 note said of the two C3 documents** (Derived from it, and compared here by eye on the local dump,
     the text not published): Croatia 2002's accepted TD2 zone is one line read twice, a wrong read counted as a hit;
     Switzerland 2003's was a manufactured hit. Today Switzerland 2003 is a `checksum_failed_specimen` miss (renamed by
     #658) and only its reason changes. So **the one correct -> refused of answer 2 is a read the 09-28 note had judged
     wrong** (Observed for the dump's zone, Derived for the judgement): by the brief's definition, a scored hit in the
     default arm, it counts as 1; by the note's reading no correct read is lost. The plan's promotion rows ask for 0
     correct -> refused on the real corpus; which of the two readings applies is for the maintainer, not this note.
   - **Synthetic, not carried over:** the 09-28 note listed three C3 synthetic seeds (TD1 51 and 71, TD3 95) at the
     older commit; the arms here move one (TD1 seed 0), a different one, and the others read no refusal (Observed).

## Fidelity

- **What the arms share (Observed).** One capture, 261 pass rows, one manifest hash; every replay's `replay_of` names
  it. The replays ran in 10 to 14 s each with no OCR model loaded (the run header says so).
- **The synthetic arms are live (Observed).** They share a binary, a machine, a thread count and the default retry
  budget, which no synthetic document comes near (answer 1: identical results in the placebo arms, seed for seed).
  Their timing is not a result.
- **The real corpus is one capture's text (Observed).** The two documents that exhaust the 52 s budget (France ID 2020,
  both sides) stop identically in every arm, because every arm replays the same passes.

## What this does not show

- **Tier 1 only.** Nothing about the later tiers or the shipped pipeline's routing.
- **The public corpus only.** The 261 public specimens and 500 synthetic documents; no private or local track
  (a replay refuses both).
- **One capture's text.** Nothing upstream of `OcrPage::text`: the OCR passes, their order and the retry stop are the
  capture's. A different live read of the same document could differ (the retry loop is wall-clock budgeted), which
  this A/B cannot see.
- **Rule (d) is not measured:** it has no switch on `main`.
- **No timing as a result.** The replay and synthetic durations above are the harness's, kept for the record.
- **The synthetic corpus is clean, seeds 0-99, five formats:** a rule that fires on a damaged profile or another seed
  range is not tested. A rule's cost on documents outside the 261 is not bounded by this run.
- **No comparison with the 09-29 note's counts** (139 hits there, 137 here): the capture, the corpus and the baseline
  have moved since.
