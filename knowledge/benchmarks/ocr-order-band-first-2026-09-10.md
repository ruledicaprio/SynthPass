# Cell (a): pass ordering — does the browser's untreated-band-first advantage transfer to native `ocrs`?

**2026-09-10.** [`ADR-0008`](../decisions/ADR-0008-mrz-detection-track.md) chunk 1C, first cell.
[`ocr-stack-gap-2026-09-09.md`](ocr-stack-gap-2026-09-09.md) named the cheapest hypothesis for the
browser-vs-native gap: the browser's first OCR attempt is an **untreated band crop**
(`preprocess::plain_band` + the OCR-B model), and it produces 112 of the browser's 122
checksum-valid reads on its own. Native runs the *same* `plain_band` function — but as the
second-to-last pass, behind every contrast-stretched and binarized variant. `synthpass-ocr` and
`web/scan.js` both assert `ocrs` "normalizes internally and gains nothing from an untreated pass";
the plan flagged that this had **never been measured natively**. This cell measures it.

## The change — `SYNTHPASS_OCR_ORDER`

New env var in `crates/synthpass-ocr/src/lib.rs`, mirroring `SYNTHPASS_OCR_TEXTURE`'s
same-binary-A/B pattern. Unset / unrecognised = `default`, byte-identical to the previous path.

| arm | what it does |
| :-- | :-- |
| `default` | today's chain: `mrz_variants` → `geometry_band_variants` → texture stage (`plain_band`, then `texture_variants`). |
| `band-first` | `plain_band` is **moved** (not duplicated) to the first retry pass, ahead of `mrz_variants`; dropped from its trailing slot so the pass count is unchanged. `DEFAULT_MAX_PASSES` already reaches every variant regardless of order — a pure reordering. |
| `control` | swaps the two blind-crop `mrz_variants` entries (contrast-stretched ↔ binarized). A `band-first` delta is only trusted if `control` tracks `default`. |

## Result — 239 local specimens, `provider-bench --real-specimens --mrz-only`, `SYNTHPASS_OCR_MAX_SECONDS=68`

| arm | hits / 144 scored | rate | `checksum_failed` | runs |
| :-- | --: | --: | --: | :-- |
| `default` | 119 | 82.64% | 7 | 3× — CI baseline + 2 local, all identical to the document |
| **`band-first`** | **120** | **83.33%** | **6** | 2× — byte-identical |
| `control` | 117 | 81.25% | 7 | 1× |

`default` reproduces the committed CI baseline (119/144) exactly. Hit/miss classification is fully
deterministic — the non-verbose and `SYNTHPASS_OCR_VERBOSE=1` runs of each arm agree on all 239.

### `band-first`: +1, zero regressions — and it is the only real change

The single recovery is **`Belgium_ID_Specimen_2021_back_mrz`** — a TD1 3-line ID-card back, from the
cluster [`ocr-stack-gap-2026-09-09.md`](ocr-stack-gap-2026-09-09.md) and the miss analysis both
flagged. Under `default` its line 2 reads `…UT0…` where the truth is `…UTO…` (the O/0 confusable,
same failure as `Afghanistan_P0_AFG_2016`), and the `composite` check digit fails. Under
`band-first` a valid combination is assembled and all check digits pass (8/9 fields matched against
ground truth). No document regressed, in either run.

### `plain_band`-first recovers nothing else — the assertion holds

`band-first`'s pass distribution over its 120 hits: **only 3 validate on pass 1 (`plain_band`)** —
`Denmark_P0_DNK_2021`, `Kazakhstan_P0_KAZ_2004`, `Moldova_PA_MDA_2014_mrz_wide`. And all three
**validated on the general full-page pass under `default`**. They are not `plain_band` wins; they are
general-pass nondeterminism (`rten`/`ocrs` inference is not bit-reproducible run to run, so a
marginal read lands on the general pass in one process and a retry variant in another). Strip that
noise out and **`plain_band`, tried first, recovers zero documents for `ocrs` on its own merit.**

The assertion the plan wanted measured — *"`ocrs` gains nothing from an untreated pass"* — is
confirmed. The browser's untreated-band-first advantage is a property of the **OCR-B recognizer**,
which was trained on exactly that kind of crop; `ocrs`, a general recognizer, is indifferent to it.

### Cost: 13 documents pay for a wasted pass

13 hits that validate on `contrast_stretched` (pass 1 under `default`) validate on pass 2 under
`band-first` — `plain_band` runs first, fails, and costs them ~2–3 s each. Deterministic total
≈ 33 s, roughly **1 %** of a full corpus run. (The single verbose run-pair shows a larger
corpus-level swing, ~40 s, but it is dominated by the general-pass nondeterminism above and is not a
reliable latency signal; a clean latency A/B would need several runs per arm to average it out.)

### `control` does not track `default`

`control` moved five documents: net **−2** (recovered Belgium, but regressed `Croatia_ID_2002_back`,
`Russian_Federation_ID_2013_back` and `Nicaragua_P0_NIC_2001`, and shifted `Russia_P0_RUS_2019`
laterally). Swapping which *treated* blind-crop variant runs first genuinely trades documents.
"Reorder the passes" is not a safe move in the abstract; `band-first` is safe only because the
untreated crop introduces no contrast or threshold artifacts, so a recognizer that fails on it has
lost nothing.

## What this establishes

1. Moving `plain_band` to the front is a **marginal +1 hit / ~1 % slower / 0 regressions**,
   reproducible. Defensible by the project's priority order (correctness over performance), but the
   evidence for adopting it is thin — one document, obscured by run-to-run noise, against a control
   that shows reordering carries risk.
2. **The browser's ordering advantage does not transfer to `ocrs`.** This rules "retry/variant
   ordering" out as an explanation for the 5.6 pp browser-vs-native gap and points at the
   **recognizer** — OCR-B vs `ocrs` — which is cell (c).
3. Belgium and Afghanistan are both **O/0-confusable recognition misses**, not localization. The
   `no_mrz_found` cluster (localization) is untouched by an ordering change, as expected.

## Decision

`SYNTHPASS_OCR_ORDER` ships with default **`default`** — unchanged production behaviour. Revisit
after cell (c): if the recognizer explains most of the gap, `band-first`'s +1 is a rounding error
not worth the reordering risk; if it does not, reconsider adopting `band-first` for the +1.

## Reproducing

```console
$ cargo build --release -p synthpass-bench --bin provider-bench
$ for arm in default band-first control; do
    SYNTHPASS_OCR_ORDER=$arm SYNTHPASS_OCR_MAX_SECONDS=68 \
      ./target/release/provider-bench --real-specimens --mrz-only \
      --out artifacts/ocr-order-ab/$arm.json
  done
$ python artifacts/ocr-order-ab/analyze.py \
    artifacts/ocr-order-ab/{default,band-first,control}.json
# pass-distribution / latency: re-run default + band-first with
# SYNTHPASS_OCR_VERBOSE=1, then artifacts/ocr-order-ab/passes2.py
```

Corpus: local `samples/` at 239 specimens (the committed CI baseline pins 238; the local tree
carries one extra passport specimen — pre-existing local/`samples_data_sha` drift, not this
change). `samples/private/` excluded. Runs paired two-at-a-time on a 4-core machine, hence the
`SYNTHPASS_OCR_MAX_SECONDS=68` (1.5×) contention headroom.
