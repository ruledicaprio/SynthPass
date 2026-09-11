# ADR-0008 chunk 2 — the orientation fix, measured (2026-09-12)

Chunk 2 of [ADR-0008](../decisions/ADR-0008-mrz-detection-track.md) set out to recover the
documents chunk 1 attributed to native page orientation
([`ocr-stack-gap-attribution-2026-09-10.md`](ocr-stack-gap-attribution-2026-09-10.md)), as a
three-layer contract: discrete orientation, continuous skew, one final resample. Two layers
shipped. Three designs were measured and rejected along the way, and the third layer was
dropped before it was built. All of that is recorded here, because the rejections cost more to
learn than the fixes did.

## Result

Same corpus (254 documents, 157 scored), CI gate reports before and after:

| | before (`87e8223`, 2026-09-10) | after (`715538a`, 2026-09-11) |
| :-- | --: | --: |
| Tier-1 hits | 128 / 157 = 81.5% | **143 / 157 = 91.1%** |
| `no_mrz_found` | 21 | **7** |
| `checksum_failed` | 8 | **7** |
| off-denominator (no zone / redacted / non-conforming) | 97 | 97 |

**Fifteen documents flipped, every one toward HIT; none flipped away.** The diff is taken
index-by-index over the two reports' `documents_detail`, not by name: three stems appear twice
in the corpus with different extensions, and a name-keyed diff undercounts both sides by two
and could hide a flip among them.

| Document | Before | Source size | Recovered by |
| :-- | :-- | :-- | :-- |
| Angola `PN_AGO_2026` (sideways spread) | `no_mrz_found` | 852×629 | layer 1 |
| Pakistan `P0_PAK_2024` (sideways) | `no_mrz_found` | 531×765 | layer 1 |
| Canada `PP_CAN_2023` | `no_mrz_found` | 389×256 | layer 1 |
| Dominican Republic `P0_DOM_2020` | `no_mrz_found` | 571×763 | layer 1 |
| Finland `P0_FIN_2007` | `no_mrz_found` | 348×485 | layer 1 |
| Finland `P0_FIN_2023` | `no_mrz_found` | 371×269 | layer 1 |
| India `P0_IND_2024` | `no_mrz_found` | 381×262 | layer 1 |
| Kuwait `P0_KWT_2023` | `no_mrz_found` | 502×379 | layer 1 |
| Monaco ID back | `no_mrz_found` | 254×162 | layer 1 |
| Nepal `P0_NPL_2019` | `no_mrz_found` | 526×749 | layer 1 |
| Oman `P0_OMN_2004` | `no_mrz_found` | 600×423 | layer 1 |
| Portugal `PX_PRT_2017` | `no_mrz_found` | 379×262 | layer 1 |
| Russia `P0_RUS_2014` | `no_mrz_found` | 466×658 | layer 1 |
| Vietnam `P0_VNM_2023` | `no_mrz_found` | 600×439 | layer 1 |
| India `P0_IND_2022` | `checksum_failed` | 877×560 | layer 2 |

Chunk 1 named eleven orientation victims. Ten are HITs above; the eleventh, Argentina
`P0_ARG_2026_mrz_blur`, prints a zone that fails its own check digits and is scored out
([`denominator-bucket-a-2026-09-10.md`](denominator-bucket-a-2026-09-10.md)), so no pipeline can
make it one. The three genuinely sideways books ingested for this chunk (Angola, Pakistan, and
Indonesia, which already read) all read. Both groups are pinned by name in
`crates/synthpass-ocr/tests/native_ocr_rotation.rs`, because the CI gate compares totals and a
change that loses one of them while gaining any other document would pass it.

## Per layer

**Layer 1 — discrete orientation (#262).** The upfront `choose_rotation` vote no longer runs on
the default path. The two quarter-turns became the outermost retry tier, where an ICAO check
digit decides instead of an aspect-ratio score, and EXIF orientation is applied losslessly at
decode. Same-binary three-arm A/B on the local corpus (155 scored; two documents short of CI's):

| `SYNTHPASS_OCR_ROTATE` | hits | vs `legacy` |
| :-- | --: | :-- |
| `legacy` (the old vote) | 126 | — |
| `off` (no rotation at all) | 137 | +12, **−1** |
| **`default`** | **140** | **+14, −0** |

The `off` arm's −1 is Indonesia, a document the retired vote got right. Neither half ships
alone: the vote costs twelve documents, and removing it without the quarter-turn tier costs
Indonesia.

**Layer 2 — continuous skew (#269).** A projection-profile estimator (±10° at 0.5°, no
per-angle resampling) replaced the 17-rotation contrast search, and **both angles now run** as
trailing variants whenever they disagree. Same-binary A/B: `legacy` 142 → `default` **143**,
+1/−0 (India 2022). The control arm reproduced the previous binary byte-for-byte, which is what
shows the retry-budget increase moved nothing by itself.

**Layer 3 — one resample, conditional upscale.** Not built; see below.

## What was measured and rejected

1. **A detected-text confidence floor on the orientation vote** (the approved layer-1 design).
   A 16-document sweep refuted its premise. Detection is healthy on the victims (27–88 words at
   0°, a fifth to a third of the page area), so no floor on detection strength can separate
   them. Word count runs *inversely* to correctness, because a horizontal-text detector
   fragments sideways text into more boxes: Angola detects 192 words wrongly oriented and 97
   correctly. The winning-ratio ranges overlap completely (victims 1.23–2.98, correct
   rotations 1.13–1.48), so no margin separates them either.
2. **The new skew estimator on its own** (layer 2 as first written): +1/−1 against the legacy
   search. It gains India 2022 (band tilted about 1.5°, variance margin +18%) and loses a Swiss
   ID-card back, whose 232×57 band gives a flat objective (+0.00° at a +0.00% margin). The loss
   reproduced on an idle machine, so it is not contention. Neither estimator is wrong. They
   optimise different objectives on an input that barely has one, so the shipped design runs
   both rather than tuning either.
3. **A text-height gate on the band upscale** (layer 3's D2). Dropped on arithmetic plus one
   measurement, before any A/B. Both `ocrs` models fix their input geometry, read from the
   shipped `.rten` files: detection runs on exactly **800×600** (height × width), padding
   smaller inputs rather than scaling them, and recognition resizes each line to a fixed **64 px**
   height. `BAND_MIN_WIDTH` only upscales bands narrower than 1600 px. At 1600 px the MRZ text
   rows measure:

   | Band (blind bottom crop, upscaled) | Width | MRZ row heights |
   | :-- | --: | :-- |
   | Canada, Finland ×2, India 2024, Oman, Portugal (TD3) | 1600 | 33–44 px |
   | Monaco ID back (TD1; the 5× cap stops it short) | 1270 | 45–48 px |

   Every upscale that fires is below recognition's 64 px, so skipping it swaps Lanczos for
   `ocrs`'s own bilinear resize rather than saving one. Detection cost does not depend on input
   size, so the only saving on offer was the resize itself.
4. **A type-level "rotate once" guard** (layer 3's D1). Not built, because visibility already
   gives the guarantee. The one continuous resample (`preprocess::rotate_rgb`) is private with a
   single caller, and every buffer that caller receives is a fresh band crop of a page that has
   only ever been quarter-turned. A wrapper type would restate that.

**ADR-0008's third licensed lever, "upscale before the general detection pass," was not built
either.** The premise holds: `ocrs` pads any page shorter than 800 px or narrower than 600 px
rather than enlarging it, so the general pass on most of this corpus detects at native size.
But the retry chain already carries the upscaled full page (`upscale_to_min_dim` to a 1000 px
short side) as its third variant, and every miss runs that far. An upscaled general pass would
repeat a pass all fourteen remaining misses already get. The population chunk 1 tied this lever
to, sub-300 px scans that stayed lost even when oriented correctly, is gone: Monaco, Canada,
Finland 2023, India 2024 and Portugal are all in the table above. Whether the lever would save
passes on documents that already read is a speed question, which belongs to
[ADR-0010](../decisions/ADR-0010-benchmark-cost-split-by-role.md).

## What is left

Fourteen scored misses, **tied seven and seven**. Detection no longer outnumbers character
accuracy, down from 2.6 : 1 when ADR-0008 opened.

- `no_mrz_found` (7): France ID 2020 back, Italy ID 2022 back, Sweden ID 2027, Netherlands
  driving licence, Argentina `P0_ARG_2021_mrz_child`, Egypt `P0_EGY_2012`, Moldova
  `PA_MDA_2014`. For the first four the manifest records no document code or issuing state, so
  nothing verified yet says what their zone should read. They want a manifest review before
  they count as detection targets. The Netherlands licence is the sharpest case: `samples/corpus.jsonl`
  marks a zone present, while [`CORPUS_COVERAGE.md`](../CORPUS_COVERAGE.md) lists it as a driving
  licence with no MRZ. One of the two is wrong, and neither shows an ICAO 9303 zone the reader
  could validate.
- `checksum_failed` (7): Afghanistan `P0_AFG_2016`, Belgium ID 2021 back, Croatia ID 2021 back,
  Czechia `P0_CZE_2005`, Romania `PE_ROU_2024`, Russia `P0_RUS_2019`, Sweden ID 2022 back. All
  seven carry a checksum-valid printed zone
  ([`checksum-failed-real-specimens-2026-09-08.md`](checksum-failed-real-specimens-2026-09-08.md)),
  so each is a genuine character-recognition error.

**One untested hypothesis, recorded rather than acted on.** Band variants are only ever scaled
*up*. A band from a very large scan therefore reaches the 800×600 detector squeezed
horizontally far harder than a 1600 px band: France's 4584 px band arrives at 13% of its width
and its full height. France is the only `no_mrz_found` from a scan wider than 1600 px. One
document cannot justify a change on its own; it is written down so the next person who looks at
France starts from the arithmetic.
