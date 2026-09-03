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

Until that runs, the stage ships **default-off** (`TextureMode::Off`) and costs nothing.
