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
| **Tier-1 hit rate, real specimens** | **140 / 152 = 92.1%** on documents that can yield a hit | `real-specimen-mrz-baseline.json` (CI, 2026-09-19) |
| Tier-1 hit rate, whole specimen corpus | 140 / 261 = 53.6% | same baseline; the gap is explained below |
| Strict name hit rate, real specimens | **12 / 45 = 26.7%** of name-scorable scored documents (36.4% of name-scorable hits) | same baseline (CI, 2026-09-19); ADR-0013 |
| False accepts (a checksum-valid MRZ returned for a document that carries none) | **0 / 109** | same baseline (CI, 2026-09-19) |
| Tier-1 hit rate, synthetic clean (100 docs per format, seed 0) | 377 / 500 = 75.4% (Derived) — TD3 78%, TD2 73%, TD1 54%, MRV-A 85%, MRV-B 87% (Observed) | Observed in CI: `bench-charts.yml` run 34831258506, 2026-09-14, MAIN `9c8f03d`, `synthpass-bench --document-type <fmt> --profile clean --count 100 --seed 0` (generated corpus, no `samples-data` input); re-observed through the registered `mrz` provider (`provider-bench --mrz-only`, local, MAIN `c617254`) with identical per-seed outcomes — [`m6-per-format-harness-comparison-2026-09-16.md`](m6-per-format-harness-comparison-2026-09-16.md) |
| Tier-2 per-field exact match, 72-fixture parity corpus | 55.6% overall (58.6% reviewed / 52.5% derived) | `crates/synthpass-llm/tests/parity.rs` |
| Browser OCR (tesseract.js) vs native (`ocrs`/`rten`) | **140 vs 140** over the 154 scored documents — a tie on count, 8 documents each way, 6 missed by both. On the browser report's own MRZ-bearing axis (212, which includes 58 documents no pipeline can hit) 144 vs 142 checksum-valid. The browser's 140 is checksum-validity, native's is a Tier-1 hit | Observed in CI: `web-ocr.yml` run 35169813105, 2026-09-17, tag `v1.5.0`, MAIN `b2a0afd`, DATA `469a4ee` — [`phase-d-native-vs-browser-2026-09-18.md`](phase-d-native-vs-browser-2026-09-18.md); supersedes the 2026-09-09 cut ([`ocr-stack-gap-2026-09-09.md`](ocr-stack-gap-2026-09-09.md)) |
| Names, browser vs native, on documents both read validly | **28 / 31 vs 11 / 31** exact on both name fields (reviewed fixtures only) — 17 browser-right/native-wrong, 0 the other way | same run — [`phase-d-native-vs-browser-2026-09-18.md`](phase-d-native-vs-browser-2026-09-18.md) |

**Why two rates.** 109 of the 261 specimens cannot produce a Tier-1 hit under any pipeline, so
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
| `no_mrz_found` | 2 | yes | No MRZ located on a document that has one — behind `checksum_failed` since 2026-09-13; two real detection targets since the San Marino template (2026-09-17) and Moldova PA 2014 (2026-09-19) left the bucket, each reclassified as carrying no zone |
| `checksum_failed` | **10** | yes | Conforming printed zone, read wrong — a genuine OCR error. The larger scored miss since 2026-09-13, and 3.3× the other since the c03/c07/c09 cohort |
| `false_positive_mrz` | 0 | yes | A checksum-valid MRZ returned for a document carrying none. **Any non-zero value here fails the build** |
| `no_mrz_expected` | 51 | no | Document carries no MRZ at all; none was read. A correct refusal |
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
and none from a document already in the corpus. Fourteen scored misses stood until the San Marino relabel; thirteen do now, **10 recognition
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

**Amended 2026-09-18 (Phase D, `web-ocr.yml` run 35169813105).** The +5.6 pp gap above is the
2026-09-09 cut and is no longer current: re-measured on one population at `v1.5.0`, the two arms
tie at 140 over the 154 scored documents, and the disagreement is 8 documents each way rather than
a one-sided advantage. What survives, and sharpens, is "concentrated in a named handful" — plus a
new axis the 2026-09-09 cut did not measure: on the 31 documents both stacks read checksum-valid
with a reviewed fixture, the browser gets both names exact on 28 against native's 11, with zero
documents going the other way. Both are in
[`phase-d-native-vs-browser-2026-09-18.md`](phase-d-native-vs-browser-2026-09-18.md), which also
records that four of the browser's extra checksum-valid reads land on documents outside every
scored population.

## What belongs here

The pipeline itself — every harness, gate, data branch, stored artifact and document, with its
owner and cadence, what it can honestly claim, and the ordered rework plan — is mapped in
[`PIPELINE.md`](PIPELINE.md). This file stays the home of the live numbers and the dated findings.

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

  The real-specimen strict rate becomes Observed, not Derived, once a CI-written
  `real-specimen-mrz-baseline.json` carries the `strict_names` counts (report-only — see
  ADR-0013); the row above states no real-specimen figure until then.
- **Dated sweeps** — `routing-sweep-YYYY-MM-DD.md`, `provider-comparison-*.md`.
  Name the exact invocation that produced them.

## Benchmark maintenance contract

A benchmark change follows this lifecycle:

`freeze → reconcile → measure → localize → change → regress → inspect → record`

**Freeze.** Pin the exact MAIN revision, the exact `samples-data` revision, the provider and model configuration, the invocation, and the population selection before interpreting a result. A moving branch is not benchmark provenance. A baseline is current only for the population and provider configuration that produced it — as of PR-1.4, `provider-bench --write-baseline`/`--assert-baseline` enforce this mechanically: they refuse to run unless every `SYNTHPASS_OCR_*` measurement arm (`synthpass_ocr::OcrArms::is_default`) is at its default, rather than relying on whoever invokes them to remember.

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
`crates/synthpass-imageprep/src/geometry.rs` for the pattern.

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

**The outcome ledger.** `real-specimen-outcomes.jsonl`, written alongside the baseline in the
same directory, holds one JSON row per document (`asset_id`, `name`, `outcome`, the full
`miss_reason`, `mrz_format`, `mrz_found`/`mrz_checksums_valid`, `names_exact`/`name_error`,
`ocr_ms`, and the native-retry fields) — the per-document evidence the aggregate counts above
are built from. It exists because the `real-specimen-gate-report` CI artifact is uploaded with
no `retention-days`, so a dated finding derived from `--verbose` output or the JSONL report
becomes unverifiable once GitHub's default retention window passes; the ledger is committed, so
it does not expire. Its SHA-256 is pinned in the baseline's `outcomes_sha256`, and
`--assert-baseline` fails the gate if the committed ledger no longer hashes to that value —
an edited-by-hand or substituted ledger is caught the same way a hand-edited baseline count
would be. When a committed ledger is present, an assert run also prints an informational
(never gate-failing) per-document diff against the committed one, so a reviewer can see exactly
which documents' outcomes moved without downloading and diffing two CI artifacts by hand.

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
baseline (and the outcome ledger next to it, when the downloaded artifact carries
one), rewriting README.md's gap sentence and this section's live block (including
the False accepts row once a baseline records `refusal_population`), and
appending a templated `## Weak-spot findings` entry to
[`FINDINGS.md`](FINDINGS.md) with its generated index regenerated in the same
step — is now
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

## Findings

The dated log of measurements, sweeps and rejected candidates lives in
[`FINDINGS.md`](FINDINGS.md), with a generated index at the top of that file. Every dated
report file stays right here in this directory and carries the header standard
`FINDINGS.md` documents. A new finding is appended to `FINDINGS.md`'s `## Weak-spot
findings` section -- by `tools/rebless.py` for a re-bless, by hand otherwise -- and the
index is regenerated with `python tools/index_findings.py --write`.
