# Neither headline moved for a reading reason: the 2026-09-19 re-bless is two denominator changes

**Date:** 2026-09-19 · **MAIN:** `9783d16` (baseline landed; `origin/main` was `d312f77` at writing) · **DATA:** `396b22f` (was `5988c7b`) · **Evidence:** Observed (CI `real-specimen-gate.yml` run 35441419567, `mode=write-baseline`, head `185e530`) · **Status:** current

The real-specimen baseline was re-blessed by CI and merged in #349. Two published rates moved in
opposite directions on the same run:

```
Tier-1, scored     140 / 153 = 91.5%   ->   140 / 152 = 92.1%    (+0.6 pp)
strict names        12 /  40 = 30.0%   ->    12 /  45 = 26.7%    (-3.3 pp)
```

**`tier1_hits` is 140 at both pins and no document's read changed.** The extraction path was not
touched between the two measurements. The first rate rose because one *correct refusal* left the
denominator; the second fell because five documents entered a different denominator with a
numerator that is structurally incapable of following them. Neither is progress, neither is
regression, and the one metric in this family that isolates name reading — `names_exact_among_hits`
— did not move at all.

## Provenance of both pins

| | Previous | This re-bless |
| :-- | :-- | :-- |
| CI run | `35232307784` (`write-baseline`) | `35441419567` (`write-baseline`, `workflow_dispatch`, success) |
| `measured_on_ci_sha` | `649cccc` (on `main`) | `185e530` (**not** an ancestor of `main`; squashed into `9783d16`) |
| `measured_date` | 2026-09-17 | 2026-09-19 |
| `samples_data_sha` | `5988c7b` | `396b22f` |
| landed as | `0a353ac` (#332) | `9783d16` (#349) |

Both Observed. `outcomes_sha256` in the committed baseline (`6b84366…`) re-hashes correctly against
the committed [`real-specimen-outcomes.jsonl`](real-specimen-outcomes.jsonl), so the per-document
ledger this report is derived from is the one CI wrote.

## Every count that moved

Observed, both columns read from the committed baseline JSON at `9783d16^` and `9783d16`.

| Field | Before | After | Δ | In the Tier-1 denominator? |
| :-- | --: | --: | --: | :-- |
| `documents` | 261 | 261 | 0 | — |
| `scored` | 153 | **152** | −1 | — |
| `refusal_population` | 108 | **109** | +1 | — |
| `tier1_hits` | 140 | 140 | 0 | numerator |
| `checksum_failed` | 10 | 10 | 0 | yes |
| `no_mrz_found` | 3 | **2** | −1 | yes |
| `false_positive_mrz` | 0 | 0 | 0 | yes — **any non-zero fails the build** |
| `ocr_error` | 0 | 0 | 0 | yes |
| `document_number_mismatch` | 0 | 0 | 0 | yes |
| `no_mrz_expected` | 50 | **51** | +1 | no |
| `redacted_mrz` | 39 | 39 | 0 | no |
| `checksum_failed_specimen` | 19 | 19 | 0 | no |
| `strict_names.strict_hits` | 12 | 12 | 0 | report-only |
| `strict_names.name_scorable_documents` | 40 | **45** | +5 | report-only |
| `strict_names.name_scorable_hits` | 33 | 33 | 0 | report-only |

Only two buckets moved, by one document each, and they are the same document moving between them.
Five report-only documents entered the name denominator. Nothing else.

## The arithmetic closes at both pins

`OFF_DENOMINATOR_KINDS` (`crates/synthpass-bench/src/bin/provider-bench.rs:909`) is exactly three
kinds — `redacted_mrz`, `no_mrz_expected`, `checksum_failed_specimen` — and
`RealSpecimenSnapshot::from_reports` sets `refusal_population` to their sum and `scored` to
`documents - off_denominator`. Both identities hold on both baselines (Derived from the committed
JSON):

```
before   39 + 50 + 19 = 108 = refusal_population      261 - 108 = 153 = scored
after    39 + 51 + 19 = 109 = refusal_population      261 - 109 = 152 = scored
```

And `scored` reconciles against the numerator plus the in-denominator misses:

```
before   140 + 10 + 3 + 0 + 0 + 0 = 153
after    140 + 10 + 2 + 0 + 0 + 0 = 152
```

So the published pair of denominators is internally consistent: 261 whole corpus, 152 scored, 109
scored out, with no document counted twice and none unaccounted for.

## Move 1 — Tier-1 rose because a correct refusal left the denominator

One document changed bucket. Moldova PA 2014 (the narrow crop) was renamed `_mrz` → `_no_mrz` on
`samples-data` `396b22f`; its strip is decorative artwork, not an ICAO zone (#349, and
[`detection-misses-2026-09-18.md`](detection-misses-2026-09-18.md) for the band-score evidence).
The ledger diff shows exactly this and nothing else (Observed):

```
- passports/Moldova_Passport_Specimen_PA_MDA_2014_mrz.jpeg       no_mrz_found
+ passports/Moldova_Passport_Specimen_PA_MDA_2014_no_mrz.jpeg    no_mrz_expected
```

Its `_wide` sibling is a different physical document, is unaffected, and remains a hit.

**The counterfactual that proves this is not progress** (Derived): had the file kept its `_mrz`
name at this same pin, with every other number identical, the rate would read `140 / 153 = 91.5%`
— the previous figure, unchanged. The reader found nothing it was not finding before; the corpus
stopped asking it for something that was never there. This is the third document in a row to leave
`no_mrz_found` by reclassification rather than by extraction (the Swedish card front and the Dutch
licence on 2026-09-13, the San Marino template on 2026-09-17, Moldova now), and
`no_mrz_found` is now **2**, both of them real detection targets: the France 2020 and Italy 2022
card backs.

## Move 2 — strict names fell because five documents entered a denominator the numerator cannot reach

`strict_tier1_hit_rate` is `strict_hits / name_scorable_documents`, and a document is
name-scorable when it is in the scored Tier-1 population **and** its ground truth carries both
`surname` and `given_names` (`crates/synthpass-bench/src/provider_bench.rs:2271`). `strict_hits`
additionally requires `miss_reason.is_none()` — a Tier-1 hit.

The five that entered, with the bucket each landed in (Observed, ledger diff `names_exact`
`null` → `false`):

| Document | Side / format | Bucket at this pin | Fields affected |
| :-- | :-- | :-- | :-- |
| France ID 2020 | card back, TD1 | `no_mrz_found` | both name fields now scorable |
| Italy ID 2022 | card back, TD1 | `no_mrz_found` | both name fields now scorable |
| Germany passport 2024 | bio-data page, TD3 | `checksum_failed` | both name fields now scorable |
| Hong Kong passport 2007 | bio-data page, TD3 | `checksum_failed` | both name fields now scorable |
| Hong Kong passport 2019 | bio-data page, TD3 | `checksum_failed` | both name fields now scorable |

**All five are scored misses.** A miss cannot be a strict hit by definition, so the numerator was
pinned at 12 before the denominator grew and pinned at 12 after it. The −3.3 pp is arithmetic on a
population change, not a name-reading result.

Independent second derivation, which is what makes the attribution safe rather than plausible: the
flat fixture directory holds 59 `.json` files at `649cccc` and 64 at `185e530` (Observed,
`git ls-tree`), and every one of the 64 carries both name fields. Bucketing the 64 against the
ledger gives `33 hit + 10 checksum_failed + 2 no_mrz_found + 19 checksum_failed_specimen = 64`, of
which the first three groups — the scored ones — total **45**, the baseline's
`name_scorable_documents` exactly. The previous pin's 59 gave `33 + 7 + 19 = 59`, totalling **40**.
The +5 is the fixture count, document for document.

### The ceiling fell; the reading did not

Because every added fixture landed on a miss, the *maximum attainable* strict rate at this pin fell
with the actual one (Derived):

```
before   best case 33 / 40 = 82.5%     actual 12 / 40 = 30.0%
after    best case 33 / 45 = 73.3%     actual 12 / 45 = 26.7%
```

The metric that is invariant to miss-side fixture coverage is `names_exact_among_hits` =
`strict_hits / name_scorable_hits`, and it is **12 / 33 = 36.4% at both pins** — identical
numerator and identical denominator. The per-document `name_error` histogram over the 21
name-scorable hits that carry a wrong name is byte-identical too: 18 `other`, 2 `split_shifted`,
1 `filler_read_as_letters`, before and after (Observed, ledger). No document's name outcome
changed on this run.

### Why `strict_hits` stayed 12 even though 19 fixtures were edited

#348 corrected hand-transcribed ground truth as well as adding the two card-back fixtures: 19
existing `.json` fixtures were modified between the two pins. Only **two** of those edits touched
`surname` or `given_names`, and both are on the Argentina 2026 pair, which sits in
`checksum_failed_specimen` and is therefore off the denominator entirely (Observed). So no
correction could have moved `strict_hits`, and none did. This matters because a moved
`strict_hits` on the same run would have made the two effects inseparable.

## The mechanism is fixture-by-stem, not the manifest's `ground_truth_stem`

The obvious attribution — Germany 2024 and Hong Kong 2007/2019 became name-scorable because #349
gave their `samples/corpus.jsonl` rows a `ground_truth_stem` — is **wrong**, and it is worth
recording because it is the attribution the diff invites.

`load_ground_truth` (`crates/synthpass-bench/src/lib.rs:508`) reads
`samples_root/ocr_fixtures/<image stem>.json` directly. `provider_bench` states outright that it
"never reads `samples/corpus.jsonl`" for this purpose
(`crates/synthpass-bench/src/provider_bench.rs:1516`); the manifest is consulted only for
`MrzExpectations` (`mrz.present` / `mrz.redacted`), which drives bucket classification, not
name-scorability. The proof is in the same diff: France 2020 back and Italy 2022 back have
`ground_truth_stem: null` in the manifest at **both** pins and still became name-scorable, purely
because their fixtures appeared.

What actually happened is a timing fact. All five fixtures are absent at `649cccc` and present at
`185e530` (Observed, `git ls-tree` at both shas):

- Germany 2024 and Hong Kong 2007/2019 were transcribed in **#336** (`daab97e`, 2026-09-17 22:54),
  which merged **after** the previous baseline was measured (`649cccc`, 2026-09-17 15:51). They
  have been name-scorable since #336; this is simply the first CI baseline taken with them in the
  tree.
- France 2020 back and Italy 2022 back were added in **#348** (`d17a3f6`, 2026-09-19).

#349's `ground_truth_stem` / `expected_document_number` additions are manifest hygiene — real, and
required for the manifest to stop contradicting the fixture directory — but they are not the cause
of the denominator move.

## Also observed: ground-truth coverage of the scored misses is now complete

| | Before | After |
| :-- | --: | --: |
| Scored misses | 13 | 12 |
| …carrying a reviewed fixture | 7 | **12** |
| Coverage | 53.8% | **100%** |

Every scored miss at this pin — all 10 `checksum_failed` and both `no_mrz_found` — now has
hand-transcribed ground truth. Every future flip in either bucket is attributable to a mechanism
rather than guessed at. That is the substantive result of this re-bless, and it is why the strict
rate fell: fixture-writing has been deliberately aimed at misses
([`denominator-t14-2026-09-16.md`](denominator-t14-2026-09-16.md) §1 for the three books, #348 for
the two card backs), so `strict_tier1_hit_rate` will fall again on the next attribution pass,
again without meaning anything about name reading.

A second small Observed consequence: France 2020 back and Italy 2022 back now report
`mrz_format: TD1` in the ledger despite being `no_mrz_found`. Format on a real specimen falls back
to `MrzFormat::guess_from_lines` over the ground-truth zone when the read resolves nothing
(`provider_bench.rs:1524`, `:1788`), so this is the fixture speaking, not a read. It is
nevertheless the first mechanical corroboration that both card backs carry a TD1 zone, which had
been a reviewer's claim.

## What this does not claim

- **It does not claim the pipeline improved.** `tier1_hits` is 140 at both pins, no document
  changed read outcome, and the extraction crates were untouched between `649cccc` and `185e530`.
- **It does not claim the pipeline regressed on names.** `names_exact_among_hits` (12 / 33) and the
  `name_error` histogram are identical at both pins.
- **It does not claim 92.1% is comparable to 91.5% as a time series.** They have different
  denominators over a different corpus pin. The comparable pair is `140 / 152` against a
  counterfactual `140 / 153` at the *same* pin, which is the same 140.
- **It does not claim the five newly scorable documents read their names wrongly.** Four of the
  five never produced a checksum-valid zone at all, or produced one that failed its check digits;
  `names_exact: false` on a miss records "not exact", not "the recognizer read the name and got it
  wrong". The name question is only meaningful inside `name_scorable_hits`.
- **It does not claim anything about the three transcribed books' conformance beyond what was
  already measured.** [`denominator-t14-2026-09-16.md`](denominator-t14-2026-09-16.md) §1 measured
  all three printed zones as conforming, all five check digits validating; that stands and is not
  re-derived here.
- **It says nothing about timing or cost.** No timing arm was run; the CI `ocr_ms` values in the
  ledger were not compared across pins, and would not be comparable across runner hardware anyway.

## Attributions rejected on the way

Recorded so they are not re-proposed, per this directory's "Record the rejections" discipline.

1. **"Germany 2024 and Hong Kong 2007/2019 became name-scorable because #349 added
   `ground_truth_stem` to their manifest rows."** Rejected: the harness resolves ground truth by
   image stem and does not read the manifest for this. Refuted directly by France/Italy, which have
   `ground_truth_stem: null` at both pins and became name-scorable anyway.
2. **"The strict-name drop is a name-reading regression, or a side effect of #348's transcription
   corrections."** Rejected: the numerator, `name_scorable_hits`, and the full `name_error`
   histogram are identical at both pins, and only two of the 19 corrected fixtures touched a name
   field — both on off-denominator documents.
3. **"`no_mrz_found` 3 → 2 is a detection win."** Rejected: no document was newly detected. The
   bucket shrank because a document with nothing to detect was reclassified, the third such
   reclassification in this bucket in seven days.
4. **"Both headline moves can be summarised as `documents` unchanged, so the corpus did not
   change."** Rejected: `documents` is the one count that *cannot* see a rename-in-place. `scored`,
   `refusal_population` and the fixture count all moved; the whole-corpus denominator staying at
   261 is what makes the rename invisible at the headline and is precisely why both denominators
   have to be published.

## Invocations

The measurement itself, dispatched against the cohort branch (not reproducible locally — local
`rten` inference differs from CI by float rounding, so a local count is never a baseline):

```
gh workflow run real-specimen-gate.yml -f mode=write-baseline
# run 35441419567, head 185e530c5c72f8a65710754e2f4d9eb3204f72f9, DATA 396b22f
# -> artifact real-specimen-mrz-baseline, committed as b7c345d, merged as 9783d16
```

Everything else in this report is read-only derivation from committed artifacts, all of which
outlive the CI artifact retention window:

```
git show 9783d16^:knowledge/benchmarks/real-specimen-mrz-baseline.json
git show 9783d16^:knowledge/benchmarks/real-specimen-outcomes.jsonl
git show 9783d16^:samples/corpus.jsonl
git ls-tree --name-only 649cccc samples/ocr_fixtures/
git ls-tree --name-only 185e530 samples/ocr_fixtures/
git diff --name-status 649cccc 185e530 -- samples/ocr_fixtures/
bash scripts/check-headline-numbers.sh
```

The ledger diff, the fixture bucketing and the `surname`/`given_names` comparison across the 19
modified fixtures were done with short `python` scripts over those outputs (`jq` is not installed
on this host).

## Stale text this re-bless leaves behind

[`README.md`](README.md)'s live block, `README.md` at the repository root and
[`ROADMAP.md`](../ROADMAP.md) all agree with the committed baseline —
`scripts/check-headline-numbers.sh` passes. Two prose passages in this directory's `README.md` do
not, and are outside what that script checks:

- The 2026-09-13 paragraph still lists Moldova `PA_MDA_2014` among the detection targets. There are
  two, and it is not one of them.
- The paragraph closing the c03/c07/c09 note reads "thirteen do now, **10 recognition against 4
  detection**, of which 3 are real detection targets — and none of the three new recognition misses
  has a hand-transcribed fixture, so whether their printed zones conform is unmeasured." At this
  pin there are **12** scored misses, 10 recognition against **2** detection, both real; and all
  three of those books have carried a hand-transcribed fixture since #336, with their zones
  measured as conforming on 2026-09-16.

Both are prose corrections to a dated narrative, not headline figures, and are left for a separate
change rather than folded in here.
