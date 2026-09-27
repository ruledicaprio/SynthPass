# The synthetic headline is 368 / 500 on `main`: #468/#471 and #483 fix 15 wrong reads, and #521 costs TD1 four hits

**Date:** 2026-09-27 · **MAIN:** `739279c` (after); `d052ae2` (before: `f80877b` + #457's report-only counter, the reader the 370 was measured on); `1ef7426` (before #521); bisect builds at `97b3e7a`, `8ce66f2`, `6dbc195`, `1cb9236` · **DATA:** n/a (generated corpus, no `samples-data` input) · **Evidence:** Observed (fifteen local release `synthpass-bench --profile clean --count 100 --seed 0` runs on one Linux container, plus 189 single-seed bisect runs) plus Derived (per-seed comparison of the JSON reports) · **Status:** current

**2026-09-27, runs 09:45–11:51 UTC, one Linux container (4 vCPU, Intel Xeon @ 2.10 GHz),
sequential.** This re-measures the synthetic row in
[`README.md`](README.md#current-headline-numbers), which reads 370 / 500 as observed on 2026-09-25
at `1595bf9`/`919b5ff` and in CI at `f80877b`
([`synthetic-headline-2026-09-25.md`](synthetic-headline-2026-09-25.md)). Issue
[#510](https://github.com/ruledicaprio/SynthPass/issues/510) reported TD3 at 75 / 100 on `405b4f0`
and said the other four formats had not been re-measured. This note re-measures all five and
attributes every seed that moved. The document set is generated, so every value below is
synthetic and safe to publish.

## The answer

| Format | Hits 2026-09-25 (`d052ae2`, re-read here) | Hits now (`739279c`) | Strict (both names exact) | Wrong accepts | Exact on all 12 scored fields |
| :-- | --: | --: | --: | --: | --: |
| TD3 | 74 | **75** | 40 → 45 | 40 → 36 | 34 → 39 |
| TD2 | 72 | **73** | 36 → 48 | 38 → 28 | 34 → 45 |
| TD1 | 52 | **48** | 15 → 13 | 37 → 35 | 15 → 13 |
| MRV-A | 85 | **85** | 50 → 50 | 51 → 51 | 34 → 34 |
| MRV-B | 87 | **87** | 59 → 60 | 41 → 41 | 46 → 46 |
| **All five** | **370 / 500** | **368 / 500 = 73.6%** | **200 → 216** | **207 → 191** | **163 → 177** |

- **Observed:** 368 / 500 on `main` at `739279c`. TD3 75, TD2 73, TD1 48, MRV-A 85, MRV-B 87.
  Hits use the harness's definition: a checksum-valid record whose document number matches truth.
  Every generated document carries a conforming zone, so the scored count is the corpus count and
  the two denominators are both 500.
- **Observed:** of the 368 hits, **216** read both names exactly and **191 are wrong in at least
  one of the twelve scored fields**. **177 / 500** are hits that are right on every scored field.
  The headline falls by 2, but correct reads rise by 14 (163 → 177) and wrong accepts fall by 16.
- **Observed:** the before arm, built here at `d052ae2`, re-reads the 2026-09-25 table exactly on
  this platform, on all four columns of all five formats.

## Every seed that moved, and the commit that moved it

Arms: before `d052ae2`, after `739279c`. Seeds were located by comparing the two full reports per
seed (hit bit, `wrong_accept`, `names_exact`, and every field's CER and `got` string). Each moved
seed was then run alone (`--seed S --count 1`) at every bisect build. "Correct" means a hit exact
on all twelve scored fields. "Wrong" means a hit that is a wrong accept. The fields are named where
the change is in the fields only.

| Format | Net hits | Seeds | Commit | Observed |
| :-- | --: | :-- | :-- | :-- |
| TD3 | 0 | 18, 26, 60, 66, 86 | #468 + #471 | wrong (`document_type`, `issuing_country`, `surname`) → **correct** |
| TD3 | 0 | 37, 72 | #468 | wrong → wrong: the line-1 prefix is repaired, both names stay wrong |
| TD3 | **+1** | 85 | #471 | miss → **wrong** (`surname`, `given_names`) |
| TD2 | 0 | 16, 18, 22, 26, 43, 54, 60, 63, 83, 85 | #483 | wrong (`document_type`, `issuing_country`, `surname`) → **correct** |
| TD2 | 0 | 11, 37 | #483 | wrong → wrong: the prefix is repaired, `optional_data_1` (11) or the names (37) stay wrong |
| TD2 | **+1** | 8 | #521 | miss → **correct** |
| TD1 | −1 | 26 | #483 | wrong (both names) → miss: a wrong read refused |
| TD1 | +1 | 50 | #483 | miss → correct |
| TD1 | **−5** | 21, 30, 50 / 42, 56 | #521 | correct → miss (21, 30, 50) and wrong → miss (42, 56) |
| TD1 | **+1** | 15 | #521 | miss → **wrong** (both names) |
| TD1 | 0 | 5 / 39 | #521 | correct → wrong (5), wrong → correct (39) |
| MRV-B | 0 | 43, 82 | #483 | wrong → wrong: the prefix is repaired. On 82 both names become exact and only `optional_data_1` stays wrong (strict +1) |
| MRV-A | 0 | none | — | identical on every seed and every `got` string |

**The TD3 +1 is #468 and #471 jointly.** Observed on the nine moved TD3 seeds at `97b3e7a` (#468
alone): seed 60 is already correct. Seeds 18, 26, 66 and 86 become **misses**, because the
repaired candidate and the unrepaired one disagree and the read is refused. Seed 99 becomes a
correct hit. At `8ce66f2` (#471), 18, 26, 66 and 86 become correct, 85 becomes a wrong accept, and
99 returns to a miss. Nothing on TD3 moves after `8ce66f2`. Seeds 18, 26, 60, 66 and 86 are five of
the seven line-1 filler-deletion reads that the 2026-09-25 note named. The other two, 37 and 72,
also lost the name separator. Seed 85 is one of
[#473](https://github.com/ruledicaprio/SynthPass/issues/473)'s two repros, which loses the
name-separator filler. It is now accepted with the merged surname: a hit that is wrong.

**#483 does all of the TD2, MRV-B and pre-#521 TD1 work.** Observed: every one of those seeds reads
at `6dbc195` (#481) as it did at `d052ae2`, and reaches its final state at `1cb9236` (#483).
**#481 moves none of the 27 tracked seeds.** #475, #473 (opt-in; default arm `first-valid`), #504
and the bench-report commits between `1cb9236` and `1ef7426` move none either. `1ef7426` is
identical per seed to `1cb9236` on the moved seeds, and its full runs differ from the before arm
only on those seeds.

**#521 is the whole TD1 −4 and the TD2 +1.** `1ef7426` → `739279c` changes `synthpass-gen` only on
the reader-and-generator path. The other two commits touch `synthpass-cli`, `-license` and
`-serve`. Observed, full runs: TD1 52 → 48, TD2 72 → 73, MRV-B and TD3 identical on every seed.
On TD1, 28 of 100 seeds change their output, and 20 of those change only strings, not the outcome.
#521 changes VIZ pixels only: the truth labels are parsed from the MRZ lines, which it leaves
alone. So this is the reader reacting to a different image. It is not a change in what is scored
(see [`viz-personal-number-410-2026-09-26.md`](viz-personal-number-410-2026-09-26.md)).

**The −2 on the headline decomposes as** TD3 +1 (#471, a wrong accept), TD2 +1 (#521, correct),
TD1 −4 (#521: −5 + 1). #483's −1 and +1 on TD1 cancel. **Neither the TD1 loss nor the TD3 gain is
an accuracy signal on its own.** TD3's extra hit is wrong. Of TD1's five lost hits, two were wrong
reads that are now misses, and one (seed 50) was a hit that #483 had gained and #521 took away.

## The prefix wrong accepts that remain

`prefix_wrong_accepts` (#453, the count `ci.yml`'s M4 gate pins at 0 on TD3 seeds 0–49) across
all 100 seeds: TD3 **8 → 1**, TD2 **13 → 1**, MRV-B **2 → 0**, TD1 and MRV-A 0 → 0. The two left
are unchanged in every arm, and neither is a filler shift:

- TD3 seed 61 accepts line 1 `P<<<<<<<<<JPNWHITFIELDKELIASCCCC…K<`. That is 9 fillers before the
  issuer, and the name run is read as `C`s.
- TD2 seed 51 accepts a zone whose line 1 is a copy of line 2 (`CAHIDP8KE1UT00404228F…` over
  `CAHIDP8KE1UTO0404228F…`). The two copies differ in one `0`/`O`.

Both lie outside the gate's seeds 0–49, which is why the gate is green. This note does not examine
them further.

## Platform: this container agrees with Windows and with CI

- **Observed, counts:** at `d052ae2` this container reads 74 / 72 / 52 / 85 / 87, the same as
  `bench-charts.yml` [run 36111444345](https://github.com/ruledicaprio/SynthPass/actions/runs/36111444345)
  (Linux CI, `f80877b`) and the 2026-09-25 Windows arms. It also agrees on strict, wrong-accept and
  exact counts with the Windows arms (CI does not publish those).
- **Observed, seeds:** every seed the 2026-09-25 note names reads the same here. The seeds are the
  TD3 misses 7, 46 and 48, the eight TD3 prefix wrong accepts, the TD3 `personal_number` collisions
  3, 15, 16, 36, 49, 56 and 93, the TD1 optional-data collisions 11, 55 and 91, the TD2 ones 11, 12
  and 93, the 13 TD2 `document_type` errors, and MRV-A's 31 `optional_data_1` errors. The
  2026-09-25 per-seed reports were not committed, so a full 500-seed comparison is not possible.
- **Observed, stability:** TD3 seeds 0–49 ran twice at `739279c`, once as `--count 50` and once
  inside `--count 100`, and matched on every `got` string. All 54 single-seed control runs at
  `d052ae2` and `1ef7426` matched those builds' full-run values.
- **Observed, time budget:** no document came near `SYNTHPASS_OCR_MAX_SECONDS`'s 52 s. The mean
  was 2.7 s on TD3 and 3.7 s on TD1, and the maximum 7.0 s. A `cargo test` in another worktree
  shared the CPU during part of the runs. It could only have mattered by pushing a document past
  52 s, so the readings do not depend on it (Derived).

## What CI has measured since 2026-09-25

Nothing on `main`. The newest `results/<fmt>-bench/history.jsonl` rows on `bench-data` for all
five synthetic tracks are at `f80877b` (2026-09-25; `read_ok_rate` TD1 0.52, TD2 0.72, TD3 0.74,
MRV-A 0.85, MRV-B 0.87). They are aggregates only, with no per-seed or per-field entries.
`bench-charts.yml` runs on a Monday cron, and no run has written a row since. So the 368 is local
only until the next run. That run should read 75 / 73 / 48 / 85 / 87 if Linux CI still agrees with
this container per seed.

## What this does not claim

- **Not a CI measurement.** Every "now" figure is a local Linux run. It agrees with CI on the
  before arm, which is evidence that CI will agree, not proof.
- **Not a same-binary A/B.** The attributions compare builds of different commits. They rest on
  per-seed determinism: two runs of one build, and single-seed against full runs, matched on every
  document. They also rest on the bisect locating each change at exactly one commit boundary.
- **Not an attribution of seeds that did not move.** Only the 27 seeds that differ between the
  before and after reports were run at the bisect builds. A seed that one commit changed and a
  later commit changed back would not be seen.
- **Not a claim about real specimens.** #468, #471 and #483 were measured on the real-specimen gate
  in their own PRs. These 500 documents say nothing about it.
- **Not that #521 made TD1 worse at reading.** It changed the image. Whether the new TD1 VIZ is
  harder to read, or whether the old one was easier by accident, is not separated here.
- **Timing is not a result.** No run was on an idle machine.

## Invocation

Environment for every run: `SYNTHPASS_OCR_MAX_PASSES` unset (this session exports `1`, which would
cap OCR at one pass), `SYNTHPASS_OCR_AUTO_DOWNLOAD=0`,
`SYNTHPASS_OCR_MODEL_DIR=/opt/synthpass/models`, and every other `SYNTHPASS_OCR_*` unset.
**`synthpass-bench` ignores `SYNTHPASS_OCR_MODEL_DIR`.** It loads `<repo>/text-detection.rten`
and `<repo>/text-recognition.rten`, with the repo path fixed at compile time. Those files hash to
the `real-specimen-gate.yml` pins (`f15cfb56…`, `e484866d…`), as do the copies in
`/opt/synthpass/models`. Effective arms, all defaults: max passes 14 (13 retry variants + 1), time
budget 52 s, texture `on`, order, rotate and skew `default`, chargrid `off`, stop `first-valid`
(confirm passes 2, inert under `first-valid`). The report does not yet record them. That is the
code half of #510.

```
# after arm, in the worktree at 739279c
cargo build -p synthpass-bench --release
target/release/synthpass-bench --document-type <fmt> --profile clean --count 100 --seed 0 \
  --out artifacts/synth-headline-2026-09-27-<fmt>.json      # fmt in td3 td2 td1 mrva mrvb
```

`synthpass-bench` is the binary that serves the headline invocation
(`crates/synthpass-bench/src/bin/synthpass-bench.rs`). It is the one `bench-charts.yml` runs
through `scripts/run-bench.ps1` for the five synthetic tracks.

The before, mid and bisect builds were each extracted with `git archive <sha>` into a separate
source tree, with the `.rten` pair copied in, and built with one shared `CARGO_TARGET_DIR`. **Every
source file was `touch`ed before building.** `git archive` stamps files with the commit time, so
without the touch cargo judges the older sources fresh and reuses another commit's `mrz` artifacts
(see Rejected, below). The same `synthpass-bench` invocation ran at `d052ae2` (all five formats)
and `1ef7426` (TD3, TD2, TD1, MRV-B), plus `--seed S --count 1` for each moved seed at `d052ae2`,
`97b3e7a`, `8ce66f2`, `6dbc195`, `1cb9236`, `c11dd99` and `1ef7426`. Every run exited 0. Report
SHA-256 prefixes, after: td3 `20a90894`, td2 `6f06cd3b`, td1 `0aa8e33e`, mrva `bba34a40`, mrvb
`bb8a24d5`. Before: td3 `372afbc2`, td2 `b64048fc`, td1 `c6ff7286`, mrva `73c3843f`, mrvb
`c2c98a42`. The reports and the three stdlib comparison scripts
(`artifacts/synth-headline-2026-09-27-{compare,diff,bisect}.py`) are local and not committed.

## Rejected on the way

- **A bisect over a shared target directory without touching sources.** Rejected after the build
  at `30cf50e` failed to compile. Its `synthpass-ocr` was built against the `mrz` artifacts cached
  from `c11dd99`, which lack `damaged_recovery`. The builds at `97b3e7a`, `8ce66f2`, `6dbc195` and
  `1cb9236` from that pass had silently reused `d052ae2`'s `mrz`. They reported "no seed moves
  until after `1cb9236`", which was false. Their outputs are kept apart as `stale-shared-target/`
  and none is cited above. The `1ef7426` full runs from that pass matched the rebuilt ones seed for
  seed, and only the rebuilt ones are cited.
- **Reading 370 → 368 as a regression.** Rejected: correct reads rose 163 → 177 and wrong accepts
  fell 207 → 191. The whole net loss is #521's image change on TD1.
- **Crediting the TD3 +1 as an accuracy gain.** Rejected: the gained hit (seed 85) is a wrong
  accept. TD3's accuracy gain is instead the five wrong reads #468/#471 made correct.
- **Crediting #481.** Rejected: it moves none of the tracked seeds.
