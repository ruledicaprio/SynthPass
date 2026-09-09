# Browser vs native OCR, both arms measured the same day

**2026-09-09.** [`ADR-0008`](../decisions/ADR-0008-mrz-detection-track.md) chunk 1, part 1: replace
the stale "64.2% vs 59.5%" comparison with one where both numbers are current, then say what the
gap is made of. No attribution experiment yet — that is part 2, and this narrows what it has to
explain.

## The published comparison was stale on both sides

ADR-0008 quotes `WEB_OCR_BASELINE.md`'s **122/190 = 64.2%** browser against **113/190 = 59.5%**
native. Neither was current when the ADR was written:

- The **browser** figure is the first of four measurements in that document. It was superseded
  three times — JS→WASM preprocessing (122 → 125), the `Triangle` upscale filter (tie on hits,
  +7 line-1 fields), and trailing rotation (125 → 127).
- The **native** figure was never a measurement in that run at all. `run-corpus.mjs` reads it from
  `samples/corpus.jsonl`'s `mrz.observed.checksums_valid`. Diffing the manifest from `c65a733`
  (2026-09-03) to `HEAD`: **0 of 232 pre-existing rows changed.** Later commits only appended new
  specimens. It had not been recomputed through `mrz` 0.7.0, 0.7.1, `geometry_band_variants`,
  texture suppression, or the `Lanczos3`/`Triangle` decision.

## Both arms, today

Native: `provider-bench --real-specimens --mrz-only`, today's binary. Browser:
`node tests/web/run-corpus.mjs` against the assembled `_site/`, tesseract.js 5.1.1 + core 5.1.1,
OEM 1, default PSM, vendored OCR-B `mrz.traineddata` (BSD-3 © DoubangoTelecom) with `eng`
4.0.0_best_int as fallback, `tessedit_char_whitelist` = `synthpass_imageprep::MRZ_CHARSET`.

| Population | Browser | Native | Gap |
| :-- | --: | --: | --: |
| All 196 `mrz.present` specimens | 131 = 66.8% | 120 = 61.2% | **+11 docs / +5.6 pp** |
| **Excluding the 36 redacted** (160) | **128 = 80.0%** | **119 = 74.4%** | **+9 docs / +5.6 pp** |

Head-to-head on the 160: **both 110, browser-only 18, native-only 9.**

Native moved **+3 documents, −0** since the manifest froze; the browser moved 122 → 131 on a
corpus that also grew by six. So the gap is real and close to its claimed size — it was just never
being measured on either side.

## The gap is concentrated, not diffuse

This is the part that matters for ADR-0008. Native has **25 in-denominator misses** (18
`no_mrz_found` + 7 `checksum_failed`, per
[the corrected baseline](denominator-correction-2026-09-09.md)). **The browser reads 17 of them.**

**11 of native's 18 detection failures**, the metric ADR-0008 targets:

```text
Canada PP_CAN_2023          India P0_IND_2024        Portugal PX_PRT_2017
Finland P0_FIN_2007         Kuwait P0_KWT_2023       Russian_Federation P0_RUS_2014
Finland P0_FIN_2023         Monaco ID XXXX back      Vietnam P0_VNM_2023
France ID 2020 back         Oman P0_OMN_2004
```

**6 of native's 7 `checksum_failed`** — and these are not just any seven. They are exactly the
anchors the [2026-09-08 writeup](checksum-failed-real-specimens-2026-09-08.md) identified as
carrying a checksum-valid printed zone, meaning *"any `checksum_failed` is 100% an OCR error"*, and
on which it closed the `checksum_failed` track as OCR-quality-bound:

```text
Afghanistan P0_AFG_2016     Croatia ID 2021 back     Sweden ID 2022 back
Belgium ID 2021 back        Czechia P0_CZE_2005      Romania PE_ROU_2024
                            (+ India P0_IND_2013)
```

Only Russia `P0_RUS_2019` of that set is missed by both. **Those documents are not
OCR-quality-bound in general — they are bound by *this* recognizer.** A different one reads six of
the seven today.

## Where native wins, and why the net is only +9

The browser loses 9 the native pipeline gets:

```text
Angola PN_AGO_2022          Kazakhstan P0_KAZ_2004            Russian_Federation ID 2013 back
Argentina P0_ARG_2026       Netherlands P0_NLD_2014_mrz_blur  Switzerland ID 2003 back
China 2012 (×2)             North_Macedonia P0_MKD_2020_mrz_rotated
```

Two `_blur`, one `_rotated`, two China. The two stacks fail on **different documents**, and the
union (110 + 18 + 9 = 137 of 160 = 85.6%) is well above either. Whatever explains the gap does not
run one way.

## What this does and does not establish

**Does:** the comparison is now between two same-day measurements rather than a live number and a
frozen manifest field; the gap survives at +5.6 pp on both populations; and it is concentrated in
17 named documents rather than spread across the corpus.

**Does not:** attribute the gap. The two stacks still differ in recognizer model, variant ordering,
`UpscaleFilter`, and rotation strategy. Two of ADR-0008's four confounders turn out to be
controlled already — `synthpass-imageprep` compiles to wasm so both run the *same Rust
preprocessing*, and `MRZ_CHARSET` constrains both recognizers — which is what shrinks the
[research note's](../research/Tesseract_OCR_studies.md) five-level factorial to three cells.

**The sharpest untested hypothesis, from reading both call sites:** the browser's *first* attempt
is an **untreated band crop** with the OCR-B model, and it wins ~88% of the browser's reads on its
own. Native has no untreated band pass at all — `preprocess::mrz_variants` starts with a
contrast-stretched crop, and `plain_band` is never called natively. Both `synthpass-ocr/src/lib.rs`
and `web/scan.js:143` assert that *"`ocrs` normalizes internally and gains nothing from an untreated
pass"*. That assertion appears never to have been measured. It is the cheapest cell to run and the
only one whose positive result is a free native win.

Next: `SYNTHPASS_OCR_ORDER=default|band-first|control` as a same-binary A/B, then band-score
instrumentation on the 17 to separate *localization* from *recognition*.

## Reproducing

```console
$ cargo build --release -p synthpass-bench --bin provider-bench
$ ./target/release/provider-bench --real-specimens --mrz-only --out native.json
$ bash scripts/build-site.sh
$ node tests/web/run-corpus.mjs --out web.json
```

Native takes ~53 min locally (~1 min on CI, which caches the `.rten` models); the browser sweep
~12 min, median 2.1 s/document. The corpus runner is single-threaded on purpose — run neither
alongside anything else, and see
[`verify-benchmark-numbers-for-contention`](README.md#the-local-bench-loop-tracks). Corpus pinned at
`samples_data_sha c31d048a`.
