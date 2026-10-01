# The synthetic headline on `main`: 391 / 500, all five formats measured at `196dbca`, and CI's nightly agrees on every document

**Date:** 2026-10-01 · **MAIN:** `196dbca` (#647, `origin/main` when the runs started); attribution builds `e9bea72` (#566 as merged), `e9bea72` with #580 reverted, `9e974f3` (#593), `26f1bba` (#598), `8de1454` (#602) and `53fb184` (the 2026-09-28 after arm) · **DATA:** n/a (generated corpus, no `samples-data` input) · **Evidence:** Observed (five local release `synthpass-bench --document-type <fmt> --profile clean --count 100 --seed 0` runs at `196dbca`, one per format; TD1 runs of the same invocation at four attribution builds; seventeen single-seed runs; CI's `bench-data-collection.yml` fixed slice, run 36697344380) plus Derived (per-seed comparison of the reports with each other, with the 2026-09-28 and 2026-09-29 reports, and with CI's rows; the totals) · **Status:** current

**Runs on 2026-10-01, 02:40–03:30 UTC, one Linux container, one process at a time.** The
benchmarks README's synthetic row has said 389 / 500 since 2026-09-29, and the figure is Derived.
TD1 was measured at `55c03a2`, #566's branch before it merged
([`synthetic-headline-2026-09-29.md`](synthetic-headline-2026-09-29.md)). The other four formats
carried over from `53fb184`
([`synthetic-headline-2026-09-28.md`](synthetic-headline-2026-09-28.md)). Since then #566 merged
as `e9bea72`, on top of 19 PRs that include #577, #578 and #580, and 46 more PRs followed. This
note measures all five formats on `main`, names every seed that moved since those two notes, and
compares the result with CI's first measurement of the same corpus. It succeeds both notes. The
documents are generated, so every value below is synthetic and safe to publish.

## The answer

| Five formats × 100 documents | The README until now (Derived) | `196dbca` (Observed) |
| :-- | --: | --: |
| Hits | 389 | **391** |
| Hits by format: TD3, TD2, TD1, MRV-A, MRV-B | 74, 81, 57, 87, 90 | 74, 81, **59**, 87, 90 |
| Strict (both names exact) | 235 | **238** |
| Wrong accepts (a hit wrong in at least one of the 12 scored fields) | 193 | 193 |
| Exact on all 12 scored fields | 196 | **198** |
| Valid misses (`document_number_mismatch`: checksum-valid, wrong document number) | 23 | **20** |
| TD1 accepted reads with a wrong document code or issuer (#578's rule) | 5 | **1** |

- **Observed:** 391 / 500 = 78.2%, five measured counts: TD3 74, TD2 81, TD1 59, MRV-A 87, MRV-B 90.
  All five come from one release build of `196dbca`, so the total is a five-format run of one
  build, not a sum across builds. Every generated document carries a conforming zone, so the
  scored count is the corpus count, and both denominators are 500.
- **Observed, the quality counts:** strict **238**, wrong accepts **193**, exact on all 12 fields
  **198 / 500**. Valid misses are **20**: TD3 4, TD2 3, TD1 7, MRV-A 3 and MRV-B 3. Of the 411
  accepted reads, **1** has a wrong document code or issuer, TD1 seed 0, and no hit has one.
- **Observed, what moved:** two TD1 misses became hits. Three seeds in other formats changed their
  field results without changing a hit. Each is named below with the PR that moved it.
- **Observed, CI:** `bench-data-collection.yml`'s scheduled run 36697344380 read this corpus at
  `04a6479` on 2026-09-30. It agrees with this note's runs on every one of the 500 documents (see
  "CI has measured this corpus, document by document").

## TD1: 57 → 59, one seed from #580 and one from #593

| TD1 build | Hits | Strict | Wrong accepts | Correct | Valid misses | Accepted, wrong code or issuer |
| :-- | --: | --: | --: | --: | --: | --: |
| `55c03a2` (#566's branch, the 2026-09-29 after arm) | 57 | 31 | 27 | 30 | 10 | 5 |
| `e9bea72` with #580 reverted | 57 | 31 | 27 | 30 | 10 | 5 |
| `e9bea72` (#566 as merged, on 19 PRs that include #577, #578 and #580) | 58 | 32 | 27 | 31 | 8 | 2 |
| `26f1bba` (adds #593, #595 and #598) | 59 | 32 | 28 | 31 | 7 | 1 |
| `8de1454` (adds #602) | 59 | 32 | 28 | 31 | 7 | 1 |
| `196dbca` (`main`) | 59 | 32 | 28 | 31 | 7 | 1 |

`55c03a2`'s last column is the 2026-09-29 note's recomputation, because that build predates #578's
count. Every other value is read from the build's own report.

- **Seed 39, miss → correct (#580).** At `55c03a2` its line 1 lost the filler after `I`, and the
  shifted reading passed its check digits: code `IU`, issuer `KRK`, document number `SYT45P050`.
  That made it a valid miss. At `e9bea72` it is exact on all 12 fields, read on pass 01. With #580
  reverted it is the same valid miss again.
- **Observed, #580 is TD1's only mover in that window.** `e9bea72` with #580 reverted equals
  `55c03a2` on every key both reports carry, for all 100 seeds, except `line1_flagged`, which
  differs on 12 seeds. So the other 18 PRs merged beneath #566, #577, #578 and a dependency
  bump (#572) among them, move no TD1 outcome or field. The flag changes are report-only, and
  this note does not attribute them.
- **Observed, what #580 changes on TD1: 17 seeds, and on every one the document code and issuer go
  from wrong to right.** No seed's prefix goes the other way.
  - Seed 39 becomes the correct hit above.
  - Seed 28 (`IU`, `KRY`) stays a valid miss. It now reads its prefix right, on pass 02, but its
    document number is still wrong: `YAK05VOV2` against the truth `YAKO5V0V2`, an `O` and a `0`
    swapped where the check digit cannot see it.
  - Seed 52 (`IB`, `GRO`) is now refused. Its check digits fail on every pass, so a wrong
    accepted read becomes an honest miss.
  - The other 14 (2, 11, 13, 18, 29, 34, 36, 40, 42, 51, 59, 74, 75, 82) are refused in both
    builds, with the prefix now right. Three of them (13, 36, 82) also change whether both names
    are exact, which no count includes, because they are misses.
- **Seed 57, miss → hit, still a wrong accept (#593).** At `55c03a2` and at `e9bea72` the name line
  was taken as line 1: code `AD`, issuer `EYE`, document number `MIYUKI`. At `26f1bba`, lines 1 and
  2 are exact, so the zone is a hit. Line 3 merges the names (`ADEYEMYUKI`; the truth is
  `ADEYEMI<<YUKI`), so the hit is a wrong accept. It is not the correct read that
  [`wrong-physical-line1-2026-09-28.md`](wrong-physical-line1-2026-09-28.md) modelled for this
  seed at `55c03a2`.
  - **Observed:** the `e9bea72` and `26f1bba` reports differ on seed 57 alone, on every key both
    carry except `elapsed_ms`.
  - **Observed, the PR.** Run alone on a build of `9e974f3`, which is #593 on top of `e9bea72`,
    seed 57 already reads as it does on `main`: a hit, wrong in both names. #593 ranks a zone that
    holds a wrong physical line below an alternative.
- **What stays.** Seed 0 keeps its line-1 shift. The filler after `I` is dropped, the code reads
  `IS` and the issuer `RBT`, and the shifted reading passes its check digits. It is the one
  accepted read with a wrong prefix left in 500. This note does not diagnose why #580 does not
  reach it.
- **Observed, #602 moves no TD1 seed.** The `26f1bba` and `8de1454` reports are identical on every
  shared key of every seed. After `8de1454`, only the miss-reason text of the seven valid misses
  differs: since #614 it no longer quotes the read and expected values. No outcome or field moves.

## The other four formats: three seeds move fields, and no hit moves

The `53fb184` reports (2026-09-28) against `196dbca`, per seed, on the hit, both names and the
wrong-accept bit:

| Format | Seed | `53fb184` | `196dbca` | Moved by |
| :-- | --: | :-- | :-- | :-- |
| TD3 | 95 | wrong accept: line 1 is a second reading of line 2 (code `PE`, issuer `NT7`) | correct | #593 |
| MRV-A | 76 | wrong accept: given names `NIKOLAL`, and `optional_data_1` | wrong accept: `optional_data_1` only; the accepted read now comes from the general pass | #602 |
| MRV-A | 83 | valid miss (`VOOJ6QOWJ`), both names exact | valid miss; the names run together (`OKONKWOASTRID`) | #602 |

- **TD2 and MRV-B:** no seed changes on those bits. #577 changed TD2 seed 87's issuer on
  2026-09-28. That seed stays a hit and a wrong accept, so no count shows it.
- **Derived, the four formats together:** strict 204 → 206 (TD3 95 and MRV-A 76), wrong accepts
  166 → 165 and exact on all 12 fields 166 → 167 (both TD3 95). Hits stay 332.
- **Observed, the window for each seed.** Each seed was also run alone on the `53fb184`,
  `e9bea72`, `26f1bba`, `8de1454` and `196dbca` binaries. TD3 95 changes between `e9bea72` and
  `26f1bba`, and a build of `9e974f3` (#593 alone on top of `e9bea72`) already reads it
  correctly. Both MRV-A seeds change between `26f1bba` and `8de1454`, which is #602 alone. At
  both ends, a seed run alone agrees with the 100-document run on the hit, both names and the
  wrong-accept bit.
- **Observed, the PRs' own A/Bs agree.**
  - #593's A/B (2026-09-29, `c21606b` → `039bd59`) turned TD3 95's line 1 into
    `P<BLRTKACHENKO<OLENA<<<…` and made its names exact.
  - #602's A/B (`a915742` → `8de1454`) moved MRV-A 76 and 83 on those three bits, and no other
    seed in any format.

## CI has measured this corpus, document by document

`bench-data-collection.yml` (#613, ADR-0027) writes a `fixed` slice every night: 100 clean
documents per format at seed 0. That is this note's invocation.

- **Observed:** the scheduled run 36697344380 measured `main` at `04a6479` (#630) on 2026-09-30,
  09:38–10:05 UTC. Its 500 rows are in `fixed.jsonl` on `bench-data`, commit `74908a3`. Its
  run headers (`runs.jsonl`) record the same model hashes as this note's runs, the class sweep
  `off` and the line-1 selector `on`.
- **Observed:** all 500 documents agree with this note's runs at `196dbca`. They agree on the
  rendered image's SHA-256, the hit, all twelve scored fields' CER, the name-error class, the
  retry variant that was accepted and the line-1 flag.
- **Observed, the counts in CI's run headers:** hits 74, 81, 59, 87 and 90; strict 48, 48, 32, 49
  and 61; wrong accepts 30, 33, 28, 60 and 42 (TD3, TD2, TD1, MRV-A, MRV-B). Accepted reads total
  411, with one wrong prefix, on TD1.
- **Derived:** none of the 15 PRs between `04a6479` and `196dbca` changes a default synthetic
  read. #631 and #633 add opt-in rules that are off by default. The others are tests, tooling,
  ledgers, CI and packaging. The identical rows confirm it.
- So 391 / 500 is confirmed in CI document by document. CI's five formats ran on three different
  processors (AMD EPYC 7763 and 9V74, Intel Xeon Platinum 8573C), and every one reproduces this
  container. The 389 it replaces was confirmed on counts only, by `bench-charts.yml`'s rows for
  `e33b177` on 2026-09-28.

## What this does not claim

- **Not a same-binary A/B.** The generator is compiled into `synthpass-bench`, so every
  attribution build is its own binary. The evidence that a rebuild adds no noise: the `26f1bba`
  and `8de1454` builds are identical on every key of every TD1 seed, and CI's runner reproduces
  this container document by document.
- **Not a claim about real specimens.** `synthpass-gen` is not on any real-read path, and no
  real-specimen run was made for this note. The real-specimen gate measures those.
- **Not a claim about the line-1 integrity flag.** TD1 hits that carry it number 28 here and 19 in
  the 2026-09-29 note. This note does not attribute that move.
- **Not every pass.** The reads quoted are the ones each report records for a seed, not every OCR
  pass the retry loop tried.
- **Timing is not a result.** The machine was not verified idle.

## Invocation

Environment for every run: `SYNTHPASS_OCR_MODEL_DIR=/opt/synthpass/models`, and every
`SYNTHPASS_OCR_*`, `SYNTHPASS_MRZ_*` and `SYNTHPASS_BENCH_ARCHIVE` knob unset. The `196dbca`
reports record the class sweep `off`, the line-1 selector `on`, and the repeated-line refusal and
the date-digit rule `off`, which are all defaults. The models are `text-detection.rten`
`f15cfb56…` and `text-recognition.rten` `e484866d…`, the ones CI pins.

```
# every build in one separate worktree with a warm release target; binaries copied out
git -C <worktree> checkout --detach <sha>   # 196dbca; e9bea72; 9e974f3; 26f1bba; 8de1454
CARGO_INCREMENTAL=0 cargo build --release -p synthpass-bench --bin synthpass-bench
# the #580 arm: e9bea72, then
git -C <worktree> revert --no-commit 1367ee1   # applied cleanly; built as above

# the five formats at 196dbca (binary sha256 prefix c478049612ee7f81), fmt in td1 td2 td3 mrva mrvb
synthpass-bench --document-type <fmt> --profile clean --count 100 --seed 0 --out <fmt>.json

# TD1 at each attribution build
synthpass-bench --document-type td1 --profile clean --count 100 --seed 0 --out td1-<sha>.json

# single seeds (document i uses seed N+i): TD3 95, MRV-A 76 and MRV-A 83 on the 53fb184, e9bea72,
# 26f1bba, 8de1454 and 196dbca binaries; TD1 57 and TD3 95 on the 9e974f3 binary
synthpass-bench --document-type <fmt> --profile clean --count 1 --seed <seed> \
  --out <sha>-<fmt>-<seed>.json
```

Every run exited 0. The `196dbca` runs took 02:43–02:58 UTC. Report SHA-256 prefixes: TD1
`e2a57cc8`, TD2 `c794421c`, TD3 `c676faf0`, MRV-A `0a180922`, MRV-B `ea927cb2`. The reports and
scripts are local session files and are not committed. CI's rows are on `bench-data`.

## Rejected on the way

- **Crediting #602 with TD1's +2.** Rejected: `26f1bba`, before #602, already reads 59, and its
  report equals `8de1454`'s on every key.
- **Carrying the other four formats over once more.** Rejected: they were measured. Three seeds
  moved, and none of them changes a hit.
- **Reading 391 against 389 as run-to-run noise.** Rejected: each of the two new hits has a named
  read and a PR, and a different machine reproduces every document.
- **Settling for CI's agreement on counts.** Rejected: the fixed slice records per-document fields,
  so the comparison is per document.
