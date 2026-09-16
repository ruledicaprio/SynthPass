# `provider-bench` and `synthpass-bench` agree on every synthetic document, all five formats

**2026-09-16.** M6 groundwork. The published per-format synthetic rates are to change source from
`synthpass-bench` (which calls `mrz::find_and_parse` on the OCR text directly) to
`provider-bench --mrz-only --document-type <fmt>` (which reads through `ProviderCatalog` +
`MrzReader`). Before the switch, both harnesses were run on identical inputs.

**Verdict: identical.** Across 500 documents (five formats × 100 seeds), both harnesses give the
same hit count in every format, the same outcome for every seed, the same miss kinds and the same
failing check-digit fields. Switching the source changes no number. The README row being replaced
is wrong for a different reason: it no longer matches either harness (see
[The published row is stale](#the-published-row-is-stale-and-mixes-two-corpus-sizes)).

## Result

Seed 0, 100 documents, profile `clean`. The synthetic corpus has no off-denominator population
(every generated document carries a conforming zone), so the scored denominator and the whole-corpus
denominator are the same 100 documents in each row.

| Format | `synthpass-bench` HIT | `provider-bench` HIT | Seeds that differ | `checksum_failed` | `doc_number_mismatch` | `no_mrz_found` |
| :-- | --: | --: | --: | --: | --: | --: |
| TD3   | 78 / 100 | 78 / 100 | 0 | 18 | 3 | 1 |
| TD2   | 73 / 100 | 73 / 100 | 0 | 17 | 3 | 7 |
| TD1   | 54 / 100 | 54 / 100 | 0 | 38 | 7 | 1 |
| MRV-A | 85 / 100 | 85 / 100 | 0 | 11 | 4 | 0 |
| MRV-B | 87 / 100 | 87 / 100 | 0 | 9  | 3 | 1 |
| **All five** | **377 / 500 = 75.4%** | **377 / 500 = 75.4%** | **0** | 93 | 20 | 10 |

The miss-kind columns are identical in both harnesses, and so is the per-seed assignment: every
seed has the same `hit` / miss-kind in both reports. The `checksum_failed, by failing field`
breakdowns also match, field for field (TD3: composite 14, document_number 12,
personal_number 8; TD1: document_number 30, composite 26, date_of_birth 1, date_of_expiry 1; and
so on).

This is what the code predicts. Both harnesses call the same `generate_corpus`, write the image with
`image.save` to a temp PNG, and OCR it with the same `NativeOcr` (`recognize` is
`recognize_detailed(..).text`). Their miss ladders are ordered the same way: no MRZ, then
checksums, then document number against ground truth parsed by the shared
`parse_ground_truth_mrz`. `provider-bench`'s three off-denominator rungs
(`no_mrz_expected`, `redacted_mrz`, `checksum_failed_specimen`) never fire on a synthetic document.

Local == CI. The same five numbers are the most recent CI rows on `bench-data`
(`results/<fmt>-bench/history.jsonl`, 2026-09-14, `git_sha` `9c8f03d`): TD3 0.78, TD2 0.73,
TD1 0.54, MRV-A 0.85, MRV-B 0.87. Those rows are aggregates only, so the comparison with CI is at
the count level, not per seed. Between `9c8f03d` and this run's `c617254` the only commits touching
`crates/` or `Cargo.lock` were a covers-track change (#299) and two dependency bumps (#292, #293),
and none of them moved these counts.

## The published row is stale and mixes two corpus sizes

`knowledge/benchmarks/README.md` line 27 read
"~55% — TD3 74%, TD2 76%, TD1 56%, MRV-A 87%, MRV-B 93%", labelled "synthetic clean (100-seed)",
source "`synthpass-bench`, v1.4.0 cycle" (written in #248, 2026-09-09).

| Format | Published | `bench-data` 100-seed rows, 08-31 → 09-07 → 09-14 | This run (both harnesses) |
| :-- | --: | :-- | --: |
| TD3   | 74% | 74 → 75 → 78 | 78% |
| TD2   | 76% | 76 → 77 → 73 | 73% |
| TD1   | 56% | 56 → 58 → 54 | 54% |
| MRV-A | 87% | 88 → 88 → 85 | 85% |
| MRV-B | 93% | 86 → 86 → 87 | 87% |

- **MRV-A 87% and MRV-B 93% were never 100-seed figures.** No 100-seed row on `bench-data` shows
  either value. Both match the 30-document, seed-42 rows (`9b77385`, 2026-08-16: 26/30 = 86.7% and
  28/30 = 93.3%), which are also the figures in
  `knowledge/archive/roadmap-execution-log.md:626`. The row mixed a 30-document corpus into a
  column labelled 100-seed.
- **TD3/TD2/TD1 were the 2026-08-31 values**, two weekly runs out of date.
- **"~55%" matches nothing.** The published per-format values average 77.2%. Today's five
  formats pool to 377 / 500 = 75.4%. No aggregate row on `bench-data` gives 55%. Its derivation
  could not be reconstructed.

## What the row carries after the source switch

    Tier-1 hit rate, synthetic clean (100 docs per format, seed 0) |
    377 / 500 = 75.4% — TD3 78%, TD2 73%, TD1 54%, MRV-A 85%, MRV-B 87% |
    `provider-bench --mrz-only --document-type <fmt>`, 2026-09-16 (`c617254`); matches CI 2026-09-14

Unlike the real-specimen rows, this row is written by hand. The weekly `bench-data` rows are the
closest thing it has to a CI source, and they agree with it today.

## Method

Worktree `m6-harness-compare` at `c617254` (`origin/main`). The OCR models are the gitignored
`text-detection.rten` / `text-recognition.rten`, copied from the main checkout into the worktree
root (the binaries resolve them from the compile-time repo root). sha256 `f15cfb56…` and
`e484866d…`, identical to the source copies.

    cargo build --release -p synthpass-bench --bins
    # for fmt in td3 td2 td1 mrva mrvb, one process at a time:
    target/release/synthpass-bench.exe --document-type <fmt> --profile clean --count 100 --seed 0 \
        --out %TEMP%\m6hc\sb-<fmt>.json
    target/release/provider-bench.exe --mrz-only --document-type <fmt> --profile clean --count 100 \
        --seed 0 --out %TEMP%\m6hc\pb-<fmt>.json

Seed 0 and count 100 are `scripts/run-bench.ps1`'s defaults for the synthetic tracks. Every CI row
on `bench-data` uses the same invocation. The per-seed join is `results[].seed` (`synthpass-bench`,
miss reason normalised from its display string to the kind name) against `documents_detail[].name`
(`provider-bench`). The reports stayed in `%TEMP%` and are not committed.

**Contention.** Another session was compiling on this 4-thread machine during the build and
possibly during some arms. `NativeOcr`'s MRZ retry loop has a 52-second wall-clock budget, so
contention can move hit counts, not only timings. One document ran into the budget: TD3 seed 75
under `synthpass-bench`. It is a `checksum_failed` miss in both harnesses, so the per-seed
agreement is not affected. No timing is quoted here, and the report's `speed` block times only the
provider call in any case.

## What this does not claim

- Not that the two harnesses agree on real specimens. This run covers only the synthetic
  corpus. The real-specimen classification ladder has rungs that never fire here.
- Not that Windows and CI agree **per document**. The CI rows are aggregates, so agreement is at
  the count level only.
- Not a run-to-run stability measurement. Each arm ran once. `rten` noise has been seen to flip
  about three real-specimen documents. Here the two harnesses matched on all 500 synthetic
  documents in separate processes, which bounds that noise on this corpus without measuring it.
- Not an explanation of the 2026-09-07 → 09-14 movement (TD3 +3, TD2 −4, TD1 −4, MRV-A −3,
  MRV-B +1). The ADR-0008 chunk 2 orientation change landed in that window and is the obvious
  candidate. It is not attributed here.
- Not a quality claim on the `mean_cer` or line-1 integrity figures either harness prints.

## Rejected on the way

Nothing was proposed and rejected. No per-seed difference turned up, so there was no cause to
trace.
