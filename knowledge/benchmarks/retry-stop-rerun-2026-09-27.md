# #473 re-run on ADR-0025's narrowed handoff: `clean` no longer loses a hit, gains one synthetic read in 500, and costs one real name

**Date:** 2026-09-27 · **MAIN:** `55f4c0c` (branch `claude/508-narrowed-handoff`, not merged: `origin/main` `b5c1b55`, plus PR #548's `3582262`, plus the handoff and ADR-0025 at Proposed) · **DATA:** the corpus in worktree `m6-dump` (sha not recorded; manifest sha256 `37274376…`; the `first-valid` arm matches the CI ledger measured at DATA `396b22f` on all 261 outcomes); none for the synthetic arms (generated corpus) · **Evidence:** Observed (one local release build, twelve runs switched by env var) plus Derived (per-document and per-seed comparison, the accept-rule arithmetic, the timing split) plus Hypothesized (one untested lever, marked where used) · **Status:** current

**2026-09-27.** This is the re-run that
[`retry-handoff-ab-2026-09-27.md`](retry-handoff-ab-2026-09-27.md#what-next) asked for. It repeats
the [#473](https://github.com/ruledicaprio/SynthPass/issues/473) same-binary A/B of
[`retry-stop-ab-2026-09-26.md`](retry-stop-ab-2026-09-26.md), this time on a build that carries
the narrowed handoff. The arms are `SYNTHPASS_OCR_STOP=first-valid` (the default) and `clean`,
with the confirm budget at its default of 2. The build is commit `55f4c0c`. Its ADR-0025
("Tier 1 reads a single OCR pass only when two passes agree", Proposed, in that commit and not on
`main`) makes Tier 1 parse one of three texts:

1. on `variant_valid_confirmed`, the confirming pass's lines;
2. on a held reading accepted unconfirmed (`repair_unconfirmed`), the text as it stood when the
   reading was first held;
3. otherwise, the full concatenation of every pass.

Under `first-valid` only rule 3 occurs, so the default path is unchanged by construction. The
build also carries PR [#548](https://github.com/ruledicaprio/SynthPass/pull/548)'s per-document
`retry_damaged_recovery` and `tier1_damaged_recovery`. The #508 A/B note found the re-run could
not be read without them.

## The answer

- **Observed: `clean` no longer loses a hit on any population.** Real specimens 139 = 139 Tier-1
  hits, with zero outcome changes on 261 documents. Synthetic hits are unchanged or up on all
  five formats: 368 → 370 of 500. On 2026-09-26, on a build without the handoff, `clean` lost two
  real hits and four TD3 seeds.
- **Observed: rule 2 is outcome-neutral, as ADR-0025 predicts.** All 46 `repair_unconfirmed`
  stops score exactly as under `first-valid`: 5 of 5 real, 41 of 41 synthetic. That covers the
  outcome, the wrong fields and the names.
- **Observed: the one gain comes from rule 1.** TD1 seed 50 goes from `checksum_failed` to
  correct. It is one of 9 `variant_valid_confirmed` stops across the 500 synthetic seeds. The
  other 8 score as before. On real specimens, `variant_valid_confirmed` fired 0 times.
- **Observed: both of the arm's other moves come from rule 3,** where a later clean reading
  superseded the held damaged one:
  - Azerbaijan `PC_AZE_2022` black-and-white loses its exact names. Real strict names go
    **12 → 11** of 45. This is the same document and the same pass-00 → pass-01 path as on
    2026-09-26.
  - MRV-B seed 95 goes from a valid miss to a wrong accept. The document number becomes right,
    and the surname and given names stay wrong.
- **Derived: `clean` passes the pre-registered accept rule as written, and the rule does not see
  either loss.** See [The accept rule](#the-accept-rule-and-what-it-does-not-count). The note
  lays out both options and leaves the decision to the owner.

## Before / after

Real specimens, `provider-bench --real-specimens --mrz-only`, 261 documents:

| | `first-valid` | `clean` |
| :-- | --: | --: |
| Tier-1 hits, scored | **139 / 151 = 92.1%** | **139 / 151 = 92.1%** |
| Tier-1 hits, whole corpus | 139 / 261 = 53.3% | 139 / 261 = 53.3% |
| `checksum_failed` / `no_mrz_found` (scored) | 10 / 2 | 10 / 2 |
| `document_number_mismatch` / `false_positive_mrz` | 0 / 0 | 0 / 0 |
| Off-denominator: `no_mrz_expected` / `redacted_mrz` / `checksum_failed_specimen` | 51 / 39 / 20 = 110 | 51 / 39 / 20 = 110 |
| Strict names (both exact) / name-scorable | 12 / 45 = 26.7% | **11 / 45 = 24.4%** |
| Names exact among name-scorable hits | 12 / 33 | 11 / 33 |
| `checksum_valid_on_failed_specimen` | 0 | 0 |
| `retry_stop`: `general_valid` / `variant_valid` / `exhausted` | 62 / 72 / 127 | 58 / 71 / 127 |
| `retry_stop`: `repair_unconfirmed` / `variant_valid_confirmed` | — / — | **5 / 0** |
| OCR time (sum of per-document `ocr_ms`) | 1 505.1 s | 1 520.6 s (+1.0%) |
| Wall time (driver log) | 1 516 s | 1 531 s |

Synthetic, `synthpass-bench --profile clean --count 100 --seed 0`, each cell `first-valid` →
`clean`. "Correct" means a hit exact on all twelve scored fields. A "valid miss" is a
checksum-valid reading of the wrong document number. Both follow `tools/synth_ab_diff.py`:

| Format | Hits | Correct | Wrong accepts | Valid misses | Refused | `strict_hits` | Paths changed | `clean` stops: `variant_valid_confirmed` / `repair_unconfirmed` | Sum of `elapsed_ms` |
| :-- | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| TD3 | 75 | 39 | 36 | 3 | 22 | 45 | 17 | 2 / 13 | 235.5 → 246.6 s |
| TD2 | 73 | 45 | 28 | 3 | 24 | 48 | 14 | 1 / 13 | 223.5 → 231.9 s |
| TD1 | 48 → **49** | 13 → **14** | 35 | 8 | 44 → 43 | 13 → 14 | 5 | 1 / 4 | 374.2 → 379.4 s |
| MRV-A | 85 | 34 | 51 | 4 | 11 | 50 | 6 | 3 / 3 | 180.4 → 183.9 s |
| MRV-B | 87 → **88** | 46 | 41 → **42** | 3 → 2 | 10 | 60 | 11 | 2 / 8 | 173.8 → 187.6 s |
| **All 500** | 368 → 370 | 177 → 178 | 191 → 192 | 21 → 20 | 111 → 110 | 216 → 217 | 53 | 9 / 41 | 1 187.4 → 1 229.4 s |

- **Derived: the M4 set** (seeds 0–49 of TD3) reads 39 hits / 18 correct / 21 wrong accepts /
  1 valid miss in both arms.
- **Observed: prefix wrong accepts** are 1 / 1 / 0 / 0 / 0 in both arms. The TD3 one is seed 61.
- **Derived: the `first-valid` totals match the synthetic headline.** 368 hits, 177 correct and
  191 wrong accepts are exactly the numbers in
  [`synthetic-headline-2026-09-27.md`](synthetic-headline-2026-09-27.md), format for format.

## What moved

**Observed: the arm changed the retry path on exactly the documents whose `first-valid` accept
was a damaged-capture reading.** That is 8 of 8 real documents and 53 of 53 synthetic seeds
(`retry_damaged_recovery` true under `first-valid`). Nothing else moved.

The table sorts the 61 changed paths by the ADR-0025 rule that decided what Tier 1 read under
`clean`:

| `clean` stop (rule) | Real | Synthetic | Outcome or names changed |
| :-- | --: | --: | :-- |
| `repair_unconfirmed` (rule 2) | 5 | 41 | **none** |
| `variant_valid_confirmed` (rule 1) | 0 | 9 | **1 gain:** TD1 seed 50 |
| `variant_valid` after a held repair was superseded (rule 3) | 3 | 3 | **2:** Azerbaijan 2022 black-and-white (names), MRV-B seed 95 |

**Real specimens.** Eight documents changed path, the same eight as on 2026-09-26. None changed
outcome.

| Document | `retry_stop` `first-valid` → `clean` | Outcome | `ocr_ms` |
| :-- | :-- | :-- | --: |
| `passports/Azerbaijan_Passport_Specimen_PC_AZE_2022_mrz_black_and_white.png` | `variant_valid` pass-00 → `variant_valid` pass-01 | hit → hit, **names exact → wrong** | 2 614 → 3 100 |
| `passports/Canada_Passport_Specimen_PP_CAN_2023_mrz_highlight.webp` | `general_valid` → `variant_valid` pass-01 | hit → hit | 1 416 → 2 565 |
| `passports/Netherlands_Passport_Specimen_P0_NLD_2006_mrz.jpg` | `variant_valid` pass-00 → `variant_valid` pass-02 | hit → hit | 1 666 → 2 996 |
| `passports/Azerbaijan_Passport_Specimen_PC_AZE_2013_mrz.webp` | `general_valid` → `repair_unconfirmed` (general) | hit → hit | 1 131 → 2 574 |
| `passports/Hungary_Passport_Specimen_P0_HUN_2012_mrz.png` | `general_valid` → `repair_unconfirmed` (general) | hit → hit | 1 616 → 2 796 |
| `passports/Kuwait_Passport_Specimen_P0_KWT_2023_mrz.png` | `general_valid` → `repair_unconfirmed` (general) | hit → hit | 1 052 → 2 289 |
| `id_cards/Romania_ID_Specimen_2021_back_mrz.png` | `variant_valid` pass-01 → `repair_unconfirmed` (pass-01) | hit → hit | 1 868 → 2 893 |
| `misc/Croatia_BorderPass_Specimen_CB_HRV_2025_back_mrz.png` | `variant_valid` pass-06 → `repair_unconfirmed` (pass-06) | hit → hit | 5 441 → 7 293 |

Azerbaijan 2013 and Kuwait 2023 are the two hits `clean` lost on 2026-09-26. Here they are
`repair_unconfirmed` hits: Tier 1 reads the text as first held, which is the text `first-valid`
reads (rule 2). On Azerbaijan 2022 black-and-white, the loop held a damaged pass-00 reading. It
then stopped on pass-01's clean reading, with both damaged flags false, so Tier 1 read the full
concatenation (rule 3). No check digit covers a name, so a clean reading is no evidence the names
are right. The 2026-09-26 note said the same.

**Synthetic: two seeds moved, one per mechanism.**

| Seed | `first-valid` | `clean` | What changed |
| :-- | :-- | :-- | :-- |
| TD1 50 | `checksum_failed` (composite): `variant_valid` pass-00, loop damaged, Tier 1 not | **correct**: `variant_valid_confirmed` pass-02, both damaged flags true | pass-02 confirmed the held reading field for field, and Tier 1 read the confirming pass alone (rule 1). `first-valid`'s parse of the concatenation had refused it |
| MRV-B 95 | valid miss (`document_number_mismatch`): `general_valid`, damaged; document number, surname and given names wrong | **wrong accept**: `variant_valid` pass-00, both damaged flags false; surname and given names wrong | pass-00's clean reading superseded the held repair (rule 3). The document number becomes right and the names stay wrong. The seed goes from 3 wrong fields to 2, and from a miss to a hit |

The other 51 changed seeds keep their state and their wrong fields. The #473 repro seeds do not
move: TD3 seed 99 is still a valid miss and seed 85 is still a wrong accept, both as
`repair_unconfirmed`.

## Control

- **Observed: real `first-valid` equals the committed ledger** (`real-specimen-outcomes.jsonl`,
  CI `eeacdf5`, DATA `396b22f`). It matches on the outcome of all 261 documents: 139 hits,
  0 differences. It also matches on `names_exact` for all 261. Five documents took a different
  retry path, and none changed outcome:
  - France ID 2020 back and front: `budget` in CI, `exhausted` locally.
  - Argentina `P0_ARG_2026`: `variant_valid` pass-09 in CI, `exhausted` locally.
  - Kuwait `P0_KWT_2023`: `variant_valid` pass-03 in CI, `general_valid` locally.
  - Cyprus `P0_CYP_2010`: `variant_valid` pass-02 in CI, pass-04 locally.

  The first four are the same local-versus-CI differences the #508 A/B note recorded. The fifth
  is new. **Derived:** it is the effect of #536, which the build carries and the ledger's CI sha
  predates. Cyprus 2010's pass-02 reading has a document number that starts with `<`, and #536
  refuses it, so the loop runs on to pass-04 and the document stays a hit. That confirms the
  outcome the #508 A/B note marked Hypothesized.
- **Observed: synthetic TD3 `first-valid` equals #508 A/B arm A** (`9a639c6`) on all 100
  seeds. Hit, wrong fields, check states, `retry_stop`, and every field's CER and returned value
  all match.
- **Observed: the nine smoke seeds under `first-valid`** (20, 22, 40, 43, 50, 66, 72, 88 and 98,
  single-seed runs on #548's head `3582262`) equal their rows in this run. That covers state,
  `retry_stop`, `retry_variant_id`, both damaged flags and the wrong fields.
- **Observed: the arms differ only in `SYNTHPASS_OCR_STOP`.** Every report records `ocr_arms`
  with `stop` = the arm and `confirm_passes` = 2, and every other arm at its default. Both real
  runs record `git_commit` `55f4c0c` with `working_tree_dirty: false`, and the same manifest.

## The smoke seeds: the question the #508 A/B note left open

The smoke run is Observed on single seeds at `3582262`. That build has #548's flags and no
handoff.

- **Seeds 40, 50, 88 and 98 are damaged-capture accepts; seed 20 is not.** These are the four
  `variant_valid` stops that Tier 1 refuses under `first-valid`: the loop's reading is damaged
  and Tier 1's is not. Seed 20 accepts a clean reading on pass-06. The #508 A/B note listed
  "which of the five were damaged-capture accepts" as unknown.
- **Seeds 22, 43, 66 and 72 carried the #473 handoff defect, and it is gone.** At `3582262`,
  under `clean`, all four score `checksum_failed` as `repair_unconfirmed`, with Tier 1's damaged
  flag false. At `55f4c0c` all four score exactly as under `first-valid`: 22 and 66 correct, 43
  and 72 wrong accepts. **Derived:** the only difference between the builds is the handoff
  commit, and the `first-valid` rows agree across the two builds.
- **Nothing makes 40, 50, 88 or 98 correct under either arm.** They are still `checksum_failed`
  as `repair_unconfirmed`. The architect's plan expected this: the cause is on the recognition
  side ([#537](https://github.com/ruledicaprio/SynthPass/issues/537)).

## Latency

The machine was clean this time, unlike on 2026-09-26.

- **Derived: the documents the arm did not touch did not drift.** On real specimens, the 253
  documents with an unchanged path went 1 488.3 → 1 494.1 s (+0.4%). The 127 `exhausted` ones, a
  negative control, went +0.5%. On synthetic, the 447 unchanged seeds went 1 106.9 → 1 106.1 s
  (−0.1%).
- **Derived: the arm's own cost on real specimens.** The 8 exercised documents went 16.8 →
  26.5 s. Scaled by the untouched documents' ratio, the cost is about **+9.6 s**, or 0.6% of the
  control's OCR time. This figure is modelled.
- **Derived: the arm's own cost on synthetic.** The 53 exercised seeds went 80.4 → 123.3 s,
  about +0.8 s each. That is one or two extra passes.
- The single job slot (`cpu/timetable.log`) ran nothing else from 21:00:34 to 22:34:15 UTC. No
  process sampler ran (see [What this does not claim](#what-this-does-not-claim)).

## The accept rule, and what it does not count

The rule was fixed before the run, in the architect's plan. Promote `clean` only if every clause
holds, and remove it otherwise. **Derived:**

| Clause | Measured | Holds |
| :-- | :-- | :-- |
| Real: zero hit → miss | 0 | yes |
| Real: no rise in `document_number_mismatch` | 0 → 0 | yes |
| Real: no rise in `checksum_failed` | 10 → 10 | yes |
| Synthetic, every format: correct not down | 39, 45, 13 → 14, 34, 46 | yes |
| Synthetic, every format: wrong accepts + valid misses not up | 39, 31, 43, 55, 44 → 44 | yes |
| At least one gain | TD1 seed 50, refused → correct | yes (one) |

Read literally, the rule passes. Four things it does not count:

1. **The real strict-name loss.** Azerbaijan 2022 black-and-white goes from exact names to wrong.
   Names are report-only under
   [ADR-0013](../decisions/ADR-0013-names-are-scored-against-mrz-form-truth.md)'s 2026-09-26
   amendment, and the rule has no name clause. This loss reproduced on a second build: it is the
   same document, path and verdict as on 2026-09-26.
2. **MRV-B seed 95.** The rule sums wrong accepts and valid misses, and a valid miss that becomes
   a wrong accept leaves the sum at 44. The bench counts one more hit (87 → 88) and one more
   wrong accept (41 → 42), and neither is a correctness gain.
   [ADR-0021](../decisions/ADR-0021-fixed-grid-mrz-strips.md)'s K3, which is Proposed and written
   for the fixed-grid aligner, disqualifies "a wrong acceptance that today's scanner does not
   make". Applied here by analogy, it reads two ways:
   - **As new:** under `first-valid` the seed had no `wrong_accept` bit, and under `clean` it
     does.
   - **As inherited:** under `first-valid` the parser already returned a checksum-valid reading
     of this seed, with three fields wrong, including the document number. K3 says inherited
     wrong readings "are reported, not charged". Under `clean` it returns a reading with two
     fields wrong.
3. **The gain and the loss sit on different populations.** The only gain is synthetic. The only
   loss is a real document. ADR-0021's K2 (again by analogy) compares "on the same population
   and unit" and never nets synthetic refusal rates against real recoveries.
4. **ADR-0025's own condition** says rules (1) and (2) are removed "if `clean` shows no gain".
   It showed one gain, so under the ADR's own wording removal is not automatic either.

**The two options, with their evidence. This is the owner's decision.**

- **Remove `clean`, together with ADR-0025's rules (1) and (2), and keep #548's
  instrumentation.** The evidence for this option:
  - On the corpus the product serves, `clean` gains nothing: 0 of 261 real documents improve, and
    `variant_valid_confirmed` never fires.
  - It costs one real name read, and that loss reproduced across two builds.
  - The only gain is 1 of 500 synthetic seeds.
  - It adds a stop mode, a snapshot and two Tier-1 branches, all off by default. `CLAUDE.md`
    counts simplicity as part of maintainability, and this code serves no measured real win.
  - [ADR-0006](../decisions/ADR-0006-m6-accuracy-first.md) puts accuracy on real documents first.
- **Promote `clean` to the default.** The evidence for this option:
  - It passes the pre-registered rule as written.
  - It adds one correct synthetic read.
  - It leaves every gated real count unchanged locally.
  - It costs about 0.6% of real OCR time.
  - The costs:
    - The real strict count drops to 11, which the baseline records (report-only).
    - **Inferred:** the gate passes on counts, but CI must measure it, because CI's retry path
      differs locally on Kuwait 2023, among others.
    - A CI `write-baseline` would still be needed to move `strict_names`.

**Hypothesized, not measured: the lever between the two.** Both of the arm's other moves happened
where a single later clean reading superseded a held repair (rule 3). The one gain happened where
a second pass confirmed the held reading (rule 1). A `clean` that stops on a superseding reading
only when a second pass agrees would keep the gain and, on this run's 6 superseded stops, avoid
both moves. ADR-0025 already lists "confirming every variant stop" as unmeasured and deferred. It
would need its own A/B.

## Invocations

Driver `473-rerun.sh 55f4c0c`, run through the session's single job slot, in worktree `m6-dump`
detached at `55f4c0c`. The build took 153 s:

```
CARGO_INCREMENTAL=0 cargo build --release -p synthpass-bench --bin synthpass-bench --bin provider-bench
# sha256 synthpass-bench e0c2a43a86d6a51adce2e181aec251e1bf55ca7630c6ad028cd532c1e3fc511d
#        provider-bench  8ff62deca5bc9f463b42a2e2df77ec832172b70beb720e1fa7c10a79538ce56e
```

Every run used the copied binaries with `SYNTHPASS_OCR_MAX_PASSES` and `SYNTHPASS_OCR_MODEL_DIR`
unset, in this order: each synthetic format under both arms, then real `first-valid`, then real
`clean`. The runs took 21:03:07–22:34:15 UTC.

```
env -u SYNTHPASS_OCR_MAX_PASSES -u SYNTHPASS_OCR_MODEL_DIR SYNTHPASS_OCR_STOP=<arm> synthpass-bench \
  --document-type <td3|td2|td1|mrva|mrvb> --profile clean --count 100 --seed 0 --out synth-<fmt>-<arm>/report.json
env -u SYNTHPASS_OCR_MAX_PASSES -u SYNTHPASS_OCR_MODEL_DIR SYNTHPASS_OCR_STOP=<arm> provider-bench \
  --real-specimens --mrz-only --progress --dump-ocr --dump-ocr-hits --out real-<arm>/report.json
python3 tools/synth_ab_diff.py synth-<fmt>-first-valid/report.json synth-<fmt>-clean/report.json
```

The real invocation adds `--progress --dump-ocr --dump-ocr-hits` to the gate's. The dump is
written after each document is classified, and the `first-valid` arm's 261 of 261 match with the
ledger shows the flags moved nothing. The comparison against the ledger, the per-rule
classification and the timing split are Python over the reports' `results`, `documents_detail`
and `provider-bench-ocr-outcomes.jsonl`. The smoke run is `473-smoke.sh 3582262`: `synthpass-bench
--document-type td3 --profile clean --count 1 --seed <s>` under both arms.

## What this does not claim

- **Not a CI measurement.** Local `rten` differs from CI by float rounding. The control matches
  the CI ledger on all 261 outcomes, but the `clean` arm's real counts are local. They are not a
  baseline, and not a prediction of the gate.
- **Not a measurement of any merged commit.** `55f4c0c` is on a branch pushed without a PR, and
  #548 is not on `main`.
- **No `samples-data` sha.** The corpus is the one synced into `m6-dump`. It is pinned only by
  the manifest hash and the 261 of 261 ledger match.
- **Not a verdict on the "confirm every variant stop" lever.** Only budget 2 was run, with the
  three rules as built.
- **TD1 seed 50 is one seed.** Its move has a traced mechanism (rule 1, a confirmed pass), not
  a noise flip: both arms ran one binary, and the path changed where the arm says it should.
  But one seed does not establish a rate.
- **No competing-process sampler ran.** Clean timing is inferred from the job-slot log and from
  the untouched documents' ±0.5%.
- **No OCR text is quoted.** No private specimen was in either real run, because the default
  walk excludes the private track.

## Rejected on the way

- **"`clean` still loses hits".** That was the 2026-09-26 build's handoff defect. On this build,
  every `repair_unconfirmed` stop scores as under `first-valid` (46 of 46), and seeds 22, 43, 66
  and 72 are back.
- **"MRV-B seed 95 is a wrong read `first-valid` did not make".** `first-valid` returned a
  checksum-valid reading of it too, with one more wrong field. What is new is the bench's
  classification (miss → hit), not the existence of a wrong reading.
- **Netting the TD1 gain against the Azerbaijan name loss.** They are different populations and
  different axes (see [the accept rule](#the-accept-rule-and-what-it-does-not-count)).
- **A confirm-3 arm.** The plan ran it only "if needed". On 2026-09-26, budgets 3 and 5 moved
  none of three seeds, on the build with the handoff defect. The real corpus produced no
  confirmed stop at 2, so it was not run.
