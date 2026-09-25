# ADR-0017 — `Checks` must distinguish "absent" from "verified"

**Status:** Accepted (amended 2026-09-22 and 2026-09-24)
**Date:** 2026-09-20

## Context

Before this decision was implemented, [`Checks`](../../crates/mrz/src/lib.rs) carried five `bool`
fields, one per ICAO check digit. When a format printed no such check digit, the field reported
`true` — the type's own doc stated the rule:

> A format that prints no such check digit reports that field as `true`: a check digit that does not
> exist cannot fail. So `personal_number` is `true` on every format but TD3, and `composite` is
> `true` on MRV-A and MRV-B.

The reasoning is sound and the documentation is honest. The problem is that **the type cannot say
it** — only the prose can, and a `bool` read in isolation is indistinguishable from a verified one.
`Checks::all_valid`'s own doctest taught the convention rather than the danger: *"TD2 prints no
personal-number check digit, so that field is vacuously true."*

### This has already shipped a defect, and it is written up in this repo

`synthpass-pipeline` used to promote `nationality` and `sex` to the Tier-2 LLM under a prefix reading
*"Verified from the machine-readable zone (trust these over the OCR text)"*, gated on
`m.checks.composite`. Its pre-fix comment recorded why that was wrong:

> **MRV-A and MRV-B have no composite check digit at all** and set `checks.composite = true`
> vacuously (`mrz::parser`, both branches). So on any visa the gate was not weak evidence — **it was
> no evidence, read as proof.** This is the identical vacuous-true trap the `personal_number` line
> below guards against with its `format == Td3` clause.

Two regression tests now pin the fix. **A defect that happened is better evidence than a defect that
could**, and this one reached a prompt that told a model to trust it.

The previous shape forced a consumer to carry **three hand-written `format`-aware guards**, while a
second consumer — `synthpass-die`'s `mrz_reader.rs` — carried none and copied all five booleans into
the serialised v2 block. The type pushed a correctness obligation onto every caller, and one caller
already failed it.

### `valid()` means three different things

Previously, `Checks::all_valid` ANDed all five fields. MRV-A/MRV-B hardwired two of them; TD1/TD2
hardwired one. And `find_and_parse_with` returned on the first candidate for which `data.valid()`
held.

So **an MRV-A reading is accepted as fully verified on three satisfied check digits, a TD1 on four,
and a TD3 on five** — one predicate, three strengths, and no way for a caller to tell which it got
without matching on `format`. That is the defect in one sentence.

### The same bias one level down

The removed `Checks::score()` was:

```rust
5 - self.failed().len() as u8
```

`failed()` filtered on `!ok`, so a vacuously-`true` field could never appear in it. **MRV-A and
MRV-B therefore began at a floor of two free points while TD3 had to earn all five.**

That was not an abstract inequality. `score()` had exactly one call site, the ranking that
`find_and_parse_with` used to choose its best-effort reading when nothing fully validated:

```rust
Some(best) if best.checks.score() >= data.checks.score() => {}
```

**State the direction carefully, because the loop order partly masked it.** Formats were tried
TD3 → MRV-B → MRV-A → TD1 → TD2, and the tie rule was `>=`, so the incumbent survived a tie and a
later candidate had to *strictly* outscore it. TD3 ran first and therefore won ties against MRV.

The unmasked direction was **MRV-A and MRV-B, holding two free points, running before TD1 and TD2,
which held one each.** On degraded input the scanner's fallback preferred a visa reading over an
identity-card reading on points neither document earned — and nothing in the output said so.

### The workspace previously patched around it, inconsistently

`synthpass-pipeline` knew the trap and guarded it with a `format` check; `synthpass-die`'s
`mrz_reader.rs` copied all five booleans verbatim into v2. A TD1, TD2, or MRV record consequently
could ship `personal_number: true` or `composite: true` where the check digit did not exist.

The implementation makes those five slots `Option<bool>` end to end. Parser fan-out stamps absence
from the const `Format` applicability matrix, pipeline promotion and hints require `== Some(true)`,
and v2 serialises an absent digit as `null`. The type, not caller memory, now carries the contract.

### Prior art in this repo

The distinction "no check digit covers this field" is already load-bearing elsewhere.
[`checksum-failed-miss-mechanisms-2026-09-18.md`](../benchmarks/checksum-failed-miss-mechanisms-2026-09-18.md)
names it as a *positional fingerprint*: TD1's optional data carries no check digit of its own, so
only the composite can observe an error there. The historical observation used
`failing_checks == ["composite"]`; current machine output records the five-state map as
`check_states`, without rewriting historical evidence. [`technical_debt.md`](../technical_debt.md)
records the same class of gap for nationality and sex, which no check digit covers on any format.

The benchmark vocabulary already models absence properly — in a hand-maintained span table, because
`Checks` cannot supply it.

### Measured format-selection consequence

`find_and_parse_with` selects a format by matching line 1 position 0, the document code. That cell
has no ICAO check digit. On the published UTOPIA/ERIKSSON specimen, changing only that cell produced
the following results:

| first cell | result |
| :--- | :--- |
| truth-format code | TD3, valid, 0 failed |
| identity-card code | TD1, invalid |
| visa code | MRV-A, valid, 0 failed |
| other tested codes | no MRZ found |

MRV-A shares TD3's two-line geometry and its document-number and date checks, but has no personal
number or composite check digit. A wrong MRV-A hypothesis can therefore satisfy every check it is
asked to satisfy while the two checks that would reject it no longer exist.

Across the 64 ground-truth fixtures, 55 genuine TD3 zones were tested: 41 became fully valid MRV-A
reads after that one-cell change. In three of those, the true TD3 read failed one or two checks while
the MRV-A read reported valid. The corpus contains one such resolution, and it is excluded from the
scored denominator as `checksum_failed_specimen`; the baseline's `false_positive_mrz` metric does not
cover a valid parse of the wrong format.

## The question

Should `Checks` be able to express "this format prints no such check digit", or should that remain a
documented convention that each caller re-implements?

## Options

- **A — change nothing; improve the documentation.** Free, and no break. The docs already say it
  plainly, in three places. Rejected because documentation is exactly where this knowledge already
  lives, and it did not prevent `synthpass-die` from serialising the conflation to the wire. A
  second paragraph will not do what the first one failed to do.

- **B — `Option<bool>`, where `None` means "not printed".** Minimal new vocabulary, immediately
  familiar, and a small diff. **Accepted and implemented.** The contract documents that `None` is
  structural absence, not unknown; code that needs proof uses `== Some(true)`, never
  `unwrap_or(true)`.

- **C — a three-state enum** (`Verified` / `Failed` / `NotPresent`). Says precisely the thing that is
  true, and makes the wrong reading unavailable rather than merely discouraged. Costs: the JSON
  shape changes from `"composite": true` to a string; it propagates into the public
  `SequenceCompleteness::Complete` variant, which embeds `Checks`; and every consumer that reads a
  field must now handle a third case.

- **D — keep the `bool`s and add an applicability mask** beside them. Purely additive: it needs no
  breaking window at all, and both old and new callers keep working. Rejected because nothing
  *forces* the second structure to be read. The default reading stays wrong, and the evidence of
  this repo is that a default wrong answer does not get corrected everywhere — it gets corrected
  where someone remembered.

- **E — keep the `bool`s and add `Checks::is_applicable(Field)`.** Same additive merit as D, smaller
  surface, and it reads well at a call site that thinks to ask. Rejected for the same reason: the
  method is opt-in, and the failure mode is precisely a caller who does not know to opt in.

## Decision and implementation

**Option B is implemented.** `Checks` and v2 `CheckDigits` use `Option<bool>`: `Some(true)` is
verified, `Some(false)` failed, and `None` means the format does not print that check digit. `None`
serialises as JSON `null`, never as an omitted check key. `all_valid` requires at least one
applicable digit and every applicable digit to verify; every proof gate uses `== Some(true)`.

The model owns a const format applicability matrix: TD3 has five printed digits; TD1/TD2 have four
without personal number; MRV-A/MRV-B have three without personal number or composite. Parser
fan-out uses that matrix to stamp `None`, and tests compare parser observations with every matrix
row.

For invalid fallback candidates, ranking uses: non-zero applicability; exact integer
cross-multiplication of `verified / applicable`; higher applicability on equal fractions; then the
incumbent. This can change a degraded-input outcome, so the real-specimen gate records a
per-document 2×2 against the pinned baseline rather than assuming neutrality.

Benchmark wire output now calls the full five-key state map `check_states`, carries it on every
parsed MRZ (including hits), and omits the key when no MRZ parsed. Counts still select only
`Some(false)`. Historical `failing_checks` observations remain literal historical notation and are
not reconstructed as current maps.

## What this ADR deliberately does not decide

- **Choosing among checksum-valid candidates.** `find_and_parse_with` still returns its first
  checksum-valid candidate in actual order TD3, MRV-B, MRV-A, TD1, TD2; this ADR changes only the
  invalid fallback rank.
- **The damaged-path `single()` unanimity gate.** It refuses disagreement rather than arbitrating.
  When its six compared fields agree while format or `Checks` differ, the first recovered hit keeps
  its format and denominator. That narrow existing behavior remains unchanged.
- **Rewriting historical benchmark observations.** Existing `failing_checks` bracket signatures
  retain their historical meaning; current `check_states` maps are recorded only from present parser
  observations.
- **`synthpass-core`'s confidence mirror.** `FieldConfidence::mrz_checksum_scope` reports
  `personal_number` as `PROVEN` on every format. This ADR does not change it, and the change is not
  mechanical. The conversion that installs that scope (`impl From<&Extraction> for ExtractionV2`)
  has only a lossy guess at the format: `MrzFormat::guess_from_lines`, from line count and length.
  It already uses that guess to gate `checks.personal_number`, a few lines from where the
  confidence is set ungated. The guess can never return `MrvA` or `MrvB` and falls back to `Td3`,
  so a fix built on it would keep the defect on the one format that prints neither digit. The
  correction belongs at the Tier-1 boundary in `synthpass-die`, where the real `MrzData::format`
  is still available. Either way it moves a
  wire-visible confidence value and needs its own measurement. Until then, a Tier-1 TD1 record can
  carry `checks.personal_number: null` beside `confidence.personal_number: 1.0`.

## Timing — this is a deadline, not background

`crates/mrz/Cargo.toml` already reads **0.8.0**, but `CHANGELOG.md`'s `[Unreleased]` section is empty
and the last released version is 0.7.1. **The version is staged, not published.**

That has a precise consequence. `cargo-semver-checks` derives the permitted bump from `Cargo.toml`,
which already sits in the breaking slot, so this change costs **one `!` changelog fragment and zero
version movement** if it lands before 0.8.0 publishes — and **a whole additional 0.9.0** if it lands
after, at which point "we broke once" stops being true.

The window is open. It is not open indefinitely, and nothing about it is self-enforcing.

## Consequences

- **Positive:** the difference between a verified check digit and an absent one is explicit in
  `Checks` and the v2 `CheckDigits` block; the MRV fallback-ranking bias is fixed;
  `synthpass-die` no longer asserts proof a visa never supplied; and pipeline promotion is a
  direct `Some(true)` proof gate.

- **Breaking wire change:** `mrz` and v2 now emit explicit `null` for an unprinted digit. The web
  demo renders that state as “not printed”, distinct from valid and failed. Schema and serde tests
  pin `true`, `false`, and `null`.

- **Benchmark consequence:** every parsed row records all five observed states; no parsed MRZ
  records no map. Aggregate failure counts deliberately include only `Some(false)`, so the new
  evidence does not inflate a failure measure.

- Callers who legitimately want "did everything that *could* be checked, check out?" must now say so
  explicitly — which is the point, but it is still work for them.

- **Documentation consequence:** `Checks`, v2, sequence-completeness, benchmark reporting, and the
  demo describe structural absence explicitly. Historical benchmark notation remains historical
  rather than being converted into invented current-state maps.

- **What would reverse it:** evidence that consumers overwhelmingly want the boolean collapse and
  write `matches!(c, Verified | NotPresent)` at every site anyway — at which point the three-state
  enum is ceremony, and D's mask with a convenience method would have been the better trade.

## Amendment 2026-09-22 — what this ADR does not cover, recorded after implementation

Implemented in #395 (`300cb06`). Three corrections to the record, made because the implementing
PR's description asserted a scope this document does not contain.

**`synthpass-core`'s confidence mirror was never excluded here.** #395's description and commit
message both state that this ADR "scopes that type out". It does not — this document names neither
`FieldConfidence` nor `synthpass-core` anywhere, and the exclusion list above originally held three
items, none of them that. A fourth item now records the exclusion the implementing PR was read as
making, so that the decision record and the merged code agree about what was decided.

**The Consequences section overclaimed.** It read that absence is explicit "in the type and wire
data". That is true of `Checks` and the v2 `CheckDigits` block, and false of one wire field:
`FieldConfidence::mrz_checksum_scope` sets `personal_number` to `PROVEN` unconditionally, so a
Tier-1 TD1 record can carry `checks.personal_number: null` beside `confidence.personal_number: 1.0`
in the same JSON object. The sentence is narrowed to what is true. Correcting the code is a separate
change with its own wire-visible confidence movement to measure.

**Status vocabulary.** The status line read `Implemented (2026-09-22)`, which is not one of
`Proposed | Accepted | Superseded by ADR-NNNN` ([README](README.md#format)). It was the only ADR of
eighteen using it, and this decision never passed through `Accepted` on its way there.

## Amendment 2026-09-24 — the `single()` gate no longer compares six fields

The scope list above says the damaged path's unanimity gate kept its six compared fields. That
bullet records what this ADR left alone, and it is no longer current behaviour.
[#431](https://github.com/ruledicaprio/SynthPass/issues/431) widened the gate. Recovered
readings are now one answer only when they agree on every `MrzData` field except
`mrz_lines`, format included, and the gate refuses otherwise. A disagreement in sex,
issuer, document code, optional data or format no longer lets the first hit decide.
