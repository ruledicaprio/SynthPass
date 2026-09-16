# benchmarks/

Methodology, metric definitions, and dated results — **including the candidates
that were rejected.**

Principle 6: a constant with no measurement behind it does not ship.

## Current headline numbers

**This section is the only place in the repo that carries live accuracy numbers.** Every other
document — `README.md`, `ROADMAP.md`, an ADR — states at most one *live* figure and links here.
A dated measurement recorded inside an ADR as the evidence its decision rests on is not a live
figure and is never updated: ADRs are amended, not rewritten, and a decision whose evidence is
edited out cannot be audited. That rule
exists because it was broken: the same numbers were restated in four documents, and `README.md`
spent a release cycle advertising a hit rate ten points low while naming the wrong dominant miss.

The machine-readable source is [`real-specimen-mrz-baseline.json`](real-specimen-mrz-baseline.json),
written only by CI (`gh workflow run real-specimen-gate.yml -f mode=write-baseline`) and enforced
on every PR by [`real-specimen-gate.yml`](../../.github/workflows/real-specimen-gate.yml).
`scripts/check-headline-numbers.sh` fails the build if `README.md` disagrees with it.

| Metric | Value | Source |
| --- | --- | --- |
| **Tier-1 hit rate, real specimens** | **140 / 154 = 90.9%** on documents that can yield a hit | `real-specimen-mrz-baseline.json` (CI, 2026-09-16) |
| Tier-1 hit rate, whole specimen corpus | 140 / 261 = 53.6% | same baseline; the gap is explained below |
| Tier-1 hit rate, synthetic clean (100 docs per format, seed 0) | 377 / 500 = 75.4% (Derived) — TD3 78%, TD2 73%, TD1 54%, MRV-A 85%, MRV-B 87% (Observed) | Observed in CI: `bench-charts.yml` run 34831258506, 2026-09-14, MAIN `9c8f03d`, `synthpass-bench --document-type <fmt> --profile clean --count 100 --seed 0` (generated corpus, no `samples-data` input); re-observed through the registered `mrz` provider (`provider-bench --mrz-only`, local, MAIN `c617254`) with identical per-seed outcomes — [`m6-per-format-harness-comparison-2026-09-16.md`](m6-per-format-harness-comparison-2026-09-16.md) |
| Tier-2 per-field exact match, 72-fixture parity corpus | 55.6% overall (58.6% reviewed / 52.5% derived) | `crates/synthpass-llm/tests/parity.rs` |
| Browser OCR (tesseract.js) vs native (`ocrs`/`rten`) | **80.0% vs 74.4%** on 160 non-redacted MRZ-bearing specimens, both arms measured 2026-09-09 — **before** ADR-0008 chunk 2 moved the native arm; not re-cut since | [`ocr-stack-gap-2026-09-09.md`](ocr-stack-gap-2026-09-09.md) |

**Why two rates.** 107 of the 261 specimens cannot produce a Tier-1 hit under any pipeline, so
counting them as failures measures the corpus rather than the reader. They are scored out, and both
numbers are published so neither can be accused of flattering by exclusion: the first says how often
extraction succeeds when success is possible, the second what a pile of real documents yields. Only
the first moves when accuracy work lands. Full analysis:
[`denominator-correction-2026-09-09.md`](denominator-correction-2026-09-09.md) (the original 94),
[`denominator-bucket-a-2026-09-10.md`](denominator-bucket-a-2026-09-10.md) (the Argentina 2026 pair),
[`manifest-review-no-mrz-found-2026-09-13.md`](manifest-review-no-mrz-found-2026-09-13.md) (the
Swedish card front, the Dutch licence, Egypt 2012's masked zone and Argentina 2021 child's
non-conforming one).

**Real-specimen outcomes** (261 documents):

| Outcome | Count | In the denominator? | Meaning |
| --- | --- | --- | --- |
| **Tier-1 HIT** | **140** | numerator | Checksum-valid MRZ, document number matches ground truth |
| `no_mrz_found` | 4 | yes | No MRZ located on a document that has one — behind `checksum_failed` since 2026-09-13 |
| `checksum_failed` | **10** | yes | Conforming printed zone, read wrong — a genuine OCR error. The larger scored miss since 2026-09-13, and 2.5× the other since the c03/c07/c09 cohort |
| `false_positive_mrz` | 0 | yes | A checksum-valid MRZ returned for a document carrying none. **Any non-zero value here fails the build** |
| `no_mrz_expected` | 49 | no | Document carries no MRZ at all; none was read. A correct refusal |
| `redacted_mrz` | 39 | no | Zone blacked out by whoever published the specimen |
| `checksum_failed_specimen` | 19 | no | Printed zone fails its own ICAO check digits — a byte-perfect read still fails |

`no_mrz_found` overtook `checksum_failed` when `mrz` 0.7.0 began rejecting structurally implausible
readings, and stayed ahead through the denominator corrections and the 2026-09-10 specimen ingest
(2.6:1, 21 against 8). ADR-0008 chunk 2's orientation fix brought the two level at 7 each on
2026-09-11 ([`orientation-fix-2026-09-12.md`](orientation-fix-2026-09-12.md)). On 2026-09-13 a
[manifest review](manifest-review-no-mrz-found-2026-09-13.md) and the reclassification that
followed found four of those seven could never have yielded a hit: a Swedish card front tagged
`_mrz` by mistake and a Dutch driving licence whose one machine-readable line is not ICAO 9303 (both
now `no_mrz_expected`), an Egyptian specimen whose zone the publisher pixel-masked (`redacted_mrz`),
and an Argentine child passport whose printed zone fails four of its own five check digits
(`checksum_failed_specimen`). That left **3 against 7**, with zero extraction change: recognition, not
detection, is now the larger scored miss. The ten that remain are France ID 2020 back, Italy ID 2022
back and Moldova `PA_MDA_2014` for detection, and the seven `checksum_failed` named in
[`orientation-fix-2026-09-12.md`](orientation-fix-2026-09-12.md).

Cohort c01 made `no_mrz_found` read 4 on 2026-09-14 without adding a detection target — the fourth
is a blank San Marino template with nothing printed in its zone to find; see the 2026-09-14 entry.

The c03/c07/c09 cohort then took `checksum_failed` 7 → **10** later the same day, every one of the
three from a newly ingested book (Germany `P0_D00_2024`, Hong Kong `P0_HKG_2019` and `P0_HKG_2007`)
and none from a document already in the corpus. Fourteen scored misses now stand, **10 recognition
against 4 detection**, of which 3 are real detection targets — and none of the three new
recognition misses has a hand-transcribed fixture, so whether their printed zones conform is
unmeasured. See the second 2026-09-14 entry.

What that means for the track is in
[`ADR-0008`](../decisions/ADR-0008-mrz-detection-track.md)'s 2026-09-12 amendment.

**The browser-vs-native row replaces ADR-0008's "64.2% vs 59.5%"**, neither side of which was
current: the browser figure was the first of four measurements in `WEB_OCR_BASELINE.md` and had
been superseded three times, and the native figure was a `samples/corpus.jsonl` field last
recomputed on 2026-09-03 — unchanged through `mrz` 0.7.0, 0.7.1, `geometry_band_variants` and the
`Lanczos3`/`Triangle` decision. Both arms are now measured the same day. The gap survives at
+5.6 pp and, more usefully, **is concentrated in a named handful**: as first measured on 2026-09-09
the browser read 11 of native's 18 detection failures and 6 of its 7 `checksum_failed` (the
detection count is 17 after the Argentina 2026 pair was scored out; see the outcomes table above).

## What belongs here

- **Methodology** — how a run is configured so two runs are comparable. The
  corpus runner is deliberately single-threaded; a parallelised version measured
  38% where the honest sequential number was 55–56%, because oversubscribing
  `rten`'s internal threads corrupts the OCR retry budget. That is a measurement
  invariant, not a style preference.
- **Metric definitions** — precisely what each number counts. "Hit rate" means
  *the MRZ provider produced a checksum-valid record whose document number matches
  ground truth*, not "the extraction was correct."
- **Strict name hit rate** — two related, differently-denominated numbers, named for what
  they divide by (the benchmark maintenance contract is explicit that two similarly-named
  rates over different denominators is exactly the trap to avoid):
  - `strict_tier1_hit_rate`: `strict_hits / name_scorable_documents`, where a document is
    "name-scorable" when its ground truth carries both `surname` and `given_names`, and the
    denominator's population is every name-scorable document inside `tier1_hit_rate`'s own
    scored population (so a labelled miss, or a labelled document whose read errored, still
    counts in the denominator — just not as a strict hit). `synthpass-bench` reports it as
    `strict_hits`/`strict_hit_rate` (denominator: `count`, since every synthetic document is
    name-scorable by construction); `provider-bench` reports it as `strict_tier1_hit_rate`
    (`Computed { strict_hits, name_scorable_documents, name_scorable_hits,
    strict_tier1_hit_rate, names_exact_among_hits }` or `NotApplicable { reason }` when no
    document in the scored population is name-scorable — never a fabricated `0.0`).
  - `names_exact_among_hits`: `strict_hits / name_scorable_hits`, the same count restricted to
    documents that are *also* a Tier-1 hit — isolating the name-read question from detection
    accuracy. Both harnesses report this field explicitly (`synthpass-bench`'s `Report` JSON,
    `provider-bench`'s `StrictNameHitRate::Computed`).

  Neither number redefines `hit`/`hit_rate`/`tier1_hit_rate`: no ICAO 9303 check digit covers
  either name field in any MRZ format, so a checksum-valid Tier-1 hit can still carry a wrong
  name. Each miss is classified as `separator_lost`, `split_shifted`, `filler_read_as_letters`,
  or `other` (`synthpass_bench::NameError`) — the first three name specific CTC-decoder
  artifacts around the MRZ `<<`/`<` filler/separator character (a repeated glyph a CTC collapse
  step can lose or under-collapse), not an ordinary character misread. Why names are scored
  only against MRZ-form truth, and why they stay out of `hit`:
  [`ADR-0013`](../decisions/ADR-0013-names-are-scored-against-mrz-form-truth.md).

  **Derived** (per the maintenance contract — this metric has not itself been run in CI yet,
  which would be Observed) from the 2026-09-16 harness-comparison run's `synthpass-bench`
  reports (MAIN `c617254`, local, seed 0, 100 clean documents/format —
  [`m6-per-format-harness-comparison-2026-09-16.md`](m6-per-format-harness-comparison-2026-09-16.md)),
  by checking each Tier-1 hit's per-field CER for `surname`/`given_names` > 0: of 377 Tier-1
  hits across the five synthetic formats, **178 carry a wrong name** — TD3 39/78, TD2 37/73,
  TD1 39/54, MRV-A 35/85, MRV-B 28/87.
- **Dated sweeps** — `routing-sweep-YYYY-MM-DD.md`, `provider-comparison-*.md`.
  Name the exact invocation that produced them.

## Benchmark maintenance contract

A benchmark change follows this lifecycle:

`freeze → reconcile → measure → localize → change → regress → inspect → record`

**Freeze.** Pin the exact MAIN revision, the exact `samples-data` revision, the provider and model configuration, the invocation, and the population selection before interpreting a result. A moving branch is not benchmark provenance. A baseline is current only for the population and provider configuration that produced it.

**Reconcile.** Run the identity auditor before reading accuracy numbers. The candidate asset set must reconcile with the manifest: missing assets, unlisted assets, SHA mismatches, same-path byte conflicts, and duplicate encoded-byte groups are structural findings, not OCR results. Preserve whether each asset came from `samples-data`, MAIN/fixtures, or both. Keep these identities distinct:

- **Asset/capture identity:** the manifest path and encoded bytes that the run consumed. A filename, stem, or SHA alone is not a physical-document identifier.
- **Logical specimen identity:** an explicitly documented specimen record when the repository provides one. Do not infer it from country, year, document number, filename similarity, or byte equality.
- **Physical-document identity:** a human-reviewed grouping across captures or encodings. Do not deduplicate the benchmark by physical-document identity until that policy and its specimen IDs are explicit.

**Measure.** Use the repository-supported CI workflow against the frozen revisions. Do not replace a run with local arithmetic. Record the full candidate population, scored denominator, hit count, outcome buckets, decode/preparation failures, skips, and provider configuration. `no_mrz_expected`, `redacted_mrz`, and `checksum_failed_specimen` are excluded from the Tier-1 denominator when the run classifies them that way; they are not OCR misses. Historical measurements remain historical, current measurements are observed from a real run, and arithmetic projections are hypotheses until measured.

**Localize.** Diagnose each divergence at the earliest supported stage: decode, orientation/geometry, MRZ-band detection, preprocessing, OCR recognition, candidate detection, normalization/repair, deterministic parse, checksum validation, or classification. A failed hit is not automatically an OCR failure. The deterministic acceptance boundary remains a checksum-valid MRZ with the expected document number; do not weaken parsing or checksums to raise a count.

**Change and regress.** Keep corpus maintenance separate from accuracy optimization. Change one measurable hypothesis at a time, then rerun the same frozen population with the same provider configuration. Provider comparisons are meaningful only when both providers consume the same assets, labels, exclusions, and invocation scope.

**Inspect.** Review both recovered cases and newly broken cases, including changes in outcome buckets and denominator membership. A net hit-count improvement does not establish a safe change if it moves failures between stages or breaks previously passing specimens.

**Record.** Re-bless a baseline deliberately from the validated CI artifact, in a separate reviewable change. The live block above must agree with that artifact; preserve prior baselines and historical reports rather than rewriting them. Label evidence as **Observed** (directly measured), **Derived** (calculated from an observed run), or **Hypothesized** (a prediction or proposed explanation). Every recorded result should include the MAIN SHA, `samples-data` SHA, workflow/run identifier, date, command, provider/model configuration, candidate and scored populations, hit count, outcome buckets, and any skips or preparation failures.

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
{"run_timestamp_unix": 1785565270, "git_sha": "abcd123", "samples_data_sha": "469a4ee...", "invocation": "provider-bench --real-specimens --format passport --verbose --out ... --limit 30", "documents": 30, "provider_id": "mrz", "read_ok_rate": 0.93, "labelled_documents": 8, "field_match_rate": 0.92, "mean_cer": 0.03, "unsupported_assertion_rate": 0.25, "mean_ms": 3}
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

`baseline.samples_data_sha` is the authoritative corpus pin. Before the gate or
real-specimen chart tracks materialize images, CI resolves that exact commit,
runs `tools/audit_benchmark_identity.py --check`, and refuses to fall back to a
moving `origin/samples-data` tip. The identity report is uploaded with the run.
A deliberate write-baseline against a newer corpus may pass an explicit
`data_ref`; assert-mode runs cannot override the committed pin. Synthetic
`bench-data` collection is independent of `samples-data`: its `(seed, profile)`
identity guard remains the relevant check, while real-track history rows record
the pinned DATA SHA.

**What counts as a regression.** The gate fails if `tier1_hits` drops below
`baseline.tier1_hits - tolerance`, **or** if any of `checksum_failed`, `no_mrz_found`,
`ocr_error`, `document_number_mismatch` or `false_positive_mrz` exceeds its baseline value plus
`tolerance` (`REGRESSION_BUCKETS`, `crates/synthpass-bench/src/bin/provider-bench.rs`). Checking
the whole histogram, not just the headline HIT count, catches a change that
moves documents `checksum_failed → no_mrz_found` (or the reverse) while the net
HIT count stays flat — a real behaviour change worth a human looking at. The three
off-denominator populations — `redacted_mrz`, `no_mrz_expected` and
`checksum_failed_specimen` — are excluded; a change in any of them, or in `documents` or
`scored`, is reported as a **warning, not a failure** — the corpus grew or shrank and the
baseline needs regenerating, which is not a parser regression. *(Corrected 2026-09-14: this
paragraph listed `checksum_failed_specimen` as a failing bucket and omitted
`false_positive_mrz`, inverting both. `check_baseline` has always warned on the first and
failed on the second.)*

**`tolerance` is data, not code.** It lives in the JSON, starts at `0` (exact
ratchet), and is raised by a one-line reviewed change only if CI runs prove
flaky across runner hardware. Same reasoning as `m4-hit-rate`'s wide margin, but
tuned from measurement rather than guessed up front.

**Re-blessing (for PR authors).** A PR that legitimately moves the numbers —
a parser improvement, or adding/removing specimens — must regenerate the
baseline in the same PR. For a cohort that has pushed new images to
`samples-data`, `tools/rebless.py` resolves that branch tip and passes it as the
write-baseline run's exact `data_ref`; the assert-mode gate continues to use the
committed baseline pin. Until the new baseline, with its new DATA pin, is
committed to the cohort branch, the assert run is expected to remain red.
Download the artifact and commit the generated file only after reviewing the
measured population. This is the forcing function that keeps the committed
number honest.

**One command (`tools/rebless.py`).** Every step above, plus the parts that used
to be done by hand for every cohort PR — classifying the diff, installing the new
baseline, rewriting README.md's gap sentence and this section's live block, and
appending a templated `## Weak-spot findings` entry — is now
`python tools/rebless.py --cohort-branch <branch> --confirm`. It classifies the
diff into `identical` / `non-scored delta` / `scored delta` first: the first two
get the mechanical doc rewrite described above and are safe to commit, push,
re-assert and mark ready automatically; a **scored delta** (`tier1_hits` or a
scored miss bucket moved) always stops after installing the baseline and
printing the diff table, because that class needs `synthpass-analyst`'s prose,
not a template. `--dry-run` previews without dispatching anything; `--run-id`
reuses an already-completed `write-baseline` run instead of dispatching a new
one. See the tool's own docstring for the two independent safety gates.

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
"scale and binarization" is the likely lever, with a pure-Rust fix. *(Cell c, below, found the
lever is more specific still: native rotates all 11 of these documents 90° before OCR.)*

### 2026-09-10 — the detection failures are a wrong-orientation failure (ADR-0008 chunk 1C, cell c)

Full writeup: [`ocr-crop-recognizer-2026-09-10.md`](ocr-crop-recognizer-2026-09-10.md). New
`SYNTHPASS_OCR_DUMP_VARIANTS` dumps the exact preprocessed crops `ocrs` is fed;
`tests/web/recognize-crops.mjs` runs the browser's OCR-B model over them. **All 11 detection-failure
documents were auto-rotated 90°/270° by native's `choose_rotation`** (the 3 recognition controls
were not) — on these low-signal scans `ocrs` detects too little text for the rotation score to mean
anything, and a wrong 90° turn clears the 1.2× margin by chance. OCR-B fails on native's sideways
crop exactly as `ocrs` does; it reads the MRZ straight off 4 of 6 sampled *originals* at 0°, and the
browser reads Canada/Oman/Monaco on its first pass (`plain_band` + OCR-B, no rotation). `web/scan.js`
removed this same upfront-orientation-probe once (cost it 9 documents, 125→116). The recognizer is
not the variable — native's orientation handling is. Levers, all pure-Rust: gate `choose_rotation`
below a text-confidence floor; move 90°/270° into the retry chain as late band-crop variants; a
modest pre-detection upscale for the sub-300px documents.

### 2026-09-10 — the OCR-stack gap, attributed (ADR-0008 chunk 1, close-out)

Full writeup: [`ocr-stack-gap-attribution-2026-09-10.md`](ocr-stack-gap-attribution-2026-09-10.md).
The synthesis of 1A–1C, answering `Tesseract_OCR_studies.md`'s nine deliverables point by point:
the gap is **native page-orientation handling**, not the OCR-B recognizer, not the band search,
not retry ordering, and only secondarily scale. OCR-B provenance (deliverable 9) is **CONFIRMED**,
not UNKNOWN (`mrz.traineddata`, BSD-3 © DoubangoTelecom, OEM 1, `MRZ_CHARSET` whitelist, no PSM on
either side). [`ADR-0008`](../decisions/ADR-0008-mrz-detection-track.md)'s second amendment records
the three pure-Rust changes this licenses; building them is the next chunk.

### 2026-09-10 — 15 passport specimens ingested; baseline re-cut (ADR-0008 chunk 2, PR A)

The corpus grew by 16 documents (15 hand-transcribed passport books + one Kenya no-MRZ front),
so the CI baseline was regenerated: `documents` 238 → 254, `scored` 142 → 157, `tier1_hits`
118 → **128**, `no_mrz_found` 17 → 21, `checksum_failed` 7 → 8. No regression bucket for an
existing document grew — every count moved only by the new documents' own share. The scored
rate dips 83.1% → 81.5% (the 15 books were chosen to be hard) while the corpus rate rises
49.6% → 50.4%. Ten of the fifteen read today; the five misses are Angola and Pakistan
(photographed sideways), Dominican Republic and Nepal (low-signal scans), and India 2022
(`checksum_failed`) — the first four are exactly the chunk-2 orientation/detection targets.
The **browser-vs-native row above (160 specimens, 2026-09-09) predates this ingest** and is not
re-cut here — it is a frozen dated measurement; a fresh run belongs with chunk 2's own A/B.
Coverage: DJI, NGA, SOM, UZB become HIT, IND and IDN flip MISS → HIT, DOM and PAK land as MISS
([`CORPUS_COVERAGE.md`](../CORPUS_COVERAGE.md), 58 → 64 HIT codes).

### 2026-09-12 — the orientation fix: 128 → 143, and the two misses level (ADR-0008 chunk 2)

The CI baseline was re-blessed after #262 and #269: `tier1_hits` 128 → **143** of 157 (81.5% →
91.1%; corpus-wide 50.4% → 56.3%), `no_mrz_found` 21 → **7**, `checksum_failed` 8 → 7, and no
other bucket moved. Fifteen documents flipped, all toward HIT and none away. Fourteen came from
retiring the upfront orientation vote in favour of quarter-turns as the last retry tier, and one
(India 2022) from running both skew estimates. Every scoreable document chunk 1 named as an
orientation victim now reads. Three designs were measured and rejected on the way: a confidence
floor on the vote, the new skew estimator on its own, and a text-height gate on the band upscale.
The last fell to `ocrs`'s fixed model input geometry (800×600 detection, 64 px recognition lines).
Full record: [`orientation-fix-2026-09-12.md`](orientation-fix-2026-09-12.md).

**The baseline this replaces was a day stale, by design and at a cost.** The re-bless was deferred
to the end of the layer track, so from #262's merge the `tolerance: 0` gate compared against a
floor fourteen hits too low and would have passed a change undoing all of it. A deferred re-bless
disarms the gate for exactly as long as it is deferred.

### 2026-09-13 — two of the seven `no_mrz_found` documents have no ICAO zone to find

Full writeup: [`manifest-review-no-mrz-found-2026-09-13.md`](manifest-review-no-mrz-found-2026-09-13.md).
The manifest review the entry above asked for, on the four documents whose code and issuing state
the manifest does not record. Opening the images splits them two and two. **France ID 2020 back and
Italy CIE 2022 back print fully conforming TD1 zones** — all four check digits validate on each,
transcribed in the writeup — so both are genuine detection targets and stay scored. **Sweden ID 2027
is a card front with no zone at all** (its `_mrz` filename tag is simply wrong; the Swedish TD1 is on
the back), and **the Netherlands licence carries one 30-character line with no `<` fillers**, which
is no ICAO layout — TD1 is 3 × 30, TD2/MRV-B 2 × 36, TD3/MRV-A 2 × 44 — and satisfies the 7-3-1 rule
at no position tested. `CORPUS_COVERAGE.md` was right about the Netherlands and `samples/corpus.jsonl`
wrong. Scoring both out is modelled at `scored` 157 → 155, HIT unchanged at 143, **91.1% → 92.3%
scored and 56.3% corpus-wide unchanged**, `no_mrz_found` 7 → 5. Detection has not been level with
character accuracy since 2026-09-11; it has been **behind it, 5 : 7**.

**The Netherlands row was wrong because OCR wrote it.** `corpus_manifest.rs` derives
`mrz.present` as `claims.mrz_present.unwrap_or(observed.found)`, so a filename carrying neither
`_mrz` nor `_no_mrz` falls through to what the recognizer returned — here a non-validating `A<` /
`ADW` TD1 on a page with no ICAO zone — and `MrzExpectations::get` reads the manifest before the
filename. Three of 254 names are untagged; the two Bosnian licences are right only because `ocrs`
returned nothing on a GIF. Same class as the 2026-09-09 redaction and non-conformance bugs: **a
property of the document decided by what the run returned.** Both fixes are renames on
`samples-data`, since `present` is re-derived on every regeneration and a hand edit reverts.
**And the gate will not ask for the re-bless** — `tier1_hits` holds at 143, `no_mrz_found` only
falls, `documents` stays 254 — so merging without one leaves a `no_mrz_found` ceiling of 7 over a
true value of 5.

**Measured the same day, and the model held exactly.** Both files were renamed on `samples-data`
(`94a551b`) and the baseline re-blessed by CI in the same PR (`measured_on_ci_sha` `e1a4cae`):
`scored` 157 → **155**, `no_mrz_found` 7 → **5**, `no_mrz_expected` 43 → **45**, HIT **143**, no
other bucket moved and no document flipped. One correction to the paragraph above: the `A<` / `ADW`
read was the manifest's **stored** observation, reused because the image's `sha256` had not
changed since whatever pipeline version last read it. Renaming forced a fresh read, and the current
stack returns no MRZ on that licence at all — so `observed` in `samples/corpus.jsonl` is a dated
read, not a current one, and the false-positive exposure on this document is lower than stated.
The mechanism finding stands unchanged: `present` still fell back to `observed.found`.

**Then extended to four documents, and re-measured.** Two more of the seven turned out to be
unreadable and were folded into the same PR, re-verified from the images before acting: Egypt
`P0_EGY_2012`, whose zone lines are pixel-masked, renamed `_redacted_mrz` on `samples-data`; and
Argentina `P0_ARG_2021_mrz_child`, whose hand-transcribed fixture fails four of its five check
digits. This review had counted both as genuine detection targets, because it opened only the four
rows with no recorded code or state. CI re-blessed the result (`measured_on_ci_sha` `1b2e3bc`,
`samples-data` `da787de`): `scored` 155 → **153**, `no_mrz_found` 5 → **3**, `redacted_mrz` 36 → **37**,
`checksum_failed_specimen` 18 → **19**, HIT **143** — the model again, exactly.

### 2026-09-14 — cohort c01: three specimens, one HIT, and a filler template in `no_mrz_found`

The first cohort from the specimen-acquisition loop (three documents, `samples/corpus.jsonl`
254 → 257 rows), and the first corpus growth since the 2026-09-10 ingest. CI re-blessed in the
same PR — `gh workflow run real-specimen-gate.yml -f mode=write-baseline` on this branch, run
`34799341756`, green in 47m0s, `measured_on_ci_sha` `2a9bff5`, `samples_data_sha` `e991ecb`:
`documents` 254 → **257**, `scored` 153 → **155**, `tier1_hits` 143 → **144**, `no_mrz_expected`
45 → **46**, `no_mrz_found` 3 → **4**; `checksum_failed` (7), `checksum_failed_specimen` (19) and
`redacted_mrz` (37) unmoved.

**Split new corpus from prior corpus, because an ingest must never be able to hide a regression.**

| Population | Documents | Scored | HIT | Scored rate | Corpus rate |
| --- | --- | --- | --- | --- | --- |
| Prior corpus (2026-09-13 baseline) | 254 | 153 | 143 | 143 / 153 = 93.5% | 143 / 254 = 56.3% |
| New this cohort (c01) | 3 | 2 | 1 | 1 of 2 | 1 of 3 |
| **Whole corpus, 2026-09-14** | **257** | **155** | **144** | **144 / 155 = 92.9%** | **144 / 257 = 56.0%** |

The prior-corpus row is subtraction across the two CI baselines, not a second run: every bucket
moved by exactly the three new documents' own share (143 + 1 = 144 hits, 153 + 2 = 155 scored,
45 + 1 = 46 `no_mrz_expected`, 3 + 1 = 4 `no_mrz_found`), so no document already in the corpus
changed class. Both rates therefore fall by dilution alone — **93.5% → 92.9% scored, 56.3% →
56.0% corpus-wide** — because two of the three new documents cannot be hits and one of those two
is scored. No code changed in this PR. A three-document cohort has no rate worth quoting, which is
why its row reads "1 of 2" rather than 50%.

The three, and what each one's *role* makes possible, decided from the images before the run:

- **`Lithuania_Passport_Specimen_P0_LTU_2019_mrz.jpg` — HIT, scored.** A conforming TD3 book page;
  line 2 reads `00000000<0LTU9003118F29010204903111045<<<86`, and the document-number, date-of-
  birth, expiry and optional-data check digits each validate on that transcription. First LTU
  specimen and first LTU HIT ([`CORPUS_COVERAGE.md`](../CORPUS_COVERAGE.md), 73 → 74 HIT codes).
  CC BY-SA, a self-published Wikimedia Commons upload rather than the issuing authority —
  migracija.lrv.lt and adic.lrv.lt both refused the scout.
- **`Latvia_ID_Specimen_2021_front_no_mrz.png` — `no_mrz_expected`, not scored.** A TD1 card front.
  Doc 9303 puts the TD1 zone on the back, and only the front was sourced, so returning nothing is
  the correct answer and the document is scored out on its role, not on what OCR returned — the
  same call as the Sweden card front and Kenya's front-only passport page. Its licence is the one
  unresolved thing in this cohort: likumi.lv states bare copyright with no reuse grant found, and
  the document was included on the reviewer's verdict anyway. That is a denominator risk as well as
  a legal one — a withdrawal moves `documents` and `no_mrz_expected` and stales the baseline again.
- **`San_Marino_ID_Specimen_2017_back_mrz.jpg` — `no_mrz_found`, scored, and it should not be.**
  See below.

**The new `no_mrz_found` is not a detection target.** The San Marino back is a blank facsimile:
every data field on the card reads `000000`, the card is overprinted NON VALIDA PER L'ESPATRIO,
and the machine-readable zone is three lines of nothing but `<` fillers — a column projection over
the band counts **30 uniform glyph runs on each of the 3 lines**, TD1's exact 3 × 30 shape, with no
character in any check-digit position. OCR returning nothing parseable is the correct answer; no
pipeline change can make this document a hit. It belongs with `redacted_mrz` and
`checksum_failed_specimen`, where *the document* decides the outcome and not the run — the same
class of misattribution the [2026-09-09 denominator
correction](denominator-correction-2026-09-09.md) and the [2026-09-13 manifest
review](manifest-review-no-mrz-found-2026-09-13.md) each corrected once already. The mechanism is
one step upstream: `mrz.present` records that a zone is *printed*, and nothing in the manifest
distinguishes a printed zone from a printed zone that carries data. The cost is small and precise:
[ADR-0008](../decisions/ADR-0008-mrz-detection-track.md)'s target metric reads **4** while the
number of documents with a findable zone is still **3** — France ID 2020 back, Italy CIE 2022 back,
Moldova `PA_MDA_2014`. Reclassifying it is a `samples-data` rename plus its own CI re-bless, and is
modelled at `scored` 155 → 154, `no_mrz_found` 4 → 3, `no_mrz_expected` 46 → 47, HIT unchanged at
144 — **144 / 154 = 93.5% scored, 144 / 257 = 56.0% corpus-wide unchanged**. Modelled, not measured;
only CI writes the baseline. Left for the next cohort rather than folded into this data-only PR, to
avoid mixing two changes in one baseline.

**The gate would not have asked for this re-bless.** `--assert-baseline` fails only when
`tier1_hits` falls, so an ingest that adds hits passes against a stale baseline without complaint
(`documents` and `scored` are recorded, not enforced). An ingest PR is therefore exactly where a
deferred re-bless goes unnoticed — it was done in the same PR here, and that is the reason it has to
be.

**What this does not claim.** Not an accuracy result: no code, no threshold and no model moved, and
no A/B was run, so nothing here says anything about the pipeline. The prior-corpus row is arithmetic
across two CI baselines, not an independent re-measurement of the 254. Three documents carry no
statistical weight — the corpus rate moved 0.3 pp, which is smaller than the ~3-document run-to-run
spread `rten` shows locally. The Lithuania row has no `expected_document_number`, so its HIT rests on
check-digit validity plus a human's visual comparison against the printed zone, as it does for 113 of
the 155 scored documents; only 42 are cross-checked against a hand-verified document number. The
browser-vs-native row in this file is a frozen 2026-09-09 measurement over 160 specimens and was not
re-cut for this cohort.

### 2026-09-14 — cohorts c03/c07/c09: eight specimens, one HIT, and the first ingest the gate blocked

The second corpus growth of the day and the largest since 2026-09-10: eight documents from three
scout cycles, folded into one PR (`samples/corpus.jsonl` 257 → 265 rows). CI re-blessed on the
branch — `gh workflow run real-specimen-gate.yml -r cohort-c03 -f mode=write-baseline`, run
`34816030310`, green in 49m17s, `measured_on_ci_sha` `5e2aa67`, `samples_data_sha` `dc953e9`:
`documents` 257 → **265**, `scored` 155 → **159**, `tier1_hits` 144 → **145**, `checksum_failed`
7 → **10**, `no_mrz_expected` 46 → **50**; `no_mrz_found` (4), `checksum_failed_specimen` (19),
`redacted_mrz` (37) and `false_positive_mrz` (0) unmoved.

**Split new corpus from prior corpus, because an ingest must never be able to hide a regression.**

| Population | Documents | Scored | HIT | Scored rate | Corpus rate |
| --- | --- | --- | --- | --- | --- |
| Prior corpus (c01 baseline, same day) | 257 | 155 | 144 | 144 / 155 = 92.9% | 144 / 257 = 56.0% |
| New this PR (c03 + c07 + c09) | 8 | 4 | 1 | 1 of 4 | 1 of 8 |
| **Whole corpus, 2026-09-14 (second re-bless)** | **265** | **159** | **145** | **145 / 159 = 91.2%** | **145 / 265 = 54.7%** |

The prior-corpus row is subtraction across two CI baselines measured the same day, not a second
run. Every bucket moves by exactly the eight new documents' own share — 144 + 1 = 145 hits,
155 + 4 = 159 scored, 46 + 4 = 50 `no_mrz_expected`, 7 + 3 = 10 `checksum_failed` — and the four
buckets no new document touches are byte-identical, so **no document already in the corpus changed
class**. No code changed in this PR. Both rates therefore fall by dilution alone — **92.9% →
91.2% scored, 56.0% → 54.7% corpus-wide** — and the scored rate falls further than c01's did
because three of the four new scored documents are misses. An eight-document cohort has no rate
worth quoting, which is why its row reads "1 of 4".

The eight, and what each one's *role* makes possible, decided from the images before the run:

- **`Singapore_Passport_Specimen_PA_SGP_2017_mrz.jpg` — HIT, scored.** A TD3 book page whose
  document code is `PA`, not `P<`. Checksum-valid on the CI pass, on the manifest's own OCR read
  (`observed.checksums_valid: true`, `agrees: true`) and under `check_sample`. First SGP specimen
  and first SGP HIT ([`CORPUS_COVERAGE.md`](../CORPUS_COVERAGE.md), 74 → 75 HIT codes). Attributed
  to the Immigration & Checkpoints Authority; `origin.licence` is honestly `none-stated`, as for
  all four c09 rows.
- **`Germany_Passport_Specimen_P0_D00_2024_mrz.jpg` — `checksum_failed`, scored.** Issuing state
  `D<<`, the legacy single-letter code. The corpus already held a `D<<` book
  (`P0_D00_2018`) that fails the same way, so `D` now carries two checksum-failed passports and
  no HIT; `DEU` keeps its own HIT row.
- **`Hong_Kong_Passport_Specimen_P0_HKG_2019_mrz.png` and `..._P0_HKG_2007_mrz.png` —
  `checksum_failed`, scored, ×2.** First HKG specimens. Both find a TD3 zone and fail a check
  digit, so HKG stays *No specimen yet* in coverage terms.
- **`Germany_ID_Specimen_2024_front_no_mrz.jpg` — `no_mrz_expected`, not scored.** A TD1 card
  front; the `Personalausweis` zone is on the back and only the front was sourced. Same call as
  the Sweden and Latvia card fronts.
- **`South_Africa_ID_Specimen_2013_front_no_mrz.png` and `..._back_no_mrz.png` —
  `no_mrz_expected`, not scored, ×2.** The DHA smart ID card carries a chip and a 2D barcode and
  prints **no MRZ on either side**, so both sides are scored out on the card type, not on what OCR
  returned. The back is the useful precedent: a back is where a TD1 zone normally lives, and this
  one is still correctly a refusal.
- **`Philippines_ID_Specimen_XXXX_front_no_mrz.png` — `no_mrz_expected`, not scored.** PhilSys is
  a national ID, not a travel document, and carries no zone at all.

**This time the gate demanded the re-bless, and that is new.** The
[c01 entry above](#2026-09-14--cohort-c01-three-specimens-one-hit-and-a-filler-template-in-no_mrz_found)
noted that `--assert-baseline` fails only when `tier1_hits` falls, so an ingest that adds hits
slides past a stale baseline unnoticed. That is only half the rule: `checksum_failed` is a
`REGRESSION_BUCKETS` member, so this cohort's 7 → 10 exceeds `baseline + tolerance` with
`tolerance: 0` and fails the assert run outright — **an ingest of MRZ-bearing-but-unreadable
specimens is, to the gate, indistinguishable from a parser regression.** Two ingests on one day
therefore exercised both halves: c01 (hits only) needed author discipline, c03/c07/c09 (misses)
could not have been merged without a re-bless. The population split table above is what tells the
two apart; the gate cannot.

**Three new `checksum_failed` with no ground truth behind them.** None of Germany `P0_D00_2024`,
Hong Kong `P0_HKG_2019` or `P0_HKG_2007` has a `samples/ocr_fixtures/<stem>.json`, a
`ground_truth_stem` or an `expected_document_number`. That is exactly the gap the
[2026-09-08 entry](#2026-09-08--checksum_failed-is-not-a-clean-ocr-accuracy-signal) closed once
for the then-23 `checksum_failed` specimens: without a hand-transcribed zone, a genuine OCR error
(`checksum_failed`, in the denominator, an accuracy target) cannot be told from a non-conforming
printed zone (`checksum_failed_specimen`, off the denominator, nothing to fix).
`CORPUS_COVERAGE.md`'s new HKG and `D` rows both attribute these three to "the
single-character-misread pattern" — plausible, and **not measured**: no fixture exists to compare
against, and no `--dump-ocr` row was pulled for them. Until three transcriptions land, the
published scored rate of 91.2% is a floor that may be up to three documents pessimistic.

**What this does not claim.** Not an accuracy result: no code, threshold or model moved, and no
A/B was run, so nothing here says anything about the pipeline. The prior-corpus row is arithmetic
across two CI baselines, not an independent re-measurement of the 257. Eight documents carry no
statistical weight; the 1.7 pp scored-rate move is dilution by construction, not a behaviour
change. Singapore's HIT carries no `expected_document_number` — only 42 rows in the whole 265
do — so it rests on check-digit validity plus a human's visual comparison against the printed
zone. Whether the three new `checksum_failed` books are OCR errors or non-conforming specimens is
unknown, so their contribution to the denominator is provisional. The browser-vs-native row in
this file remains a frozen 2026-09-09 measurement over 160 specimens and was not re-cut.

**Deferred, not done.** The San Marino reclassification the c01 entry modelled (`scored` 155 →
154, `no_mrz_found` 4 → 3) was *not* folded in here either, so it is now two cohorts old and
ADR-0008's detection metric still reads 4 against a true 3. Transcribing the three new zones into
`samples/ocr_fixtures/` was also left out, to keep this PR data-only and its baseline delta
attributable to the ingest alone. Both are re-bless-bearing changes and should travel together in
the next corpus PR rather than one each.

### 2026-09-15 — cohorts c10/c12: one redacted bio page in the walk, six covers outside it

Seven specimens in one PR (`samples/corpus.jsonl` 265 → 272 rows) — and the baseline's `documents`
moves by **one**, not seven. Six of the seven are passport covers, and
[ADR-0012](../decisions/ADR-0012-cover-only-specimens-are-a-labelled-class.md)'s 2026-09-15
amendment put `samples/covers/` outside the default `provider-bench --real-specimens` walk, behind
`--include-covers`. That is the whole shape of this delta. CI re-blessed on the branch —
`real-specimen-gate.yml` dispatched on `cohort-c10` with `mode=write-baseline`, run `34980671424`,
green in 41m42s, `measured_on_ci_sha` `12b6858`, `samples_data_sha` `46b8473`: `documents` 265 →
**266**, `no_mrz_expected` 50 → **51**, and **every other count byte-identical** — `scored` 159,
`tier1_hits` 145, `checksum_failed` 10, `checksum_failed_specimen` 19, `no_mrz_found` 4,
`redacted_mrz` 37, `false_positive_mrz` 0.

**Split new corpus from prior corpus, because an ingest must never be able to hide a regression.**

| Population | Documents | Scored | HIT | Scored rate | Corpus rate |
| --- | --- | --- | --- | --- | --- |
| Prior corpus (c03/c07/c09 baseline, 2026-09-14) | 265 | 159 | 145 | 145 / 159 = 91.2% | 145 / 265 = 54.7% |
| New in the default walk (c10) | 1 | 0 | 0 | — | 0 of 1 |
| **Whole corpus, 2026-09-15** | **266** | **159** | **145** | **145 / 159 = 91.2%** | **145 / 266 = 54.5%** |
| Off the walk, opt-in only: `samples/covers/` (c12) | 6 | 0 | 0 | — | not counted |

The prior-corpus row is subtraction across two CI baselines, not a second run: the single in-walk
document lands off the denominator, so `scored` does not move and no document already in the corpus
changed class. This is the first corpus growth that leaves the **scored** population untouched —
**91.2% scored, unchanged** — while the corpus-wide rate falls 54.7% → **54.5%** by the dilution of
one document. No code changed in this PR.

The seven, and what each one's *role* makes possible, decided from the images before the run:

- **`Cambodia_Passport_Specimen_PN_KHM_XXXX_no_mrz.jpg` — `no_mrz_expected`, not scored.** A `PN`
  bio page, and the only specimen in this corpus that is a real person's document rather than an
  official blank — admitted under the revised H5 standard (2026-09-14) precisely because every
  personal field *including the MRZ band* is fully redacted. There is no zone left to find, so
  returning nothing is the correct answer and the document is scored out on its role, as the Sweden
  and Latvia card fronts are. `KHM` stays *No specimen yet*: this is document-type depth, not a
  coverage HIT.
- **Six passport covers — Panama, Guatemala, Honduras, Sri Lanka (2024 `P` series and the earlier
  `N` series) and Papua New Guinea — the first rows of `samples/covers/`, outside the walk.** A
  cover has no data page and no MRZ, so it can never be scored either way; ADR-0012's amendment
  keeps the class out of the default corpus rather than parking six permanent refusals in
  `no_mrz_expected` and quietly moving the corpus-wide rate. They are reachable only with
  `provider-bench --real-specimens --include-covers`, never in CI. Five `CORPUS_COVERAGE.md` rows
  gain a `Passport (cover)` Docs entry and none gains a status.

**The gate would not have asked for this re-bless.** `tier1_hits` held at 145 and no
`REGRESSION_BUCKETS` member moved, so the assert run passes against the 2026-09-14 baseline;
`documents` and `no_mrz_expected` are warn-only. Same half of the rule as
[c01](#2026-09-14--cohort-c01-three-specimens-one-hit-and-a-filler-template-in-no_mrz_found), and
the reason the re-bless was done in the same PR anyway. Provenance note: c12 was the first scout
cycle run under prefix v3 (official-host-only signal) and the first cohort judged by the
`synthpass-screener` agent.

**What this does not claim.** Not an accuracy result: no code, threshold or model moved, and no A/B
was run. The prior-corpus row is arithmetic across two CI baselines, not an independent
re-measurement of the 265. One in-walk document carries no statistical weight; the 0.2 pp
corpus-rate move is dilution by construction. The six covers are **unmeasured** by this baseline,
not measured-and-zero — nothing here says what the reader returns on them. The browser-vs-native
row in this file remains a frozen 2026-09-09 measurement over 160 specimens and was not re-cut.

**Still deferred.** The San Marino reclassification modelled in the c01 entry (`scored` 155 → 154,
`no_mrz_found` 4 → 3) is now three cohorts old, so ADR-0008's detection metric still reads 4
against a true 3; the three c03/c07/c09 `checksum_failed` books still have no hand-transcribed
`samples/ocr_fixtures/` zone.

### 2026-09-16 — no image in the default walk is cover-like, and OCR volume cannot find one

The plan's cover-like detector (T15) was run as an analyst report before anything moved: every
`no_mrz_expected` document of the default walk (51 of 266) was OCR'd for text volume from the exact
`samples-data` bytes behind the committed baseline, and the two most recent CI gate reports agree on
all 266 `miss_reason` values. The 22 images below the Kenya control's text volume (501 characters, a
redacted bio spread that must not be listed) were opened one by one: all are data faces — ID-card
fronts and backs, passport bio-data pages — and the two lowest-text images are a driving-licence
category table and a Swiss ID card face, both small scans of dense documents. Text volume ranks how
legible a scan is, not what the document is. **Relocation candidates: 0.** Both denominators are
unchanged: 145 / 159 = 91.2% scored, 145 / 266 = 54.5% corpus-wide. The label, not a detector, stays
the class boundary (ADR-0012).

Two things surfaced on the way. `United_Arab_Emirates_Passport_Specimen_P0_ARE_2018_no_mrz.png` and
`Vietnam_Passport_Specimen_P0_VNM_2022_no_mrz.jpg` do carry a zone (barred out and blurred
respectively), so their `_no_mrz` token is wrong: a checksum-valid read on the Vietnam page would
land in `false_positive_mrz` and fail the build, where `_redacted_mrz` would absorb it. The rename
is owed (`no_mrz_expected` 51 → 49, `redacted_mrz` 37 → 39, scored rate untouched). And three
specimens sit in the walk twice in two formats (Azerbaijan 2013, China 2012, Türkiye ID 2020), two
of them HIT twice, so de-duplicated the scored rate reads 143 / 157 = 91.1%; any per-document join
on `documents_detail` must not key by name. Full method, table and the control case:
[`cover-like-detector-2026-09-16.md`](cover-like-detector-2026-09-16.md).

### 2026-09-16 — provider-bench and synthpass-bench agree on every synthetic document

Before the per-format synthetic rates changed source to the provider path (M6, `ADR-0011`'s
amendment), both harnesses were run on the same inputs: five formats × 100 documents, seed 0,
profile `clean`, MAIN `c617254`, run locally. **Observed:** same hit count and same per-seed outcome
in every format, and the same per-format counts as CI (`bench-charts.yml` run 34831258506, MAIN
`9c8f03d`); **Derived:** 377 / 500 = 75.4% pooled. The published synthetic row it replaced was stale on
three counts: TD3/TD2/TD1 were the 2026-08-31 values, MRV-A 87% and MRV-B 93% were 30-document
figures in a column labelled 100-seed, and its "~55%" aggregate matched no run. Full table:
[`m6-per-format-harness-comparison-2026-09-16.md`](m6-per-format-harness-comparison-2026-09-16.md).
