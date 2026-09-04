# Texture suppression: three-arm A/B, 2026-09-03

Measurement for `preprocess::texture_variants` (see
[`../research/document-pipeline-stage-taxonomy.md`](../research/document-pipeline-stage-taxonomy.md)).

**Headline: no measurable effect on this corpus — and the corpus cannot test the hypothesis.**
Every safety property held. The result is recorded rather than discarded.

## Method

Harness `crates/synthpass-ocr/examples/mrz_corpus.rs` (Tier-1 only, no Tier-2/LLM), 20 positives +
42 negative controls derived from `samples/corpus.jsonl`.

Three arms toggled by `SYNTHPASS_OCR_TEXTURE`, run back-to-back from **one release binary**,
`sha256 4994f84eb16829dc0263fecc89cefb40e8aab996d30908dbeeaa85fa13d6f2f7` — not a rebuild-based
before/after, which has hidden a real regression on this project before.

| arm | what it is |
| :-- | :-- |
| `off` | baseline; behaviour identical to before the stage existed |
| `control` | placebo — `plain_band`, one extra trailing pass with real budget and text cost, already measured as inert on `ocrs` |
| `on` | the median texture treatment |

`SYNTHPASS_OCR_VERBOSE` was deliberately **left off**: its branch calls `region_count`, an extra
OCR detection pass per variant, which would have inflated every arm's wall-clock and distorted the
budget question being asked.

## Results

| arm | Tier-1 hits | false positives (42 negatives) | total wall-clock | median/doc | max/doc |
| :-- | --: | --: | --: | --: | --: |
| `off` | 18/20 | 0 | 623.6s | 9.1s | 64.7s |
| `control` | 18/20 | 0 | 690.9s | 10.2s | 65.1s |
| `on` | 18/20 | 0 | 737.8s | 11.0s | 66.8s |

Per-document flips, all three pairwise comparisons: **zero** miss→hit, **zero** hit→miss.

Decision rule was `on > control` **and** `control >= off`. `control == off` held; `on > control`
did not. Verdict: null.

## What the run does establish

1. **No regression.** Zero hit→miss flips in any arm. The trailing/additive contract held.
2. **No hallucinated MRZs.** Zero false positives across 42 no-MRZ documents in every arm — the
   risk that a new preprocessing pass manufactures a checksum-valid read on a document that has
   none.
3. **The pass genuinely executes.** `control` costs +67.3s over `off` and `on` costs +114.2s
   (+46.9s over the placebo, which is the median filter's own cost). A variant truncated by the
   budget could not cost 114 seconds. This is stronger evidence of reachability than a verbose log,
   and it cost nothing extra to obtain.
4. **The wall-clock budget is not masking anything here.** Exactly one document reaches the 45s
   `SYNTHPASS_OCR_MAX_SECONDS` ceiling — `France_ID_Specimen_2020_front_no_mrz.png`, a *negative*
   control, and it was already at 64.7s in the baseline, so its truncation predates this change and
   is harmless (it must not validate, and does not). Every positive sits far below: median ~9-11s.
   **`SYNTHPASS_OCR_MAX_SECONDS` therefore did not need raising**, which was an open question going
   in.

## Why the null is uninformative about the hypothesis

This is the finding that matters, and it is a property of the corpus, not of the treatment.

Texture suppression targets **`checksum_failed`** misses — an MRZ that is found and then misread,
which is what background texture bleeding into binarization produces. The two positives that miss
here are:

```
MISS  Oman_Passport_Specimen_P0_OMN_2004_mrz.jpg:    no MRZ found in text
MISS  Vietnam_Passport_Specimen_P0_VNM_2023_mrz.webp: no MRZ found in text
```

Both are **`no_mrz_found`** — the other failure mode entirely, and both are already-known,
not-yet-root-caused stale corpus entries. So this corpus contains **zero instances of the failure
class the treatment addresses**, and no result it produces could have confirmed or refuted the
idea.

Compounding it: the 18 passing positives validate on early variants (3-8s each) and never reach
variant 9 at all. The treatment's entire opportunity here was 2 documents of the wrong kind.

## What would actually test it

The real-specimen `provider-bench` corpus, where the 66 recorded `checksum_failed` misses live and
do reach the trailing variants:

```powershell
cargo build -p synthpass-bench --release --bin provider-bench
$bin = "target/release/provider-bench.exe"
foreach ($arm in @("off","control","on")) {
    $env:SYNTHPASS_OCR_TEXTURE = $arm
    & $bin --real-specimens --dump-ocr --out "artifacts/texture-ab-$arm.json"
}
Remove-Item Env:\SYNTHPASS_OCR_TEXTURE
```

Roughly 1h37m per arm. Compare `documents_detail[]` per-document, not the aggregate rate, and diff
the `off`/`on` OCR dumps for documents that *still* fail — that distinguishes "right idea, wrong
radius" from "wrong idea".

---

# Real-specimen result, 2026-09-04 — the treatment is real, and small

Run overnight on 232 real specimens (229 scored), three arms from one CUDA binary
`sha256 a8a41d556c1a6f2cc26549fba94cae460cfce541e5e3fa4d63228916e0227f52`.
81 / 85 / 82 minutes per arm.

| arm | Tier-1 hits | rate | `checksum_failed` | `no_mrz_found` |
| :-- | --: | --: | --: | --: |
| `off` | 111/229 | 48.5% | 70 | 48 |
| `control` | 112/229 | 48.9% | 71 | 46 |
| `on` | 113/229 | **49.3%** | **68** | 48 |

**Decision rule satisfied:** `on (113) > control (112) >= off (111)`, and **zero** hit→miss
regressions against baseline. Net **+2 documents, +0.9pp**.

## The mechanism checks out, which is what makes +2 believable

Both documents gained by `on` came from `checksum_failed` — the exact miss class texture
suppression targets — and the miss-kind delta is precisely the predicted signature:

```
on  vs off:  checksum_failed -2,  no_mrz_found  0
```

A coincidental gain would not land exclusively in the one bucket the mechanism predicts while
leaving the other untouched. Gained: `Argentina_Passport_Specimen_P0_ARG_2026_mrz`,
`Portugal_Passport_Specimen_PC_PRT_2012_mrz`, both `checksum_failed` → hit.

**+2 of 229 is a small effect and should not be oversold.** Because the pipeline is deterministic
these are real per-document facts rather than sampling noise, but "real" is not "large".

## The unplanned finding, which is worth more than the +2

The `control` arm was built only to isolate the cost of adding a pass. It found something else:

```
control vs off:  checksum_failed +1,  no_mrz_found -2
```

`plain_band` — the untreated band crop the placebo uses — fixes a **different miss class**. It
recovers `no_mrz_found` documents (a *detection* failure), where the median filter recovers
`checksum_failed` documents (a *recognition* failure). And they **compete for the same slot**:
each arm has exactly one variant 9, so `on` does not include `plain_band`.

The proof is `Switzerland_Passport_Specimen_PM_CHE_2022_mrz`: `off` = `checksum_failed`,
`control` = **HIT**, `on` = `checksum_failed`. The placebo recovers a document the treatment
cannot, and vice versa.

**So the next step is not tuning the median — it is running both.** Variants 9 *and* 10
(`plain_band` then `texture_variants`), which plausibly reaches 114/229 by taking all three
documents. That needs `MAX_RETRY_VARIANTS` 9 → 10 and its own measured run; it is not a change to
make unmeasured.

Note the irony worth recording: `plain_band`'s own doc comment says it is deliberately excluded
from `mrz_variants` because `ocrs` "normalizes internally, so an untreated variant adds nothing
there". On this corpus, as a trailing variant, it adds one document. That claim was measured on
the fixture corpus, not this one.

## CUDA: much less speedup than expected, and the reason matters

Predicted 40–50 min per arm from ADR-0004's ~2.5x. Actual: **81–85 min**, against a recorded
~1h37m CPU baseline — roughly **16% faster, not 2.5x**.

The prediction was wrong because it assumed the real-specimen run is dominated by Tier-2 LLM
generation. It is not: it is dominated by **Tier-1 OCR**, which is pure CPU and untouched by the
`cuda` feature. ADR-0004's 2.5x is a claim about *LLM generation*, and generalising it to
*provider-bench wall-clock* was the error. Budget CPU time for real-specimen runs regardless of
GPU.

CUDA remains correct to use — GPU output is byte-identical, so it confounds nothing — but it is
not the lever it looks like for this particular benchmark.

## Recommendation

Defaulting `TextureMode::On` is defensible on this evidence: a measured gain in the predicted
direction, zero regressions, and a trailing/checksum-gated design that structurally cannot lower
the hit rate. But the cleaner end state is **both variants**, so the sequencing is: land 9+10,
measure, then flip the default once — rather than flipping twice.

Until then the stage remains **default-off** and costs nothing.
