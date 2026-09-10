# Denominator refinement — the Argentina 2026 specimen has a fake printed MRZ

**2026-09-10.** A third pass over the Tier-1 denominator, in the lineage of
[`denominator-correction-2026-09-09.md`](denominator-correction-2026-09-09.md) (the 94-document
correction) and the Malaysia specimen. Surfaced by the user's hand-read of the 25 in-denominator
misses during ADR-0008 chunk 1C.

## What moved

`Argentina_Passport_Specimen_P0_ARG_2026_mrz` and its `_mrz_blur` variant carry a **printed MRZ
that is non-conforming by construction** — it disagrees with its own visual-inspection zone and
fails its own ICAO 9303 check digits:

| field | VIZ | printed MRZ line 2 |
| :-- | :-- | :-- |
| sex | F | **M** |
| date of birth | 10.02.1957 | 14 Feb (`570214`) |
| date of expiry | 03.03.2036 | 2033-03-02 (`330302`) |
| document-number check digit | — | printed `0`, computed `2` |

A byte-perfect OCR read of that zone still fails. Same class as the 16 `TEMPLATE`/`ÖRNEK`/all-zeros
specimens the 2026-09-08 `checksum_failed` writeup put outside the denominator, and the same shape
as Malaysia.

**Mechanism:** each now has a reviewed `samples/ocr_fixtures/<stem>.json` carrying the hand-
transcribed printed zone. `provider_bench::prep_specimens` computes `printed_zone_nonconforming`
from that transcription failing `mrz::find_and_parse().valid()`, and the classifier reports the two
as `checksum_failed_specimen` (off-denominator) rather than `checksum_failed` / `no_mrz_found`
(in-denominator). No extraction code changed.

**The HIT count drops by one, and that is the point.** `Argentina_..._2026_mrz` was counted as a
Tier-1 **hit** (119 → 118) — with no `ocr_fixtures/` label to check against, `ocrs` had misread the
non-conforming printed zone into a *different*, self-consistent, checksum-valid string, and the
final classifier rung had no ground truth to contradict it. This is the same failure the
[denominator correction](denominator-correction-2026-09-09.md) named for MRZ-less fronts — *"a
checksum-valid MRZ returned for a document that has none was counted as a hit, because such
documents are unlabelled by construction"* — one step along: a checksum-valid MRZ misread off a
document whose real printed zone can never be one. Adding the ground truth removes the spurious
hit. The honest scored rate is **118 / 142 = 83.1%**.

## What did not move, and why

The user's hand-read flagged five candidates. Only Argentina (×2) is acted on here:

- **`Croatia_ID_Specimen_2021_back_mrz`** — its all-zeros printed zone **validates**
  (`mrz::find_and_parse(...).valid() == true`; all-zeros with `0` check digits is ICAO-conforming).
  A faithful OCR read of it would be a Tier-1 hit, so the current miss is a genuine OCR misread,
  not a specimen artifact. It **stays in the denominator** as a `checksum_failed` recognition miss.
- **`Sweden_ID_Specimen_2027_mrz`** — plausibly a correct refusal (no readable MRZ), which would be
  `no_mrz_expected`. But `corpus_manifest.rs::recorded_mrz_presence_matches_the_filename_tag`
  forces `mrz.present` to match the `_mrz` filename tag, so a manifest-only flip fails CI — it
  needs the image renamed `_no_mrz` on the `samples-data` branch. Deferred.
- **`Egypt_Passport_Specimen_P0_EGY_2012_mrz`** — the MRZ is fully X-redacted. Belongs in
  `redacted_mrz`, but `redacted` is derived from the `redacted` token in the filename
  (`provider_bench.rs`, and `recorded_redaction_matches_the_filename_tag`), so it needs the image
  renamed `_redacted_mrz` on `samples-data`. Deferred.
- **`Moldova_Passport_Specimen_PA_MDA_2014_mrz`** — `corpus.jsonl` records a maintainer judgement
  (2026-09-04) that it is a genuine older Moldovan passport format, and its `_wide` variant
  validates cleanly. Reclassifying reverses a recorded decision; left for the maintainer to
  reconcile.

Genuine detector-movable miss set after this pass: **~22** (the 25 in-denominator misses minus
Argentina ×2 minus the Sweden refusal once the rename lands). Croatia stays counted.

## Re-baseline

`Argentina ×2` moving off the denominator: `scored` 144 → 142, `tier1_hits` 119 → 118 (the spurious
hit above), `no_mrz_found` 18 → 17, `checksum_failed_specimen` 16 → 18, `checksum_failed` unchanged
at 7. Regenerated via CI (`gh workflow run real-specimen-gate.yml -f mode=write-baseline`), never by
hand; `README.md` and this directory's headline follow it.
