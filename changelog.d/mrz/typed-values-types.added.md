- **`MrzDate`, `Sex`, `RawDateField` and `DateRole`: typed date and sex values** (ADR-0019).
  - `MrzDate` gives each thing an MRZ date field can hold its own variant:
    - `Calendar(Date)` for six digits naming a real day;
    - `OutOfCalendar(Date)` for six digits naming none, such as the `000000` placeholder some
      specimens print;
    - `PartiallyUnknown` and `Unknown` for the issuer fillers Doc 9303 Part 3 §4.8 permits;
    - `Malformed` for a misread.
  - `Sex` separates ICAO's three zone values (`M`, `F`, and `<` for unspecified) from a
    `NonConformant(char)` kept as read, rather than collapsing every other character into `"X"`.
  - Their text form is a contract (ADR-0020): `Display`, `FromStr` and serde agree with the zone.
    A six-digit date renders byte-identically to `expand_date_with_pivot`, which a property test
    checks for every field, role and pivot. Values produced by the parser round-trip; caller-built variants can normalize or be rejected.
  - Under the `zeroize` feature, `Date`, `RawDateField`, `MrzDate` and `Sex` implement `Zeroize`,
    so a typed date of birth is still wiped. `RawDateField` is six inline bytes, so all four types
    stay `Copy`.
