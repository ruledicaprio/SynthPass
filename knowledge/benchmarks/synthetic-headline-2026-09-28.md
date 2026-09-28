# The MRZ at the ISO pitch: the synthetic headline goes 368 → 389 / 500, with 225 seeds churning under it

**Date:** 2026-09-28 · **MAIN:** `53fb184` (after: #411 PR 1, one commit on `a398079`); `a398079` (before: `origin/main` with #552 merged) · **DATA:** n/a (generated corpus, no `samples-data` input) · **Evidence:** Observed (ten local release `synthpass-bench --profile clean --count 100 --seed 0` runs on one Linux container, five formats × two builds) plus Derived (per-seed comparison of the JSON reports, including against the 2026-09-27 reports) plus Hypothesized (mechanisms, marked where used) · **Status:** current

**2026-09-28, runs 07:22–08:14 UTC, one Linux container, sequential, before and after
interleaved per format.** Issue [#411](https://github.com/ruledicaprio/SynthPass/issues/411) PR 1
draws every MRZ cell 22 px wide, which is 1.026 × the rendered 21.44 px cap. The ISO pitch is
2.54 mm ÷ 2.46 mm = 1.033 cap. Until now the cell was the line width divided by the character
count: 24 px on TD3 and TD1, 25 px on TD2 and MRV-B, and 23 px on MRV-A. This note does two jobs.
It checks that today's `main` still reads the committed row, 368 / 500 at `739279c`
([`synthetic-headline-2026-09-27.md`](synthetic-headline-2026-09-27.md)). It then measures PR 1
against `main`. The document set is generated, so every value below is synthetic and safe to
publish.

## The answer

| Format | Cell px | Hits | Strict (both names exact) | Wrong accepts | Exact on all 12 scored fields | Prefix wrong accepts | Valid misses |
| :-- | :-- | --: | --: | --: | --: | --: | --: |
| TD3 | 24 → 22 | 75 → **74** | 45 → 47 | 36 → 31 | 39 → 43 | 1 → 1 | 3 → 4 |
| TD2 | 25 → 22 | 73 → **81** | 48 → 48 | 28 → 33 | 45 → 48 | 1 → 1 | 3 → 3 |
| TD1 | 24 → 22 | 48 → **57** | 13 → 23 | 35 → 35 | 13 → 22 | 0 → 0 | 8 → 8 |
| MRV-A | 23 → 22 | 85 → **87** | 50 → 48 | 51 → 60 | 34 → 27 | 0 → 0 | 4 → 3 |
| MRV-B | 25 → 22 | 87 → **90** | 60 → 61 | 41 → 42 | 46 → 48 | 0 → 0 | 3 → 3 |
| **All five** | | **368 → 389 / 500** | **216 → 227** | **191 → 201** | **177 → 188** | **2 → 2** | **21 → 21** |

- **Observed:** 389 / 500 = 77.8% with PR 1 at `53fb184`. TD3 74, TD2 81, TD1 57, MRV-A 87, MRV-B 90.
  Hits use the harness's definition: a checksum-valid record whose document number matches truth.
  Every generated document carries a conforming zone, so the scored count is the corpus count and
  the two denominators are both 500.
- **Observed:** of the 389 hits, **227** read both names exactly and **201 are wrong in at least
  one of the twelve scored fields**. **188 / 500** are right on every scored field.
- **Derived:** the +21 on the headline is +11 correct reads and +10 wrong accepts. Half the gain
  is reads that are right, and half is reads that are accepted and wrong. Valid misses (a
  checksum-valid reading with the wrong document number) hold at 21.
- **Observed:** the before arm at `a398079` re-reads the committed row exactly: 368 / 216 / 191 /
  177 in total and on every format. It also matches the 2026-09-27 per-seed reports on all 500
  seeds (next section).
- **Observed, the M4 check.** `ci.yml`'s `m4-hit-rate` job reads TD3 seeds 0–49 and needs a hit
  rate of at least 0.30 with `prefix_wrong_accepts` at 0. In the after arm those seeds read
  **38 / 50 = 0.76**, with **0** prefix wrong accepts. The before arm reads 39 / 50, also with 0.
  Both prefix wrong accepts lie outside seeds 0–49.

## (a) Today's `main` re-reads the committed row, seed for seed

The row reads 368 / 500 at `739279c`. Twenty commits landed between `739279c` and `a398079`
(`git log --oneline 739279c..a398079`). Ten of them touch the read-path crates (`mrz`,
`synthpass-ocr`, `-imageprep`, `-pipeline`, `-bench`):

- two change what `mrz` accepts: #544 (#536), which refuses a document number whose first cell is
  the filler, and #547 (#539), with nine alternate country names;
- #535 was merged and then reverted by #538;
- #548 (per-document `damaged_recovery`) and #550 (removes the `clean` retry-stop arm) are no-ops
  on the default path by construction;
- #527, #528, #532 and #534 add report or trace fields, re-point citations, or stop the extraction
  path downloading models (Derived from their subjects and file lists; not measured one by one).

- **Observed:** the before arm's five reports equal the 2026-09-27 after-arm reports on every key
  both reports carry, for all 500 seeds. That covers hit, `wrong_accept`, `names_exact`,
  `check_states`, `name_error`, and every field's CER and `got` string, `mrz_lines` included.
  Only `elapsed_ms` was excluded. Those reports are
  `.claude/worktrees/m6-dump/artifacts/synth-headline-2026-09-27-<fmt>.json`, and their SHA-256
  prefixes (td3 `20a90894`, td2 `6f06cd3b`, td1 `0aa8e33e`, mrva `bba34a40`, mrvb `bb8a24d5`) are
  the ones the 2026-09-27 note publishes.
- **Observed:** the only per-document difference is keys the older reports lack:
  `retry_stop`, `retry_variant_id`, `retry_damaged_recovery` and `tier1_damaged_recovery`, plus
  the top-level `ocr_arms`. Those keys came from #527 and #548.
- **Derived:** no seed needs attributing. Between them, the twenty commits move nothing on this
  corpus, #544 and #547 included.
- A commit that moved a seed and a later one that moved it back would not be seen. Only the two
  endpoints were compared.

## (b) PR 1's effect: every moved seed

Arms: before `a398079`, after `53fb184`. `git diff --stat a398079 53fb184 -- crates` touches only
`crates/synthpass-gen/src/{layout,render}.rs` and one comment in
`crates/synthpass-ocr/examples/probe_matrix.rs`. So the reader is byte-identical source in both
arms, and **the cell pitch is the only variable**. It is not a same-binary A/B: the generator is
compiled in. Classes are from `tools/synth_ab_diff.py`. "Correct" means a hit that is exact on all
twelve scored fields. "Wrong" means a wrong accept. A "valid-miss" is a checksum-valid reading
with the wrong document number.

| Format | Moved | Hits gained | Hits lost | Class (before → after): seeds |
| :-- | --: | :-- | :-- | :-- |
| TD3 | 42 | 7, 46, 47, 82, 99 | 27, 29, 39, 43, 56, 71 | wrong → correct: 1, 3, 5, 9, 12, 14, 15, 44, 54, 55, 72, 85, 87, 89, 90, 92 · correct → wrong: 0, 18, 19, 20, 26, 32, 66, 67, 69, 78, 91, 95 · refused → wrong: 7, 46, 47, 82 · wrong → refused: 27, 29, 56, 71 · wrong → wrong: 21, 49, 61 · correct → valid-miss: 39 · wrong → valid-miss: 43 · valid-miss → correct: 99 |
| TD2 | 39 | 3, 20, 25, 29, 35, 47, 71, 87, 90, 95, 97 | 11, 16, 83 | correct → wrong: 13, 21, 23, 43, 44, 54, 56, 70, 79, 86, 88 · wrong → correct: 14, 51, 65, 73, 76, 78, 81, 91, 92, 93 · refused → correct: 3, 25, 29, 35, 90, 97 · refused → wrong: 20, 47, 71, 87 · wrong → wrong: 1, 12, 53, 64 · correct → refused: 16, 83 · valid-miss → wrong: 95 · wrong → valid-miss: 11 |
| TD1 | 52 | 1, 21, 26, 30, 43, 49, 50, 51, 56, 60, 61, 63, 69, 92 | 0, 3, 55, 58, 68 | wrong → correct: 14, 35, 44, 47, 48, 65, 72, 84, 90, 91, 97 · correct → wrong: 9, 10, 17, 19, 31, 62, 78, 80, 87 · wrong → wrong: 15, 20, 32, 33, 45, 70, 76, 88, 99 · refused → correct: 1, 26, 30, 50, 60, 61, 69 · refused → wrong: 21, 43, 49, 51, 56, 63, 92 · wrong → refused: 0, 3, 55, 58, 68 · refused → valid-miss: 8, 75 · valid-miss → refused: 22, 24 |
| MRV-A | 52 | 25, 27, 51, 54, 76 | 8, 43, 82 | wrong → wrong: 1, 2, 4, 9, 11, 15, 16, 26, 31, 33, 37, 39, 42, 56, 57, 64, 66, 75, 95 · correct → wrong: 0, 3, 13, 20, 41, 45, 53, 65, 67, 73, 84, 87, 89, 90, 92 · wrong → correct: 5, 6, 10, 32, 38, 44, 60, 81, 99 · refused → wrong: 25, 27, 51 · wrong → refused: 8, 82 · valid-miss → wrong: 54, 76 · correct → refused: 43 · refused → valid-miss: 40 |
| MRV-B | 40 | 25, 27, 47, 61, 95 | 43, 82 | correct → wrong: 1, 10, 33, 42, 54, 55, 56, 62, 70, 72, 85, 97 · wrong → correct: 8, 14, 19, 23, 29, 45, 64, 83, 88, 89, 96 · wrong → wrong: 2, 5, 7, 12, 16, 37, 50, 57, 75, 87 · refused → correct: 25, 47 · refused → wrong: 27, 61 · valid-miss → correct: 95 · wrong → refused: 43 · wrong → valid-miss: 82 |
| **All** | **225** | **40** | **19** | wrong → correct 57 · correct → wrong 59 · wrong → wrong 45 · refused → wrong 20 · refused → correct 15 · wrong → refused 12 · the six valid-miss and refusal classes 16 |

**Observed, the rest of the 500.** Another **214 seeds** keep their `synth_ab_diff` class (and,
if wrong, their set of wrong fields), but differ in something the tool does not print. That includes the
retry path (`retry_stop`, `retry_variant_id`, damaged-recovery flags), `check_states`, a
different wrong `got` string, or `mrz_lines`. They are report-only, in the spirit of #557. Only
**61 of 500** seeds are identical apart from `elapsed_ms`. The per-format split is:

| Format | Moved | Changed only in unprinted fields | Identical |
| :-- | --: | --: | --: |
| TD3 | 42 | 49 | 9 |
| TD2 | 39 | 42 | 19 |
| TD1 | 52 | 44 | 4 |
| MRV-A | 52 | 33 | 15 |
| MRV-B | 40 | 46 | 14 |

## What the pitch did, by format

The pitch is the only variable, so the pitch caused every move above. That is a fact about the
A/B, not a mechanism. How it acts on the recognizer is Hypothesized throughout this section.

- **TD1 gains most, and the gain is accuracy.** Observed: hits +9, correct +9, strict 13 → 23,
  wrong accepts flat at 35. Among hits, surname errors fall 34 → 21.
- **TD2 gains hits, mostly as wrong accepts.** Observed: hits +8, but correct only +3 and wrong
  accepts +5. Strict is flat at 48. Six refusals become correct and four become wrong.
- **TD3 loses one hit and gains accuracy.** Observed: correct +4 and wrong accepts −5. The
  name-separator merge falls, and a new I → T confusion appears (below).
- **MRV-A is the only format that loses accuracy.** Observed: hits +2, but correct 34 → 27 and
  wrong accepts 51 → 60. Of its 52 movers, 19 stay wrong with different fields wrong. MRV-A's cell
  changed least, from 23 px to 22 px. Hypothesized: the recognizer responds to where the glyphs
  fall, not to how much the cell narrows.
- **MRV-B gains a little.** Observed: hits +3, correct +2, wrong accepts +1.

**The losses do not cluster by name length or line fill.** Derived from the truth `mrz_lines` of
each seed. On every format, the seeds that got better and the seeds that got worse have a mean
name-run length within 0.8 characters of the population's (about 16). Their mean line fill is
within 0.02 of it. "Better" and "worse" order the outcomes correct > wrong > refused >
valid-miss; `wrong → wrong` seeds count as neither.

**They move by failure mode instead** (Derived: counts over hits, per arm):

| Format | Name-separator merges | Same-length name read with an I → T substitution |
| :-- | :-- | :-- |
| TD3 | 18 → 8 | 0 → 8 (seeds 10, 18, 20, 26, 32, 49, 57, 79) |
| TD2 | 20 → 25 | 0 → 1 |
| TD1 | 9 → 10 | 1 → 5 |
| MRV-A | 15 → 11 | 7 → 0 |
| MRV-B | 23 → 15 | 1 → 0 |

A merge is a hit whose surname was read as the surname run straight into the first three letters
of the given names, with `<<` lost. Examples are TD3 seed 1, `KOVALENKOANDRII` → exact, and seed 0,
exact → `BONDARIEVHENITAC`. Two formats are worth naming:

- On TD3 at 22 px, `I` read as `T` appears in the names: `IVAN` → `TVAN`, `NADIIA` → `NADITA`,
  `ANIKA` → `ANTKA`.
- On MRV-A the same confusion was present at 23 px and is gone at 22 px.

Hypothesized: the narrower cell changes which glyphs the recognizer merges or splits. It is not a
monotone "narrower is worse" effect.

**The two prefix wrong accepts change seeds, but not families.** Observed:

- TD3 seed 61 (the leading-filler shift the 2026-09-27 note names) and TD2 seed 51 (line 1 read
  as a copy of line 2) are both correct or plain-wrong in the after arm.
- The two new ones repeat the same two families on the other format:
  - **TD3 seed 95**, correct → wrong: line 1 is read as `PENT7PGCI2BLR7405235F…`, a garbled copy
    of line 2.
  - **TD2 seed 87**, refused → wrong: line 1 is read as `I<<<<<<<<<<<<USAHALVORSENELENACCCCC<`,
    with the issuer shifted eleven cells right.
- Both are outside the M4 gate's seeds 0–49.

## What CI has measured

Nothing of PR 1. `bench-charts.yml` has not written a synthetic row on `bench-data` since
2026-09-25 at `f80877b` (370 / 500; see the 2026-09-27 note). The PR's own `m4-hit-rate` job has
not run, because the commit is local. So 389 is local only. After merge, the next
`bench-charts.yml` run should read 74 / 81 / 57 / 87 / 90 if Linux CI agrees with this container
per seed, as it did on 2026-09-27.

## What this does not claim

- **Not a CI measurement.** Every figure is a local Linux run.
- **Not a same-binary A/B.** The generator is compiled into `synthpass-bench`, so the arms are two
  builds. The reader's source is byte-identical in both, and the before build reproduces the
  `739279c` reports, twenty commits older, on every shared field of every seed. That is the evidence that rebuild
  noise is not in the reader's outcome.
- **Not a claim about real specimens.** Derived: real specimens cannot move. `synthpass-gen` is
  not on any real-read path. `synthpass-ocr` takes it as a dev-dependency only, and
  `provider-bench --real-specimens` renders nothing. No real-specimen run was made.
- **Not a claim that 22 px reads better.** The headline gain is half wrong accepts, and MRV-A loses
  seven correct reads. PR 1 makes the generator print conforming geometry (#411, principle 4). The
  numbers describe how today's reader meets that print. They are not the reason for it.
- **Not a mechanism.** The merge and I → T tallies describe what moved. Why a 1–3 px change in
  cell width moves them is not separated here.
- **Not a chargrid result beyond its own assertion.** `cargo test --release -p synthpass-bench
  --test chargrid_repair_synthetic -- --ignored --nocapture` passed on the after commit
  (08:14–08:26 UTC). Over its 100 documents, chargrid `on` kept every hit `off` had: 83 against
  83. Names exact went from 58 with chargrid `off` to 57 with it `on`, one fewer. The test asserts
  only that no hit is lost, so that difference is recorded, not gated (Observed).
- **Timing is not a result.** No run was on an idle machine.

## Invocation

Environment for every run: `SYNTHPASS_OCR_MAX_PASSES` and `SYNTHPASS_OCR_MODEL_DIR` unset (`env
-u`). Every other `SYNTHPASS_OCR_*` was at its default. The reports record `ocr_arms` as
texture `on`, order, rotate and skew `default`, and chargrid `off`. The models
(`text-detection.rten` `f15cfb56…`, `text-recognition.rten` `e484866d…`) match the
`real-specimen-gate.yml` pins.

```
# both arms built in the .claude/worktrees/m6-dump worktree (warm release target)
git -C <worktree> checkout --detach <sha>        # a398079, then 53fb184
CARGO_INCREMENTAL=0 cargo build --release -p synthpass-bench --bin synthpass-bench
# binary sha256 prefixes: before 492454c972547660, after f029e9798e53bc40
env -u SYNTHPASS_OCR_MAX_PASSES -u SYNTHPASS_OCR_MODEL_DIR \
  bin/<arm>/synthpass-bench --document-type <fmt> --profile clean --count 100 --seed 0 \
  --out <arm>-<fmt>.json                          # fmt in td3 td2 td1 mrva mrvb
python3 tools/synth_ab_diff.py before-<fmt>.json after-<fmt>.json
```

All ten runs exited 0. The mean time per document was 3.5 s before and 2.8 s after. The maximum
was 10.3 s, far under the 52 s budget. Report SHA-256 prefixes:

- after: td3 `099ac8f7`, td2 `bb49c5a6`, td1 `40313e34`, mrva `8b470a2a`, mrvb `93558542`;
- before: td3 `f8173e31`, td2 `630c3a72`, td1 `5e1eca96`, mrva `a37c5ee0`, mrvb `5e882f46`.

The reports, `run.sh` and `run.log` are local session files and are not committed.

## Rejected on the way

- **Reading +21 as an accuracy gain of 21.** Rejected: +11 correct and +10 wrong accepts.
- **Reading MRV-A's +2 as a gain.** Rejected: it is −7 correct and +9 wrong accepts.
- **Attributing any seed to #544 or #547.** Rejected: the before arm equals the `739279c` reports
  on every shared field of every seed. Neither commit moves a synthetic seed.
- **A name-length or line-fill explanation for the losses.** Rejected: losers and gainers match
  the population on both.
