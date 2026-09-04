# Tier-2 parity under MRZ holdout, 2026-09-04

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
