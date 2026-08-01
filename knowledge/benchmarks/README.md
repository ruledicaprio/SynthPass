# benchmarks/

Methodology, metric definitions, and dated results — **including the candidates
that were rejected.**

Principle 6: a constant with no measurement behind it does not ship.

## What belongs here

- **Methodology** — how a run is configured so two runs are comparable. The
  corpus runner is deliberately single-threaded; a parallelised version measured
  38% where the honest sequential number was 55–56%, because oversubscribing
  `rten`'s internal threads corrupts the OCR retry budget. That is a measurement
  invariant, not a style preference.
- **Metric definitions** — precisely what each number counts. "Hit rate" means
  *the MRZ provider produced a checksum-valid record whose document number matches
  ground truth*, not "the extraction was correct."
- **Dated sweeps** — `routing-sweep-YYYY-MM-DD.md`, `provider-comparison-*.md`.
  Name the exact invocation that produced them.

## Record the rejections

The most valuable entries are the signals that looked promising and did not
survive contact with the corpus. Precedent in the codebase:
`synthpass-core/src/fusion.rs` documents a name-reconstruction integrity check
that was measured, false-positived on genuine specimens, and was thrown away —
with the reason. Without that note, someone re-proposes it every year.

## Naming a threshold

When a sweep produces a constant, the constant's doc comment names this file and
the invocation. See `MRZ_BAND_CONFIDENT_SCORE` in
`crates/synthpass-ocr/src/geometry.rs` for the pattern.

## The local bench loop (tracks)

`scripts/run-bench.ps1 -Track <name>` runs `provider-bench --real-specimens`
scoped to one named track, appends the flattened result to that track's
`results/<track>-bench/history.jsonl` on the `bench-data` branch (the same
branch `bench-data-collection.yml` uses for the Tier-1 `dataset.jsonl`,
checked out via the same isolated `git worktree` pattern, but pushed by hand
rather than on a schedule), and regenerates that track's trend chart. One
script and one chart binary (`bench-chart`) serve every track — adding a new
one is a new `-Track` value, not new tooling. Tracks map to
`provider-bench --format` (via `synthpass_bench::classify_specimen`):

| Track | `--format` | Scope |
| --- | --- | --- |
| `passport` | `passport` | `samples/passports/` |
| `id_card` | `id_card` | `samples/id_cards/` |
| `driving_license` | `driving_license` | `samples/driving_licenses/` |
| `real-specimens` | *(none)* | the whole `samples/` real corpus |

`-FromReport PATH` flattens an existing report instead of running
provider-bench again — used both for folding in a slow manual run without
redoing it, and to backfill historical reports (see below).

Row shape (one JSON object per `(run, provider)` — an aggregate, not a
per-document row, since the point is a trend line across runs, not a
document-level dataset):

```json
{"run_timestamp_unix": 1785565270, "git_sha": "abcd123", "invocation": "provider-bench --real-specimens --format passport --verbose --out ... --limit 30", "documents": 30, "provider_id": "mrz", "read_ok_rate": 0.93, "labelled_documents": 8, "field_match_rate": 0.92, "mean_cer": 0.03, "unsupported_assertion_rate": 0.25, "mean_ms": 3}
```

`read_ok_rate` (fraction of documents where OCR + parse produced a result at
all) and `unsupported_assertion_rate` are always computable, no ground truth
needed. `field_match_rate`/`mean_cer` are `null` until a run includes at
least one labelled specimen — `labelled_documents` says how many did.

## Live tracks

Each chart is regenerated locally by `scripts/run-bench.ps1` and committed
by hand on a normal branch/PR — never auto-committed from the script itself,
since `main` is branch-protected by design.

### `passport` — `samples/passports/`

![Passport bench trend](../img/passport-bench-trend.svg)

### `real-specimens` — the whole `samples/` real corpus

![Real-specimens bench trend](../img/real-specimens-bench-trend.svg)

The first four `real-specimens` data points (2026-08-01) are backfilled from
`qwen-real-10.json`/`qwen-real-10-run-2.json`/`qwen-real-30.json`/
`qwen-real-50.json` below, via `-FromReport` — those predate `--verbose`
(no `read_ok_rate`) and predate per-row `git_sha` tracking (recorded as
`"unknown (backfilled from ...)"` rather than attributed to a commit they
weren't actually measured against).
