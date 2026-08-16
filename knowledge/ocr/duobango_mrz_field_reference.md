# duobango_mrz_field_reference.md

Distilled from Doubango's `ultimateMRZ-SDK` docs, MRZ parser page:
<https://www.doubango.org/SDKs/mrz/docs/MRZ_parser.html>, plus the
check-digit section below from a sibling SDK's docs tree that republishes the
same content:
<https://www.doubango.org/SDKs/kyc-documents-verif/docs/MRZ.html>. Kept as a
**secondary cross-check on a third-party SDK's field layout, not a source of
truth** — ICAO 9303 remains canonical ([`knowledge/docs9303/`](../docs9303/)). The
raw scrapes this was distilled from have been deleted; re-fetch the URLs
above if the source pages need rechecking. (2026-08-16: mapped the rest of
doubango.org — everything else is other SDKs (face recognition, ANPR, MICR,
credit-card OCR), pricing/marketing, or the same Sphinx-templated boilerplate
repeated across doc trees; nothing else cleared the 80/20 filter.)

## Document-type detection heuristic

Doubango's C++ sample dispatches on line count + line length + first
character, worth keeping as a compact cross-check against SynthPass's own
type detection:

- 3 lines, 30 chars each → **TD1**
- 2 lines, 44 chars each → first char `P` → **TD3**, else **MRVA**
- 2 lines, 36 chars each → first char `V` → **MRVB**, else **TD2**

## Caveat: the sex-field regex is a documented bug, not a pattern to copy

Every line-2 regex below writes the sex field as `[M|F|X|<]`. That's a
character class containing the literal characters `M`, `F`, `X`, `<`, and
`|` — **not** an alternation of `M`/`F`/`X`/`<` as the author clearly
intended. It happens to work in practice only because none of the four valid
sex-field values is literally `|`. This is a bug in Doubango's own docs, not
an ICAO 9303 requirement — do not port it into anything that expects a real
alternation.

## Field boundaries by format

Each table is `group → regex fragment → meaning`, transcribed from the SDK
docs (HTML stripped). Sample data throughout is Doubango's own placeholder
identity ("ERIKSSON, ANNA MARIA", country UTO).

### TD1 (3 lines × 30 chars)

Line 1 — `([A|C|I][A-Z0-9<]{1})([A-Z]{3})([A-Z0-9<]{9})([0-9]{1})([A-Z0-9<]{15})`

| Group | Field |
|---|---|
| 1 | Document type (`A`/`C`/`I`) |
| 2 | 3-letter country code |
| 3 | Document number (up to 9 alphanumeric) |
| 4 | Check digit on document number |
| 5 | Optional data, issuing-state discretion |

Line 2 — `([0-9]{6})([0-9]{1})([M|F|X|<]{1})([0-9]{6})([0-9]{1})([A-Z]{3})([A-Z0-9<]{11})([0-9]{1})`

| Group | Field |
|---|---|
| 1 | Date of birth (YYMMDD) |
| 2 | Check digit on DOB |
| 3 | Sex |
| 4 | Date of expiry (YYMMDD) |
| 5 | Check digit on expiry |
| 6 | Nationality (3-letter code) |
| 7 | Optional data, issuing-state discretion |
| 8 | Overall check digit (lines 1+2) |

Line 3 — `([A-Z0-9<]{30})` — single group, names (`SURNAME<<GIVEN<NAMES<...`).

### TD2 (2 lines × 36 chars)

Line 1 — `([A|C|I][A-Z0-9<]{1})([A-Z]{3})([A-Z0-9<]{31})` → type, country, primary identifier (names).

Line 2 — `([A-Z0-9<]{9})([0-9]{1})([A-Z]{3})([0-9]{6})([0-9]{1})([M|F|X|<]{1})([0-9]{6})([0-9]{1})([A-Z0-9<]{7})([0-9]{1})`

| Group | Field |
|---|---|
| 1 | Document number |
| 2 | Check digit on document number |
| 3 | Nationality |
| 4 | Date of birth |
| 5 | Check digit on DOB |
| 6 | Sex |
| 7 | Date of expiry |
| 8 | Check digit on expiry |
| 9 | Optional data |
| 10 | Overall check digit |

### TD3 (2 lines × 44 chars — passports)

Line 1 — `(P[A-Z0-9<]{1})([A-Z]{3})([A-Z0-9<]{39})` → type (`P`), country, primary identifier.

Line 2 — `([A-Z0-9<]{9})([0-9]{1})([A-Z]{3})([0-9]{6})([0-9]{1})([M|F|X|<]{1})([0-9]{6})([0-9]{1})([A-Z0-9<]{14})([0-9]{1})([0-9]{1})`

Same groups 1–8 as TD2 line 2, plus:

| Group | Field |
|---|---|
| 9 | Optional data (e.g. personal number) |
| 10 | Check digit on optional data |
| 11 | Overall check digit |

### MRVA (2 lines × 44 chars — type A visas)

Line 1 — `(V[A-Z0-9<]{1})([A-Z]{3})([A-Z0-9<]{39})` → type (`V`), country, primary identifier.

Line 2 — `([A-Z0-9<]{9})([0-9]{1})([A-Z]{3})([0-9]{6})([0-9]{1})([M|F|X|<]{1})([0-9]{6})([0-9]{1})([A-Z0-9<]{16})`

Same groups 1–8 as TD2/TD3 line 2; group 9 is optional data. **No overall
check digit field** — MRVA/MRVB visas don't carry one.

### MRVB (2 lines × 36 chars — type B visas)

Line 1 — `(V[A-Z0-9<]{1})([A-Z]{3})([A-Z0-9<]{31})` → type (`V`), country, primary identifier.

Line 2 — `([A-Z0-9<]{9})([0-9]{1})([A-Z]{3})([0-9]{6})([0-9]{1})([M|F|X|<]{1})([0-9]{6})([0-9]{1})([A-Z0-9<]{8})`

Same shape as MRVA line 2, shorter optional-data field (group 9) to fit the
36-char line.

## Check-digit algorithm (secondary cross-check)

Doubango republishes the same MRZ content (field tables + a "Data validation"
section not present on the `MRZ_parser.html` page above) under a second SDK's
docs tree: <https://www.doubango.org/SDKs/kyc-documents-verif/docs/MRZ.html>.
Same site template, same underlying content — worth citing only for this one
addition, the check-digit implementation:

- **Weights**: repeating cycle `7, 3, 1` applied left to right over the
  digits being checked.
- **Character mapping**: `0`–`9` → their own value, `A`–`Z` → `10`–`35`,
  `<` → `0`.
- **Check digit**: `(sum of digit × weight) mod 10`.

Position ranges (0-indexed, inclusive) each check digit covers, per format —
useful as a cross-check against `crates/mrz`'s own offsets:

| Format | Field | Line | Positions checked | Check-digit position |
|---|---|---|---|---|
| TD1 | Document number | 0 | 5–13 | 14 |
| TD1 | Date of birth | 1 | 0–5 | 6 |
| TD1 | Date of expiry | 1 | 8–13 | 14 |
| TD1 | Composite (upper+middle lines) | 0/1 | line 0: 5–29; line 1: 0–6, 8–14, 18–28 | line 1, position 29 |
| TD2 | Document number | 1 | 0–8 | 9 |
| TD2 | Date of birth | 1 | 13–18 | 19 |
| TD2 | Date of expiry | 1 | 21–26 | 27 |
| TD2 | Composite | 1 | 0–9, 13–19, 21–34 | 35 |
| TD3 | Document (passport) number | 1 | 0–8 | 9 |
| TD3 | Date of birth | 1 | 13–18 | 19 |
| TD3 | Date of expiry | 1 | 21–26 | 27 |
| TD3 | Personal/optional number | 1 | 28–41 | 42 |
| TD3 | Composite | 1 | 0–9, 13–19, 21–42 | 43 |
| MRVA/MRVB | Document number | 1 | 0–8 | 9 |
| MRVA/MRVB | Date of birth | 1 | 13–18 | 19 |
| MRVA/MRVB | Date of expiry | 1 | 21–26 | 27 |

MRVA/MRVB have no composite check digit — consistent with the "no overall
check digit" note already above. All check digits reduce to the same ICAO
9303 modulus-10/731-weighting scheme; this table only pins down the exact
line/offset each one is computed over, per format.
