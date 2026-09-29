# The TD1 MRZ at Doc 9303's 37 px pitch: TD1 holds 57 / 100 while 20 seeds swap, correct reads go 22 → 30, and the headline stays 389 / 500

**Date:** 2026-09-29 · **MAIN:** `55c03a2` (after: #411 PR 2, draft PR #566, one commit on `d489fd3`); `d489fd3` (before: #559, `origin/main` when the runs started) · **DATA:** n/a (generated corpus, no `samples-data` input) · **Evidence:** Observed (two local release `synthpass-bench --document-type td1 --profile clean --count 100 --seed 0` runs on one Linux container, one per build; `synthpass generate` output hashed for five formats × two builds; `chargrid_repair_synthetic` at both commits) plus Derived (per-seed comparison of the JSON reports and label files, including against the 2026-09-28 reports; the five-format total) plus Hypothesized (mechanisms, marked where used) · **Status:** current

**Runs on 2026-09-28, 14:24–16:56 UTC, one Linux container, one process at a time.** Issue
[#411](https://github.com/ruledicaprio/SynthPass/issues/411) PR 2 (#566) moves TD1's three MRZ
lines to Doc 9303's pitch. Until now TD1 spaced them like every other format: a 50 px line rect
plus a 5 px gap, 55 px apart or 2.565 cap. Doc 9303-5 Figure 6 puts them 4.23 mm apart, which is
1.71–1.84 cap for a conforming print. #566 draws them 37 px apart (1.726 cap), with no gap, so the
MRZ block is 111 px tall instead of 160. `MRZ_FONT_PX = 40` keeps the glyphs the size every format
drew before, so the other four formats should not change at all. This note checks that they do
not, and measures TD1 against `main`. It succeeds
[`synthetic-headline-2026-09-28.md`](synthetic-headline-2026-09-28.md) (#411 PR 1, 368 → 389)
for TD1. That note still carries the other four formats' figures. The document set is generated,
so every value below is synthetic and safe to publish.

## The answer

| TD1 (100 documents) | Before `d489fd3` | After `55c03a2` |
| :-- | --: | --: |
| Line pitch | 55 px (2.565 cap) | 37 px (1.726 cap) |
| Hits | 57 | **57** |
| Strict (both names exact) | 23 | **31** |
| Wrong accepts (a hit wrong in at least one of the 12 scored fields) | 35 | **27** |
| Exact on all 12 scored fields | 22 | **30** |
| Prefix wrong accepts (hits only; gated by `--max-prefix-wrong-accepts`) | 0 | 0 |
| Valid misses (`document_number_mismatch`: checksum-valid, wrong document number) | 8 | **10** |
| Accepted reads with a wrong document code or issuer (#578's rule, recomputed) | 3 | **5** |
| Hits with a line-1 integrity finding | 20 | 19 |

- **Observed:** 389 / 500 = 77.8%, five measured counts: TD3 74, TD2 81, TD1 57, MRV-A 87, MRV-B 90.
  Only TD1's 57 was measured at `55c03a2`. The other four are the 2026-09-28 counts at `53fb184`.
  Their images and labels are byte-identical between this note's two builds (next section), so
  **the total at `55c03a2` is Derived**. It is not a five-format run of this build. Every
  generated document carries a conforming zone, so the scored count is the corpus count, and both
  denominators are 500 (100 for TD1 alone).
- **Observed, TD1:** hits hold at **57 / 100**, but 20 seeds swap: 10 misses become hits and 10
  hits become misses. Correct reads rise **22 → 30** and wrong accepts fall **35 → 27**. Strict
  rises **23 → 31**.
- **Observed:** checksum-valid wrong reads rise **8 → 10**. Among accepted reads, the ones with a
  wrong document code or issuer rise **3 → 5**. The router accepts both kinds, and neither is
  gated. They are the cost side of this PR. See "The two new checksum-valid wrong reads" below.
- **Derived, the five-format quality totals.** The four unchanged formats keep their 2026-09-28
  values, so strict is **235**, wrong accepts **193**, and exact on all 12 **196 / 500**. Valid
  misses are **23**. Before #566 these were 227, 201, 188 and 21.
- **Derived, the M4 check.** `ci.yml`'s `m4-hit-rate` job reads TD3 seeds 0–49. TD3's generated
  output is byte-identical, so the job's input does not change.

## The other four formats do not change, byte for byte

`git diff --stat d489fd3 55c03a2 -- crates` touches only `crates/synthpass-gen/src/layout.rs` and
`render.rs`. The reader is byte-identical source in both arms.

- **Observed:** `synthpass generate --profile clean --count 100 --seed 0` writes 200 files per
  format and build (100 PNGs, 100 label files). For **TD2, TD3, MRV-A and MRV-B**, the SHA-256 of
  every file is **identical** between `d489fd3` and `55c03a2`.
- **Observed, TD1:** all 200 files differ. In the labels, exactly two fields differ, the same way
  on all 100 seeds: **`mrz_rect.y` 342 → 391** and **`mrz_rect.height` 160 → 111**. `mrz_rect.x`
  (40) and `.width` (660) are unchanged. So is every other field, including `mrz_lines`, each
  VIZ field box, and the image size (822 × 518). The zone's bottom edge stays at y 502, and 111 is
  3 × 37.
- **Not verified from the images:** #566's commit message says the watermark, centred between the
  VIZ and the MRZ, moves from y 291 to 315 at the same size. The labels do not record the
  watermark, and no pixel comparison was made.
- **Derived:** the four unchanged formats cannot move. Their input images are identical and the
  reader is identical. Their 2026-09-28 figures carry over, TD3 74, TD2 81, MRV-A 87 and MRV-B 90.
- **Observed, what merged since.** #577 and #578 are now on the branch, merged from `origin/main`
  at `7a3e572`. Neither moves a hit, on the evidence below:
  - #577 changes the line-1 fit for TD3, TD2, MRV-A and MRV-B only. TD1 keeps calling
    `fit_length` (Derived from its diff). Its A/B, same invocation, compared `3e64ce5` (main) with
    `c21606b` (the #577 branch) on 2026-09-28, 17:45–18:40 UTC. It moved one seed in 500: **TD2
    seed 87**. That seed's issuer is now right. It stays a hit and a wrong accept (surname, given
    names), so TD2's prefix wrong accepts go 1 → 0. That A/B ran on `main`'s generator, not
    #566's. #566's TD1 on top of #577 is Derived unchanged, not measured.
  - #578 is report-only. It adds the accepted-read counts and changes no outcome.

## (a) The before arm reproduces this morning's run, seed for seed

- **Observed:** the before arm's TD1 report at `d489fd3` equals the 2026-09-28 after-arm TD1 report
  at `53fb184` (#411 PR 1 before its squash-merge as #559). They match on every key both reports
  carry, for all 100 seeds, with only `elapsed_ms` excluded. That covers hit, `wrong_accept`,
  `names_exact`, `check_states`, `name_error`, every field's CER and `got` string, and the retry
  fields.
- **Derived:** the three commits between the two, #553, #554 and #558 (mrz 0.9.0), move no TD1
  seed. The #577 A/B's `3e64ce5` and `c21606b` TD1 reports also equal this before arm on every
  shared key.

## (b) #566's effect on TD1: every flip attributed

Arms: before `d489fd3`, after `55c03a2`. The line pitch is the only variable. It is not a
same-binary A/B, because the generator is compiled in. Classes are from `tools/synth_ab_diff.py`.
"Correct" means a hit that is exact on all twelve scored fields. "Wrong" means a wrong accept. A
"valid-miss" is a checksum-valid read with the wrong document number. "Refused" means the check
digits failed. A *line-1 shift* means OCR dropped the filler after `I`: the document code reads
`IS`, `IU` or `IB`, and the issuer and document number sit one cell to the left. The reads quoted
are the MRZ lines the report records for each seed.

**Miss → hit (10).** Six become correct and four become wrong accepts.

| Seed | Before | After | What moved |
| --: | :-- | :-- | :-- |
| 4 | checksum invalid: `document_number`, `composite` | correct | the line-1 shift (`ISWESIC6R6WB73`) is gone |
| 12 | checksum invalid: `document_number`, `composite` | wrong (names) | the line-1 shift (`IBLR40VRPSCAE8`) is gone; line 3 loses its `<<` (`TKACHENKOPETROC`) |
| 53 | checksum invalid: `document_number` | wrong (names) | the line-1 shift (`IBLRJFPVF6SPOTK`) is gone; the names merge (`BONDARDMITRO`) |
| 54 | checksum invalid: `document_number` | wrong (names) | the document number's last cell and check digit read right (`WPOZ` → `WP07`); the names merge (`KOVALENKOOLENA`) |
| 55 | checksum invalid: `document_number` | correct | the line-1 shift (`IBGRZN515SB717`) is gone; line 3 is no longer read from the watermark |
| 58 | checksum invalid: `composite` | correct | line 2's optional data loses an `I` → `T` (`MVYDWZTU5L4`) |
| 66 | checksum invalid: `document_number` | correct | the line-1 shift (`IBGRNOVLBROAK`, check digit lost) and the truncated line 2 are gone |
| 68 | checksum invalid: `composite` | correct | line 2's optional data loses a `Z` → `7` (`X4JUHW87BJ8`) |
| 81 | checksum invalid: `document_number`, `composite` | correct | the line-1 shift (`IUTO9QD7FO1NZO`) is gone |
| 93 | document number mismatch: `57VVI48T6` | wrong (`optional_data_2`) | the checksum-valid misread of the document number (`Z` → `7`, `L` → `T`) is gone; line 2's `0` → `O` (`XOUOIF2RNK9`) is in both arms |

**Hit → miss (10).** Five were correct and five were wrong accepts.

| Seed | Before | After | What moved |
| --: | :-- | :-- | :-- |
| 11 | wrong (line 3 read from the watermark) | checksum invalid: `document_number` | a line-1 shift appears (`IBLRS2HONT9WE2`) |
| 26 | correct | checksum invalid: `composite` | line 2's optional data reads `O` → `0` (`KY670CTK26K`); line 3 loses its `<<` |
| 33 | wrong (`NIKOLAIC`) | checksum invalid: `composite` | line 2's optional data reads `O` → `0` (`9LYD5YGT3R0`); line 3 is read from the watermark |
| 39 | correct | document number mismatch: `SYT45P050` | a line-1 shift appears **and passes its check digits**: code `IU`, issuer `KRK` |
| 43 | wrong (names merged) | checksum invalid: `document_number` | the document number reads `O` → `0` (`NZ138880S`); line 3 is read as a copy of line 1 |
| 49 | wrong (line 3 read from the watermark) | checksum invalid: `document_number`, `composite` | the document number reads `I` → `1` (`F4AB2F212`) |
| 51 | wrong (line 3 read as a copy of line 2) | checksum invalid: `document_number`, `composite` | a line-1 shift appears (`IUTOCAHIDP8KET`); line 3 is read from the watermark |
| 60 | correct | checksum invalid: `composite` | a stray `K` in line 1's optional data, and line 2 reads `0` → `O` (`LOQE10PZFXG`) |
| 61 | correct | checksum invalid: `document_number`, `composite` | the document number's check digit reads `4` → `A`; line 3 ends in a `C` run |
| 91 | correct | checksum invalid: `composite` | line 2 reads `I` → `T` (`SWETMW3…`); line 3 is read from the watermark |

- **Derived, the swap in two lines.** Of the ten gains, six are line-1 shifts that went away (4,
  12, 53, 55, 66, 81). The other four are single-cell misreads that went away (54, 58, 68, 93). Of
  the ten losses, three are new line-1 shifts (11, 39, 51) and seven are new single-cell misreads
  (26, 33, 43, 49, 60, 61, 91). So shifts net +3 hits, and single-cell misreads net −3.
- **Observed, the rest of the 100.** The class moves under the unchanged hit count are wrong →
  correct 12 (5, 10, 17, 20, 21, 31, 62, 63, 70, 76, 78, 87), correct → wrong 5 (14, 35, 69, 90,
  97), and wrong → wrong 8 (9, 15, 32, 45, 56, 71, 73, 88). 49 seeds change `synth_ab_diff` class
  in total.

**Fifteen more seeds stay misses but change miss reason** (Observed):

| Seed | Before | After |
| --: | :-- | :-- |
| 0 | checksum invalid: `document_number` | document number mismatch: `O337R26FA` (a line-1 shift that passes, code `IS`, issuer `RBT`) |
| 2 | checksum invalid: `document_number` | checksum invalid: `document_number`, `composite` |
| 3 | checksum invalid: `document_number`, `composite` | checksum invalid: `composite` |
| 22 | checksum invalid: `document_number`, `composite` | document number mismatch: `GOQCGRMOU` (`6` → `G`) |
| 23 | checksum invalid: `document_number`, `composite` | document number mismatch: `CZAFUCFD2` (`7` → `2` in the last cell and in the check digit) |
| 28 | document number mismatch: `YAK05VOV2` | document number mismatch: `AK05VOV2Z` (now a line-1 shift, code `IU`, issuer `KRY`) |
| 29 | checksum invalid: `document_number` | checksum invalid: `document_number`, `composite` |
| 36 | checksum invalid: `document_number`, `composite` | checksum invalid: `document_number` |
| 42 | checksum invalid: `composite` | checksum invalid: `document_number` |
| 57 | document number mismatch: `2HPBKQ210` | document number mismatch: `MIYUKI` (the name line taken as line 1) |
| 59 | checksum invalid: `composite` | checksum invalid: `document_number` |
| 74 | checksum invalid: `composite` | checksum invalid: `document_number` |
| 75 | document number mismatch: `MGZTQCG1Z` | checksum invalid: `document_number` |
| 79 | checksum invalid: `document_number`, `composite` | checksum invalid: `composite` |
| 94 | checksum invalid: `document_number`, `composite` | checksum invalid: `document_number` |

## The two new checksum-valid wrong reads

`document_number_mismatch` goes from 8 seeds (8, 28, 52, 57, 75, 77, 93, 98) to 10 (0, 8, 22, 23,
28, 39, 52, 57, 77, 98). Four are new: **0, 22, 23 and 39**. Seed 39 was a correct hit before.
Two are gone: **75** is now refused, and **93** is now a hit.

- **Observed, by mechanism, over the ten in the after arm:**
  - **line-1 shift that passes its check digits:** 0 (`IS`/`RBT`), 28 (`IU`/`KRY`), 39
    (`IU`/`KRK`) and 52 (`IB`/`GRO`);
  - **the name line taken as line 1:** 57 (code `AD`, issuer `EYE`, number `MIYUKI`);
  - **a single-cell misread the check digits cannot see, prefix right:** 22 (`6` → `G`; the two
    values differ by 10, so the check digit is blind to the swap), 23 (`7` → `2` in both the
    last cell and its check digit), and 8, 77 and 98, which are unchanged from before.
- **So the shift explains the wrong prefixes, not all the mismatches.** It covers four of the
  five accepted reads with a wrong code or issuer; 57 is the fifth. Of the four new mismatches,
  two are shifts (0, 39) and two are in-place misreads (22, 23).
- **Derived, #578's count recomputed.** An accepted read is a hit or a `document_number_mismatch`
  miss. It is counted when its per-field results put `document_type` or `issuing_country` wrong:
  a field is wrong when its CER is non-zero, the rule the report's `wrong_fields` uses. Neither
  report carries #578's fields, because both builds predate it. The method gives **3 before**
  (seeds 52, 57, 75) and **5 after** (0, 28, 39, 52, 57), over 65 and 67 accepted reads.
- These reads are tracked as [#574](https://github.com/ruledicaprio/SynthPass/issues/574). Its
  PR 5, the accepted-read prefix count, merged as #578. PR 6, where TD1 prefers the line-1 reading
  whose issuer resolves, is in progress. On this corpus, PR 6 targets seeds 0, 28, 39 and 52, and
  maybe 57. Nothing here measures it.

## What the pitch did to TD1

The pitch is the only variable, so the pitch caused every move above. That is a fact about the
A/B, not a mechanism. How the pitch acts on the recognizer is Hypothesized throughout.

| Failure mode (Derived, counted per arm) | Before | After |
| :-- | --: | --: |
| Line-1 shift in the recorded read, all 100 seeds | 21 | 18 |
| Line 3 read from the watermark, all 100 seeds | 11 | 7 |
| Hits whose surname is the watermark text | 5 (11, 49, 92, 95, 99) | 1 (35) |
| Hits with a same-length name read with an `I` → `T` substitution | 5 (10, 15, 20, 32, 63) | 1 (71) |
| Hits the harness files as `separator_lost` | 4 (37, 43, 73, 89) | 11 (15, 37, 45, 53, 54, 69, 80, 88, 90, 95, 97) |
| Hits the harness files as `filler_read_as_letters` | 7 | 2 |

- **Accuracy gains come from the name line.** Observed: `I` → `T` in names mostly goes (5 → 1),
  and trailing fillers read as letters mostly go (7 → 2). The watermark is read as line 3 less
  often (11 → 7). The before arm's 5 is the 2026-09-28 note's TD1 figure (1 → 5 under PR 1), and
  this PR takes it back to 1.
- **Name-separator loss rises 4 → 11.** Observed: the harness's `separator_lost` class, where the
  `<<` between surname and given names is lost and the names run together. Of the five correct →
  wrong seeds, three are this class (69, 90, 97). Seed 14 is a merge the harness files as `other`
  (`BLACKWOODELENA K`), and seed 35's surname is the watermark.
- **Line 1 changes least.** Observed: the line-1 shift falls only 21 → 18, and different seeds
  carry it: 11 leave, 8 join. Three of the joiners pass their check digits (0, 28, 39).
  Hypothesized: the tighter pitch does not change how the recognizer reads the one-cell filler
  after `I`. It changes which seeds cross a threshold.

## The chargrid no-regression test moved with it

`cargo test --release -p synthpass-bench --test chargrid_repair_synthetic -- --ignored
--nocapture` generates TD1, TD2, TD3, MRV-A and MRV-B seeds 0–19 in-process. It reads them with
chargrid `off`, then `on`, and asserts that `on` loses no hit that `off` had.

| Tree | Hits off / on | Names exact off / on | Result |
| :-- | :-- | :-- | :-- |
| `d489fd3` (16:46–16:56 UTC) | 83 / 83 | 58 / 57 | pass |
| `55c03a2` (15:33–15:44 UTC) | 84 / 84 | 59 / 58 | pass |

- **Observed:** #566 moved the test's own counts by one hit and one exact name, in both arms.
  `d489fd3` prints what `53fb184` printed on 2026-09-28 (83 / 83, 58 / 57).
- **Derived:** the move is TD1's. The other four formats render byte-identically. In this note's
  TD1 reports, seeds 0–19 go from 10 hits to 11 (4 and 12 in, 11 out) and from 9 exact names to
  10. That matches the test's +1 and +1. The test prints totals only, so this is a match on
  counts, not seeds.
- The test still passes. Chargrid `on` still costs one exact name in both trees; that is recorded,
  not gated.

## What CI has measured

Nothing of #566. It is a draft PR, and `bench-charts.yml` measures `main`.

- **Observed:** `bench-charts.yml` wrote synthetic rows on `bench-data` on 2026-09-28,
  15:34–15:53 UTC, for `main` at `e33b177`. `e33b177` has PR 1's generator (#559) and not #566's.
  The TD1 row is `bench-data` commit `8cfb50a`. The `read_ok_rate` values, which are the hit
  rates, are 0.57, 0.81, 0.74, 0.87 and 0.90 for TD1, TD2, TD3, MRV-A and MRV-B. That is 389 / 500.
- **So PR 1's 389 is now confirmed in CI, on counts.** CI records counts only, not seeds. The
  2026-09-28 note's "What CI has measured" (nothing of PR 1) predates these rows.
- After #566 merges, the next `bench-charts.yml` run should read the same five counts if Linux CI
  agrees with this container. On counts, it has so far.

## What this does not claim

- **Not a CI measurement of #566.** Every TD1 figure is a local Linux run.
- **Not a same-binary A/B.** The generator is compiled into `synthpass-bench`, so the arms are two
  builds. The reader's source is identical in both. The before build reproduces the `53fb184`
  report, from a build without #553, #554 and #558, on every shared field of every seed. That is
  the evidence that rebuild noise is not in the reader's outcome.
- **Not a five-format run at `55c03a2` or at `7a3e572`.** 389 / 500 is Derived: TD1 is measured
  here, and the other four are carried over on byte identity.
- **Not a claim about real specimens.** Derived: real specimens cannot move. `synthpass-gen` is
  not on any real-read path, and no real-specimen run was made for this note.
- **Not a claim that 37 px reads better.** #566 makes the generator print Doc 9303's TD1 geometry
  (#411, principle 4). The numbers describe how today's reader meets that print. They include +8
  correct reads and also +2 checksum-valid wrong reads and +2 accepted reads with a wrong prefix.
  They are not the reason for the change.
- **Not a mechanism.** The failure-mode counts describe what moved, not why a 37 px pitch moves it.
- **Not every pass.** The line-1 shift and watermark counts are over the read each report records
  for a seed, not over every OCR pass the retry loop tried.
- **Timing is not a result.** The mean per document was 4.0 s before and 2.8 s after, with a
  maximum of 7.6 s. The machine was not verified idle, and the after arm ran after a container
  restart.

## Invocation

Environment for every run: `SYNTHPASS_OCR_MODEL_DIR=/opt/synthpass/models`, and
`SYNTHPASS_OCR_MAX_PASSES`, `_MAX_SECONDS`, `_CHARGRID`, `_VERBOSE` and `_DUMP_VARIANTS` unset.
Both reports record `ocr_arms` as texture `on`, order, rotate and skew `default`, and chargrid
`off`. The models (`text-detection.rten` `f15cfb56…`, `text-recognition.rten` `e484866d…`) are
the ones the 2026-09-28 runs used.

```
# both arms built in one separate worktree (warm release target); binaries copied out
git -C <worktree> checkout --detach <sha>        # d489fd3, then 55c03a2
CARGO_INCREMENTAL=0 cargo build --release -p synthpass-bench -p synthpass-cli \
  --bin synthpass-bench --bin synthpass
# sha256 prefixes: before synthpass-bench fc13063dfc1bb230, synthpass 40e29e2480247be6
#                  after  synthpass-bench 42169d48889632e0, synthpass 4b0d61ff4a958137

# generator byte identity, fmt in td1 td2 td3 mrva mrvb
bin/<arm>/synthpass generate --document-type <fmt> --profile clean --count 100 --seed 0 \
  --out-dir gen/<arm>/<fmt>
(cd gen/<arm>/<fmt> && sha256sum -- *) > gen/<arm>-<fmt>.sha256
cmp gen/before-<fmt>.sha256 gen/after-<fmt>.sha256

# the TD1 A/B
bin/<arm>/synthpass-bench --document-type td1 --profile clean --count 100 --seed 0 \
  --out <arm>-td1.json
python3 tools/synth_ab_diff.py before-td1.json after-td1.json

# the chargrid no-regression test, tree at 55c03a2, then at d489fd3
cargo test --release -p synthpass-bench --test chargrid_repair_synthetic -- --ignored --nocapture
```

Every run exited 0. The before arm ran 14:32–14:38 UTC. The first after-arm run was killed by a
container restart at about 15:26 UTC. The same binary and command were re-run from scratch,
15:28–15:33 UTC, by a resume script with the same steps. Report SHA-256 prefixes: before
`4c588ad1`, after `294e2191`. The reports and scripts are local session files and are not
committed.

## Rejected on the way

- **Reading 57 → 57 as "no effect on TD1".** Rejected: 20 seeds swap, correct reads rise by 8 and
  wrong accepts fall by 8, and checksum-valid wrong reads rise by 2.
- **Reading the +2 valid misses as `rten` noise.** Rejected: the before arm reproduces a separate
  build's report on every field of every seed, so run-to-run noise is zero on this corpus. Each
  new mismatch has a named read (0, 22, 23, 39).
- **Carrying over the other four formats on the grounds that "only TD1 changed".** Rejected as a
  premise, then checked: their generated files were hashed in both builds and are identical.
- **Crediting chargrid, or the reader, with the chargrid test's +1.** Rejected: `off` and `on` both
  move by one, and the test's generated images are what changed.
- **Attributing any TD1 seed to #577.** Rejected: TD1 is outside #577's changed path, and #577's
  own A/B moved no TD1 seed. On #566's images this is Derived, not measured.
