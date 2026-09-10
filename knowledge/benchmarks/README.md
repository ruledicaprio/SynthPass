# benchmarks/

Methodology, metric definitions, and dated results — **including the candidates
that were rejected.**

Principle 6: a constant with no measurement behind it does not ship.

## Current headline numbers

**This section is the only place in the repo that carries live accuracy numbers.** Every other
document — `README.md`, `ROADMAP.md`, an ADR — states at most one figure and links here. That rule
exists because it was broken: the same numbers were restated in four documents, and `README.md`
spent a release cycle advertising a hit rate ten points low while naming the wrong dominant miss.

The machine-readable source is [`real-specimen-mrz-baseline.json`](real-specimen-mrz-baseline.json),
written only by CI (`gh workflow run real-specimen-gate.yml -f mode=write-baseline`) and enforced
on every PR by [`real-specimen-gate.yml`](../../.github/workflows/real-specimen-gate.yml).
`scripts/check-headline-numbers.sh` fails the build if `README.md` disagrees with it.

| Metric | Value | Source |
| --- | --- | --- |
| **Tier-1 hit rate, real specimens** | **119 / 144 = 82.6%** on documents that can yield a hit | `real-specimen-mrz-baseline.json` (CI, 2026-09-09) |
| Tier-1 hit rate, whole specimen corpus | 119 / 238 = 50.0% | same baseline; the gap is explained below |
| Tier-1 hit rate, synthetic clean (100-seed) | ~55% — TD3 74%, TD2 76%, TD1 56%, MRV-A 87%, MRV-B 93% | `synthpass-bench`, v1.4.0 cycle |
| Tier-2 per-field exact match, 72-fixture parity corpus | 55.6% overall (58.6% reviewed / 52.5% derived) | `crates/synthpass-llm/tests/parity.rs` |
| Browser OCR (tesseract.js) vs native (`ocrs`/`rten`) | **80.0% vs 74.4%** on 160 non-redacted MRZ-bearing specimens, both arms measured 2026-09-09 | [`ocr-stack-gap-2026-09-09.md`](ocr-stack-gap-2026-09-09.md) |

**Why two rates.** 94 of the 238 specimens cannot produce a Tier-1 hit under any pipeline, so
counting them as failures measures the corpus rather than the reader. They are scored out, and both
numbers are published so neither can be accused of flattering by exclusion: the first says how often
extraction succeeds when success is possible, the second what a pile of real documents yields. Only
the first moves when accuracy work lands. Full analysis:
[`denominator-correction-2026-09-09.md`](denominator-correction-2026-09-09.md).

**Real-specimen outcomes** (238 documents):

| Outcome | Count | In the denominator? | Meaning |
| --- | --- | --- | --- |
| **Tier-1 HIT** | **119** | numerator | Checksum-valid MRZ, document number matches ground truth |
| `no_mrz_found` | **18** | yes | No MRZ located on a document that has one — **the dominant miss and the current bottleneck** |
| `checksum_failed` | 7 | yes | Conforming printed zone, read wrong — a genuine OCR error |
| `false_positive_mrz` | 0 | yes | A checksum-valid MRZ returned for a document carrying none. **Any non-zero value here fails the build** |
| `no_mrz_expected` | 42 | no | Document carries no MRZ at all; none was read. A correct refusal |
| `redacted_mrz` | 36 | no | Zone blacked out by whoever published the specimen |
| `checksum_failed_specimen` | 16 | no | Printed zone fails its own ICAO check digits — a byte-perfect read still fails |

`no_mrz_found` overtook `checksum_failed` when `mrz` 0.7.0 began rejecting structurally implausible
readings, and stayed ahead after the denominator correction (2.6:1). The bottleneck is MRZ
*detection*, not *parsing*; see [`ADR-0008`](../decisions/ADR-0008-mrz-detection-track.md), whose
target metric this correction reduced from 85 documents to 18.

**The browser-vs-native row replaces ADR-0008's "64.2% vs 59.5%"**, neither side of which was
current: the browser figure was the first of four measurements in `WEB_OCR_BASELINE.md` and had
been superseded three times, and the native figure was a `samples/corpus.jsonl` field last
recomputed on 2026-09-03 — unchanged through `mrz` 0.7.0, 0.7.1, `geometry_band_variants` and the
`Lanczos3`/`Triangle` decision. Both arms are now measured the same day. The gap survives at
+5.6 pp and, more usefully, **is concentrated in 17 named documents**: the browser reads 11 of
native's 18 detection failures and 6 of its 7 `checksum_failed`.

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

**Known corpus defect, found 2026-08-16: 13 of 61 `samples/id_cards/`
specimens are misplaced passport bio-data pages, not ID cards** — Iceland
(×2), India (×2), Indonesia (×3), Iran (×2), Iraq (×1), Israel (×3), all
named `<Country>_Passport_Specimen_*`. Visually confirmed for Iceland/Iraq/
Israel: each is a two-line 44-character `P<...` TD3 MRZ passport data page,
not the country's actual ID card format. `provider-bench`'s own per-document
`mrz_format` (new this session — see `miss_reason`/`tier1_hit_rate` above)
corroborates it mechanically: both `Iceland_Passport_Specimen_*` files parse
as checksum-valid **TD3** — genuine Tier-1 hits, meaning the `id_card`
track's `tier1_hit_rate` is currently *inflated* by counting misplaced
passport successes as ID-card successes, not just diluted by noise.
`classify_specimen` (`crates/synthpass-bench/src/lib.rs:230`) classifies
purely by directory — "directory wins over a disagreeing label" is
deliberate for the common case (a real label disagreeing with a
correctly-placed file), but has no defense against a file placed in the
wrong directory to begin with. Not yet moved: relocating a specimen is a
`samples-data` orphan-branch operation (see `CONTRIBUTING.md`'s "Adding a
corpus specimen"), a different process from this session's Tier-1
diagnostics/metric work, and deliberately deferred rather than folded in
here. The filename pattern (`*_Passport_Specimen_*` outside `passports/`) is
a reliable first-pass check for more of the same; the per-document
`mrz_format` `provider-bench --verbose`/report JSON now records (see
"Per-format comparison chart" below) is the exhaustive one.

A second, unrelated finding from the same data: 4 *correctly-placed*
`id_card` specimens (Italy CIE 2022, two Switzerland ID crops, one unlabelled
Bosnia-format card) resolve to **TD2**/**MRV-B** — ICAO formats no ID card
should parse as — rather than the ID-card-shaped TD1, and all four fail
checksum. This is a genuine cross-format-confusion weak spot in the
deterministic pipeline, not a corpus placement error, and is a candidate for
the "weak-spot findings" writeup once all four real-specimen tracks have a
current `tier1_hit_rate` run.

**Synthetic tracks: `td1`, `td2`, `td3` (M6), `mrva`, `mrvb` (MRV-A/MRV-B
visas, added alongside M6's own generation/extraction coverage).** A
different axis entirely — `synthpass-bench --document-type
td1|td2|td3|mrva|mrvb`, the per-format Tier-1 hit-rate gate over the
*generated* corpus, not a real-specimen scope. Not to be confused with the
`passport` track above, which scores real photographed specimens through
`provider-bench` — same ICAO format as `td3`, entirely different corpus and
binary; `mrz::Format` vs. `DocumentType::document_code()`'s inability to
tell TD1 from TD2 is exactly the collision M6 spent its time untangling
(see `knowledge/archive/roadmap-execution-log.md`), so the naming stays
deliberately distinct here too.

`td3` is a *second*, independent way to measure TD3: the existing
`bench-data-collection.yml` scheduled workflow already covers it (run
nightly, feeding a large `dataset.jsonl`, no `read_ok_rate`/trend-chart
shape); `-Track td3` adds a `results/td3-bench/history.jsonl` data point in
the same small, `-Count`/`-Seed`-controlled shape the other synthetic
tracks already use, specifically so all five formats land in the same row
shape for the per-format comparison chart below. The two don't duplicate
each other's job.

Flattened into `results/td1-bench/history.jsonl` /
`results/td2-bench/history.jsonl` / `results/td3-bench/history.jsonl` /
`results/mrva-bench/history.jsonl` / `results/mrvb-bench/history.jsonl` —
the same row shape as the real-specimen tracks: `hit_rate` → `read_ok_rate`,
`provider_id` fixed at `"mrz"` (the only thing synthpass-bench measures —
OCR + ICAO 9303 checksum, no per-provider comparison), and
`field_match_rate`/`mean_cer`/`unsupported_assertion_rate` left `null` —
synthpass-bench doesn't measure them, and a fabricated `0.0` would read as
"measured, perfect" per this file's own "Record the rejections" discipline.

`-FromReport PATH` flattens an existing report instead of running
provider-bench again — used both for folding in a slow manual run without
redoing it, and to backfill historical reports (see below).

**`read_ok_rate` cutover, 2026-08-16.** Before this date, the real-specimen
tracks' `read_ok_rate` was computed from `provider-bench`'s
`documents_detail[].read_ok` — which only means "the provider returned
without erroring." For the deterministic `mrz` provider that is
unconditionally `true` (`MrzReader::read` is documented to never return
`Err`: a document with no MRZ is a legitimate answer, not an error), so the
old `read_ok_rate` sat at ~1.0 on every real-specimen track regardless of
whether Tier-1 actually succeeded — not a useful signal, and not comparable
to the synthetic tracks' `hit_rate`-derived numbers on the same chart despite
sharing a field name. `provider-bench` now computes a genuine
`tier1_hit_rate` (MRZ found, ICAO checksums valid, document number matches
ground truth when labelled — mirroring `synthpass-bench`'s own hit
definition) and `run-bench.ps1` records that as `read_ok_rate` for
real-specimen tracks from this date onward. Rows recorded before the cutover
are not comparable to rows after it on the same trend line — a real
correctness signal appearing for the first time, not a regression.

**`tier1_hit_rate` `NotApplicable` carve-out, 2026-08-17.** `provider-bench`'s
`tier1_hit_rate` was itself a fabricated zero for the `llm` (Tier-2) provider
on every run: `LlmFieldReader` never sets `Evidence::mrz_checksums_valid`
(correctly — it doesn't compute ICAO check-digit validity), so every LLM
document was classified as a checksum-failed miss regardless of actual
accuracy. `tier1_hit_rate` in `provider-bench`'s JSON output is now a tagged
enum, `{"status": "computed", "rate": ...}` for a `capability.deterministic`
provider or `{"status": "not_applicable", "reason": ...}` otherwise, mirroring
`unsupported_assertion`'s existing `NotApplicable` shape for `vision`
providers. `run-bench.ps1` records `read_ok_rate` as `$null` (not `0.0`) for
an `llm` row from this date onward — an absent metric, not a measured-and-zero
one.

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

**`mean_ms` is charted as of 2026-09-02.** `run-bench.ps1` has always written
it, but `bench-chart` had no field for it and silently dropped it, so every
trend chart showed only rates and the harness's own speed measurements were
invisible. `bench-chart`'s trend mode now draws a fourth panel, **mean latency
per document (ms)**, gated the same way the accuracy panel is: only when at
least one row in the history carries a non-null value. Every historical row
already has one, so no backfill was needed and existing charts gain the panel
on their next regeneration.

**GPU runs: the `llm-cuda` series (added 2026-09-02).**
`./scripts/run-bench.ps1 -Track <real-specimen track> -Cuda` builds
`provider-bench` with `--features cuda` (ADR-0004's GPU offload for Tier-2) and
appends the result to that track's *existing* `history.jsonl` under
`provider_id` `"llm-cuda"`. `bench-chart` groups by `provider_id` string
equality, so it renders as a third trend line next to `mrz` and `llm` with no
chart-side change and no second track directory.

Two things to expect rather than report as bugs:

- **Its accuracy lines overlay the plain `llm` line exactly.** GPU output is
  byte-identical to CPU (ADR-0004 verified this on a GTX 970). The latency
  panel is the only place the ~2.5x is visible — which is why the GPU feature
  shipped in 2026-08 but could not be charted until that panel existed.
- **A `-Cuda` run records no `mrz` row.** `provider-bench` has no
  `--providers` filter so the deterministic reader still runs, but it is pure
  CPU and unaffected by the feature; recording it would double-weight `mrz` on
  the trend line with a duplicate CPU measurement.

The numbers are hardware-specific (a GTX 970 today) and **local-only**: no
GitHub-hosted runner has a GPU, so `-Cuda` is deliberately absent from every
track list in `.github/workflows/bench-charts.yml`. A scheduled run would
otherwise build the feature, execute on CPU anyway, and record a mislabelled
row. It is also rejected outright on the synthetic tracks, which run
`synthpass-bench` — that binary has no LLM provider for the feature to affect.

Unlike the three rate panels — which share a fixed 0.0-1.0 axis so they stay
honestly comparable — the latency panel auto-scales, because milliseconds have
no natural ceiling. It is still anchored at zero and never cropped to the
data's own range, so the same "no series is exaggerated by a cropped axis"
rule holds. One consequence worth expecting rather than reporting as a bug:
the deterministic `mrz` provider runs in microseconds and the `llm` provider
in tens of seconds, so on a shared axis `mrz` renders as a flat line on zero.
That is the true shape of a five-orders-of-magnitude difference. The panel
earns its place by comparing *like with like* — most usefully the same LLM
provider across backends.

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

## The per-PR real-specimen regression gate

`.github/workflows/real-specimen-gate.yml` runs `provider-bench --real-specimens
--mrz-only` on every PR that touches the extraction path (`crates/mrz`,
`crates/synthpass-ocr`, `crates/synthpass-imageprep`, `crates/synthpass-die`,
`crates/synthpass-core`, `crates/synthpass-pipeline`, `crates/synthpass-bench`,
`samples/ocr_fixtures/`, `Cargo.lock`) and fails the build on a Tier-1
regression. It is the `m4-hit-rate` gate (`.github/workflows/ci.yml`, synthetic
clean TD3) extended to the real `samples/` corpus — the population every
`checksum_failed` / `mrz` 0.7.x change actually moved, and which `bench-charts.yml`
only touched weekly and only as an advisory chart PR.

**`--mrz-only`.** The gate scores the deterministic `mrz` provider alone. The
Tier-2 LLM pass is ~19-37 s per document (hours for the whole corpus) against the
deterministic reader's µs-ms, and its GGUF is a ~1 GB download; the deterministic
core is also what the product is sold on (`knowledge/project_principles.md`,
`knowledge/MRZ_SEQUENCE_COMPLETENESS.md`). `provider-bench --mrz-only` builds a
one-reader `ProviderCatalog` directly from `synthpass_die::MrzReader` — no
`Pipeline`, no model — so the job needs only the ~12 MB OCR models.

**The baseline.** `real-specimen-mrz-baseline.json` in this directory holds the
`mrz` provider's HIT count, the miss-kind histogram, the denominator, and a
`tolerance`. Its **counts are produced only by CI** and never hand-edited — local
`rten` inference and CI `rten` inference differ by a few characters of float
rounding, so a locally-measured number would fail the gate on the first CI run.
Regenerate it with:

```
gh workflow run real-specimen-gate.yml -f mode=write-baseline
# then: download the `real-specimen-mrz-baseline` artifact, commit the file
```

**What counts as a regression.** The gate fails if `tier1_hits` drops below
`baseline.tier1_hits - tolerance`, **or** if any of `checksum_failed`,
`checksum_failed_specimen`, `no_mrz_found`, `ocr_error`, or
`document_number_mismatch` exceeds its baseline value plus `tolerance`. Checking
the whole histogram, not just the headline HIT count, catches a change that
moves documents `checksum_failed → no_mrz_found` (or the reverse) while the net
HIT count stays flat — a real behaviour change worth a human looking at.
`redacted_mrz` is excluded (it sits outside the denominator); a change in its
count, or in `documents`, is reported as a **warning, not a failure** — the
corpus grew or shrank and the baseline needs regenerating, which is not a parser
regression.

**`tolerance` is data, not code.** It lives in the JSON, starts at `0` (exact
ratchet), and is raised by a one-line reviewed change only if CI runs prove
flaky across runner hardware. Same reasoning as `m4-hit-rate`'s wide margin, but
tuned from measurement rather than guessed up front.

**Re-blessing (for PR authors).** A PR that legitimately moves the numbers —
a parser improvement, or adding/removing specimens — must regenerate the
baseline in the same PR (`gh workflow run real-specimen-gate.yml -r <branch> -f
mode=write-baseline`, download the artifact, commit it). The `assert` run on that
PR then compares against the updated file and goes green. This is the forcing
function that keeps the committed number honest.

**Rollout.** The gate lands **advisory** (visible, red on a regression, but not a
required check). Once a handful of `assert` runs confirm the HIT count and
histogram are identical run-to-run across CI runner SKUs, it is promoted to a
required check — which needs a trailing always-running `result` job so a
path-filtered skip on a docs-only PR does not leave the required check pending
forever.

**Envisioned next.** A second, `per-release` gate that runs the *full*
`provider-bench` (both providers) and gates a release, plus accuracy graphs in
the top-level `README.md` — both read the same baseline JSON(s) this gate writes.

## Per-format comparison chart (`bench-chart --bars`)

`bench-chart`'s original mode plots one track's `history.jsonl` as a trend line over
time (see "Live tracks" below). A second mode, added for the M6 TD1 accuracy
milestone, draws a **bar chart comparing several tracks' latest numbers side by
side** — the question "how does TD1 stack up against TD2, TD3, MRV-A, and MRV-B
*right now*" isn't answerable from five separate trend charts without holding five
numbers in your head:

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
way, from `td1`/`td2`/`td3`/`mrva`/`mrvb`'s five `history.jsonl` files — ID
documents and visas side by side on one chart, a deliberate choice: all five
measure the identical thing (Tier-1 hit rate over `synthpass-bench`'s generated
corpus, same checksum-based methodology), so one chart answers "how does the MRZ
parser do across every format it supports" without implying visas and ID documents
are the same *kind* of document, just the same *measurement*. Regenerated in the
same step `run-bench.ps1`/`bench-charts.yml` regenerate the per-track trend
charts, so it never goes stale relative to them.

## Live tracks

Each chart is regenerated locally by `scripts/run-bench.ps1`, or on a schedule by
`.github/workflows/bench-charts.yml`, and lands on `main` only via a normal, human-reviewed
PR — never auto-committed from either the script or the workflow, since `main` is
branch-protected by design. The workflow opens the PR for you when a scheduled run produces a
changed SVG; a local `run-bench.ps1` run leaves the regenerated file for you to commit yourself.

### `passport` — `samples/passports/`

![Passport bench trend](../img/passport-bench-trend.svg)

### `id_card` — `samples/id_cards/`

![ID card bench trend](../img/id_card-bench-trend.svg)

### `driving_license` — `samples/driving_licenses/`

![Driving license bench trend](../img/driving_license-bench-trend.svg)

Corpus is small (3 specimens as of 2026-08-16) and has **zero labelled ground
truth** — no `samples/ocr_fixtures/` entries exist for this class, so
`field_match_rate`/`mean_cer` are `null` on every row, not a bug. A known,
deliberately deferred gap (see `knowledge/archive/roadmap-execution-log.md` for
the parallel TD2 deferral pattern this follows): growing this corpus and
labelling at least a few specimens is separate, open-ended work, not scoped
into the Tier-1 diagnostics/metric work that added this track's chart.

### `real-specimens` — the whole `samples/` real corpus

![Real-specimens bench trend](../img/real-specimens-bench-trend.svg)

The first four `real-specimens` data points (2026-08-01) are backfilled from
`qwen-real-10.json`/`qwen-real-10-run-2.json`/`qwen-real-30.json`/
`qwen-real-50.json` below, via `-FromReport` — those predate `--verbose`
(no `read_ok_rate`) and predate per-row `git_sha` tracking (recorded as
`"unknown (backfilled from ...)"` rather than attributed to a commit they
weren't actually measured against).

### `td1` — synthetic TD1 corpus (`synthpass-bench --document-type td1`)

![TD1 bench trend](../img/td1-bench-trend.svg)

### `td2` — synthetic TD2 corpus (`synthpass-bench --document-type td2`)

![TD2 bench trend](../img/td2-bench-trend.svg)

### `td3` — synthetic TD3 corpus (`synthpass-bench --document-type td3`)

![TD3 bench trend](../img/td3-bench-trend.svg)

### `mrva` — synthetic MRV-A visa corpus (`synthpass-bench --document-type mrva`)

![MRV-A bench trend](../img/mrva-bench-trend.svg)

### `mrvb` — synthetic MRV-B visa corpus (`synthpass-bench --document-type mrvb`)

![MRV-B bench trend](../img/mrvb-bench-trend.svg)

## Weak-spot findings

Dated entries surfaced by a corpus run, kept here rather than only in a commit
message so the next person doesn't have to re-derive them from raw numbers —
same "Record the rejections" discipline as above, applied to what a
measurement *found* rather than only to what it rejected.

### 2026-08-16 — first genuine real-specimen Tier-1 numbers

The first `tier1_hit_rate` run (see the `read_ok_rate` cutover note above)
over the full local corpus, deterministic `mrz` provider, no `-Limit`:

| Track | Documents | Tier-1 hit rate | `no_mrz_found` | `checksum_failed` |
| --- | --- | --- | --- | --- |
| `passport` | 131 | 53.4% | 24 | 37 |
| `id_card` | 63 | 17.5% | 25 | 27 |
| `driving_license` | 3 | 0.0% | 2 | 1 |
| `real-specimens` (union) | 203 | 42.4% | 51 | 66 |

Over `real-specimens`, `checksum_failed` (66) outnumbers `no_mrz_found` (51)
as the dominant miss kind. **(Superseded — this ordering inverted on 2026-09-08 when
`mrz` 0.7.0 reclassified ~38 phantom `checksum_failed`; see
[Current headline numbers](#current-headline-numbers). The rest of this entry's
reasoning stands, and is why the reclassification was worth making.)**
That split matters: `no_mrz_found` conflates two
very different populations — a genuinely MRZ-less document (many real ID
cards and driving licenses have no machine-readable zone at all, a fact the
`unsupported_assertion`'s `with_mrz_anchor`/`without_mrz_anchor` split
already exists to separate) and an actual band-detection failure on a
document that does carry one. `checksum_failed` was assumed to carry no such
ambiguity — "OCR found MRZ-shaped text and got a character wrong" — but the
2026-09-08 dump analysis below shows it is also conflated: ~19 of it is
hallucinated MRZ on `*_no_mrz` documents, ~25 is redacted MRZ, and the rest is
unverifiable. The tooling: `provider-bench --real-specimens --dump-ocr` (added
2026-08-31, extended 2026-09-08) prints, for every `checksum_failed` miss, the
full pre-parse OCR text + MRZ band score + recovered MRZ zone + failing check
digit(s), and writes one row per miss to
`artifacts/provider-bench-checksum-failed-dump.jsonl` — the real-specimen
equivalent of `synthpass-bench --dump-ocr`. Analysis in the 2026-09-08 entry
below.

Two more specific findings from the same run, detailed above under "The local
bench loop (tracks)": a **corpus placement defect** (13 passport pages
misplaced into `samples/id_cards/`, inflating that track's hit rate by
counting misplaced passport hits as ID-card hits) and a **cross-format
confusion** (4 correctly-placed ID cards resolving to TD2/MRV-B instead of
their expected shape, all `checksum_failed`) — both candidates for follow-up
investigation, neither actioned this pass.

### 2026-08-17 — a full-corpus real-OCR scan found and fixed a real parser bug

The corpus grew substantially this session (contributor additions across dozens of
countries — Türkiye ×6, the UK ×4, Cyprus ×3, and many more), and the id_cards/passports
cover-vs-biodata placement defect above was fixed by hand. Scanning the whole thing with
`mrz_corpus.rs`'s fixed `CORPUS` list would have meant hand-verifying a document number for
every new specimen first; `crates/synthpass-ocr/examples/integrity_survey.rs` doesn't need
that (no fixed corpus list, just walks a directory), but scanning it unfiltered was pacing
toward ~70 minutes. Added `--dir <subpath>` (scope the walk instead of all of `samples/`) and
`--mrz-only` (skip files this corpus already tags `no_mrz` in the filename) — cut a full
`passports/` pass to ~20 minutes.

That scan (`--dir passports --mrz-only`, ~125 real specimens) found `UnrecognizedIssuingCountry`
firing on 57 checksum-valid documents. Cross-checking the garbled values against each
specimen's real ICAO code found 34 of them (60%, across 24 distinct countries) sharing one
exact shape: a single spurious character inserted right after line 1's position-1 filler,
shifting `issuing_country` one position right and losing its real third letter — e.g. Czechia
(`CZE`) read as `SCZ`, Iceland (`ISL`) as `AIS`, the UK (`GBR`) as `SGB`, the USA (`USA`) as
`SUS`. Not scattered noise — one mechanism, the mirror image of an already-shipped
dropped-filler fix whose own doc comment had already flagged this direction as incomplete.
Fixed in PR #138 (`crates/mrz/src/parser.rs`'s `shift_line1_right_at_country`, composed with
the existing fix via `shift_or_unshift_line1` — full writeup in `knowledge/ROADMAP.md`'s M6
section). Re-scanning post-fix: `UnrecognizedIssuingCountry` fires dropped from 57 to 20, zero
new false positives, all 34 matching specimens now resolve correctly. Synthetic-corpus
same-binary A/B showed zero hit-rate impact on any format (this specific seed's OCR noise
model doesn't happen to produce the artifact — the same conclusion the `fusion.rs`
line-1-integrity work reached earlier this session).

`knowledge/CORPUS_COVERAGE.md`'s per-country table was rewritten from this same pair of scans
(pre- and post-fix) — most of its previous "specimen present, not yet wired" placeholders now
carry a real, measured HIT/MISS status instead.

### 2026-09-08 — `checksum_failed` is not a clean OCR-accuracy signal

Full writeup: [`checksum-failed-real-specimens-2026-09-08.md`](checksum-failed-real-specimens-2026-09-08.md).
Pulled the raw OCR text for all 71 `checksum_failed` misses (the follow-up the 2026-08-16
entry asked for, now that `provider-bench --dump-ocr` exists). The "OCR found MRZ-shaped text
and got a character wrong" framing does not hold: **19** of the 71 are `*_no_mrz` specimens
with no MRZ at all (`mrz::find_and_parse` accepts VIZ boilerplate as a TD2/MRV-B), **25** are
`*_redacted_mrz` (unrecoverable by construction), and the remaining **27** have a real MRZ but
zero verified ground truth — several carry deliberately non-conforming template MRZs, so an
OCR misread cannot be told from a bad specimen.

**Step 1 shipped** (`mrz` 0.7.0, PR #238): `find_and_parse` returns `NotFound` when the
best-scoring non-validating reading has an unrecognized issuing state *and* nationality *and*
a non-numeric date of birth. Full-corpus re-run: `checksum_failed` **71 → 33**, `no_mrz_found`
**47 → 85**, Tier-1 HIT **118 → 118** (zero regression). 38 documents moved — the hallucinated
MRZ and most redacted specimens are now honestly not-found. Residual `checksum_failed` (33) is
23 genuine `*_mrz` + 8 partially-readable redacted + 2 stubborn no-MRZ.

**Step 3 shipped** (PR 1): all 23 genuine `*_mrz` now have a hand-transcribed
`samples/ocr_fixtures/<stem>.json` recording the true printed MRZ. **7 carry a checksum-valid
zone** (any `checksum_failed` on them is 100 % an OCR error — the step-4 target list), **16
are non-conforming by design** (`TEMPLATE` / `ÖRNEK` / all-zeros zones with wrong or blank
check digits). `corpus.jsonl` gains `ground_truth_stem` ×23 and `expected_document_number`
×7.

**Step 4 tooling shipped** (PR 2, repo `b3965f7`): `provider-bench --real-specimens` splits a
`checksum_failed` miss into `checksum_failed_specimen` (labelled specimen whose printed zone
native OCR recovered exactly — the check digits are non-conforming, not misread) vs plain
`checksum_failed`; `--dump-ocr` rows gain `ground_truth_mrz` + `zone_mismatch`. Full-corpus
re-run (`mrz`, 238 docs): HIT **120**, `checksum_failed` **32**, `checksum_failed_specimen`
**1** (Colombia `PP_COL_2026`), `no_mrz_found` **85** — a pure relabel. The other 22 labelled
specimens stay `checksum_failed` because native OCR does not reproduce their printed zone
(`zone_mismatch` 1–112); Afghanistan `P0_AFG_2016` is one character off.

**Step 2 shipped** (PR 3): `provider-bench` classifies a `*_redacted_mrz` specimen as
`redacted_mrz` ahead of the checksum gate and drops it from the Tier-1 hit-rate denominator —
a redaction bar carries no recoverable zone. Deterministic relabel off the step-4 run (repo
`b3965f7`): **9** move to `redacted_mrz` (8 from `checksum_failed`, plus Malaysia
`P0_MYS_2019_redacted_mrz_blur` which was scoring a HIT over its redacted zone),
`checksum_failed` **32 → 24**, HIT **120 → 119**, denominator **238 → 229**, Tier-1 hit rate
**50.4 % → 52.0 %**. The other 27 redacted specimens were already `no_mrz_found` and stay
there. Step 4 proper (character-level `mrz` fixes — start with Afghanistan) stands.

### 2026-09-04 — Tier-2 date misses were mostly the normalizer, not the model

Classifying the corrected parity run's per-field detail before starting
`VIZ_TIER2_DESIGN.md` §2.3 found that **14 of 70 date mismatches were the model
reading the printed date correctly while `normalize::date` discarded it** —
48.1% → 52.5% overall, deterministically. The same classification found only
**6 of 168 misses are `null`**, which undercuts §2.3's stated first lever
(telling the model which fields are still missing), and that the `mrz_hint`
built in issue #102 is unmeasurable on this corpus by construction. Full
writeup, including the 56 date misses that are genuine model errors:
[normalize-date-forms-2026-09-04.md](normalize-date-forms-2026-09-04.md).

### 2026-09-05 — demonyms, and a replay harness for vocabulary changes

The same per-field classification, applied to the non-date misses, **refuted two of three
candidates before any code was written**: `sex` misses carry no foreign-language words at all
(6 of 13 are the model answering `M` for `F` — a directional bias, not vocabulary), and
`document_type` misses are a representation disagreement between the fixture's 2-char MRZ code
and the normalizer's single letter. The one real gap was adjectival country forms —
`mrz::code_for_name` holds `Canada`, never `CANADIAN`. Six entries recovered **10 fields,
52.5% → 55.6%, zero regressions**:
[normalize-country-demonyms-2026-09-05.md](normalize-country-demonyms-2026-09-05.md).

That writeup also documents `vocab_replay`, which re-scores a finished parity log against the
current normalizers in under a second — and the accept rule it enforces (**≥1 miss→hit and 0
hit→miss**), whose useful side effect is that vocabulary no document exercises cannot enter the
codebase, whatever proposed it.

### 2026-09-09 — the denominator counted 94 documents that could never be read

Full writeup: [`denominator-correction-2026-09-09.md`](denominator-correction-2026-09-09.md).
Reconnaissance for [`ADR-0008`](../decisions/ADR-0008-mrz-detection-track.md)'s measurement, before
any OCR ran, found that the metric it targets was measuring something else. Of the 229 scored
specimens, **94 cannot produce a Tier-1 hit under any pipeline** — 42 carry no machine-readable
zone, 36 have it blacked out, 16 print a zone whose own check digits fail — and every one was
counted as a failure to produce one. Hit rate **52.0% → 82.6%** with the HIT count unchanged at
119 and no extraction code touched; the `no_mrz_found` target **85 → 18**.

Two of the three populations were classified by what OCR happened to return rather than by what the
document is. A redacted specimen was scored out only if its blackout bar OCR'd into parseable
noise, so **the cleaner the redaction, the worse it scored**; a non-conforming printed zone was
recognised only when OCR recovered it byte-for-byte, catching 1 of the 16 that the
[2026-09-08 entry above](#2026-09-08--checksum_failed-is-not-a-clean-ocr-accuracy-signal) had
already concluded "belong outside the denominator". The durable lesson: **ask what the document
makes possible before asking what the run achieved.**

Also found: a checksum-valid MRZ returned for a document carrying none was counted as a **Tier-1
HIT**, because such documents are unlabelled by construction and reached the ground-truth rung with
nothing to contradict them. Now `false_positive_mrz`, and a build failure. The corpus has zero.

### 2026-09-09 — the browser/native OCR gap, both arms measured the same day

Full writeup: [`ocr-stack-gap-2026-09-09.md`](ocr-stack-gap-2026-09-09.md).
[`ADR-0008`](../decisions/ADR-0008-mrz-detection-track.md) chunk 1, part 1. Its "64.2% vs 59.5%"
was stale on both sides — the browser figure had been superseded three times, and the native
figure was never a measurement in that run at all but a `samples/corpus.jsonl` field frozen on
2026-09-03 (**0 of 232 pre-existing rows changed** since). Re-measured today on 160 non-redacted
MRZ-bearing specimens: **browser 128 = 80.0%, native 119 = 74.4%**, gap +9 documents / +5.6 pp.
Both 110, browser-only 18, native-only 9.

**The gap is concentrated, not diffuse — and that is the finding.** Native has 25 in-denominator
misses; **the browser reads 17 of them.** Eleven are detection failures, the metric ADR-0008
targets. The other six are exactly the anchors the
[2026-09-08 entry](#2026-09-08--checksum_failed-is-not-a-clean-ocr-accuracy-signal) named as
carrying a checksum-valid printed zone — *"any `checksum_failed` is 100% an OCR error"* — and on
which it closed the `checksum_failed` track as OCR-quality-bound. Six of those seven are read
correctly by a different recognizer today, so **they are bound by *this* recognizer, not by OCR
quality in general.** Only Russia `P0_RUS_2019` is missed by both.

The two stacks also fail on *different* documents (native wins 9 — two `_blur`, one `_rotated`,
two China), so their union reads 137 of 160 = 85.6%, well above either.

Not yet attributed. Two of ADR-0008's four confounders turn out to be controlled already —
`synthpass-imageprep` compiles to wasm so both stacks run the same Rust preprocessing, and
`MRZ_CHARSET` constrains both recognizers. The cheapest untested cell: the browser's *first*
attempt is an untreated band crop that wins ~88% of its reads, while native runs the same
`plain_band` function as its second-to-last pass, on an assertion in two call sites that `ocrs`
"gains nothing" from an untreated pass that had never been measured.

### 2026-09-10 — pass ordering (ADR-0008 chunk 1C, cell a)

Full writeup: [`ocr-order-band-first-2026-09-10.md`](ocr-order-band-first-2026-09-10.md).
`SYNTHPASS_OCR_ORDER=band-first` moves `plain_band` to the first retry pass and measures.
**The assertion holds:** `plain_band` tried first recovers **zero** documents for `ocrs` on its own
merit (the 3 that validate on it under `band-first` all validated on the general pass under
`default` — run-to-run inference noise, not `plain_band` wins). The one real change is +1 hit
(`Belgium_ID_2021_back`, an O/0-confusable recognition miss), against 13 documents that pay ~2–3 s
for a wasted pass and a `control` arm that does not track `default`. So the browser's
untreated-band-first advantage is a property of the **OCR-B recognizer**, not of the ordering —
which rules variant ordering out as an explanation for the gap and points at the recognizer
(cell c). `SYNTHPASS_OCR_ORDER` ships default `default`; production behaviour is unchanged.

### 2026-09-10 — the gap is a text-*detection* gap (ADR-0008 chunk 1C, cell b)

Full writeup: [`ocr-gap-is-detection-2026-09-10.md`](ocr-gap-is-detection-2026-09-10.md).
`--dump-ocr` (widened to `no_mrz_found` in [#257](https://github.com/ruledicaprio/SynthPass/pull/257))
records `mrz_band_score` and the full OCR text per in-denominator miss. The 25 split: **~11 are OCR
*detection* failures** — `ocrs`/`rten` returns a few dozen stray characters for the *entire page*
(`Canada_PP_CAN_2023`'s complete output is 17 characters of noise) on clean passport images the
user hand-read without difficulty, and these are exactly the ones tesseract.js recovers. ~8 are
recognition misses (band found, one wrong character — mostly O/0 confusables — or the zone drowned
in page text). ~6 are specimen artifacts (X-redacted, novelty, all-zeros) that belong off the
denominator. So the research note's *localization vs recognizer* framing misses the point: the
dominant factor is upstream of both — `ocrs` is not detecting text on these images at all. §8's
"scale and binarization" is the likely lever, with a pure-Rust fix.
