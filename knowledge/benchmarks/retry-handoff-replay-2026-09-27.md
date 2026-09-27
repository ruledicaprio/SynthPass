# #508 replay: `single()` refuses, the damaged-pass budget is never close

**Date:** 2026-09-27 · **MAIN:** `9a639c6` · **DATA:** samples present in the worktree at 2026-09-27 07:34 (sha not recorded: no `samples-data` checkout metadata in this container); none for the synthetic seeds (generated corpus) · **Evidence:** Observed (one local release build, single-seed and stride-26 traces per arm, plus a replay of every dumped `text` through an instrumented copy of `mrz`) · **Status:** current

**2026-09-27.** This is the replay the owner's decision on
[#508](https://github.com/ruledicaprio/SynthPass/issues/508) asked for first. It feeds each
dumped `text` through an instrumented `find_and_parse` and prints the hit count and the remaining
budget. It decides between the two branches
[`retry-stop-ab-2026-09-26.md`](retry-stop-ab-2026-09-26.md) left open ("Hypothesized: the
exact parser branch"):

- **(a)** `single()`, #440's unanimity gate in `damaged_pass`, refuses because the larger line set
  yields more than one disagreeing `accept_damaged` reading.
- **(b)** the shared `MAX_DAMAGED_ATTEMPTS` budget (200 000) is consumed before the general
  pair is reached.

## The answer

- **Observed: (a), on every document that moved, and never (b).** On all six (four synthetic
  seeds, two real specimens), `damaged_pass` over the combined text accepts **2 to 48** distinct
  `accept_damaged` readings. `single()` refuses, and `find_and_parse` returns the ordinary
  checksum-invalid fallback.
- **Observed: the budget is never close.** The TD1 section spends **0** attempts on every text.
  The general pass's pair is reached with the full **200 000** left. The most any whole pass
  spent was **3 868** (Azerbaijan 2013, 196 132 left).
- **Observed: the fix's premise holds.** Handed only the accepted pass's lines, every one of the
  six parses to exactly the reading the loop accepted (`single()`: 1 hit, `ACCEPT`,
  `damaged_recovery=true`). For the five `repair_unconfirmed` cases, those lines are
  byte-identical to the `first-valid` text. For seed 22 at `SYNTHPASS_OCR_CONFIRM_PASSES=3`
  (`variant_valid_confirmed`), the confirming pass's lines alone and the held general text alone
  give the same answer.
- **Observed: "restores the hit" is not "restores a correct read".** Of the six, three of the
  readings the fix would hand back are wrong on fields no check digit covers. Seeds 43 and 72 are
  wrong on names, as under `first-valid`. Azerbaijan 2013 reads nationality **`A7E`** and surname
  **`HUSEYNLIS`**, where the printed zone says `AZE` and `HUSEYNLI`.

## Reproduction

The same binary, with the arm switched by env var. Local `rten` can differ from the 2026-09-26
A/B machine by float rounding. Every move reproduced anyway:

| Document | `first-valid` | `clean` | `ocr_ms` FV → clean |
| :-- | :-- | :-- | --: |
| seed 22 | hit, `general_valid` | **`checksum_failed`** (`document_number`, composite), `repair_unconfirmed` | 1 396 → 2 687 |
| seed 22, confirm 3 | hit, `general_valid` | **`checksum_failed`**, `variant_valid_confirmed` | 1 393 → 3 124 |
| seed 43 | hit (names wrong), `general_valid` | **`checksum_failed`** (`document_number`), `repair_unconfirmed` | 1 316 → 2 652 |
| seed 66 | hit, `general_valid` | **`checksum_failed`** (`personal_number`), `repair_unconfirmed` | 1 460 → 2 577 |
| seed 72 | hit (names wrong), `general_valid` | **`checksum_failed`** (`personal_number`), `repair_unconfirmed` | 1 320 → 2 560 |
| seed 85 | hit (names wrong), `variant_valid` | hit (names wrong), `repair_unconfirmed` | 3 098 → 3 934 |
| seed 99 | `document_number_mismatch` | same, `repair_unconfirmed` | 1 380 → 2 680 |
| Azerbaijan 2013 `.webp` | hit, `general_valid` | **`checksum_failed`**, `repair_unconfirmed` | 1 274 → 2 927 |
| Kuwait 2023 | hit, `general_valid` | **`checksum_failed`**, `repair_unconfirmed` | 1 194 → 2 756 |

The real stride is `--format passport --limit 26`, the smallest stride over the 192 passports
that contains both index 14 (Azerbaijan 2013 `.webp`) and index 110 (Kuwait 2023). Nothing else
moved in it:

| | `first-valid` | `clean` |
| :-- | --: | --: |
| Tier-1 hits, scored | 17 / 18 | 15 / 18 |
| Tier-1 hits, whole stride | 17 / 26 | 15 / 26 |
| `checksum_failed` / `checksum_failed_specimen` / `redacted_mrz` | 1 / 1 / 7 | 3 / 1 / 7 |
| `false_positive_mrz` | 0 | 0 |

This is a 26-document stride, not the corpus. It is not comparable to the 139 / 151 baseline.
The longest single document took 14.8 s of OCR, against the 52 s budget. No run hit
`retry_budget_hit`, and no competing cargo or bench process ran during any measurement.

## The replay

In every moved case, the `first-valid` text is a **line-exact prefix** of the `clean` text. The
confirm passes appended 6 lines (4 for Kuwait): each pass's `mrz_shaped_lines`, in pass order.
So the dump does separate the passes. The general pass's lines are the `first-valid` text, and
each confirm pass is the next block of 3 lines (2 for Kuwait), cross-checked against the verbose
trace. For confirm 3, variant 2's lines are the confirm-3 text's suffix after the confirm-2
`clean` text, also an exact prefix.

(i) is the full `clean` text, what Tier 1 got. (ii) is the accepted pass's lines only.
(iii) is the `first-valid` text.

| Text | Lines | Ordinary scan | `accept_damaged` zones (distinct answers) | `single()` | Budget left | Result |
| :-- | --: | :-- | :-- | :-- | --: | :-- |
| s22 (i) | 17 | no valid | 2 (2) | REFUSE | 198 250 | `checksum_failed`, doc `60QCGRMOU` |
| s22 (ii) = (iii) | 11 | no valid | 1 (1) | ACCEPT | 198 317 | valid, doc `6OQCGRMOU` |
| s22 c3 (i) | 20 | no valid | 2 (2) | REFUSE | 198 183 | `checksum_failed` |
| s22 c3 (ii) variant 2 | 3 | no valid | 1 (1) | ACCEPT | 199 933 | valid, `MORAVEC/MAREN` |
| s43 (i) | 18 | no valid | 2 (2) | REFUSE | 199 740 | `checksum_failed`, doc `NZ138880S` |
| s43 (ii) = (iii) | 12 | no valid | 1 (1) | ACCEPT | 199 870 | valid, doc `NZ13888OS` |
| s66 (i) | 22 | no valid | 3 (3) | REFUSE | 197 205 | `checksum_failed`, opt `…LU7UOAF` |
| s66 (ii) = (iii) | 16 | no valid | 1 (1) | ACCEPT | 199 938 | valid, opt `…LU7U0AF` |
| s72 (i) | 18 | no valid | 6 (6) | REFUSE | 199 702 | `checksum_failed`, opt `…X6EDOZ` |
| s72 (ii) = (iii) | 12 | no valid | 1 (1) | ACCEPT | 199 951 | valid, opt `…X6ED0Z` |
| AZE 2013 (i) | 45 | no valid | 48 (48) | REFUSE | 196 132 | `checksum_failed` |
| AZE 2013 (ii) = (iii) | 39 | no valid | 1 (1) | ACCEPT | 199 862 | valid, `HUSEYNLIS/ORKHAN`, nat `A7E` |
| KWT 2023 (i) | 51 | no valid | 10 (8) | REFUSE | 197 426 | `checksum_failed`, doc `PO6829955` |
| KWT 2023 (ii) = (iii) | 47 | no valid | 1 (1) | ACCEPT | 199 850 | valid, doc `P06829955` |

The ordinary scan validates on none of the 14 texts, so the damaged pass always runs. Each (i)
still contains the general pair that (ii) recovers. It is reached first, at full budget, and its
reading is accepted as hit #1 in every (i) trace. It is then outvoted.

**What disagrees.** Seed 22 (`clean`, full text):

```
[instr]   accept_damaged #1 at pair i=9 w44 P (budget left 198449): doc=6OQCGRMOU … name=MORAVEC/MAREN
[instr]   accept_damaged #2 at pair i=12 w44 P (budget left 198315): doc=6OQCGRMOU … name=MORAVECMAREN/
[instr] damaged_pass end: budget left 198250 of 200000, distinct zones accepted 2, raw accepts 3
[instr] single(): 2 hit(s), 2 distinct answer(s) -> REFUSE
[instr] damaged_pass returned None -> ordinary checksum-failed fallback is returned
```

Pair 9 is the general pass. Pair 12 is variant 0, whose line 1 lost the `<<` separator. Both
readings carry the true document number. They differ **only in the name split**, and `single()`
correctly will not choose between them. The same pattern holds elsewhere:

- **Seed 43:** `PELLETIERSOREN/` (general) against `PELLETIER/SOREN` (variant 0).
- **Seed 66:** `IURII` (general) against `IURIL` and `IURI` (variant 0).
- **Seed 72:** three name readings, plus three optional-data readings from variant 1's line 2
  (`JS4TD5MEX6ED07`, `JS4TDSNEX6ED07`, `JS4TDSMEX6E007`).
- **Azerbaijan 2013:** variant 1's line 2 arrived 43 cells wide. `restored()` sweeps the missing
  cell to 47 checksum-valid readings, all at pair 43.
- **Kuwait 2023:** the confirm passes add issuer `WTA` and a `LTAMIMI/HASSANA…` name split.

**Seed 22 at confirm 3.** Variant 2 alone parses to the held `MORAVEC/MAREN`, which is why the
loop logs `variant_valid_confirmed`. The combined text still carries variant 0's `MORAVECMAREN/`
at pair 12. That is what `single()` refuses on, not variant 2.

## Invocations

Worktree `m6-dump`, detached at `9a639c6`. `CARGO_INCREMENTAL=0`. Every bench command ran under
`env -u SYNTHPASS_OCR_MAX_PASSES`, with no other `SYNTHPASS_OCR_*` variable set except the arm.
The `SYNTHPASS_MRZ_CLASS_SWEEP` arm was off, so `class_sweep_pass` was a no-op throughout.

```
cargo build --release -p synthpass-bench --bin synthpass-bench      # 5 m 46 s
cargo build --release -p synthpass-bench --bin provider-bench       # 1 m 58 s, incremental on the above
SYNTHPASS_OCR_VERBOSE=1 SYNTHPASS_OCR_STOP=<first-valid|clean> [SYNTHPASS_OCR_CONFIRM_PASSES=3] \
  target/release/synthpass-bench --count 1 --seed <22|43|66|72|85|99> --profile clean \
  --document-type td3 --dump-ocr --out <scratch>/runs/s<N>-<arm>.json
SYNTHPASS_OCR_VERBOSE=1 SYNTHPASS_OCR_STOP=<arm> target/release/provider-bench --real-specimens \
  --mrz-only --format passport --limit 26 --dump-ocr --dump-ocr-hits --progress \
  --out <scratch>/real-<arm>/508-real-passport26-<arm>.json          # 150 s per arm
```

The synthetic `text` was rebuilt from `--dump-ocr`'s `[i] "…"` lines, Debug-unescaped and
joined with `\n`. The real `text` is the dump's `raw_ocr_text`. The replay copies `crates/mrz`
to scratch and adds `eprintln!` only:

- in `consider` when the ordinary scan validates, and before `damaged_pass`;
- in `damaged_pass`, per accepted candidate with its pair index and budget, per pair's
  budget-before and attempts used, and at the TD1-section end and the pass end;
- in `single()`, the distinct-answer count and the decision.

A 25-line binary calls `mrz::find_and_parse(text)`, the same default-options call
`synthpass-bench` makes. **Control:** the same binary built against the worktree's unmodified
`crates/mrz` returns an identical `RESULT` line on all 37 replayed texts. Every (i) result also
matches the bench's own outcome for that document.

## What this does not claim

- **Not a measurement of the fix.** (ii) is a replay of text the loop already produced, not a
  run of a patched `synthpass-ocr`. Whether other documents move under the fix, on the full real
  corpus or TD3 ×100, is unmeasured.
- **Not a CI or baseline figure.** These are local `rten` numbers on a 26-document stride. The
  `DATA` sha is unknown.
- **Only the moved documents were replayed.** Four synthetic seeds, one confirm-3 run and two
  real specimens. Not the other six real documents whose retry path changed on 2026-09-26.
- **Truth for the real specimens is the printed zone read by eye.** No `ocr_fixtures` entry
  exists for either.

## Rejected on the way

- **(b), the shared budget.** At least 196 132 of 200 000 attempts remain on every text. The
  TD1 section spends 0. The general pair is always reached at full budget and always recovered.
- **"The confirm passes' lines are checksum-invalid".** The loop's verbose label says so. But
  replaying each pass alone shows several of them produced **checksum-valid damaged readings**
  that `single()` refused within the pass:
  - seed 66 v0: 2;
  - seed 72 v0: 2, and v1: 3;
  - Kuwait v0: 5, and v1: 4 zones, 2 answers;
  - Azerbaijan v1: 47.

  Seed 66 v1's damaged pass accepts nothing. Seed 22 v1, seed 43 v1 and Azerbaijan v0 return
  `NotFound`. Seed 22 v0 and seed 43 v0 validate alone, which the loop logs as "valid but
  damaged-capture, holding".
