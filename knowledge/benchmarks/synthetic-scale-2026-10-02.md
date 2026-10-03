# Synthetic scale, 10,000 fresh documents: 75.3% Tier-1 hits (TD1 52.3% to MRV-B 86.2%), the fixed slice's 391 / 500 reproduced, 3,880 hits with a wrong scored field

**Date:** 2026-10-02 · **MAIN:** `a779c7f` (the commit the binary was built from; `origin/main` has since moved, with no change to what this bench runs) · **DATA:** none (generated renders, `clean` profile, seeds 100000-101999 per format plus the fixed slice, seeds 0-99; the vendored font) · **Evidence:** Observed on synthetic renders, `clean` profile, one laptop, commit `a779c7f` (105 `synthpass-bench` runs plus a 5-run thread-scaling series, counts from their reports and ledgers) · **Status:** current

**2026-10-02.** This is the synthetic pipeline measured at scale on one machine, for
[#613](https://github.com/ruledicaprio/SynthPass/issues/613) (the synthetic nightly's count × formats
question). **Evidence label: Observed on synthetic renders, `clean` profile, one laptop, commit `a779c7f`.**
It changes no code, default, gate, baseline or headline number, and says nothing about real documents.

The synthetic headline is 100 `clean` documents per format, seeds 0-99 (391 / 500). This run measures three
things it cannot: thread scaling on this CPU; 2,000 fresh seeds per format (100000-101999) at one commit; and the
fixed slice again, on the same machine and commit, as the control.

## The answer

- **Fresh documents: 7,526 of 10,000 are Tier-1 hits, 75.26% (Wilson 95% 74.40-76.10)**, per format 52.25% (TD1),
  78.60% (TD2), 73.95% (TD3), 85.35% (MRV-A), 86.15% (MRV-B). n = 2,000 per format, so each format's interval is
  about ±2 points where the fixed slice's is about ±9 (Observed).
- **The fixed slice reproduces the headline: 391 / 500, 78.20% (74.37-81.60)**, and its 500 per-seed outcomes are
  identical to the newest CI nightly's fixed rows (secondary comparison, below). The fresh rate is 2.94 points below
  the fixed slice's (two-proportion z = -1.49); no format's difference reaches |z| = 1.4 (Observed).
- **Wrong accepts, hits with at least one of the 12 scored fields different from truth: 3,880 of 7,526 fresh hits
  (51.55%, Wilson 50.43-52.68)**, against 193 of 391 on the fixed slice (49.4%); as a share of documents, 38.80%
  fresh and 38.60% fixed (z = +0.09). **Name fields account for most of them**: 3,026 of the 3,880 (78.0%)
  have a wrong `given_names` or `surname` (`given_names` 2,985 and `surname` 2,220 hits; table below). The rare classes are small and named below by seed: 44 hits read
  `document_type` or `issuing_country` wrong (63 accepted reads including `document_number_mismatch`), and one
  hit reads `date_of_expiry` wrong (Observed).
- **Thread scaling (TD3, seeds 0-49, one process):** 6.4 documents per minute at 1 thread, 19.7 at 8 threads, 18.0
  at all 12 logical processors; p50 6,212 ms at 1 thread, 1,850 ms at 8. The 50 per-seed outcomes are identical at
  all five thread counts (Observed).
- **Memory limit, as measured:** one `synthpass-bench` process renders every image of its run before the first OCR
  pass (`generate_corpus`), so memory grows with `--count`. Five formats at `--count 2000` and at `--count 1000`
  each aborted in about 10 s with `memory allocation of N bytes failed` on 15.7 GB of RAM; five processes at
  `--count 20` peaked at 215-247 MB each. The fresh window was therefore run as 20 windows of 100 seeds per format
  (Observed).

## Machine and build

| | |
|---|---|
| CPU | 12th Gen Intel Core i7-1255U; Windows reports 10 cores, 12 logical processors and does not show a performance / efficiency split (Intel's specification for this model is 2 + 8, not measured here) |
| RAM | 15.7 GB |
| OS and power | Windows 11 Pro 10.0.26200; High performance scheme; AC power; sleep and hibernate timeouts off on AC |
| Toolchain | rustc 1.98.1 (48a229cea 2026-09-01); `cargo build -p synthpass-bench --release --bin synthpass-bench`, exit 0 in 4 m 15 s |
| Binary | built from `a779c7f`; the archive headers read `a779c7f+dirty`, and the only untracked entries in the tree were tooling folders (`.agents`, `.codex`, `AGENTS.md`, `node_modules`, `package.json`, `package-lock.json`); no tracked file was changed |
| OCR arms | the defaults (`texture` on, `chargrid` off; `max_passes` 14, `max_seconds` 52); no `SYNTHPASS_*` variable set |
| Concurrency | one CPU slot (`slot.ps1` version 3) for every build and measurement, so no other slot job ran during them; ordinary desktop applications (a browser, the ChatGPT and Claude apps) were open throughout |

## What was run

All runs: `synthpass-bench --profile clean`, one `--document-type` per run, `--out` and `--ledger` per run.

| Phase | Invocation | Threads |
|---|---|---|
| 1 | TD3, `--count 50 --seed 0 --no-archive`, one process, five times in sequence | `RTEN_NUM_THREADS` 1, 2, 4, 8, 12 |
| 2b | the five formats at once, `--count 100 --seed 0` (the headline invocation), archive on | 2 each |
| 2a | the five formats at once, 20 windows of `--count 100 --seed 100000+100k` (k = 0..19) per format, each window its own invocation, archive on | 2 each |

2 threads is `floor(12 / 5)`. Document `i` of a run uses seed `N+i`, so the 20 windows of a format are the
documents of one 2,000-seed run. Each format's worker ran 2b first and then its 20 windows, in its own process;
the five workers ran in one slot job, 2026-10-01 21:23:57Z to 2026-10-02 03:29:41Z (21,944.6 s). All 105 runs
exited 0. **Seed coverage:** per format, the 20 windows cover seeds 100000-101999 exactly once and the 2b window
covers 0-99 exactly once.

Three earlier attempts left no data in this note: two Phase 2 launches aborted on memory (above), and a third, in
250-seed chunks, was stopped after 43 minutes because its archive headers recorded a different commit than the
binary's (the checkout had been switched back to a feature branch during Phase 1; the binary was not rebuilt).

## Phase 1: thread scaling, TD3, seeds 0-49, `clean`, one process, nothing else running

| Threads | Wall s | Documents / min | p50 ms | p95 ms | Hits |
|---|---|---|---|---|---|
| 1 | 470.7 | 6.4 | 6,212 | 22,552 | 38 / 50 |
| 2 | 267.8 | 11.2 | 3,358 | 13,205 | 38 / 50 |
| 4 | 185.4 | 16.2 | 2,284 | 9,526 | 38 / 50 |
| 8 | 152.6 | 19.7 | 1,850 | 7,586 | 38 / 50 |
| 12 | 166.8 | 18.0 | 2,173 | 8,372 | 38 / 50 |

The per-seed `hit` and `miss_kind` are identical across the five counts (0 differences). The series ran
2026-10-01 19:59:42Z to 20:20:25Z in one slot job (1,244 s).

## Phase 2a: 2,000 fresh seeds per format (100000-101999), five processes at once

`Strict` is hits that also read both names exactly. `Accepted reads` are hits plus `document_number_mismatch`
misses (checksum-valid reads the router accepts).

| Format | n | Hits | Hit % [Wilson 95%] | Strict | Wrong accepts | Prefix-wrong hits | Accepted reads | Prefix-wrong accepted reads |
|---|---|---|---|---|---|---|---|---|
| TD1 | 2,000 | 1,045 | 52.25 [50.06, 54.43] | 441 | 626 | 0 | 1,131 | 17 |
| TD2 | 2,000 | 1,572 | 78.60 [76.75, 80.34] | 838 | 756 | 7 | 1,609 | 7 |
| TD3 | 2,000 | 1,479 | 73.95 [71.98, 75.83] | 1,001 | 564 | 32 | 1,525 | 34 |
| MRV-A | 2,000 | 1,707 | 85.35 [83.73, 86.83] | 1,094 | 1,058 | 1 | 1,773 | 1 |
| MRV-B | 2,000 | 1,723 | 86.15 [84.57, 87.59] | 1,126 | 876 | 4 | 1,773 | 4 |
| All | 10,000 | 7,526 | 75.26 [74.40, 76.10] | 4,500 | 3,880 | 44 | 7,811 | 63 |

Misses by kind, counted from the ledgers:

| Format | `checksum_failed` | `document_number_mismatch` | `no_mrz_found` |
|---|---|---|---|
| TD1 | 860 | 86 | 9 |
| TD2 | 391 | 37 | 0 |
| TD3 | 475 | 46 | 0 |
| MRV-A | 226 | 66 | 1 |
| MRV-B | 212 | 50 | 15 |

Retry stops (`exhausted` / `general_valid` / `variant_valid`): TD1 934 / 703 / 363, TD2 308 / 1,180 / 512,
TD3 421 / 1,049 / 530, MRV-A 181 / 1,335 / 484, MRV-B 175 / 1,444 / 381.

## Wrong accepts by field name (2a, hits only)

A hit counts once per wrong field; a hit with two wrong fields is in two columns.

| Format | `given_names` | `surname` | `optional_data_1` | `optional_data_2` | `personal_number` | `nationality` | `document_type` | `issuing_country` | `date_of_expiry` |
|---|---|---|---|---|---|---|---|---|---|
| TD1 | 602 | 520 | 25 | 40 | 0 | 5 | 0 | 0 | 0 |
| TD2 | 732 | 613 | 40 | 0 | 0 | 10 | 7 | 7 | 0 |
| TD3 | 451 | 318 | 11 | 0 | 112 | 5 | 31 | 23 | 0 |
| MRV-A | 609 | 325 | 684 | 0 | 0 | 12 | 1 | 0 | 0 |
| MRV-B | 591 | 444 | 413 | 0 | 0 | 13 | 3 | 1 | 1 |

No hit reads `sex` wrong. The one hit with a wrong `date_of_expiry` is MRV-B seed 101245.

Accepted reads with `document_type` or `issuing_country` wrong, by seed (the synthetic seeds, no values):

| Format | Accepted reads with `document_type` or `issuing_country` wrong | Seeds (hit seeds, then `document_number_mismatch` seeds in brackets) | Fields |
|---|---|---|---|
| TD1 | 17 (0 hits, 17 mismatch) | [100017, 100117, 100391, 100395, 100867, 100888, 100983, 101157, 101160, 101180, 101256, 101427, 101441, 101639, 101777, 101817, 101954] | document_type+issuing_country 17 |
| TD2 | 7 (7 hits, 0 mismatch) | 100022, 100075, 100943, 101105, 101648, 101727, 101993 | document_type+issuing_country 7 |
| TD3 | 34 (32 hits, 2 mismatch) | 100162, 100188, 100245, 100568, 100590, 100593, 100633, 100710, 100747, 100754, 100825, 100876, 101050, 101058, 101165, 101175, 101270, 101281, 101420, 101455, 101458, 101529, 101587, 101648, 101650, 101655, 101708, 101727, 101887, 101977, 101993, 101997; [100035, 100298] | document_type 10, document_type+issuing_country 22, issuing_country 2 |
| MRV-A | 1 (1 hits, 0 mismatch) | 101449 | document_type 1 |
| MRV-B | 4 (4 hits, 0 mismatch) | 100335, 100607, 101420, 101606 | document_type 3, issuing_country 1 |

## Phase 2b: the fixed slice, seeds 0-99 per format, same machine, binary and concurrency

| Format | n | Hits | Hit % [Wilson 95%] | Strict | Wrong accepts | Accepted reads | Prefix-wrong accepted reads |
|---|---|---|---|---|---|---|---|
| TD1 | 100 | 59 | 59.00 [49.20, 68.13] | 32 | 28 | 66 | 1 |
| TD2 | 100 | 81 | 81.00 [72.22, 87.49] | 48 | 33 | 84 | 0 |
| TD3 | 100 | 74 | 74.00 [64.63, 81.60] | 48 | 30 | 78 | 0 |
| MRV-A | 100 | 87 | 87.00 [79.02, 92.24] | 49 | 60 | 90 | 0 |
| MRV-B | 100 | 90 | 90.00 [82.56, 94.48] | 61 | 42 | 93 | 0 |
| All | 500 | 391 | 78.20 [74.37, 81.60] | 238 | 193 | 411 | 1 |

Misses: TD1 34 `checksum_failed` + 7 `document_number_mismatch`; TD2 16 + 3; TD3 22 + 4; MRV-A 10 + 3; MRV-B 7 + 3.
The one prefix-wrong accepted read is TD1 seed 0 (a `document_number_mismatch`).

## Fresh against fixed

| Format | Fresh hit % | Fixed hit % | Difference (points) | z | Fresh wrong accepts | Fixed wrong accepts | z |
|---|---|---|---|---|---|---|---|
| TD1 | 52.25 | 59.00 | -6.75 | -1.32 | 626 / 2,000 | 28 / 100 | +0.70 |
| TD2 | 78.60 | 81.00 | -2.40 | -0.57 | 756 / 2,000 | 33 / 100 | +0.97 |
| TD3 | 73.95 | 74.00 | -0.05 | -0.01 | 564 / 2,000 | 30 / 100 | -0.39 |
| MRV-A | 85.35 | 87.00 | -1.65 | -0.46 | 1,058 / 2,000 | 60 / 100 | -1.39 |
| MRV-B | 86.15 | 90.00 | -3.85 | -1.09 | 876 / 2,000 | 42 / 100 | +0.35 |
| All | 75.26 | 78.20 | -2.94 | -1.49 | 3,880 / 10,000 | 193 / 500 | +0.09 |

Wrong accepts here are per document (hits with a wrong scored field over all documents of the arm).

## CI parity

**No same-commit row exists.** The brief's rule asks for a `bench-data` `fixed.jsonl` nightly row at the same
commit, or at one that differs only in docs, tools or tests. The newest nightly's fixed rows (run 36846714772, 500
rows) are at `eb1f4ff`; `a779c7f` descends from it and differs in source files (`crates/mrz`,
`crates/synthpass-bench`, `crates/synthpass-llm`, the corpus manifest), so the strict comparison is skipped.

**Secondary comparison, labelled as such:** 2b's 500 ledger rows against that run's 500 fixed rows, joined on
(format, seed): `hit`, `miss_kind`, `name_error` and `line1_flagged` are identical in 500 of 500, and the render
SHA-256 is identical in 500 of 500. On these four keys, this machine's five concurrent processes and the CI runner's
run read the same documents the same way.

## Time

Per-document `elapsed_ms` with five processes running at once ("contended"; the last three TD1 windows ran with the
other four workers already finished):

| Format | Mean ms | p50 | p95 | p99 | Max |
|---|---|---|---|---|---|
| TD1 | 10,349 | 7,432 | 22,179 | 25,752 | 36,463 |
| TD2 | 8,423 | 5,402 | 22,325 | 24,937 | 30,847 |
| TD3 | 9,228 | 5,860 | 23,007 | 26,514 | 30,137 |
| MRV-A | 7,906 | 5,942 | 21,280 | 23,871 | 37,439 |
| MRV-B | 7,408 | 5,226 | 21,988 | 24,423 | 38,767 |
| All 2a | 8,663 | 5,822 | 22,319 | | 38,767 |

No document reached the 52 s retry budget (0 of 10,000 at or above 52,000 ms). Worker wall time: TD1 2b 1,220 s
and 2a 20,716 s; TD2 826 s and 16,874 s; TD3 991 s and 18,486 s; MRV-A 748 s and 15,843 s; MRV-B 687 s and
14,843 s. For comparison, the same TD3 seeds one process at a time at 2 threads took p50 3,358 ms (Phase 1).

## The archive

The 105 runs wrote 105 archive files under `<git common dir>/synthpass-bench-archive/public/`, 18,231,338 bytes
in all, none left as `.partial`. `python tools/archive_query.py runs` lists them: 20 runs of 100 records per format
for 2a (about 3.1-3.7 MiB per format: TD1 3.7, TD2 3.1, TD3 3.5, MRV-A 3.2, MRV-B 3.1) and one run of 100 records
per format for 2b (0.2 MiB each). No record was printed, copied or uploaded. The archive is described in [PIPELINE.md](PIPELINE.md) §2.5.

## What this does not say

- Nothing here is about real documents; the generator, the `clean` profile and the OCR are the synthetic path
  only.
- The ledger holds `elapsed_ms` per document; the contended times above depend on five concurrent processes on
  this CPU and are not a latency claim for a single process (Phase 1 is the single-process series, TD3 only).
- Wrong accepts count any of 12 scored fields; a name misread is one, so the 51.55% is not a rate of
  identity-field errors on fields that carry a check digit.
- The CI comparison is against `eb1f4ff`, not `a779c7f`.
