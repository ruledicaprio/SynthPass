# The chargrid A/B, reconciled: +30 and +33 are two metrics of one run, and all 13 regressions were synthetic

**Date:** 2026-09-24 · **MAIN:** `e6fa10c` (the A/B binary was built from `daab97e`) · **DATA:** not recorded by the A/B (a local 257-document `samples/` tree, drifted from `samples-data`) · **Evidence:** Observed (per-document transitions re-derived from the 30 retained A/B reports; no cargo, no benchmark run) · **Status:** current

**2026-09-24.** Two reviews found that three committed documents describe the 2026-09-18 chargrid
A/B (the fixed-grid name-line repair behind `SYNTHPASS_OCR_CHARGRID`, #334) in ways that do not
agree. This note goes back to the run's own reports rather than to the prose, and settles each
statement against them.

The short answer: **the three statements are not measuring different runs, they are different
metrics of one run.** Each one got a word wrong. On 500 synthetic documents the repair made 46
documents' names exact and broke 13 (`names_exact`, net **+33**). Restricted to Tier-1 hits, that
is +43 / −13 (strict hits, net **+30**, 199 → 229). On 257 real specimens it fixed one document and
broke one (strict hits net **0**, 12 → 12). All 13 regressions are **synthetic**. No artifact
attributes a cause to them, and the one cause ever named for them was later tested: the probe
ruled out a whole-cell grid-origin miss.

## The artifacts

The run is `artifacts/chargrid-ab-20260917-2356/` in the main checkout. It is gitignored, so it
exists on one machine only. This note's tables are its first committed record.

| Item | Value | Label |
| --- | --- | --- |
| Build | `build.log`: release, workspace v1.5.0, finished 2026-09-18 00:02 | measured |
| MAIN of the binary | `daab97e` (#336). The main checkout fast-forwarded to it at 2026-09-17 23:56:04, and `.git/logs/HEAD` has no further entry until after the last arm finished at 07:43. It contains #334 (`f092f9c`). | inferred (the reports carry no sha) |
| Arms | `off`, `control`, `on`. Each report's `ocr_arms.chargrid` confirms the arm the binary actually ran; the other arms were `texture=on, order/rotate/skew=default`. | measured |
| Synthetic population | `source: synthetic-corpus`, profile `clean`, seeds 0–99, 5 formats × 100 = **500** documents, one run per arm (`syn-<fmt>-<arm>.json`) | measured |
| Real population | `source: real-specimens`, **257** documents, two runs per arm (`real-<arm>-run{1,2}.json`) | measured |
| Real corpus | 257, not CI's 261: the local tree had drifted ([`corpus-drift-local-vs-samples-data-2026-09-19.md`](corpus-drift-local-vs-samples-data-2026-09-19.md)). No DATA sha was recorded. | measured / inferred |

`off` and `control` are identical on every document in both populations: same hit, same
`names_exact`, same strict outcome. So the placebo is clean, and every delta below belongs to the
repair itself, not to the extra pass. Run 1 and run 2 are also identical per document within each
real arm, so the usual `rten` noise did not show up on these fields.

## Synthetic: 500 documents

Per format, `off` → `on`. `control` equals `off` everywhere, so it is not repeated. Hits are
Tier-1 hits; "strict" means hit ∧ both names exact; `names_exact` counts every document with
both names exact, hit or not.

| Format | Hits (all arms) | Strict off → on | Per-doc strict +/− | `names_exact` off → on | Per-doc `names_exact` +/− | Repairs applied (`on`) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| TD3 | 78 / 100 | 39 → 45 | +7 / −1 | 39 → 45 | +7 / −1 | 35 |
| TD2 | 73 / 100 | 36 → 44 | +9 / −1 | 37 → 45 | +9 / −1 | 29 |
| TD1 | 54 / 100 | 15 → 18 | +3 / −0 | 17 → 20 | +3 / −0 | 10 |
| MRV-A | 85 / 100 | 50 → 51 | +9 / −8 | 52 → 54 | +10 / −8 | 55 |
| MRV-B | 87 / 100 | 59 → 71 | +15 / −3 | 60 → 74 | +17 / −3 | 54 |
| **All** | **377 / 500** | **199 → 229** | **+43 / −13 = +30** | **205 → 238** | **+46 / −13 = +33** | **183** |

Both denominators: strict hits went from 199 / 377 hits to 229 / 377 (52.8% → 60.7%), and from
199 / 500 documents to 229 / 500 (39.8% → 45.8%). Tier-1 hits did not move on any document (zero
hit flips), and 377 / 500 matches the CI synthetic figure recorded in `FINDINGS.md` on 2026-09-16.

**Why +33 and +30 differ.** The gap is three documents the repair made name-exact but that are not
Tier-1 hits: MRV-A seed 76 and MRV-B seeds 28 and 95, each `document_number_mismatch` with
`chargrid: repaired`. They count toward `names_exact` and cannot count toward strict hits. Both
metrics share the same 13 losses.

**The 13 regressions, by name.** Each of them was a Tier-1 hit with both names exact under `off`,
stayed a hit under `on`, lost `names_exact`, and carries `chargrid: repaired`. In other words, the
repair passed its own gates and its re-parse stayed checksum-valid, but the name line it rebuilt
was wrong.

| Format | Seeds |
| --- | --- |
| TD3 | 32 |
| TD2 | 86 |
| MRV-A | 0, 12, 21, 29, 33, 41, 48, 75 |
| MRV-B | 4, 31, 92 |

Eight of the 13 are MRV-A, where the repair nets +1 on strict hits.

## Real specimens: 257 documents

| Arm | Hits | Scored | Strict (name-scorable) | Names exact among hits | Repairs applied |
| --- | ---: | ---: | ---: | ---: | ---: |
| `off` run 1 / run 2 | 141 / 141 | 155 | 12 / 43 | 12 / 33 | — |
| `control` run 1 / run 2 | 141 / 141 | 155 | 12 / 43 | 12 / 33 | — (96 `control`) |
| `on` run 1 / run 2 | 141 / 141 | 155 | 12 / 43 | 12 / 33 | 40 / 40 |

Both denominators: 141 / 155 scored = 91.0%, and 141 / 257 corpus-wide = 54.9%, in every arm. The
scored count is 257 minus 46 `no_mrz_expected`, 37 `redacted_mrz` and 19 `checksum_failed_specimen`.

The flat 12 hides a swap. Exactly two documents change, and each changes the same way in both `on`
runs:

| Document | `off` / `control` | `on` | `on` chargrid status |
| --- | --- | --- | --- |
| `passports/Somalia_Passport_Specimen_P0_SOM_2023_mrz.png` | hit, names wrong | hit, **names exact** | `repaired` |
| `passports/Serbia_Passport_Specimen_P0_SRB_2012_mrz.jpg` | hit, **names exact** | hit, names wrong | `repaired` |

So "0 on real" is a net of **+1 / −1**, with 40 repairs applied. The real arm also contains one
correct→wrong regression, and like the synthetic ones it is invisible in the marginal total.

## Verdict per statement

| # | Where | Statement | Verdict |
| --- | --- | --- | --- |
| 1 | [`twelve-scored-misses-2026-09-19.md`](twelve-scored-misses-2026-09-19.md) §8 | "a net `+33` on synthetic hid 13 real regressions" | **Imprecise.** +33 is right for `names_exact` (+46 / −13) over 500 synthetic documents. The strict-hit net is +30. "Real" was meant as *genuine*, but in a note about real specimens it reads as the real population, and the 13 are all synthetic. The real arm's own regression (Serbia) is not mentioned. |
| 2 | [`ADR-0015`](../decisions/ADR-0015-geometric-mrz-band-location.md), Consequences | "the 2026-09-18 A/B traced 13 synthetic regressions to grid-origin error" | **Wrong on the cause; right on the count and the population.** No artifact traces the 13 to anything, because the reports carry no read text and no grid geometry. Grid-origin error was a hypothesis. #342 (`7a8f84e`, merged about four hours after this ADR, #341 `d2d5855`) measured the origin bias over 217 synthetic fits: +0.126 cell, and no fit reached a whole cell. Its commit message says this "rules out a whole-cell origin miss as the cause of the overnight chargrid regressions". A sub-cell origin effect was neither shown nor excluded. The cause is **unattributed**. |
| 3 | [`ocrb-filler-geometry-2026-09-23.md`](ocrb-filler-geometry-2026-09-23.md), intro | "chargrid: +30 names on synthetic, 0 on real" | **Correct as net strict-hit figures; imprecise.** It gives no metric or denominator, and "0 on real" is +1 / −1, not "no effect". |
| — | Memory note "+30 on synthetic, exactly 0 on real" / "13 correct→wrong regressions behind a +33 net" | — | Correct. It is the source of both numbers, each quoted with its own metric. |

A fourth figure is also in circulation: #334's commit message (`f092f9c`) reports "hits unchanged
at 84, names exact 48 → 55" over 100 synthetic documents (20 seeds × 5 formats). That was an
earlier, smaller synthetic A/B, run before merge. It is **not** the 2026-09-18 run and should not
be reconciled with it.

## Proposed replacement text

The calling session applies these. This note edits none of the three documents.

**1. `twelve-scored-misses-2026-09-19.md` §8, the first bullet's second sentence:**

> The question is not whether it fixes Croatia — it does — but how many documents it silently
> *breaks*. That is exactly what the chargrid A/B's per-document 2×2 caught: on 500 synthetic
> documents it fixed 46 names and broke 13 (net `+33` `names_exact`; strict hits net `+30`), and on
> 257 real specimens it fixed one and broke one (net `0`)
> ([reconciliation](chargrid-ab-reconciliation-2026-09-24.md)). Three-arm, same-binary, real arm
> required.

**2. ADR-0015, Consequences, second bullet** (as a dated amendment, if the ADR's amendment
convention requires one):

> The crop handed to recognition arrives with a measured pitch and phase. That is exactly what
> `chargrid` and any ADR-0014 arm need. The 2026-09-18 chargrid A/B broke the names on 13 synthetic
> documents it had read correctly (all 13 `repaired`, 8 of them MRV-A). Their cause is not
> attributed: grid-origin error was suspected, but #342 measured the synthetic origin bias at
> +0.126 cell with no fit reaching a whole cell, which rules out a whole-cell miss
> ([reconciliation](../benchmarks/chargrid-ab-reconciliation-2026-09-24.md)). A locator that
> supplies a verified origin is a hypothesis to test against those 13, not a known fix.

**3. `ocrb-filler-geometry-2026-09-23.md`, intro sentence:**

> The project has measured that the synthetic corpus does not predict real results for a repair
> class (chargrid, 2026-09-18: strict names net +30 on 500 synthetic documents, net 0 on 257 real
> specimens — one fixed, one broken;
> [reconciliation](chargrid-ab-reconciliation-2026-09-24.md)), so ...

## The invocation

No new run was made. The transitions were computed with Python's standard library over the
retained reports. For each document keyed by `asset_id` (real) or `name` (synthetic seed):
hit = `miss_reason is None and mrz_checksums_valid`; strict = hit ∧ `names_exact is True`. Each
arm pair was compared document by document: `off→control`, `off→on`, `control→on`, plus run 1 vs
run 2 on real. The original arm commands were not saved alongside the reports. The file names and
each report's `ocr_arms` block establish the arm, format and population; the exact command line is
**not recorded**.

## What this does not claim

- **It does not attribute the 13 synthetic regressions to any cause.** The reports hold no read
  characters and no grid geometry, so they can show *that* a repaired name line was wrong, not
  *why*. The single cause ever named is refuted only in its whole-cell form.
- **It does not re-establish the grid-origin measurement.** The 217-fit, +0.126-cell figure exists
  only in #342's commit message. The committed probe summary
  (`mrz-matrix-probe-2026-09-18.json`) does not carry it, and no raw output was retained.
- **It does not restate the result on the current corpus.** The real arm ran on a drifted
  257-document tree with 43 name-scorable documents. CI's corpus is 261 documents, and the current
  baseline has 45 name-scorable documents (the figure ADR-0014's threshold cites). The 2026-09-18
  measurement is frozen at that tree and binary.
- **It does not say the repair is harmful on real documents.** One fixed and one broken is net
  zero, and the sample is one document each way.

## Rejected on the way

- **"+30 and +33 are two different runs, or before/after a fix."** Rejected: both come out of the
  same 15 synthetic reports. They differ only by the three non-hit `document_number_mismatch`
  documents.
- **"The 13 regressions were on real specimens."** Rejected: the real arms show exactly one
  regression (Serbia), in both runs.
- **"#334's 48 → 55 is an early reading of the same A/B."** Rejected: different N (20 seeds × 5
  formats = 100, against 100 × 5 = 500). It was also committed (2026-09-17 21:43) before this
  run's binary was built (2026-09-18 00:02).
