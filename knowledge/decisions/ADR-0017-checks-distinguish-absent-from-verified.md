# ADR-0017 — `Checks` must distinguish "absent" from "verified"

**Status:** Proposed
**Date:** 2026-09-20

## Context

[`Checks`](../../crates/mrz/src/lib.rs) carries five `bool` fields, one per ICAO check digit. When a
format prints no such check digit, the field reports `true` — the type's own doc states the rule:

> A format that prints no such check digit reports that field as `true`: a check digit that does not
> exist cannot fail. So `personal_number` is `true` on every format but TD3, and `composite` is
> `true` on MRV-A and MRV-B.

The reasoning is sound and the documentation is honest. The problem is that **the type cannot say
it** — only the prose can, and a `bool` read in isolation is indistinguishable from a verified one.
`Checks::all_valid`'s own doctest teaches the convention rather than the danger: *"TD2 prints no
personal-number check digit, so that field is vacuously true."*

### This has already shipped a defect, and it is written up in this repo

`synthpass-pipeline` used to promote `nationality` and `sex` to the Tier-2 LLM under a prefix reading
*"Verified from the machine-readable zone (trust these over the OCR text)"*, gated on
`m.checks.composite`. Its own doc comment now records why that was wrong:

> **MRV-A and MRV-B have no composite check digit at all** and set `checks.composite = true`
> vacuously (`mrz::parser`, both branches). So on any visa the gate was not weak evidence — **it was
> no evidence, read as proof.** This is the identical vacuous-true trap the `personal_number` line
> below guards against with its `format == Td3` clause.

Two regression tests now pin the fix. **A defect that happened is better evidence than a defect that
could**, and this one reached a prompt that told a model to trust it.

What survives is the shape: a single consumer carrying **three hand-written `format`-aware guards**,
and a second consumer — `synthpass-die`'s `mrz_reader.rs` — carrying none, copying all five bools
into the serialised v2 block unconditionally. The type pushes a correctness obligation onto every
caller, and one caller already failed it.

### `valid()` means three different things

`Checks::all_valid` ANDs all five fields. MRV-A/MRV-B hardwire two of them; TD1/TD2 hardwire one. And
`find_and_parse_with` returns on the first candidate for which `data.valid()` holds.

So **an MRV-A reading is accepted as fully verified on three satisfied check digits, a TD1 on four,
and a TD3 on five** — one predicate, three strengths, and no way for a caller to tell which it got
without matching on `format`. That is the defect in one sentence.

### The same bias one level down

`Checks::score()` ([`lib.rs:821`](../../crates/mrz/src/lib.rs)) is:

```rust
5 - self.failed().len() as u8
```

`failed()` filters on `!ok`, so a vacuously-`true` field can never appear in it. **MRV-A and MRV-B
therefore begin at a floor of two free points while TD3 must earn all five.**

That is not an abstract inequality. `score()` has exactly one call site
([`parser.rs:1213`](../../crates/mrz/src/parser.rs)), the ranking that `find_and_parse_with` uses to
choose its best-effort reading when nothing fully validates:

```rust
Some(best) if best.checks.score() >= data.checks.score() => {}
```

**State the direction carefully, because the loop order partly masks it.** Formats are tried
TD3 → MRV-B → MRV-A → TD1 → TD2, and the tie rule is `>=`, so the incumbent survives a tie and a
later candidate must *strictly* outscore it. TD3 runs first and therefore wins ties against MRV.

The unmasked direction is **MRV-A and MRV-B, holding two free points, running before TD1 and TD2,
which hold one each.** On degraded input the scanner's fallback prefers a visa reading over an
identity-card reading on points neither document earned — and nothing in the output says so.

### The workspace already patches around it, inconsistently

`synthpass-pipeline` knows the trap and guards for it, twice, with a long comment explaining why:

```rust
m.checks.personal_number && m.format == mrz::Format::Td3
```

`synthpass-die`'s `mrz_reader.rs` does not. It copies all five bools verbatim into the v2 schema's
`CheckDigits`, so **every TD1, TD2 and MRV record serialised by this workspace ships
`personal_number: true` and `composite: true` to the wire**, where no consumer can tell a verified
digit from an absent one. `routing.rs` then reads the derived `mrz_checksums_valid` and treats an
MRV-A with three real check digits as fully proven.

One crate compensating and another not is the signature of a defect that belongs in the type. The
guard is correct in `synthpass-pipeline` because someone remembered; it is missing in
`synthpass-die` because someone did not.

### Prior art in this repo

The distinction "no check digit covers this field" is already load-bearing elsewhere.
[`checksum-failed-miss-mechanisms-2026-09-18.md`](../benchmarks/checksum-failed-miss-mechanisms-2026-09-18.md)
names it as a *positional fingerprint*: TD1's optional data carries no check digit of its own, so
only the composite can observe an error there, and `failing_checks == ["composite"]` alone localises
the defect before a character is examined. [`technical_debt.md`](../technical_debt.md) records the
same class of gap for nationality and sex, which no check digit covers on any format.

The benchmark vocabulary already models absence properly — in a hand-maintained span table, because
`Checks` cannot supply it.

## The question

Should `Checks` be able to express "this format prints no such check digit", or should that remain a
documented convention that each caller re-implements?

## Options

- **A — change nothing; improve the documentation.** Free, and no break. The docs already say it
  plainly, in three places. Rejected because documentation is exactly where this knowledge already
  lives, and it did not prevent `synthpass-die` from serialising the conflation to the wire. A
  second paragraph will not do what the first one failed to do.

- **B — `Option<bool>`, where `None` means "not printed".** Minimal new vocabulary, immediately
  familiar, and a small diff. Its merit is real: `Option` is the idiom every Rust caller already
  knows. Rejected because `None` reads as *"unknown"* rather than *"not applicable"*, and the
  natural way to collapse it is `unwrap_or(true)` — which reintroduces the exact bug at every call
  site, now with the library's blessing.

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

## Recommendation

**Option C.**

D and E are the tempting answers — they are additive, they need no window, and they cost nobody an
upgrade. The argument against both is empirical rather than aesthetic: this codebase already
contains the additive fix in spirit (`synthpass-pipeline`'s `&& format == Td3` guard), and it is
applied in one crate out of three. An opt-in correction to a wrong default gets applied where
someone remembers.

The one thing that justifies spending a break here is that **C removes the wrong answer rather than
providing an alternative to it.** A caller matching on three states cannot accidentally read
"absent" as "proof"; a caller reading a `bool` can, and has.

B deserves more sympathy than a one-line rejection, because `Option<bool>` is genuinely the smaller
change. It fails on the collapse: the ergonomic path is `unwrap_or(true)`, and the library would be
handing callers a footgun shaped like an idiom.

## What this ADR deliberately does not decide

- **The spelling of the variants.** `NotPresent` / `NotApplicable` / `NotPrinted` all read
  acceptably; this ADR argues the arity, not the names.
- **Whether `score()` is re-derived from the new state, or simply stops counting absent digits.**
  Both fix the MRV bias. `score()` is `pub(crate)` with one call site, so it is free either way and
  can be settled in review.
- **Whether `synthpass-core`'s `CheckDigits` mirror follows.** That type is a separate, deliberately
  independent schema with its own published contract; changing it is a decision about the v2 wire
  format, not about `mrz`.
- **Whether `failing_checks` grows a third state in the benchmark vocabulary**, or stays a list of
  failed field names. The fingerprint that
  [`checksum-failed-miss-mechanisms-2026-09-18.md`](../benchmarks/checksum-failed-miss-mechanisms-2026-09-18.md)
  relies on works either way, but it must be stated before implementation.

## Timing — this is a deadline, not background

`crates/mrz/Cargo.toml` already reads **0.8.0**, but `CHANGELOG.md`'s `[Unreleased]` section is empty
and the last released version is 0.7.1. **The version is staged, not published.**

That has a precise consequence. `cargo-semver-checks` derives the permitted bump from `Cargo.toml`,
which already sits in the breaking slot, so this change costs **one `!` changelog fragment and zero
version movement** if it lands before 0.8.0 publishes — and **a whole additional 0.9.0** if it lands
after, at which point "we broke once" stops being true.

The window is open. It is not open indefinitely, and nothing about it is self-enforcing.

## Consequences

- **Positive:** the difference between a verified check digit and an absent one becomes impossible
  to misread; the MRV ranking bias in `find_and_parse_with` is fixed as a side effect;
  `synthpass-die`'s wire output stops asserting proof a visa never supplied; and the per-call-site
  guards in `synthpass-pipeline` become redundant rather than load-bearing.

- **Negative — and the worst of it is silent, not loud.** 13 production lines break loudly across
  the workspace, which is fine. The JSON shape also changes, and `mrz` carries **zero
  `#[serde(...)]` attributes** to cushion it. `mrz-wasm` serialises `MrzData` whole, and the demo
  renders each check with:

  ```js
  function chip(ok) {
    return `<span class="chip ${ok ? 'ok' : 'bad'}">${ok ? '✔ valid' : '✘ failed'}</span>`;
  }
  ```

  **Every non-empty string is truthy in JavaScript.** If `checks.*` serialises as `"Verified"` /
  `"Failed"` / `"NotPresent"`, the demo renders **✔ valid for a failed check digit** — no exception,
  no console warning, no test. `web/index.html`'s `chip` must be fixed in the same change, or the
  serialised representation must stay boolean-compatible. **If the answer is that the JSON must stay
  booleans, that is an argument for option B**, and B should be revisited on exactly that ground.

- **Two mirrors will not fail to compile, and will silently diverge:** `synthpass-core`'s
  `v2::CheckDigits` and `synthpass-bench`'s `PrivateSidecar` each redeclare the same five booleans
  and re-implement the five-way AND. The second is worse than duplication — it deserialises
  `samples/private/<stem>.json`, so **a human hand-writes `"personal_number": true, "composite":
  true` for an MRV specimen**, asserting check digits that do not exist, or the specimen scores
  checksums-invalid. The vacuity has escaped the crate into a file format.

- Callers who legitimately want "did everything that *could* be checked, check out?" must now say so
  explicitly — which is the point, but it is still work for them.

- **Documents that must change with it:** `Checks`'s own type doc and the `all_valid` doctest, both
  of which currently teach the vacuous-`true` convention as correct; the `Field` enum's doc, which
  states a one-to-one correspondence with `Checks`'s field names; and
  [`MRZ_SEQUENCE_COMPLETENESS.md`](../MRZ_SEQUENCE_COMPLETENESS.md), which chose to **wrap rather
  than replace** `Checks` on the explicit ground that *"changing either shape would be a breaking
  0.x bump every downstream crate has to absorb at once."* That objection is correct and is not
  answered here — it is *paid*. The 0.8.0 breaking window exists precisely so the absorption happens
  once, deliberately, rather than never.

- **What would reverse it:** evidence that consumers overwhelmingly want the boolean collapse and
  write `matches!(c, Verified | NotPresent)` at every site anyway — at which point the three-state
  enum is ceremony, and D's mask with a convenience method would have been the better trade.
