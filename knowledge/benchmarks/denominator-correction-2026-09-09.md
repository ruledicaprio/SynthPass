# The Tier-1 denominator counted documents that could never be read

**2026-09-09.** Reconnaissance for [`ADR-0008`](../decisions/ADR-0008-mrz-detection-track.md)'s
first chunk, before any OCR measurement was taken.

The published Tier-1 hit rate was **119 / 229 = 52.0%**, and the miss it named as dominant was
`no_mrz_found` at 85 — the number the whole detection track was aimed at. Both were wrong, in the
same direction, for the same reason: **94 of the 229 scored specimens cannot produce a Tier-1 hit
no matter how good the pipeline gets**, and every one of them was being counted as a failure to
produce one.

Nothing in the extraction path changed. The HIT count is identical before and after.

## The three populations

| Population | n | Why no pipeline can hit it | How it used to score |
| :-- | --: | :-- | :-- |
| **No MRZ at all** | 42 | ID-card fronts, border passes, driving-license faces. There is no zone on the document. | `no_mrz_found` — a *correct refusal* counted as a detection failure |
| **MRZ redacted** | 36 | The zone is blacked out by whoever published the specimen. | 9 scored out, **27** counted as detection failures |
| **Printed zone non-conforming** | 16 | `TEMPLATE` / `ÖRNEK` / `VZOR` / all-zeros zones whose own ICAO check digits fail. A byte-perfect read still fails. | 1 scored out, **15** counted as OCR failures |

Each had a written justification in this repo already. `MissReason::Redacted`'s own doc says a
redacted read is "neither an OCR-accuracy signal nor something a better parser could recover". The
[2026-09-08 `checksum_failed` writeup](checksum-failed-real-specimens-2026-09-08.md) says of the 16
non-conforming specimens, in as many words, that they "belong outside the denominator". Neither
conclusion had been implemented.

## Two of the three were decided by OCR noise

This is the part worth keeping, because it is a class of bug rather than an off-by-42.

**Redaction.** The gate sat *after* the `mrz_found` check, so a redacted specimen was scored out
only if its blackout bar happened to OCR into something `mrz::find_and_parse` could shape into a
zone. Whether that happens is a property of the bar's texture. The consequence ran backwards: **the
cleaner the redaction, the worse the document scored**, because a bar that produced no parseable
noise was indistinguishable from a passport whose MRZ we had failed to find.

**Non-conformance.** `checksum_failed_specimen` required that OCR recover the hand transcription
*byte for byte*. That conflates a fact about the specimen with an achievement of the run. A document
whose printed zone is malformed **and** which OCR misread answered "no" and was filed as an ordinary
OCR failure — 15 of the 16, scored against a hit that was never reachable.

In both cases the classification asked what the run returned before asking what the document made
possible. The rungs are now ordered the other way round.

### This reverses a deliberate decision, not an oversight

The redaction ordering was argued for in writing. The
[2026-09-08 writeup](checksum-failed-real-specimens-2026-09-08.md) says of the 27:

> The other 27 redacted specimens were already `no_mrz_found` (their bar OCR'd as junk in all three
> structural fields, caught by `mrz` 0.7.0's gate) and stay there — the new rung sits after the
> `mrz_found` gate on purpose.

That is defensible on its own terms: "no MRZ was found" is *literally* true of those documents. The
reason it is overturned here is that the metric is not asking whether text was found — it is asking
**whether our detector can find MRZs that exist**, and a blacked-out zone does not exist to be
found. Under the old ordering the same document could change buckets between runs on OCR noise
alone, and the 27 sat inside the target metric of a track specifically aimed at reducing it. The
same writeup calls redacted specimens "unrecoverable by construction" three sections earlier, which
is the position taken here.

### One redacted specimen reads perfectly, and is now scored out

`Malaysia_Passport_Specimen_P0_MYS_2019_redacted_mrz_blur` returns a checksum-valid TD3 whose
document code and issuing state **agree** with ground truth. Passing every ICAO check digit by
chance is vanishingly unlikely, so the evidence says its MRZ is readable and the `redacted` tag
describes something else on the page — the same shape as
`Monaco_ID_Specimen_XXXX_front_no_mrz.png`, which was reported as a Tier-1 false positive and
turned out to be a filing error ([trust the data over the label](README.md)).

It was already excluded before this change (the 2026-09-08 rung caught it), so nothing regressed
here — but excluding it costs a genuine hit if the tag is wrong. `provider-bench` now prints a
warning naming any such specimen, rather than leaving the question to whoever next reads the JSON.
Resolving it means looking at the image; it is one document and it is not resolved here.

## A hallucinated MRZ was scored as a hit

Found while fixing the first population, and the most serious thing here.

A document with no MRZ has no `samples/ocr_fixtures/` label — that population is precisely the
unlabelled one. So a checksum-**valid** read off one passed the found gate, passed the checksum
gate, reached the document-number rung, found no ground truth to be compared against, and fell out
of the chain with `miss_reason: None`. That is the harness's spelling of **HIT**.

Confirmed by removing the new gate and re-running:

```text
assertion `left == right` failed: a hallucinated MRZ must never be counted as a Tier-1 hit
  left: None
 right: Some("false_positive_mrz")
```

`Monaco_ID_Specimen_XXXX_front_no_mrz.png` really did return a checksum-valid TD1 once. It turned
out to be a mislabelled file rather than a hallucination — the read was correct and the *name* was
wrong ([`trust-the-data-over-the-label`](README.md)) — but that only became knowable because
someone noticed. The harness said nothing. It now reports `false_positive_mrz`, inside the
denominator, in the regression buckets, and printed as a warning rather than a count.

The current corpus has **zero**.

## Effect

Measured by `provider-bench --real-specimens --mrz-only`, CI-written baseline.

| | before | after |
| :-- | --: | --: |
| documents | 238 | 238 |
| scored (denominator) | 229 | **144** |
| Tier-1 HIT | 119 | **119** |
| **hit rate** | **52.0%** | **82.6%** |
| `no_mrz_found` | 85 | **18** |
| `checksum_failed` | 24 | **7** |
| `no_mrz_found` : `checksum_failed` | 3.5 : 1 | **2.6 : 1** |

The two ratios matter more than the rate. Detection remains the dominant miss, so ADR-0008's
decision survives — but its target is **18 documents, not 85**, and roughly four fifths of the
number it was aimed at could never have moved. A detector that fixed every genuine detection
failure would have taken the old metric from 52.0% to 60.3% and looked like a failure.

## What this does not claim

The corrected rate is **not** an improvement in extraction. It is the same 119 documents over an
honest denominator. Both numbers are published for that reason — see
[`README.md`](README.md#current-headline-numbers): the corpus-level figure says what fraction of a
pile of real specimens yields a record, and the engineering figure says how often we succeed when
success is possible. Only the second one moves when accuracy work lands, and only the first one
answers "what happens if I point this at my drawer of documents".

## The 18

The genuine detection failures, for whoever picks up ADR-0008's measurement:

```text
Argentina P0_ARG_2021_mrz_child     Italy ID 2022 back                Portugal PX_PRT_2017
Argentina P0_ARG_2026_mrz_blur      Kuwait P0_KWT_2023                Russian_Federation P0_RUS_2014
Canada PP_CAN_2023                  Moldova PA_MDA_2014               Sweden ID 2027
Egypt P0_EGY_2012                   Monaco ID XXXX back               Vietnam P0_VNM_2023
Finland P0_FIN_2007                 Netherlands Driving_License
Finland P0_FIN_2023                 Oman P0_OMN_2004
France ID 2020 back                 India P0_IND_2024
```

**Most of these are documents the browser demo already reads.**
[`WEB_OCR_BASELINE.md`](../WEB_OCR_BASELINE.md) lists its browser-only wins as "Canada, Czechia,
Finland (×2), India (×2), Kuwait, Oman, Portugal (×2), Romania, Russia, Vietnam" plus ID-card backs
"Belgium, Croatia, France, Monaco, Sweden, Switzerland" — an overlap of well over half this list.
Six of the 18 are three-line TD1 ID-card backs, against a native band search built around the
two-line TD3 case.

That is a far sharper lead than "85 documents, cause unknown", and it is what the ADR-0008
measurement should be pointed at.
