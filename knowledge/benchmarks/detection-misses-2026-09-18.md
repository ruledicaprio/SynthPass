# Three detection misses attributed: two obscured TD1 bands and one decorative zone

**Date:** 2026-09-18 · **MAIN:** `b392014` · **DATA:** `5988c7b` · **Evidence:** Observed · **Status:** current

The three detection misses frozen by [ADR-0011](../decisions/ADR-0011-split-m6-packaging-into-m8.md)'s
2026-09-17 amendment have been inspected. This entry supplies the dated attributions that its
Definition of Done accepts in place of a Tier-1 hit.

## France ID 2020 back

`id_cards/France_ID_Specimen_2020_back_mrz.png` carries a conforming TD1 zone:

```text
IDFRAX4RTBPFW46<<<<<<<<<<<<<<<
9007138F3002119FRA<<<<<<<<<<<6
MARTIN<<MAELYS<GAELLE<MARIE<<<
```

All four printed check digits were independently verified against ICAO 7-3-1 weights: document
number `X4RTBPFW4` → `6`; date of birth `900713` → `8`; expiry `300211` → `9`; composite → `6`.
The document number is also printed vertically on the card's right edge, and the expiry agrees
with `11 02 2030` in the ghost portrait.

The mechanism is a **preprocessing variant gap**, not background clutter alone. Dense micro-text
from the Declaration of the Rights of Man, stars, and other graphics run through the band, but the
band is detectable. The [Phase D native-versus-browser
report](phase-d-native-vs-browser-2026-09-18.md), from `web-ocr.yml` run `35169813105` on the same
bytes and population, records native OCR stopping on its retry budget with no MRZ at `72,227 ms`;
the browser (`tesseract.js`) produced a checksum-valid read on pass 3, a contrast stretch, in
`1,305 ms`. The native variant chain never tries the transform that resolves the band.

The committed [outcome ledger](real-specimen-outcomes.jsonl) records a separate native run at
`ocr_ms: 66705` with `retry_budget_hit: true`. Its 66-second budget exhaustion remains measured
evidence, while the Phase D timing belongs to its own run and is not reconciled into this value.

## Italy CIE 2022 back

`id_cards/Italy_ID_Specimen_2022_back_mrz.jpg` also carries a conforming TD1 zone:

```text
C<ITACA00000AA4<<<<<<<<<<<<<<<
6412308F2212304ITA<<<<<<<<<<<0
ROSSI<<BIANCA<<<<<<<<<<<<<<<<<
```

All four printed check digits were independently verified: document number `CA00000AA` → `4`;
date of birth `641230` → `8`; expiry `221230` → `4`; composite → `0`.

The mechanism is a **magenta FACSIMILE watermark** running diagonally across the whole card and
crossing the MRZ band.

## Moldova PA_MDA_2014

`passports/Moldova_Passport_Specimen_PA_MDA_2014_mrz.jpeg` has no machine-readable zone to
detect. The two lines printed where one would sit are:

```text
4615SACHAROV<<<NADIA<<<<<<<<<<<<<<<<<<<<<<<<<
AA08202016<<<ALIENSWITHEXTRAORDINARYSKILLS<<<08272016<<<<
```

The second line is 57 characters long, while ICAO 9303 defines 44 characters for TD3 and MRV-A,
36 for TD2 and MRV-B, and 30 for TD1. Its content is also not MRZ data: a TD3 first line starts
with the document code and issuing state (`P<MDA…`), whereas this one starts `4615`, and
`ALIENSWITHEXTRAORDINARYSKILLS` is a US visa-category string. The zone above it is a normal,
legitimate specimen; the lower zone is decorative.

Reclassifying this specimen by a `samples-data` rename to `_no_mrz`, following the San Marino
precedent, is pending and outside this documentation change. No baseline count moves here.

The inspection also found a corpus naming defect. `passports/Moldova_Passport_Specimen_PA_MDA_2014_mrz_wide.jpg`
shares the stem but is a different document: it is CIOBANAȘ / RITA, born 17.07.1981, with a valid
TD3 zone that the bench already scores as a hit; the base image is SACHAROV / NADIA, born
20.08.1988. They share a template and issue/expiry dates, not holder data. Elsewhere `_wide`
means a wider crop of the same image, so this needs either a rename or manifest `notes`; neither
is changed here.

## M6 consequence

None of these three is a recognition defect. Italy and Moldova close by explanation: the former
has a watermark across its band and the latter has no band. France remains an open, actionable
preprocessing target: a contrast-stretch pass is a named candidate, but it has not been tried in
the native pipeline. The browser result makes that candidate worth testing; it does not establish
that the fix works here.