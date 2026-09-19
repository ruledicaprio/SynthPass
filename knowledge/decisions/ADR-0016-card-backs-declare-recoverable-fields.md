# ADR-0016 — Which fields a document side may be scored on

**Status:** Accepted
**Date:** 2026-09-19 (proposed), 2026-09-19 (accepted)

## Context

The MRZ holdout in [`parity.rs`](../../crates/synthpass-llm/tests/parity.rs) strips the MRZ-shaped
lines from a fixture's OCR text and scores what a provider recovers from the printed visual zone
alone. Its purpose is stated in [`VIZ_TIER2_DESIGN.md`](../VIZ_TIER2_DESIGN.md) §5.2: the parity
corpus is by construction the set of documents Tier 2 never sees in production, and the holdout
removes that bias.

**Scope note, because it sets how much this decision is worth.** The holdout is `#[ignore]`d,
gated on `SYNTHPASS_PARITY_HOLDOUT=1`, needs a ~1 GB GGUF, and reports into **no committed baseline
and no CI gate**. `parity.rs`'s own module doc calls it *"a regression smoke check, not an accuracy
gate"*. Nothing in this ADR moves a gated number. What is at stake is whether a measurement is
honest, not whether the product is correct.

[#346](https://github.com/ruledicaprio/SynthPass/pull/346) added a per-document escape hatch: a
fixture may declare `visual_zone_present: false`, and the holdout skips and reports it instead of
scoring zeros. A pre-flight fails loudly for any fixture whose text strips to nothing *without* the
flag. [#348](https://github.com/ruledicaprio/SynthPass/pull/348) set it on the French 2020 and
Italian 2022 identity-card backs, whose text strips to nothing at all.

**What nobody checked is that every TD1 fixture in this corpus is an identity-card back.** There are
nine in the reviewed set and **six more in [`samples/ocr_fixtures/derived/`](../../samples/ocr_fixtures/derived)**
— Bosnia 2013, Norway 2021, Portugal 2024, Romania 2021, Türkiye 2023 and the United States. France
and Italy are two of the nine. The rest were never examined, because the pre-flight had no reason to
complain: their text does not strip to empty.

### What the card backs actually print

Examined by opening each image (**Observed**, 2026-09-19); all are byte-identical to `samples-data`,
sha256-verified.

| Fixture | What the imaged side prints | Holder's name? |
| :--- | :--- | :--- |
| Belgium 2021 | date and place of issue | **no** |
| Croatia 2021 | residence, issuing authority, date of issue, OIB/PIN | **no** |
| Serbia 2008 | personal number, place of birth, place of residence, a serial | **no** |
| Slovenia 2022 | permanent residence, competent authority, card number | **no** |
| Sweden 2022 | remarks, date and place of issue, place of birth | **no** |
| Switzerland 2023 | place of origin, height, date of issue, authority, card number | **no** |
| Türkiye 2020 | mother's name, father's name, issuing authority, a PIN | **no** |
| France 2020, Italy 2022 | nothing but the zone | **no** |
| Austria 2021 (image only, no fixture yet) | date of issue, authority, place of birth, nationality, height, title | **no** |

**Ten of ten print no holder name.** That is the format working as designed: an identity card carries
its biographic fields on the front and its MRZ on the back. Each fixture declares `surname` and
`given_names` regardless, because a fixture transcribes the **MRZ**, which carries them.

So under holdout these documents are scored for recovering a name the imaged side does not contain.
That is not a recovery failure — it is the class of error the
[2026-09-09 denominator correction](../benchmarks/denominator-correction-2026-09-09.md) closed for
MRZ-less fronts and [ADR-0012](ADR-0012-cover-only-specimens-are-a-labelled-class.md) closed for
covers.

### The defect is not about names, and not about card backs

Two of the ten print the MRZ document number in the clear (Slovenia, Switzerland) and one prints the
personal number (Serbia). None prints date of birth or expiry. The recoverable set therefore differs
**per side and per field**.

The counter-example that shows the real shape of it is in the private set, which no benchmark walks
(`parity.rs:217-219` reads only `ocr_fixtures/` and `derived/`; `lib.rs:312` skips any path
containing `private`). `samples/private/` holds one **driver's licence**, and it is
`front` + `no_mrz`: it prints the biographic fields in the clear and carries **no MRZ at all**.

A card back has an MRZ and prints almost none of the fields it encodes. A driver's-licence front
prints the fields and has no MRZ. They are the two extremes of a single fact:

> **A document *side* prints some fields and not others, and the harness assumes the MRZ's field set
> equals the visible field set.**

The side is already named in every filename. That is the fact this ADR proposes to use.

### A tracked document already disagrees with the data

[`VIZ_TIER2_DESIGN.md`](../VIZ_TIER2_DESIGN.md) §5.2 justifies the holdout with:

> "the four proven fields are all recoverable from the VIZ, which prints the document number and
> both dates in human-readable form."

The four proven fields are `document_number`, `date_of_birth`, `date_of_expiry` and
`personal_number` ([`v2.rs:397-409`](../../crates/synthpass-core/src/v2.rs)). Against the ten card
backs: date of birth **0/10**, date of expiry **0/10**, document number **2/10**, personal number
**1/10**.

The sentence describes a **passport data page** accurately, and at the commit where §5.2 was measured
(`b93b89a`, 2026-09-04) 64 of its 72 fixtures were passports. But it is not scoped to passports, and
**eight of those same 72 were already identity-card backs** — Serbia and Slovenia in the reviewed
set, six in `derived/`. The claim did not go stale when card backs arrived; it was already
contradicted by 11% of the population it was measured over (**Derived**, from `git ls-tree` at that
commit).

That has a consequence for the derived six that is easy to state backwards. `Fixture::scored_fields`
([`parity.rs:179-190`](../../crates/synthpass-llm/tests/parity.rs)) restricts an unreviewed fixture
to checksum-proven fields — which protects them from the *name* defect, since names are not proven.
It gives **no** protection on the four proven fields, because those are precisely what an unreviewed
fixture is scored on. The six derived card backs have been scored on four fields they print almost
none of, since before the holdout was written.

`parity.rs:179-190` is also the mechanism this ADR should build on rather than beside: the harness
already gates which fields a fixture may be scored on, per fixture.

### Why a text-based test cannot answer this

The card backs were first surfaced by asking whether each fixture's `surname` and `given_names`
appear in its OCR text after the MRZ-shaped lines are stripped. That method produced **two false
positives**, both visible only by opening the image:

- **Slovenia** — the given names appeared to survive. The substring matched inside the city name on
  the residence line.
- **Türkiye 2020** — both names appeared to survive. The card prints *mother's name* and *father's
  name*, whose specimen values are the same token as the holder's MRZ name.

**Absence from the OCR text is not evidence the side lacks the field, and presence is not evidence it
has it.** "The side does not print it" and "the pipeline failed to read it" are opposite in meaning
and identical in the text. Only the image separates them, which makes any label here human-verified
in the sense [ADR-0012](ADR-0012-cover-only-specimens-are-a-labelled-class.md) already uses for
covers.

## The question

**How does the holdout learn which fields a document side can be scored on?**

`visual_zone_present` is one boolean over a whole document. It can say "there is nothing here"
(France, Italy) and "everything here is fair game" (its default). It cannot say what is true of the
other eight: *the side prints a visual zone, and the holder's name is not in it*.

## Options

- **A — Change nothing.** The card backs keep scoring an impossible recovery. Wrong for the reason
  the denominator correction and ADR-0012 already settled elsewhere: a zero no provider could avoid
  is not a measurement.
- **B — Set `visual_zone_present: false` on the eight.** One-line data change, and **false**: each
  prints a real visual zone, and three print a field the holdout can legitimately score. It also
  removes documents from the denominator silently, the failure mode invisible in a report.
- **C — Declare recoverable fields per fixture**, as a negative list (`absent_from_visual_zone`).
  Every claim explicit. Costs a human audit of **fifteen** fixtures, not nine, and a promotion guard
  — a `derived/` fixture becomes reviewed by a `git mv`, so the defect re-enters on promotion.
- **D — A class rule scoped to names.** A fixture whose stem carries the side token `back` is not
  name-scorable. Discards nothing: the evidence is 10/10 on names and only 2/10 on the document
  number, and those are different claims.
- **E — Derive it from the image automatically.** Requires exactly the capability under test.
  Circular.
- **G — D as the default, C as the exception.** The side token drives it; any fixture may override
  per field with `absent_from_visual_zone`. `visual_zone_present: false` stays as the degenerate
  case.

## Recommendation

**G.**

It matches the mechanism [ADR-0012](ADR-0012-cover-only-specimens-are-a-labelled-class.md) chose —
*the filename is the lookup key and says what an image is* — rather than the sidecar flag ADR-0012
explicitly rejected. It inherits to the six `derived/` fixtures and to every future card back with
no audit, which is what "fix the failure class, not the specimen" means here. It moves no gated
number, needs no amendment to
[ADR-0011](ADR-0011-split-m6-packaging-into-m8.md) or
[ADR-0013](ADR-0013-names-are-scored-against-mrz-form-truth.md), and reverses in one commit.

The honest objection is this ADR's own line — *a filename is a claim about content that nothing
verifies*. It applies with equal force to a hand-written per-field key, and to fifteen such claims
rather than one rule. Anyone who distrusts the filename convention should take **C** instead; the
argument for a per-field declaration over a per-document boolean holds either way.

## What this ADR deliberately does not decide

- **Front/back pairing.** The corpus holds front/back pairs for eight of the ten card backs and has
  never recorded the relation, and the product is used with both sides in hand. That is a real
  question and it is **not this one**. The
  [benchmark maintenance contract](../benchmarks/README.md) already governs it: specimen identity
  must not be inferred from filename similarity, and deduplication by physical-document identity is
  barred until that policy and its specimen IDs are explicit. Pairing therefore needs its own ADR,
  scoped to the whole corpus, taken after M6 closes — and it would cost eight of the MRZ-less
  negative controls ADR-0012 values, which is a correctness cost, not a convenience one.
- **The Türkiye 2020 mislabel.** `Turkiye_ID_Specimen_2020_front_no_mrz.webp` is a both-sides image
  whose back half carries a readable TD1 zone, so it is neither a front nor `no_mrz`. The class and
  the fix are already on record in
  [`MissReason::FalsePositiveMrz`](../../crates/synthpass-bench/src/lib.rs)'s doc comment, via the
  Monaco precedent. It is a data-only relabel, not ADR material. The baseline records
  `false_positive_mrz: 0` today, so nothing is being paid for it yet.
- **The Austria specimen.** `Austria_ID_Specimen_2021_back_mrz.png` is on disk and absent from
  `samples/corpus.jsonl`. Manifesting it moves `documents` and `scored` during
  [ADR-0011](ADR-0011-split-m6-packaging-into-m8.md)'s deliberate freeze. A maintainer decision,
  recorded separately.

## Sub-questions, as resolved on acceptance

1. **Which side tokens count — resolved by enumeration, not by pattern.** The corpus was counted
   rather than guessed. Excluding `samples/private/`, the tokens in use are `_front_` (40), `_back_`
   (25), `_inner_` (1), `_front_back_` (1) and `_face_` (1). The rule keys on an **explicit closed
   set**, and a filename carrying no recognised token gets **no class rule at all** — it falls
   through to today's behaviour rather than to a guessed default. Silence is the safe direction: an
   unrecognised name must not silently acquire a scoring policy.

   The two singletons are why a pattern match would have been wrong. `_face_`
   (`Serbia_ID_Specimen_2008_face_no_mrz.png`) means `back` and is spelled nothing like it.
   `_inner_` belongs to a passport inner page — a data page, not a card side — so it must **not**
   inherit the card-back rule, though it matches no `front`/`back` pattern either way.
2. **The pre-flight does not grow a matching guard.** It still catches an empty strip with no flag.
   It cannot catch "declared a field the side does not print" without the per-fixture audit this
   decision exists to avoid, and a guard that only appeared to check that would be worse than none.
   The negative list is the mechanism; the audit is not reintroduced by the back door.
3. **The rule is per field, not name-only.** Stated explicitly, as the sub-question asked. The
   four-proven-fields defect is older and wider than the name defect and already affects the six
   `derived/` fixtures, so a name-only rule would leave the larger half unfixed while looking
   complete. `absent_from_visual_zone` is a per-field negative list for that reason.

## Consequences

- **Positive:** the holdout stops reporting unearnable zeros; what a side can and cannot prove is
  explicit; the rule inherits rather than needing an audit per fixture; §5.2's premise gets corrected
  against measured evidence.
- **Negative:** a filename token becomes load-bearing for scoring, so a misnamed file changes a
  number — and the corpus has exactly that defect today in the Türkiye file above. The
  `_ID_Private_Specimen_ID_` double-token in `samples/private/` shows the convention is not perfectly
  regular.
- **Documents that must change with it:** `VIZ_TIER2_DESIGN.md` §5.2 (its premise is contradicted for
  ten documents); `samples/README.md`, whose filename grammar has no side token though its prose
  mentions sides; `parity.rs`'s `visual_zone_present` doc, which states France/Italy as the whole use
  case. [`parity-mrz-holdout-2026-09-04.md`](../benchmarks/parity-mrz-holdout-2026-09-04.md) gets a
  **dated successor**, never an edit — its own rule is that every number is quoted rather than
  replaced, and its population included eight card backs.
- **What would reverse it:** a corpus identity policy that pairs sides into documents, which would
  make a per-side label redundant for everything that has both sides.

**No MRZ character value, holder name or zone text appears in this document.**
