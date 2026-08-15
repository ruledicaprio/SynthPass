# duobango_mrz_field_reference.md

Distilled from Doubango's `ultimateMRZ-SDK` docs, MRZ parser page:
<https://www.doubango.org/SDKs/mrz/docs/MRZ_parser.html>. Kept as a
**secondary cross-check on a third-party SDK's field layout, not a source of
truth** — ICAO 9303 remains canonical (`docs/icao9303-source-of-truth`). The
raw scrape this was distilled from has been deleted; re-fetch the URL above
if the source page needs rechecking.

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
