# ADR-0027 — GitHub runners measure synthetic and public benchmark arms; nothing private, no text, no timings

**Status:** Accepted (2026-09-29: the owner accepted the plan's twelve recommendations)
**Date:** 2026-09-29

## Context

Benchmark A/B arms run on one local machine with four cores, one arm at a time, and they queue
for hours. A two-arm real-specimen A/B takes about two hours; a five-arm OCR measurement takes
about four and a half.

The nightly `bench-data-collection.yml` measures TD3 only. It appends seven keys per document to
`dataset.jsonl` on the `bench-data` branch: seed, profile, `hit`, `reason`, `elapsed_ms`, run time
and commit. Nothing consumes that dataset, and it cannot see what the pipeline now changes:
- On one binary, its nightly hit rate varies exactly as binomial sampling predicts (pooled χ² 19.4
  on 21 degrees of freedom).
- Across 55 nights of reader changes, the hit rate never moved beyond that noise.
- Wrong accepts, names and individual fields are not recorded at all.

The OCR retry loop stops on elapsed time (`SYNTHPASS_OCR_MAX_SECONDS`, default 52 s, in
`crates/synthpass-ocr`). A slower or busier machine can therefore run fewer passes, and read a
document differently, with no code change. On the nightly's synthetic documents the budget has never
bound: the slowest of 14,110 took 15.7 s. On real specimens it can.

The repository is public, so a workflow artifact or a job log is a publication.
[ADR-0010](ADR-0010-benchmark-cost-split-by-role.md) keeps private specimens out of CI.
[ADR-0024](ADR-0024-per-document-benchmark-archive.md) discloses real documents only by asset ID,
counts, cell positions and character classes.

## Decision

1. **The nightly synthetic dataset is an allowlisted projection** of `synthpass-bench`'s report,
   never an archive record.
   - **Coverage:** all five formats every night. Per format: 200 fresh documents across the five
     profiles, and a fixed slice of 100, the synthetic-headline invocation (clean, seeds 0–99).
   - **Schema:** new rows carry `schema: 2` and go to new files. `dataset.jsonl` is frozen and read
     under a schema-1 rule: a row with no `schema` key is schema 1, its `reason` maps to a miss kind
     by prefix, and every other key reads as absent, never as false or zero.
   - **Row contents:** identity (format, seed, profile, and a hash of the rendered document) plus
     outcome values: `hit`, the miss kind, the check-digit states, per-field CER, the name-error
     class, the line-1 flag, the retry stop and variant, the damaged-capture recoveries, and the
     pass ids and outcomes.
   - **Derived values:** wrong accepts, strict names, accepted reads and prefix-wrong reads are not
     stored. They are recomputed from the row and checked against the report's own counts on every
     run.
   - **No text:** no OCR text, no field values, no zone text and no `reason`. Seed and commit
     regenerate the document.
2. **The nightly measures the product as shipped.** It runs the default OCR arms and budget, and
   records them for each run. A `budget` retry stop marks that document's measurement invalid, not a
   result.
3. **An A/B on runners pins the budget.** Both arms set `SYNTHPASS_OCR_MAX_SECONDS` far above any
   document's need, so machine speed cannot enter the diff. A change whose value depends on the
   budget is measured locally.
4. **Both arms of an A/B run on one runner, one after the other,** from binaries built in the same
   workflow run. A CI arm is never compared with a local arm.
5. **Only synthetic corpora, and the public corpus at the committed `samples_data_sha`.**
   - CI never dumps real-specimen OCR text, not even as a short-lived artifact.
   - Synthetic OCR text may be an artifact kept at most 3 days.
6. **No CI timing is a result.** Timings are per-document diagnostics only
   ([PIPELINE.md](../benchmarks/PIPELINE.md) §7 item 6).
7. **Advisory, never gating.**
   - **Fixed slice:** judged seed by seed against the last night whose rendered documents hash the
     same.
   - **Fresh slice:** judged in weekly pools against the prior four weeks.
   - **Reporting:** the run reports in its job summary, as warnings, and in one aggregate line per
     night on `bench-data`.
   - **When it goes red:** only for a severe fixed-slice flag or an invalid measurement. A severe
     flag is a fixed seed newly read as a wrong accept or a prefix-wrong read.
   - **What it never does:** open an issue, run on `pull_request`, or become a required check.
   - **Thresholds:** a one-week A/A run sets the alarm threshold first.
8. **Least privilege.**
   - Jobs that run branch code hold `contents: read`.
   - The single job that writes `bench-data` runs `main`'s code and pins its third-party actions to
     commit SHAs.
   - Every `bench-data` writer rebases and retries a rejected push.

## Alternatives rejected

- **The shipped 52 s budget in an A/B.** Machine speed would enter the diff as if it were code.
- **A pinned budget in the nightly.** The nightly would then measure a configuration that does not
  ship, and on synthetic documents the budget has never bound.
- **Arms on different runners.** Hosted CPU models vary, so the pair must share one.
- **The full synthetic report on `bench-data`.** It carries text nobody needs; seed and commit
  regenerate it.
- **Real-specimen OCR dumps as short-lived artifacts.** On a public repository that publishes zone
  text.
- **A nightly real-specimen run on `main`.** The real-specimen gate already runs on every push to
  `main` that touches the extraction path.
- **One format per night.** It delays each format's signal by up to five days.
- **The committed synthetic headline as the nightly's reference.** The headline is a local
  measurement, and a generator change re-renders every seed.
- **One concurrency group for all `bench-data` writers.** It cannot cover a developer's local
  `scripts/run-bench.ps1` push, and it would queue the nightly behind the weekly chart job.

## Consequences

- **Positive.**
  - Measurement arms that need no private specimen and no real-specimen text move off the local
    machine.
  - Every format gets a nightly record that can see wrong accepts and names.
  - Each run records the conditions it ran under.
- **Negative.**
  - A CI A/B measures a budget that is not the shipped default.
  - A runner's result is not a local result.
  - After any generator change, the fresh slice needs a week of history before it can flag anything.
- **Once the fixed slice has run for two weeks** alongside `bench-charts.yml`'s weekly synthetic
  tracks, the maintainer decides whether those tracks retire.
- **Explicitly not licensed:** private or local specimens in CI; publishing archive records; any
  change to a gate, baseline, tolerance or denominator; a latency figure from CI; automatically
  opened issues.

## Build order

1. Both `bench-data` writers rebase and retry a rejected push.
2. This ADR.
3. Nightly v2: the five-format matrix, a single writer job, schema 2 in new files, the fixed slice,
   and the rendered-document hash.
4. The advisory: its statistic, the job summary, and the nightly aggregate line.
5. `bench-ab.yml`: an A/B dispatch with both arms on one runner, verified by A/A dispatches.
6. `tools/bench_ab_diff.py` refuses to compare arms measured on different machines.
