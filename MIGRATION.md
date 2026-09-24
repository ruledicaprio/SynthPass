# Migration guide: `mrz` 0.7 → 0.8, SynthPass 1.5 → 1.6

This release has two audiences, and the most important thing this guide does is keep them apart:

- **If you depend on the [`mrz`](crates/mrz/README.md) crate**, `0.8.0` is a breaking release
  (the minor slot is the breaking slot before 1.0). See [Part 1](#part-1-the-mrz-crate-07--08).
- **If you consume SynthPass's JSON** (the CLI, `synthpass-serve`, the exports, the browser
  demo), most of the `mrz` changes stop at the crate boundary. Two reach you. See
  [Part 2](#part-2-synthpass-json-15--16).

SynthPass 1.6.0 is a minor release by maintainer decision, even though two of its JSON changes are
breaking for a strict consumer. Both are listed below.

## Part 1: the `mrz` crate, 0.7 → 0.8

### 1. `Checks` distinguishes "absent" from "verified" ([ADR-0017](knowledge/decisions/ADR-0017-checks-distinguish-absent-from-verified.md))

Every `Checks` field is now `Option<bool>`:

| Value | Meaning |
| --- | --- |
| `Some(true)` | the check digit is printed, and it verifies |
| `Some(false)` | the check digit is printed, and it fails |
| `None` | this format prints no such check digit |

```rust
// 0.7
if data.checks.composite { /* ... */ }
// 0.8: an absent check is no longer a pass
if data.checks.composite == Some(true) { /* ... */ }
```

In JSON an absent check serialises as an explicit `null`; the key is never omitted. Candidate
ranking compares the fraction of *applicable* checks that verify, per format, so a layout that
prints fewer check digits is no longer scored as if the missing ones passed.

### 2. Optional data is named for what it holds ([ADR-0018](knowledge/decisions/ADR-0018-optional-data-named-for-what-it-holds.md))

The `personal_number` field is gone. The optional-data regions keep their own slots:

| Format | 0.7 | 0.8 |
| --- | --- | --- |
| TD3 | `personal_number` | `optional_data_1`, read through the `personal_number()` accessor |
| TD1 | `personal_number` = both regions joined with a space | `optional_data_1` (line 1) and `optional_data_2` (line 2), kept separate |
| TD2, MRV-A, MRV-B | `personal_number` | `optional_data_1` |

```rust
// 0.7
let pn = data.personal_number.as_deref();
// 0.8: a personal number exists only on TD3
let pn = data.personal_number();              // Option<&str>, None off TD3
let od1 = data.optional_data_1.as_deref();    // every format
let od2 = data.optional_data_2.as_deref();    // TD1 only
```

The serde keys follow the fields: `personal_number` is gone, and `optional_data_1` and
`optional_data_2` take its place. `Field::PersonalNumber` and `Checks::personal_number` keep their
names, because they name the TD3 check digit, which is unchanged.

### 3. `MrzError::BadCharacter` says where

It is a struct variant now: `BadCharacter { character, line, position }`. `line` is zero-based
and `None` when the error did not come from parsing a zone (for example the standalone
`check_digit` helper, where `position` is relative to the field).

```rust
// 0.7
Err(MrzError::BadCharacter(c)) => { /* ... */ }
// 0.8
Err(MrzError::BadCharacter { character, line, position, .. }) => { /* ... */ }
```

Two more `MrzError` payloads changed. Both matter only if you match on the values:

- **Line length is counted in characters, not bytes.** In 0.7, a 44-character TD3 line holding
  `É` was 45 bytes and failed as `BadLength { expected: 44, got: 45 }`. In 0.8 its length is
  right, so it fails as `BadCharacter { character: 'É', line: Some(0), position }`, which names
  the character that is actually wrong. `BadLength.got` counts characters too.
- **`BadDocumentCode` carries both raw code cells on every format.** On TD1 and TD2, 0.7 trimmed
  the trailing filler (`"Z"`), while TD3, MRV-A and MRV-B did not (`"Z<"`). In 0.8 all five
  formats report `"Z<"`.

### 4. `ParseOptions`, `Date` and `DateValidity` are `#[non_exhaustive]`

Only `ParseOptions` needs a caller change. Build it with the builder, because a struct literal is
rejected from another crate, and so is functional-update syntax (`E0639`):

```rust
// 0.7
let opts = ParseOptions { pivot_yy: 30 };
// 0.8
let opts = ParseOptions::default().with_pivot_yy(30);
```

With the `serde` feature, `ParseOptions` JSON written by 0.7 (`{"pivot_yy":30}`) still
deserializes. The new `class_sweep` field defaults to `false`.

### 5. Dates and sex are typed ([ADR-0019](knowledge/decisions/ADR-0019-typed-values-on-mrzdata.md), [ADR-0020](knowledge/decisions/ADR-0020-mrz-value-wire-contract.md))

`MrzData::date_of_birth` and `date_of_expiry` are `MrzDate`, and `MrzData::sex` is `Sex`.
`date_of_birth_completeness` is removed, because the date carries its own kind.

| `MrzDate` variant | The six zone characters |
| --- | --- |
| `Calendar(Date)` | six digits naming a real day |
| `OutOfCalendar(Date)` | six digits that name no day (a `000000` placeholder) |
| `PartiallyUnknown(RawDateField)` | digits mixed with fillers (`74<<12`) |
| `Unknown` | all fillers (`<<<<<<`) |
| `Malformed(RawDateField)` | anything else in the MRZ alphabet |

| `Sex` variant | Zone character |
| --- | --- |
| `Male` / `Female` | `M` / `F` |
| `Unspecified` | `<` |
| `NonConformant(char)` | anything else, kept as read (`1`, `0`, `S`) |

```rust
// 0.7
if data.date_of_birth == "1974-08-12" && data.sex == "F" { /* ... */ }
let kind = data.date_of_birth_completeness;
// 0.8
if data.date_of_birth.to_string() == "1974-08-12" && data.sex == Sex::Female { /* ... */ }
let kind = data.date_of_birth.completeness();
let day: Option<Date> = data.date_of_birth.calendar();   // Some only for a real day
```

**The JSON wire** ([ADR-0020](knowledge/decisions/ADR-0020-mrz-value-wire-contract.md)):

- Dates keep their text form: `YYYY-MM-DD` for the two six-digit kinds, the six raw characters
  otherwise.
- `date_of_birth_completeness` is gone.
- `sex` is the zone's own character, `"<"` for unspecified. In 0.7, both unspecified and a
  misread cell arrived as `"X"`.
- **Old JSON still deserialises**, but a 0.7 `"X"` reads back as `Sex::NonConformant('X')`. The
  original cell cannot be recovered from old JSON; re-parse the MRZ lines if you need it.
- Under the `zeroize` feature, `MrzDate` clears its payload and keeps its variant; `Sex` is
  overwritten whole.

### 6. The emitters take typed sex and dates ([ADR-0019](knowledge/decisions/ADR-0019-typed-values-on-mrzdata.md))

`Td3Fields`, `Td2Fields`, `Td1Fields`, `MrvAFields` and `MrvBFields` take `MrzDate` for
`date_of_birth` and `date_of_expiry`, and `Sex` for `sex`, the same types `MrzData` returns.
Emit and parse therefore share one typed vocabulary, and a parse-then-emit round trip needs no
string handling.

```rust
// 0.7
let fields = Td3Fields {
    date_of_birth: "740812".into(),
    sex: "F".into(),
    date_of_expiry: "120415".into(),
    ..Default::default()
};
// 0.8
let fields = Td3Fields {
    date_of_birth: MrzDate::Calendar(Date::new(1974, 8, 12)),
    sex: Sex::Female,
    date_of_expiry: MrzDate::Calendar(Date::new(2012, 4, 15)),
    ..Default::default()
};
```

- To write six zone characters as they are (a placeholder, or a partly unknown date), classify
  them with `MrzDate::from_field(RawDateField::try_from("74<<12")?, DateRole::Birth, pivot)`.
- Unspecified sex is `Sex::Unspecified`, written as `<`. The old API wrote `<` for anything
  that was not `M` or `F`. A `Sex::NonConformant(c)` is now written as `c`, so a parsed zone
  re-emits exactly as it was read. The parser only produces MRZ characters there. If you build
  `NonConformant(c)` yourself with a character outside `A`–`Z`, `0`–`9` and `<` (for example
  `'a'` or `'É'`), the emitter writes `<`, so the zone stays well-formed.
- For equivalent values the emitted bytes and check digits are unchanged. The defaults are still
  `<<<<<<` and `<`.
- The old `String` fields also accepted lowercase or wrong-length dates and padded or cleaned
  them. A `RawDateField` is exactly six MRZ characters, so such input now has to be fixed before
  it reaches the emitter.

### Behaviour changes you may notice (no code change needed)

- **A legal `K` in a document code survives the damaged-capture repair.** 0.7 turned any `K` in
  the second cell of the document code into `<`. That destroyed a legal code: ICAO lets the
  issuer choose the second character of TD1, TD2 and visa codes. In 0.8 only a passport's `PK`
  becomes `P<`, because `K` is not in Doc 9303 Part 4 §4.4's passport code table.
- **`country_name`'s scope.** It covers a documented subset of the Part 3 §5
  registry, not all of it. `UNK` now returns ICAO's full name.

## Part 2: SynthPass JSON, 1.5 → 1.6

The product schema (`extracted`, `extracted_v2`, the exports, the benchmark reports) is shielded
from most of Part 1. One mapping (`synthpass_core::mrz_product::sex`) decides the product's sex
vocabulary, and the date promotion gates decide which dates are certified. Two changes do reach you:

### A. Optional data has its own slots (breaking for a strict consumer)

- `personal_number` is `null` off TD3.
- On TD1, TD2, MRV-A and MRV-B, the value that used to arrive under `personal_number` now
  arrives in `optional_data_1`, and for TD1's second region in `optional_data_2`. For TD1 the old
  value was two printed fields joined with a space.
- Its confidence is `0.9`, not `1.0`, because no check digit covers optional data on any format.
- On TD3 nothing moves.
- Benchmark reports score the two new columns, so every mean-over-fields CER changes with no
  change in accuracy. That is a discontinuity, not an improvement.

### B. The pass-through `mrz` object follows the 0.8 wire (breaking for a strict consumer)

This affects the browser demo's `parse_mrz_text` result, and the `mrz` key in `synthpass-serve`'s
streamed `done` event and document-status responses. They show the crate's own wire (Part 1 §5):
no `date_of_birth_completeness`, and `sex` as the zone character. The demo's copied JSON and its
check-in form keep `M`/`F`/`X`.

### Behaviour fixes you may notice (not breaking)

- **A misread sex cell is unknown, not `"X"`.** A non-conformant cell (`1`, `0`, `S`) now leaves
  the product's `sex` `null` instead of asserting `X`. Fusion no longer reports it as contradicting
  the LLM, which may fill it. The benchmark's sex column is empty for those documents.
- **Only a real calendar date is certified.** A checksum-valid date of birth or expiry reaches
  `extracted_v2` at confidence `1.0`, and the Tier-2 MRZ hint, only when it names a real day.
  Placeholders such as `000000`, all-filler and partly unknown dates keep the LLM's value at its
  own confidence.
