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

Prerequisite: `samples/` images aren't tracked on `main` — run
`./scripts/sync-samples.ps1` once to pull the corpus down from the orphan
`samples-data` branch before a track's first run (see CONTRIBUTING.md's
"Adding a corpus specimen").

`scripts/run-bench.ps1 -Track <name>` runs `provider-bench --real-specimens`
(real-specimen tracks) or `synthpass-bench` (the two synthetic `td1`/`td2`
tracks — see below) scoped to one named track, appends the flattened result
to that track's `results/<track>-bench/history.jsonl` on the `bench-data`
branch (the same branch `bench-data-collection.yml` uses for the Tier-1
`dataset.jsonl`, checked out via the same isolated `git worktree` pattern,
but pushed by hand rather than on a schedule), and regenerates that track's
trend chart. One script and one chart binary (`bench-chart`) serve every
track — adding a new one is a new `-Track` value, not new tooling.
Real-specimen tracks map to `provider-bench --format` (via
`synthpass_bench::classify_specimen`):

| Track | `--format` | Scope |
| --- | --- | --- |
| `passport` | `passport` | `samples/passports/` |
| `id_card` | `id_card` | `samples/id_cards/` |
| `driving_license` | `driving_license` | `samples/driving_licenses/` |
| `real-specimens` | *(none)* | the whole `samples/` real corpus |

**Synthetic tracks (M6): `td1`, `td2`, `td3`.** A different axis entirely —
`synthpass-bench --document-type td1|td2|td3`, the per-format Tier-1
hit-rate gate over the *generated* corpus, not a real-specimen scope. Not
to be confused with the `passport` track above, which scores real
photographed specimens through `provider-bench` — same ICAO format as
`td3`, entirely different corpus and binary; `mrz::Format` vs.
`DocumentType::document_code()`'s inability to tell TD1 from TD2 is exactly
the collision M6 spent its time untangling (see
`knowledge/ROADMAP.md`'s M6 execution note), so the naming stays
deliberately distinct here too.

`td3` is a *second*, independent way to measure TD3: the existing
`bench-data-collection.yml` scheduled workflow already covers it (run
nightly, feeding a large `dataset.jsonl`, no `read_ok_rate`/trend-chart
shape); `-Track td3` adds a `results/td3-bench/history.jsonl` data point in
the same small, `-Count`/`-Seed`-controlled shape `td1`/`td2` already use,
specifically so all three formats land in the same row shape for the
per-format comparison chart below. The two don't duplicate each other's
job.

Flattened into `results/td1-bench/history.jsonl` /
`results/td2-bench/history.jsonl` / `results/td3-bench/history.jsonl` —
the same row shape as the real-specimen tracks: `hit_rate` → `read_ok_rate`,
`provider_id` fixed at `"mrz"` (the only thing synthpass-bench measures —
OCR + ICAO 9303 checksum, no per-provider comparison), and
`field_match_rate`/`mean_cer`/`unsupported_assertion_rate` left `null` —
synthpass-bench doesn't measure them, and a fabricated `0.0` would read as
"measured, perfect" per this file's own "Record the rejections" discipline.

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

**`unsupported_assertion_rate` and date fields (fixed 2026-08-01).** The
predicate is "does the provider's asserted value appear verbatim in the OCR
text it was given" — but `date_of_birth`/`date_of_expiry` are always
reformatted to ISO (`crates/synthpass-core/src/normalize.rs`) before reaching
this harness, while the OCR text only ever carries the raw MRZ `YYMMDD`
digits. A verbatim-only check therefore flagged a *correct* date read as
unsupported almost every time — the 130-specimen passport run's `--verbose`
breakdown showed `date_of_birth`/`date_of_expiry` in nearly every document's
unsupported-fields list, for both the `mrz` and `llm` providers, which is
what surfaced this. `provider_bench.rs::is_supported` now also accepts the
raw `YYMMDD` substring for date fields specifically (`mrz_date_digits`) —
scoped to those two fields only, so a coincidental digit match elsewhere
can't paper over a genuinely fabricated non-date value. This is the same
category of false positive `synthpass-core/src/fusion.rs`'s retired
name-reconstruction check hit (see "Record the rejections" above), caught
here instead of thrown away because the fix (compare against the
pre-formatting representation) is cheap and doesn't weaken the check.

## Per-format comparison chart (`bench-chart --bars`)

`bench-chart`'s original mode plots one track's `history.jsonl` as a trend line over
time (see "Live tracks" below). A second mode, added for the M6 TD1 accuracy
milestone, draws a **bar chart comparing several tracks' latest numbers side by
side** — the question "how does TD1 stack up against TD2 and TD3 *right now*" isn't
answerable from three separate trend charts without holding three numbers in your
head:

```
bench-chart --bars --series LABEL=PATH [--series LABEL=PATH ...] --out PATH
```

Each `--series` names one track's `history.jsonl`; that bar's height is the
**latest** row's `read_ok_rate` (the same field the trend mode's first panel already
plots — a track's trend chart and its bar here always read the same number). The
y-axis is a fixed `0.0`-`1.0` on every render, never cropped to the data's own
range, so the formats stay honestly comparable to each other and no bar is
exaggerated by a narrowed axis. Each bar is labelled with its percentage, and the
caption states every series' document count explicitly (never assumed uniform) —
a 30-document corpus must not get to look like a larger claim than it is.

`knowledge/img/format-comparison.svg` (embedded on the README) is generated this
way, from `td1`/`td2`/`td3`'s three `history.jsonl` files, regenerated in the same
step `run-bench.ps1`/`bench-charts.yml` regenerate the per-track trend charts, so
it never goes stale relative to them.

## Live tracks

Each chart is regenerated locally by `scripts/run-bench.ps1`, or on a schedule by
`.github/workflows/bench-charts.yml`, and lands on `main` only via a normal, human-reviewed
PR — never auto-committed from either the script or the workflow, since `main` is
branch-protected by design. The workflow opens the PR for you when a scheduled run produces a
changed SVG; a local `run-bench.ps1` run leaves the regenerated file for you to commit yourself.

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
