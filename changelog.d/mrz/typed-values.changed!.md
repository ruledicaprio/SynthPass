- **`MrzData`'s dates are `MrzDate` and its sex is `Sex`; `date_of_birth_completeness` is gone**
  (ADR-0019).
  - `date_of_birth` and `date_of_expiry` change from `String` to `MrzDate`, and `sex` from
    `String` to `Sex`. Read a date with `calendar()` (the day, when it is a real one), branch on
    its variant, or render it with `to_string()`, which gives exactly the string the field used
    to hold: ISO `YYYY-MM-DD` for six digits, the raw field otherwise.
  - `MrzData::date_of_birth_completeness` is removed. `date_of_birth.completeness()` returns the
    same `DateCompleteness`, and `date_of_expiry.completeness()` now answers the same question
    for expiry. `SequenceCompleteness::Complete { date_of_birth }` is unchanged.
  - `validity()` reads the typed dates. `dates_well_formed` is true exactly when both dates are
    `MrzDate::Calendar`, as before.
  - A misread sex cell is no longer indistinguishable from an unspecified one: `<` is
    `Sex::Unspecified`, and any character other than `M`, `F` and `<` is `Sex::NonConformant(c)`,
    `X` included. Callers that want the visual zone's vocabulary (`M`/`F`/`X`) map it themselves.
  - **The `serde` shape changes in two places** (ADR-0020). The `date_of_birth_completeness` key
    is gone. `sex` serialises as the zone character: `"M"` and `"F"` are unchanged, an
    unspecified cell is `"<"` where it was `"X"`, and a non-conformant cell keeps its character
    where every such cell used to be `"X"`. For example, a zone whose sex cell reads `1` used to
    serialise `"sex":"X"` and now serialises `"sex":"1"`. Dates serialise byte-identically.
  - Under the `zeroize` feature the three fields are wiped through their own `Zeroize` impls.
    `MrzData`'s documentation states that the wipe is best-effort and what it skips:
    `MrzDate`'s payload is cleared while its kind survives, and `Sex` is fully overwritten.
  - Migration: `d.date_of_birth == "1974-08-12"` becomes `d.date_of_birth.to_string() ==
    "1974-08-12"`, or `d.date_of_birth == MrzDate::Calendar(Date::new(1974, 8, 12))`;
    `d.date_of_birth_completeness` becomes `d.date_of_birth.completeness()`; `d.sex == "X"`
    becomes a match on `Sex::Unspecified | Sex::NonConformant(_)`.
  - JSON written by 0.7 still deserialises: the removed `date_of_birth_completeness` key is
    ignored, but its `"sex": "X"` now reads as `Sex::NonConformant('X')`. 0.7 wrote `"X"` for
    `<` and for every misread cell alike, so the original cannot be recovered from old JSON;
    re-parse `mrz_lines` instead.
