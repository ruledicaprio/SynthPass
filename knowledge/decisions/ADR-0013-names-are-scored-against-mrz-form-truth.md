# ADR-0013 — Name accuracy is a separate axis, scored only against MRZ-form truth

**Status:** Accepted
**Date:** 2026-09-16

## Context

A Tier-1 hit is a checksum-valid MRZ whose document number matches ground truth
([benchmark maintenance contract](../benchmarks/README.md#benchmark-maintenance-contract), "Localize").
No ICAO 9303 check digit covers `surname` or `given_names` in any of the five formats: TD1/TD2/TD3
check only the document number, dates, personal number and the composite
([`fusion.rs`](../../crates/synthpass-core/src/fusion.rs)'s module doc), and MRV-A/MRV-B put the
name on line 1, which carries no check digit at all
([`parser.rs`](../../crates/mrz/src/parser.rs)). A hit therefore proves nothing about the name.

The 2026-09-16 harness-comparison run (MAIN `c617254`, seed 0, profile `clean`, 100 documents per
format, run locally — [`m6-per-format-harness-comparison-2026-09-16.md`](../benchmarks/m6-per-format-harness-comparison-2026-09-16.md))
shows how far apart the two are. **Derived** from its `synthpass-bench` reports: 178 of 377 Tier-1
hits carry a wrong surname or given name, in every format (per-format counts in
[`benchmarks/README.md`](../benchmarks/README.md)). **Hypothesized** from the dumped OCR text:
recognition collapses the `<` filler (`KOVALENKO<<ANDRII` → surname `KOVALENKOANDRII`, given names
empty) or reads filler runs as letters (`CCCC`), and on TD1 the mandatory synthetic watermark is
sometimes read as line 3. The per-field error rates the bench already reports average this away;
nothing states how many *accepted* documents are wrong on a name.

Measuring names raises a question the hit never had to answer: which spelling is the truth. The
printed visual zone (VIZ) writes `ESPAÑOL`, mixed case and a native split; the MRZ writes
`ESPANOL<ESPANOL<<JUAN`. The two ground-truth sources in the tree today both use the MRZ form.
The synthetic corpus parses the generator's own MRZ lines back through `mrz`
(`parse_ground_truth_mrz`, [`synthpass-bench`](../../crates/synthpass-bench/src/lib.rs)).
The real-specimen fixtures in `samples/ocr_fixtures/` were transcribed from the printed MRZ
([`CORPUS_COVERAGE.md`](../CORPUS_COVERAGE.md), 2026-09-08 note). All 59 carry both name fields,
upper-case A–Z and spaces only. For each of the 40 whose zone validates, the names equal what
`mrz` parses from the fixture's own `mrz_line`. The exception shows why this has to be written
down. Argentina 2026's printed zone has no `<<` (`GONZALES<RODRIGUEZ<MARIANA<FLORENCIA`), and
its fixture splits the name the way the visual zone does. The zone fails its own check digits,
so the fixture is off the denominator, but a VIZ split had already entered an MRZ-form file.

## Decision

- **`hit` does not change.** Checksum-valid plus the expected document number, as today. Names are a
  separate, reported axis. `hit`, `hit_rate`, `tier1_hit_rate`, both CI gates and every committed
  baseline keep their meaning.
- **Name truth is MRZ form, defined operationally:** the `surname` and `given_names` that `mrz`
  parses from the document's printed zone. That form is A–Z and spaces, transliterated by the
  issuer, with `<` read as a space, split where `mrz` splits (at `<<`, else at the first `<`).
  A name is scored only against truth in this form.
- **A VIZ-form label is never scored against an MRZ read.** Any label source must declare which
  form it carries. VIZ-form names need either a documented transliteration step into MRZ form or a
  track of their own. This covers future sources and the private sidecars under `samples/private/`.
- **Two reported figures.** `strict_hit_rate` is the number of Tier-1 hits with both names exact,
  over the name-scorable documents in the Tier-1 rate's own scored denominator.
  `names_exact_among_hits` is the same count over Tier-1 hits.
  A document is name-scorable when its truth carries both fields. Unlabelled documents are left out
  of the denominator, not counted as failures. A labelled document whose read errored counts as a
  failure. Wherever a strict rate appears, its denominator appears with it.
- **Name errors are classified for diagnosis only.** The four kinds are `separator_lost`,
  `split_shifted`, `filler_read_as_letters` and `other`. Reports record the kind per document and
  never the name values. Classification never changes a read. Any repair is a separate change,
  measured on its own.

## Alternatives

- **Fold names into `hit`.** One number, but every committed baseline and history series would
  break, and a hit would stop meaning "what a checksum can prove", the one claim Tier-1 can back
  with a check digit.
- **Score VIZ labels with fuzzy matching.** A lost separator is one edit away from the truth.
  A threshold loose enough to absorb transliteration (`Ñ`→`N`, truncation to the field width)
  would also absorb the misreads this metric exists to count. The threshold would also be a
  heuristic constant with no sweep that could honestly set it.
- **Wait until a repair exists.** A repair with no baseline cannot show it helped, and
  [principle 6](../project_principles.md#6-benchmark-everything-never-trust-anecdotes) rules that
  order out.
- **Keep per-field error rates only.** They already exist and stay, but an average cannot say how
  many accepted documents carry a wrong name. A split shift also spreads one mistake across two
  fields.

## Consequences

- **Positive:** a name error on an accepted document becomes a number that can be tracked,
  broken down by mechanism. A future repair has a baseline to beat. The truth form is written
  down before a second label source arrives.
- **Negative:** the real-specimen strict rate covers only documents that have a fixture, so its
  denominator is smaller than the Tier-1 rate's, and every table must state both. A fixture whose
  zone validates must keep its names equal to `mrz`'s parse of its `mrz_line`; a guard test in
  `synthpass-bench` enforces this, so a hand transcription cannot reintroduce a VIZ split unseen.
  Fixtures whose zone fails its own check digits (Argentina 2026 today) are outside that test and
  keep their transcription as it is.
- The headline table publishes the strict rate next to the Tier-1 rate only once a CI run has
  measured it (Observed, per the maintenance contract), never from local arithmetic. Gating on it
  is a separate decision.
- **Next step, not decided here:** align the name line to the fixed MRZ character grid in
  `synthpass-ocr`, using the character positions `ocrs` returns, so that dropped filler cells are
  restored. It is benchmark-first, and this metric is how it will be judged.
- **What would reverse this:** a label source that can only be had in VIZ form, and is worth more
  than a separate track, would reopen the "one form" rule. A strict rate that stays equal to the
  Tier-1 rate across formats and real specimens would make the second column redundant.
