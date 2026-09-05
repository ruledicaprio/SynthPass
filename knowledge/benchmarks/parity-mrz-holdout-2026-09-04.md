# Tier-2 parity under MRZ holdout, 2026-09-04

> **Superseded the same day — the numbers below were measured by a harness that
> did not normalize the model's output, while both pipeline entry points do.
> They understate the shipped product by roughly 20 percentage points.** The
> corrected run is in the second half of this file; read it first. The original
> is kept because its *relative* finding (holding out the MRZ costs ~5pp)
> survived the correction, and because the defect is worth not repeating.

The "before" required by [`../VIZ_TIER2_DESIGN.md`](../VIZ_TIER2_DESIGN.md) §5.3, and the first
measurement of the escalation case described in §5.2.

**Headline: holding out the MRZ costs 5–6 percentage points, not the collapse the design
anticipated — because the baseline is already low. Tier 2 is weak on this corpus with the MRZ
present, and only slightly weaker without it.**

## Method

`crates/synthpass-llm/tests/parity.rs`, 72 fixtures, `qwen2.5-1.5b-instruct-q4_k_m`, CPU.

Both arms from **one test binary**,
`sha256 496540e4aa3cbf0bb3db3107d3983498c8829861624595d93b3ceb168461a042`, run back-to-back —
same discipline as the texture A/B, for the same reason.

| arm | invocation |
| :-- | :-- |
| baseline | `parity --ignored --nocapture --test-threads=1 native_llm_field_accuracy_over_sample_set` |
| holdout | the same, with `SYNTHPASS_PARITY_HOLDOUT=1` |

CPU rather than CUDA: unlike `provider-bench`, this harness *is* LLM-dominated and would have
benefited, but a `llama-cpp-sys-2` rebuild against the CUDA 12.9 toolkit costs more than the ~10
minutes it would have saved. Recorded so the next run can make that trade knowingly.

The model-free pre-flight (`holdout_strips_every_fixture_on_disk` and the three predicate tests)
was run first and passed. It exists to stop a ~30-minute run producing a leak-inflated number.

## Results

| arm | reviewed (9 fields × 18 docs) | derived (3 check-digited fields × 54 docs) | legacy 7-field | wall-clock |
| :-- | --: | --: | --: | --: |
| baseline | 43/162 — **26.5%** | 47/162 — **29.0%** | 38/126 — 30.2% | 30.6m |
| holdout | 34/162 — **21.0%** | 39/162 — **24.1%** | 30/126 — 23.8% | 25.4m |
| delta | −9 fields, −5.5pp | −8 fields, −4.9pp | −8 fields, −6.4pp | −5.2m |

Stripping removed **417 MRZ-derived lines across all 72 fixtures; 0 fixtures had none removed**,
and 0 leaks were detected. 417 matches an independent Python analysis of the same corpus exactly —
two implementations, one number.

Holdout mode applies **no accuracy floor**, by design: the floors are a regression gate for the
standard corpus, and a holdout measures a harder task where a low number is the finding.

## What the numbers say

**1. The MRZ was contributing about a fifth of what Tier 2 got right, no more.** Removing the
checksum-proven source of every one of these fields entirely costs ~5pp of ~27pp — roughly a 20%
relative drop. That is far less than a *"the model reads the MRZ and little else"* model of its
behaviour predicts, which is what the design assumed going in.

Two readings are consistent with that, and this run does not separate them:

- the model was already recovering these fields from the visual zone in the baseline, and the MRZ
  was largely redundant to it; or
- the model was not reliably reading *either* source, and removing one of two weak signals moves a
  weak number a little.

The absolute baseline (26.5%) makes the second more likely, and it is the more actionable reading.

**2. The headline finding is the baseline, not the delta.** 26.5% of prompt fields correct *with*
a checksum-valid MRZ in the prompt is the number that should provoke work. This is a 1.5B model
asked a vague question over text with a measured median 23.7% noise-line fraction — exactly the
four failures VIZ_TIER2_DESIGN §2 enumerates. The holdout arm did not discover a VIZ weakness; it
confirmed the baseline was never MRZ-carried in the first place.

**3. The escalation case is now measurable, which it was not before.** Tier 2 runs only when Tier 1
fails, so every fixture here is by construction a document Tier 2 never sees in production. The
holdout arm is the first number on this project describing the population Tier 2 actually serves.
It is 21.0%, and every subsequent VIZ change now has a before to beat.

**4. Holdout runs are ~17% cheaper** (25.4m vs 30.6m) — less prompt text, fewer tokens. Useful for
budgeting the §2.1–§2.3 iteration loop, which will re-run this arm repeatedly.

## Caveats that limit what this can be used for

- **Derived-fixture ground truth is one OCR pass's reading** for anything a check digit does not
  cover, which is why derived fixtures are scored on three fields only. The split is load-bearing
  here, not bookkeeping.
- **Over-stripping deflates the holdout score by an unmeasured amount.** The union predicate
  deliberately errs toward stripping, since a leaked fragment inflates the score fatally while an
  over-stripped visual-zone line only removes context. The 21.0% is therefore a *lower* bound on
  what the model could do given a perfectly-stripped input.
- **One model, one quantisation, one prompt version.** Nothing here generalises to a larger Tier-2
  provider.

## What this changes about the plan

VIZ_TIER2_DESIGN's ordering (clean → label → ask narrowly → new fields) is unaffected and, if
anything, better justified: a 26.5% baseline is not a corpus problem to be measured around, it is a
prompt-and-noise problem, and §2.1–§2.3 attack precisely that.

What it does change is the expected size of the prize. The holdout gap is small, so *closing the
holdout gap* is not the goal worth chasing — **raising both arms together** is. A VIZ cleaning pass
that helps the escalation case should show up in the baseline arm too, and if it only moves the
holdout number, that is a signal to be suspicious of it.

---

# Corrected run, 2026-09-04 — the harness was scoring something we do not ship

Same binary discipline, same 72 fixtures, same model. Two changes, both in the measurement rather
than the model:

1. **The harness now calls `synthpass_core::normalize::extraction` on the model's output**, which
   both `synthpass-pipeline` entry points already do on every Tier-2 result. It previously called
   nothing, relying on a local `normalize_date` that understood only `DD.MM.YYYY`.
2. **`normalize::date` learned named months** — `"14 OCT /OUT 2000"` → `"2000-10-14"` — the
   bilingual rendering travel documents actually print. This one is a production change, not a
   harness change: it improves what ships.

| arm | reviewed (9 fields) | derived (3 check-digited) | legacy 7-field |
| :-- | --: | --: | --: |
| baseline, as first reported | 43/162 — 26.5% | 47/162 — 29.0% | 38/126 — 30.2% |
| **baseline, corrected** | **78/162 — 48.1%** | **78/162 — 48.1%** | **54/126 — 42.9%** |
| holdout, as first reported | 34/162 — 21.0% | 39/162 — 24.1% | 30/126 — 23.8% |
| **holdout, corrected** | **64/162 — 39.5%** | **72/162 — 44.4%** | **45/126 — 35.7%** |

**Superseded later the same day for the baseline arm.** Two printed date forms
the normalizer had been discarding were added, and the baseline arm re-ran at
**85/162 — 52.5%** on reviewed, **85/162 — 52.5%** on derived, **61/126 —
48.4%** legacy. Nothing about the model changed; see
[normalize-date-forms-2026-09-04.md](normalize-date-forms-2026-09-04.md).

**Holdout re-measured 2026-09-05, and both arms moved together.** This file's
standing rule — *a change that moves only one arm is suspect* — is satisfied:

| arm | pre-#206 | post-#206 | Δ fields |
| :-- | --: | --: | --: |
| baseline reviewed | 78/162 (48.1%) | 85/162 (52.5%) | +7 |
| holdout reviewed | 64/162 (39.5%) | **70/162 (43.2%)** | +6 |
| holdout derived | 72/162 (44.4%) | **83/162 (51.2%)** | +11 |
| holdout legacy 7-field | 45/126 (35.7%) | **51/126 (40.5%)** | +6 |

The date normalizer is not a baseline-only artifact: it helps the escalation
case as much as the easy one, which is what a *deterministic* fix should do and
what a prompt-shaped one might not have.

The holdout gap widened slightly (8.6pp → 9.3pp on reviewed) because the
baseline gained marginally more — expected, since a fixture whose MRZ is
stripped has fewer printed dates left to read.

> **Provenance, recorded because it nearly went unrecorded.** These holdout
> numbers came out of a run whose *baseline* arm was invalid: the runner's build
> had failed and it silently re-used the previous binary. That made the run
> useless for the change it was launched to measure (country demonyms) and
> exactly right for this one, since the stale binary was post-#206/pre-demonym.
> The incident is why `normalize::vocabulary_fingerprint()` and
> `scripts/measure-parity.sh` now exist — see
> [normalize-country-demonyms-2026-09-05.md](normalize-country-demonyms-2026-09-05.md).
> Any run logged without a `normalizer vocabulary:` line predates that guard and
> cannot prove which code produced it.

Per field, baseline arm:

| field | as first reported | corrected |
| :-- | --: | --: |
| `document_number` | 38/72 (53%) | 38/72 (53%) |
| `date_of_birth` | 11/72 (15%) | **36/72 (50%)** |
| `date_of_expiry` | 15/72 (21%) | **34/72 (47%)** |
| `document_type` | 1/18 (6%) | **12/18 (67%)** |
| `issuing_country` | 4/18 (22%) | **12/18 (67%)** |
| `nationality` | 4/18 (22%) | **7/18 (39%)** |
| `surname` | 5/18 (28%) | 5/18 (28%) |
| `given_names` | 7/18 (39%) | 7/18 (39%) |
| `sex` | 5/18 (28%) | 5/18 (28%) |

## What was actually wrong

Nothing about the model changed. Every gain above is a value the model had already extracted
correctly and the harness then marked wrong for its *format*:

- **Dates** — the model reads `"14 OCT /OUT 2000"`; the fixture holds `"2000-10-14"`. 64% of date
  mismatches in the original run were of exactly this shape.
- **`document_type` and `issuing_country`** — the model answers `PASSPORT` and `CROATIA`;
  `normalize::document_type` and `normalize::country_code` map those to `P` and `HRV` in
  production, and the harness never ran them.

The fields that did not move — `document_number`, `surname`, `given_names`, `sex` — are the ones
where no normalizer applies, and their numbers were correct all along. That is the useful control
built into this result: a normalization defect should leave exactly those untouched, and it did.

## Why this was hard to see

`knowledge/prompts/README.md` requires a measurement per `PROMPT_VERSION` bump. But
`PROMPT_VERSION` and `prompt_digest` cover the compiled-in *template* only — the digest is computed
with `{content}` empty, deliberately. **Anything that changes the content the model is given, or
the post-processing applied to what it returns, is invisible to that mechanism.** The parity
harness's own divergence from the pipeline was in precisely that blind spot, and sat there across
several prompt versions.

The general form: this project guards the prompt skeleton by digest and the deterministic path by
checksum, and the Tier-2 *pre- and post-processing* by neither. The parity harness is the only
instrument that would ever show it, which is an argument for keeping it faithful to the pipeline
by construction rather than by resemblance.

## What survived the correction

The original run's *relative* finding holds and is, if anything, sharper: holding the MRZ out costs
**8.6 percentage points** on reviewed fixtures (48.1% → 39.5%), against ~5pp measured wrongly.
Tier 2 still is not MRZ-carried — but the VIZ-only escalation case now measures 39.5%, not 21.0%,
which is a materially different starting point for the §2 work.

## What this says about VIZ_TIER2_DESIGN §2.1

§2.1 ("clean the visual zone") was the next planned piece of work, on the strength of a measured
median 23.7% noise-line fraction. Two measurements taken before writing any of it argue against:

- That fraction is over **geometry-detected lines** (`OcrPage.lines`). The model receives
  `NativeOcr`'s whole-page text, a different artifact. Applying the survey's own noise predicate to
  the markdown, with the MRZ guard it requires, drops **61 of 2970 lines (2.1%)** — nearly all of
  them single characters like `?`, `$`, `(`.
- **Every scored field is already present in the OCR text**, 97-100% of fixtures (dates 72/72 once
  looked for as MRZ `YYMMDD` rather than ISO). Cleaning cannot help a model that already has the
  value in front of it.

Two traps found in the same pass, worth recording because the naive implementation hits both:
the survey's `≤2 non-whitespace characters` rule deletes **31 lines that are exactly a ground-truth
value** (`M`, `F`, `P`, `PP` — `sex` and `document_type` are scored fields), and its
`≥50% non-alphanumeric` rule deletes **54 MRZ lines**, because `<` filler is not alphanumeric. The
survey applies that rule only to lines that are not MRZ-shaped; that guard is the entire thing.

§2.1 is therefore **deferred, not rejected** — image-level cleaning ahead of the general OCR pass
is untested and is a different proposition from line filtering. What is refuted is text-level
line-dropping, which is what "clean" would most naturally have been built as.
