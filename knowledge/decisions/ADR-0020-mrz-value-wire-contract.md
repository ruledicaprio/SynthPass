# ADR-0020 — The wire form of `MrzDate` and `Sex`: `Display`, serde and the zone agree

**Status:** Accepted (2026-09-23)
**Date:** 2026-09-23

## Context

[ADR-0019](ADR-0019-typed-values-on-mrzdata.md) makes `MrzData`'s dates an `MrzDate` enum and its
sex a `Sex` enum. That decides the Rust types. It does not decide what they look like as text, and
text is what most consumers see:

- `mrz-wasm` serialises `MrzData` whole with `serde_json`, and the browser demo reads
  `r.date_of_birth`, `r.date_of_expiry` and `r.sex` directly (`web/index.html`,
  `web/checkin.html`).
- `tests/web/run-corpus.mjs` compares those values with `String(...)` against the fixtures. An
  object would become `"[object Object]"` and score false silently, in a workflow that is
  deliberately not a pull-request gate.
- `synthpass-serve` returns `MrzData` in its JSON.
- Every downstream crate turns these values into strings for the product schema, the benchmark
  columns and the Tier-2 hint.

Serde's derived form for an enum with data is an externally tagged object
(`{"Calendar": {"year": 1974, ...}}`). That shape would break every consumer above, most of them
without a compiler error. A published type's text form is a contract of its own. It can change
for reasons unrelated to `MrzData`, which is why it gets its own record.

## Decision

**For `MrzDate` and `Sex`, `Display`, the serde string and the zone agree.**

| Value | Text |
| --- | --- |
| `MrzDate::Calendar(d)`, `MrzDate::OutOfCalendar(d)` | `YYYY-MM-DD`, zero-padded, century from the parse pivot: exactly the string `expand_date_with_pivot` produces for the same field |
| `MrzDate::PartiallyUnknown(raw)`, `MrzDate::Malformed(raw)` | the six raw characters, e.g. `74<<12`, `R38473` |
| `MrzDate::Unknown` | `<<<<<<` |
| `Sex::Male` / `Female` / `Unspecified` | `M` / `F` / `<` |
| `Sex::NonConformant(c)` | `c`, as printed |

Serialisation is `Display`. Deserialisation is its exact inverse:

- A ten-character `YYYY-MM-DD` of digits gives `Calendar` if the date is well-formed, otherwise
  `OutOfCalendar`. It must **not** go through the crate's `parse_iso`, which rejects exactly the
  out-of-calendar values this type exists to represent.
- Six `<` gives `Unknown`.
- Six MRZ-charset characters mixing digits and `<`, and nothing else, give `PartiallyUnknown`.
- Six MRZ-charset characters containing any other character give `Malformed`.
- A single character gives the `Sex` whose zone character it is.
- Anything else is an error. That includes six bare digits: a century cannot be recovered without
  a pivot, and serialisation never produces that form.

`from_str(to_string(x)) == x` holds for every value, and is property-tested over every six-character
charset field, both date roles and every pivot.

**`Date` keeps its own derived serde shape.** Only `MrzDate` and `Sex` get hand-written impls, and
both sit behind the existing optional `serde` feature, so the default build stays zero-dependency.

## Consequences

- **Dates are byte-identical on the wire.** Every date `MrzData` serialises today serialises the
  same way after ADR-0019. That is proved by a property test comparing `MrzDate`'s `Display` with
  `expand_date_with_pivot` for every field, and by a committed JSON snapshot of all 118 fixture
  parses.
- **Sex changes on the wire, deliberately, in two cases.** Unspecified is `"<"` rather than `"X"`.
  A misread is its own character (`"1"`) rather than `"X"`. The fixture corpus has four of the
  second kind and none of the first. `"X"` on the `mrz` wire now means only that the zone printed
  an `X`, which ICAO does not allow (`X` belongs to the visual zone).
- **`date_of_birth_completeness` leaves the wire** with the field ADR-0019 removes. No consumer in
  this repository reads it.
- **This is the serde pin `mrz`'s roadmap scheduled for 0.9**, pulled forward. The JSON snapshot
  test holds the whole `MrzData` shape, not only these two types. A change to it has to be blessed
  and reviewed as a diff.
- **The product vocabulary is separate.** `synthpass-core` maps `Sex` to ICAO's VIZ letters
  (`M`/`F`/`X`) in one function, so the app's JSON keeps its meaning while `mrz`'s wire says what
  the zone says.

## What would reverse it

A consumer that needs the kind of a date without re-deriving it from the string. The string is
lossless, so the kind can always be recovered with the grammar above. A tagged form would then be
added alongside, never instead: this contract is intended to survive to 1.0.
