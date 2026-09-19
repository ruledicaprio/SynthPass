- **`ParseOptions`, `Date` and `DateValidity` are now `#[non_exhaustive]`, and `ParseOptions` is built with `ParseOptions::default().with_pivot_yy(..)`.**
  These were the last three types in this crate's public surface that could grow only by breaking.
  Every public enum already carried the attribute, and so did `Checks` and `MrzData` — `Checks` for a
  reason that applies to all of them: *"a future MRZ format may carry a check digit these five fields
  don't name, and adding it should not be a breaking change."*

  **Breaking once, deliberately, while it is cheap.** `mrz` is pre-1.0, so cargo's breaking slot is
  the minor version; from 1.0 it becomes the major, and a major bump on a published crate splits the
  ecosystem. Paying for all three now costs one release. Paying per type later costs three.

  **Only `ParseOptions` changes anything for callers.** Replace `ParseOptions { pivot_yy: 30 }` with
  `ParseOptions::default().with_pivot_yy(30)`. `Date` is constructed through `Date::new` and
  `Date::from_epoch_days`, which already reach every field, and `DateValidity` is returned by
  `validity()` and never built by callers — so neither needs a builder and no workspace crate
  changes.

  **Functional update syntax is not a workaround, and this is the part worth knowing.**
  `ParseOptions { pivot_yy: 30, ..Default::default() }` is rejected with the same `E0639` — the
  restriction covers struct expressions including the `..base` form. Verified by compiling it rather
  than by reading the reference, after a first attempt to check it turned out vacuous. The `with_*`
  builder is the pattern instead, and each future option adds one method additively.

  **The five `*Fields` emit structs stay exhaustive, on purpose.** `Td1Fields`, `Td2Fields`,
  `Td3Fields`, `MrvAFields` and `MrvBFields` mirror field layouts ICAO 9303 fixes; they do not grow,
  and callers build them with struct expressions. Future tunables belong in a separate
  `#[non_exhaustive]` companion rather than as a new field on one of these — the separation
  `ParseOptions` already has from the data it parses. Each of the five now says so in its own
  rustdoc, and the policy is recorded in `knowledge/ARCHITECTURE.md` §13.4.

  No behaviour changes. `Default` still yields `CURRENT_YY`, `with_pivot_yy` is `const`, and no
  parser is touched.
