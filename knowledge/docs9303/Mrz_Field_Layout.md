# ICAO 9303 MRZ Field Layout — Machine-Actionable Specification

Scope: field positions, lengths, check digits and invariants for the five MRZ
formats handled by SynthPass: TD1, TD2, TD3, MRV-A, MRV-B.

- The JSON block in §4 is the NORMATIVE source of truth.
- The tables in §3 are human renderings of that JSON; on conflict, JSON wins.
- Colour coding from the legacy diagram is presentation-only and not part of this spec.

## 1. Reading contract

| Convention | Rule |
| --- | --- |
| `pos` | 1-based, inclusive at both ends (ICAO convention). |
| `slice` | 0-based, end-exclusive `[start, end)` — usable verbatim in Rust/Python/JS. |
| Charset | `A–Z`, `0–9`, `<` (OCR-B). Nothing else may appear. |
| Filler | `<` pads unused positions; `<<` separates primary/secondary name parts; single `<` separates components within a part. |
| Dates | `YYMMDD`; century resolution is policy, not parse logic. |
| Keys | Stable `snake_case` identifiers; code must use these names. |
| Legacy mapping | `H` → `*_check_digit`; `FH` → `composite_check_digit`; `S` → `sex`. |

## 2. Check digits

### 2.1 Algorithm (7-3-1, modulus 10)
Char values: `0–9` → 0–9; `A–Z` → 10–35; `<` → 0.
Multiply values by repeating weights `7,3,1,…`; sum; digit = sum mod 10.
(Example: `L898902C3` → 316 mod 10 = `6`.)

### 2.2 Composite coverage (concatenate ranges in order, then apply §2.1)

| Format | Composite at | Covered ranges | Explicitly excluded |
| --- | --- | --- | --- |
| TD1 | line 2, pos 30 | line 1 [6–30]; line 2 [1–7], [9–15], [19–29] | name (line 3), sex, nationality |
| TD2 | line 2, pos 36 | line 2 [1–10], [14–20], [22–35] | nationality, sex |
| TD3 | line 2, pos 44 | line 2 [1–10], [14–20], [22–43] | nationality, sex |
| MRV-A / MRV-B | — | none (no composite digit) | — |

### 2.3 Issuer options (parsers must accept; generators must not emit ambiguously)
- TD3 unused personal number: pos 43 may be `0` or `<`.
- Document numbers > 9 chars use the format-specific continuation rules (ICAO 9303 Parts 4–6); apply before interpreting optional data.

## 3. Field tables (rendering of §4)

### 3.1 TD1 — 3 × 30

| **Line** | **pos** | **slice** | **key** |
|---|---|---|---|
| 1 | 1-2 | [0,2) | document_code |
| 1 | 3–5 | [2,5) | issuing_country |
| 1 | 6–14 | [5,14) | document_number |
| 1 | 15 | [14,15) | document_number_check_digit |
| 1 | 16–30 | [15,30) | optional_data_1 |
| 2 | 1–6 | [0,6) | date_of_birth |
| 2 | 7 | [6,7) | date_of_birth_check_digit |
| 2 | 8 | [7,8) | sex (`F`,`M`,`<`) |
| 2 | 9–14 | [8,14) | date_of_expiry |
| 2 | 15 | [14,15) | date_of_expiry_check_digit |
| 2 | 16–18 | [15,18) | nationality |
| 2 | 19–29 | [18,29) | optional_data_2 |
| 2 | 30 | [29,30) | composite_check_digit |
| 3 | 1–30 | [0,30) | name |

### 3.2 TD2 — 2 × 36

| **Line** | **pos** | **slice** | **key** |
|---|---|---|---|
| 1 | 1–2 | [0,2) | document_code |
| 1 | 3–5 | [2,5) | issuing_country |
| 1 | 6–36 | [5,36) | name |
| 2 | 1–9 | [0,9) | document_number |
| 2 | 10 | [9,10) | document_number_check_digit |
| 2 | 11–13 | [10,13) | nationality |
| 2 | 14–19 | [13,19) | date_of_birth |
| 2 | 20 | [19,20) | date_of_birth_check_digit |
| 2 | 21 | [20,21) | sex |
| 2 | 22–27 | [21,27) | date_of_expiry |
| 2 | 28 | [27,28) | date_of_expiry_check_digit |
| 2 | 29–35 | [28,35) | optional_data |
| 2 | 36 | [35,36) | composite_check_digit |

### 3.3 TD3 — 2 × 44

| Line | pos | slice | key |
|---|---|---|---|
| 1 | 1–2 | [0,2) | document_code |
| 1 | 3–5 | [2,5) | issuing_country |
| 1 | 6–44 | [5,44) | name |
| 2 | 1–9 | [0,9) | document_number |
| 2 | 10 | [9,10) | document_number_check_digit |
| 2 | 11–13 | [10,13) | nationality |
| 2 | 14–19 | [13,19) | date_of_birth |
| 2 | 20 | [19,20) | date_of_birth_check_digit |
| 2 | 21 | [20,21) | sex |
| 2 | 22–27 | [21,27) | date_of_expiry |
| 2 | 28 | [27,28) | date_of_expiry_check_digit |
| 2 | 29–42 | [28,42) | personal_number |
| 2 | 43 | [42,43) | personal_number_check_digit |
| 2 | 44 | [43,44) | composite_check_digit |

### 3.4 MRV-B — 2 × 36

Line 1 identical to TD2 line 1. Line 2 identical to TD2 line 2 through pos 28, then:

| Line | pos | slice | key |
|---|---|---|---|
| 2 | 29–36 | [28,36) | optional_data |

No composite check digit.

### 3.5 MRV-A — 2 × 44

Line 1 identical to TD3 line 1. Line 2 identical to TD3 line 2 through pos 28, then:

| Line | pos | slice | key |
|---|---|---|---|
| 2 | 29–44 | [28,44) | optional_data |

No personal-number check digit, no composite check digit.

## 4. Normative machine-readable layout (JSON)

```json
{
  "conventions": {
    "pos": "1-based inclusive",
    "slice": "0-based end-exclusive",
    "charset": "A-Z0-9<",
    "filler": "<"
  },
  "check_digit": {"weights": [7, 3, 1], "modulus": 10, "values": {"<": 0, "A": 10, "Z": 35}},
  "formats": {
    "TD1": {
      "icao_part": 5, "lines": 3, "line_length": 30,
      "fields": [
        {"line":1,"pos":[1,2],"slice":[0,2],"key":"document_code"},
        {"line":1,"pos":[3,5],"slice":[2,5],"key":"issuing_country"},
        {"line":1,"pos":[6,14],"slice":[5,14],"key":"document_number"},
        {"line":1,"pos":[15,15],"slice":[14,15],"key":"document_number_check_digit","check_for":"document_number"},
        {"line":1,"pos":[16,30],"slice":[15,30],"key":"optional_data_1"},
        {"line":2,"pos":[1,6],"slice":[0,6],"key":"date_of_birth","format":"YYMMDD"},
        {"line":2,"pos":[7,7],"slice":[6,7],"key":"date_of_birth_check_digit","check_for":"date_of_birth"},
        {"line":2,"pos":[8,8],"slice":[7,8],"key":"sex","values":["F","M","<"]},
        {"line":2,"pos":[9,14],"slice":[8,14],"key":"date_of_expiry","format":"YYMMDD"},
        {"line":2,"pos":[15,15],"slice":[14,15],"key":"date_of_expiry_check_digit","check_for":"date_of_expiry"},
        {"line":2,"pos":[16,18],"slice":[15,18],"key":"nationality"},
        {"line":2,"pos":[19,29],"slice":[18,29],"key":"optional_data_2"},
        {"line":2,"pos":[30,30],"slice":[29,30],"key":"composite_check_digit"},
        {"line":3,"pos":[1,30],"slice":[0,30],"key":"name"}
      ],
      "composite": {"at":{"line":2,"pos":[30,30]},
        "covers":[{"line":1,"pos":[6,30]},{"line":2,"pos":[1,7]},{"line":2,"pos":[9,15]},{"line":2,"pos":[19,29]}]}
    },
    "TD2": {
      "icao_part": 6, "lines": 2, "line_length": 36,
      "fields": [
        {"line":1,"pos":[1,2],"slice":[0,2],"key":"document_code"},
        {"line":1,"pos":[3,5],"slice":[2,5],"key":"issuing_country"},
        {"line":1,"pos":[6,36],"slice":[5,36],"key":"name"},
        {"line":2,"pos":[1,9],"slice":[0,9],"key":"document_number"},
        {"line":2,"pos":[10,10],"slice":[9,10],"key":"document_number_check_digit","check_for":"document_number"},
        {"line":2,"pos":[11,13],"slice":[10,13],"key":"nationality"},
        {"line":2,"pos":[14,19],"slice":[13,19],"key":"date_of_birth","format":"YYMMDD"},
        {"line":2,"pos":[20,20],"slice":[19,20],"key":"date_of_birth_check_digit","check_for":"date_of_birth"},
        {"line":2,"pos":[21,21],"slice":[20,21],"key":"sex","values":["F","M","<"]},
        {"line":2,"pos":[22,27],"slice":[21,27],"key":"date_of_expiry","format":"YYMMDD"},
        {"line":2,"pos":[28,28],"slice":[27,28],"key":"date_of_expiry_check_digit","check_for":"date_of_expiry"},
        {"line":2,"pos":[29,35],"slice":[28,35],"key":"optional_data"},
        {"line":2,"pos":[36,36],"slice":[35,36],"key":"composite_check_digit"}
      ],
      "composite": {"at":{"line":2,"pos":[36,36]},
        "covers":[{"line":2,"pos":[1,10]},{"line":2,"pos":[14,20]},{"line":2,"pos":[22,35]}]}
    },
    "TD3": {
      "icao_part": 4, "lines": 2, "line_length": 44,
      "fields": [
        {"line":1,"pos":[1,2],"slice":[0,2],"key":"document_code"},
        {"line":1,"pos":[3,5],"slice":[2,5],"key":"issuing_country"},
        {"line":1,"pos":[6,44],"slice":[5,44],"key":"name"},
        {"line":2,"pos":[1,9],"slice":[0,9],"key":"document_number"},
        {"line":2,"pos":[10,10],"slice":[9,10],"key":"document_number_check_digit","check_for":"document_number"},
        {"line":2,"pos":[11,13],"slice":[10,13],"key":"nationality"},
        {"line":2,"pos":[14,19],"slice":[13,19],"key":"date_of_birth","format":"YYMMDD"},
        {"line":2,"pos":[20,20],"slice":[19,20],"key":"date_of_birth_check_digit","check_for":"date_of_birth"},
        {"line":2,"pos":[21,21],"slice":[20,21],"key":"sex","values":["F","M","<"]},
        {"line":2,"pos":[22,27],"slice":[21,27],"key":"date_of_expiry","format":"YYMMDD"},
        {"line":2,"pos":[28,28],"slice":[27,28],"key":"date_of_expiry_check_digit","check_for":"date_of_expiry"},
        {"line":2,"pos":[29,42],"slice":[28,42],"key":"personal_number"},
        {"line":2,"pos":[43,43],"slice":[42,43],"key":"personal_number_check_digit","check_for":"personal_number"},
        {"line":2,"pos":[44,44],"slice":[43,44],"key":"composite_check_digit"}
      ],
      "composite": {"at":{"line":2,"pos":[44,44]},
        "covers":[{"line":2,"pos":[1,10]},{"line":2,"pos":[14,20]},{"line":2,"pos":[22,43]}]}
    },
    "MRV-B": {
      "icao_part": 7, "lines": 2, "line_length": 36,
      "fields": [
        {"line":1,"pos":[1,2],"slice":[0,2],"key":"document_code"},
        {"line":1,"pos":[3,5],"slice":[2,5],"key":"issuing_country"},
        {"line":1,"pos":[6,36],"slice":[5,36],"key":"name"},
        {"line":2,"pos":[1,9],"slice":[0,9],"key":"document_number"},
        {"line":2,"pos":[10,10],"slice":[9,10],"key":"document_number_check_digit","check_for":"document_number"},
        {"line":2,"pos":[11,13],"slice":[10,13],"key":"nationality"},
        {"line":2,"pos":[14,19],"slice":[13,19],"key":"date_of_birth","format":"YYMMDD"},
        {"line":2,"pos":[20,20],"slice":[19,20],"key":"date_of_birth_check_digit","check_for":"date_of_birth"},
        {"line":2,"pos":[21,21],"slice":[20,21],"key":"sex","values":["F","M","<"]},
        {"line":2,"pos":[22,27],"slice":[21,27],"key":"date_of_expiry","format":"YYMMDD"},
        {"line":2,"pos":[28,28],"slice":[27,28],"key":"date_of_expiry_check_digit","check_for":"date_of_expiry"},
        {"line":2,"pos":[29,36],"slice":[28,36],"key":"optional_data"}
      ],
      "composite": null
    },
    "MRV-A": {
      "icao_part": 7, "lines": 2, "line_length": 44,
      "fields": [
        {"line":1,"pos":[1,2],"slice":[0,2],"key":"document_code"},
        {"line":1,"pos":[3,5],"slice":[2,5],"key":"issuing_country"},
        {"line":1,"pos":[6,44],"slice":[5,44],"key":"name"},
        {"line":2,"pos":[1,9],"slice":[0,9],"key":"document_number"},
        {"line":2,"pos":[10,10],"slice":[9,10],"key":"document_number_check_digit","check_for":"document_number"},
        {"line":2,"pos":[11,13],"slice":[10,13],"key":"nationality"},
        {"line":2,"pos":[14,19],"slice":[13,19],"key":"date_of_birth","format":"YYMMDD"},
        {"line":2,"pos":[20,20],"slice":[19,20],"key":"date_of_birth_check_digit","check_for":"date_of_birth"},
        {"line":2,"pos":[21,21],"slice":[20,21],"key":"sex","values":["F","M","<"]},
        {"line":2,"pos":[22,27],"slice":[21,27],"key":"date_of_expiry","format":"YYMMDD"},
        {"line":2,"pos":[28,28],"slice":[27,28],"key":"date_of_expiry_check_digit","check_for":"date_of_expiry"},
        {"line":2,"pos":[29,44],"slice":[28,44],"key":"optional_data"}
      ],
      "composite": null
    }
  }
}
```