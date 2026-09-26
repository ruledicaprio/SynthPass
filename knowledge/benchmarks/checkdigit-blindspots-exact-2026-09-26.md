# Check-digit blind spots, exact — what the arithmetic can and cannot promise

**Date:** 2026-09-26 · **MAIN:** `011e9f1` · **DATA:** not stated · **Evidence:** Derived (closed form and exact integer enumeration, `crates/mrz/examples/checkdigit_blindspots_exact.rs`, with the laws pinned against the parser in `crates/mrz/tests/checkdigit_algebra.rs`; every measured input is cited to its note) plus one Observed count over the tracked fixtures (§4) · **Status:** current

Companion to [`checksum-blindspots-measured-2026-08-05.md`](checksum-blindspots-measured-2026-08-05.md)
(what got past the check digit on 76 generated misreads) and
[`observed-ocrb-confusions-2026-09-21.md`](observed-ocrb-confusions-2026-09-21.md) (which glyph the
recognizer returns, 126 aligned events). This note is the algebra those two are measuring against,
carried through to the composite digit, to the fields with no digit at all, and to the shape an
external claim can honestly take. Every rate below is a ratio of integers computed exactly; the
reproducer prints the key ones as integer ratios. It was proposed in
[#504](https://github.com/ruledicaprio/SynthPass/issues/504) and lands with the corrections that
issue's decision asked for.

## The answer in six lines

1. **Uniform substitution is the right closed-form baseline and the wrong risk model.** It is
   exact and content-free (7.03% / 10.09% / 10.00% for the document number at k = 1 / 2 / ≥3), but
   real errors are correlated. Two misreads with the *same* Δ — a dark scan turning every `0` into
   `O` — pass the check digit **25%** of the time at k = 2 (9 of 36 position pairs), not 10%; and
   two `2`↔`7` swaps pass **100%** of the time. Report both rows.
2. **The composite digit is an independent second equation only for the dates.** For the
   document number (TD1, TD2, TD3) and the TD3 personal number its weights line up with the
   field's own, so for errors inside those fields it is the same equation twice
   ([ADR-0021](../decisions/ADR-0021-fixed-grid-mrz-strips.md) Amendment 1 §2 derived this; the
   rates are new here). TD1 and TD2 optional data have no own digit, and the composite is their
   only cover. For dates, k = 2 goes 11.11% → 6.41% (DOB) / 5.47% (expiry), and k = 4 → 2.70% /
   1.93%. The floor for two equations is **2%, not 1%**: every weight is odd, so nothing in ICAO
   9303 can ever add information mod 2.
3. **The only `CONFUSABLES` pairs a check digit cannot separate are `1`↔`L` and `6`↔`G`.** The
   other fourteen cross residue classes; `repair.rs`'s description of the table said the reverse
   and is corrected alongside this note. A printed `6` read as `G` in a document number is
   accepted as read today, with no signal. The measured recognizer adds `0`→`<` (3), `A`→`0` (1),
   `W`→`M` (1): **5 of 126** aligned events (3.97%) are single-error blind, *below* the uniform
   model's 7.81%.
4. **Line 1 has no check digit, and the parser rejects no unlisted code.** A registry check
   *would* be a strong detector: a single substitution of a listed issuer or nationality code
   lands on another listed code only **616 / 21,840 = 2.82%** of the time (280 codes in
   `mrz::codes()`), against a check digit's 7.81% single-error blind share. Names get soundness
   guarantees only — grammar, separator structure, transliteration image — and that is worth
   stating as such.
5. **Exhaustive enumeration factors by field, and it verifies agreement, not soundness.** Under a
   4-alternative confusion model, all patterns with ≤ 4 errors on a document number plus its
   check digit are **62,200**; on a 14-cell personal number **380,300**. A field's check digit
   sees only that field, so the 35 M and 662 M whole-line figures are not needed. What enumeration
   can verify is that the parser agrees with the closed-form prediction; "never a different
   accepted string" is false as soon as the model holds a blind pair or a shared shift of 5.
6. **An external report needs three rows per denominator, not one rate.** Consistent /
   covered-fields-identical / byte-identical, each with a confidence interval. The live baseline
   is **139 / 151** (re-blessed 2026-09-24, [`README.md`](README.md)). Zero failures in **299**
   consecutive labelled reads gives a 99% lower bound at *one-sided* 95%; the two-sided 95%
   Clopper–Pearson interval needs **368**.

## 1. The algebra, completed

Character values `0–9 → 0–9`, `A–Z → 10–35`, `< → 0`; weights 7, 3, 1 cycling; check digit
`Σ vᵢwᵢ mod 10`. A read that changes value by Δᵢ at data cell i and by Δcd at the check-digit cell
is undetected iff

    Σ wᵢΔᵢ − Δcd ≡ 0 (mod 10).

[`blindspot.rs`](../../crates/mrz/src/blindspot.rs) states the single-substitution law and the
(7, 3) cancellation of two `0`↔`O` misreads. Three consequences go further.

**1a. Same-Δ cancellation has a closed form.** k substitutions with a common Δ pass iff
`Δ · Σwᵢ ≡ 0 (mod 10)`. For k = 2 with weights from {7, 3, 1}:

| Δ mod 10 | condition | position pairs that pass (9-cell field) |
| :--- | :--- | :--- |
| 0 | always | 36 / 36 (the single-error blind class) |
| 5 | Σw even — true for any two odd weights | **36 / 36** |
| odd, ≠ 5 | Σw ≡ 0 (mod 10) → only 7 + 3 | 9 / 36 = 25% |
| even, ≠ 0 | Σw ≡ 0 (mod 5) → only 7 + 3 | 9 / 36 = 25% |

The (7, 3) pairs are cells (0,1), (0,4), (0,7), (3,1), (3,4), (3,7), (6,1), (6,4), (6,7): nine,
not the three adjacent pairs `blindspot.rs` and the 2026-08-05 note named (both corrected). This is
the structural fact behind the 44 `0`↔`O` rows of the 2026-08-05 note. Exact for every k and Δ
(reproducer §B):

| document number, same Δ | Δ ≡ 0 | Δ ≡ 5 | Δ odd ≠ 5 | Δ even ≠ 0 |
| :--- | ---: | ---: | ---: | ---: |
| k = 2 | 100% | 100% | 25.0% | 25.0% |
| k = 3 | 100% | 0% | 0% | 21.4% |
| k = 4 | 100% | 100% | 11.9% | 11.9% |

Δ ≡ 5 is `2`↔`7` (both directions), `<`→`Z`, `5`↔`0`, `S`↔`D`, `Z`↔`U`. Two `2`/`7` confusions in
one field are invisible at *every* position pair; an odd number of them is always caught.
`crates/mrz/tests/checkdigit_algebra.rs` pins the law against `parse_td3` for every pair of cells
and every pair of shifts in the document number, and for every pair of cells and shared shift in
the personal number.

**1b. Nothing is ever gained mod 2.** All weights are odd, so every check sum is ≡ Σ Δᵢ (mod 2)
regardless of position, and every check digit in the standard — own or composite — agrees mod 2
with every other over the same cells. Two constraints can therefore be independent only mod 5,
and the joint floor is ½ · 1⁄25 = 2%.

**1c. Composite alignment is a per-field property.** The composite string restarts its 7-3-1
cycle at its own first cell. A field whose first cell sits at composite index ≡ 0 (mod 3) has the
*same* weight vector under both digits; errors inside its data cells satisfy one equation iff they
satisfy the other. ADR-0021 Amendment 1 §2 derived this for the document number and the TD3
personal number; reproducer §C, from ICAO 9303 Part 4/5/6 §4.2.2.2, gives every field:

| format | document number | dates | personal / optional |
| :--- | :--- | :--- | :--- |
| TD3 | aligned (index 0) | DOB index 10, expiry 17 — misaligned | personal index 24 — aligned |
| TD2 | aligned | DOB 10, expiry 17 — misaligned | optional data — composite only |
| TD1 | aligned | DOB 25, expiry 32 — misaligned | optional data 1 and 2 — composite only |

The document number's own *check-digit cell* is the one exception: it enters the own comparison
at −1 and the composite at +7 (composite index 9), so patterns that touch that cell are partly
separated — which is why the table in §3 shows 10.09% → 8.27% for the document number even though
its data cells are aligned.

## 2. The baseline model, and how to present two honestly

Reproducer §A. k cells chosen uniformly among the field's data cells plus its check-digit cell;
each substitution drawn from the model's Δ distribution.

| Δ model | field | k=1 | k=2 | k=3 | k=4 | k=5 |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: |
| uniform (37 symbols, content-free) | document number (9) | 7.03% | 10.09% | 10.00% | 10.00% | 10.00% |
| | date (6) | 0% | 11.11% | 9.88% | 10.01% | 10.00% |
| | personal number (14) | 7.29% | 10.08% | 10.00% | 10.00% | 10.00% |
| `CONFUSABLES` (both directions, 32 ordered pairs) | document number | 11.25% | 9.77% | 9.84% | 9.99% | 10.00% |
| | personal number | 11.67% | 10.23% | 9.92% | 9.99% | 10.00% |
| observed (126 events, 2026-09-21 §2) | document number | 3.57% | 13.22% | 10.56% | 10.34% | 10.12% |
| | date | 0% | 11.51% | 9.66% | 10.11% | 9.97% |
| | personal number | 3.70% | 13.42% | 10.65% | 10.42% | 10.15% |

Per-data-cell single-error blind share: uniform **104 / 1,332 = 7.81%**; `CONFUSABLES` **4 / 32 =
12.50%** (`1`↔`L`, `6`↔`G`); observed **5 / 126 = 3.97%**.

Two rows are deliberately absent. The `CONFUSABLES` date row is degenerate — the table's only
digit↔digit pair is `2`↔`7` (Δ ≡ 5), so it alternates 0% / 100% with k and says nothing about
dates. The observed date row rests on 6 digit↔digit events and is printed by the reproducer but
is not quotable.

**Reading it.** The uniform model is what the "exact for any k" claim is exact *about*: it is
content-free, has a closed form, and needs no corpus. It is not a risk estimate. Measured errors
are (a) rarer in the blind class than uniform predicts (3.97% vs 7.81%), because the recognizer's
big confusions — `<`→`C`, `0`→`O`, `<`→`E` — all cross residue classes; and (b) far more dangerous
at k = 2, because they arrive with the *same* Δ and cancel at the (7, 3) pairs. Neither the uniform
row (10.09%) nor the observed-marginal row (13.22%) captures that; §1a's 25% / 100% does.

**Presentation rule.** For every field, publish three numbers with their denominators and models
named: the closed-form uniform rate (no corpus), the same-Δ worst case for the field's dominant
confusion (no corpus; Δ named), and the confusion-weighted rate under the measured matrix (corpus,
event count, date). Never blend them into one "detection rate". When a larger measured confusion
matrix lands, the observed row is recomputed by replacing `OBSERVED` in the reproducer; the other
two rows do not move.

## 3. The composite digit, exactly

Reproducer §C, uniform model, k errors among the format's check-covered cells (data cells, own
check-digit cells, composite cell). Two probabilities: every own digit passes; every own digit
*and* the composite pass.

| errors anywhere in covered cells | k=1 | k=2 | k=3 | k=4 | k=5 | k=6 |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| TD3, own digits only | 6.99% | 2.87% | 1.07% | 0.48% | 0.24% | 0.14% |
| TD3, own + composite | 4.49% | 2.09% | 0.65% | 0.26% | 0.11% | 0.06% |
| TD2, own digits only | 27.20% | 8.63% | 3.46% | 1.68% | 0.92% | 0.56% |
| TD2, own + composite | 3.90% | 1.90% | 0.53% | 0.22% | 0.11% | 0.06% |
| TD1, own digits only | 54.32% | 29.74% | 16.53% | 9.40% | 5.51% | 3.36% |
| TD1, own + composite | 5.36% | 3.37% | 1.68% | 0.95% | 0.55% | 0.34% |

The "own digits only" row for TD1 and TD2 is high because those formats have 26 (TD1) and 7 (TD2)
optional-data cells with no own digit; the composite is their *only* cover. That is a layout
fact, not a weakness of the arithmetic.

Whether the composite lowers the ~10% floor for k ≥ 2 has a per-field answer, because own checks
are per field and errors spread over two fields must pass both (k = 1 in a date already passes 0%,
so cross-field k = 2 patterns are mostly dead on arrival; that is the 2.87% above).

| errors confined to one field (data + own cd cell) | k=1 | k=2 | k=3 | k=4 |
| :--- | ---: | ---: | ---: | ---: |
| TD3 / TD1 document number, own | 7.03% | 10.09% | 10.00% | 10.00% |
| TD3 / TD1 document number, own + composite | 7.03% | 8.27% | 7.33% | 6.44% |
| TD3 / TD1 date of birth, own | 0% | 11.11% | 9.88% | 10.01% |
| TD3 / TD1 date of birth, own + composite | 0% | **6.41%** | **3.61%** | **2.70%** |
| TD3 / TD1 expiry, own | 0% | 11.11% | 9.88% | 10.01% |
| TD3 / TD1 expiry, own + composite | 0% | **5.47%** | **2.35%** | **1.93%** |
| TD3 personal number, own | 7.29% | 10.08% | 10.00% | 10.00% |
| TD3 personal number, own + composite | 7.29% | 8.86% | 8.22% | 7.63% |
| TD2 optional / TD1 optional-1 / optional-2, composite only | 7.81% | 10.05% | 10.00% | 10.00% |

Verdict: **yes for dates, converging on the 2% floor; effectively no for the document number and
personal number** (the residual 10.09% → 8.27% is entirely the check-digit *cell* being weighted
differently, not the data). The same-Δ case makes it concrete: two `0`→`O`-type errors (Δ ≡ 4) at
two data cells of a TD3 date of birth pass the own digit 4 / 15 times and the pair of digits
**0 / 15**; in the TD3 document number they pass 9 / 36 under either. Both are pinned against
`parse_td3` in `crates/mrz/tests/checkdigit_algebra.rs`.

Implementation consequence: the parser already requires both digits, so nothing changes in
acceptance. What changes is *what the composite is evidence of*: on a document number it is not a
second witness, and a coverage vocabulary that says `own_check_digit` for the document number and
`own + composite` for a date would be telling the truth about the strength of each. The per-cell
map of [#421](https://github.com/ruledicaprio/SynthPass/issues/421) is where that vocabulary
belongs.

## 4. Deciding `S`/`8`, `G`/`6`, `1`/`L` inside a check-covered field

First, which ambiguities exist. Within `CONFUSABLES` the blind pairs are exactly **`1`↔`L` and
`6`↔`G`**; `8`↔`S`, `8`↔`I` and `5`↔`Z` are blind in the algebra but are not confusions the crate
generates or the recognizer has been seen to make (`8` wrong 1 in 52, `5`→`S` 1 in 30; 2026-09-21
§2). Measured blind events are `0`→`<`, `A`→`0`, `W`→`M`. Today a blind misread inside a field that
verifies is accepted as read, with no signal: a zone whose ordinary reading verifies never reaches
the damaged-capture pass, where the line-level `CONFUSABLES` sweep lives, and the parser does not
call `solve_substitution` at all.

The rule, in the order the evidence is strong, each step deterministic and each step's outcome
reportable:

1. **Grammar first — it is the only step that can prove.** If the issuer's document-number
   grammar fixes the cell's class (digit-only, letter-only, or a fixed literal), the ambiguity is
   resolved by construction and the check digit was never needed for that cell. The grammar must
   be a data table with provenance — issuer code, pattern, source specimen count, date — never a
   hand-typed regex; an issuer whose pattern has a single supporting specimen gets no entry. A
   cell the grammar leaves free falls through. (The corpus already carries per-issuer document
   numbers; deriving the table is a counting job with a stated denominator per issuer.)
2. **Glyph evidence second — it measures.** A per-cell classifier over the 37 OCR-B templates
   (ISO 1073-2 geometry; ADR-0014's fixed grid) scores both members of the residue class on the
   same cell crop. Accept the higher-scoring member only when the margin exceeds a threshold
   *calibrated on labelled cells and stated with its denominator* (e.g. "margin ≥ t separated
   6 from G on n / n labelled cells"); below the threshold, fall through. The 2026-09-21 note's
   §3 shows the recognizer is biased toward the denser glyph, so the margin must be measured per
   ordered pair, not assumed symmetric.
3. **Diverse multi-pass agreement third — it is weak, and only if the passes are independent.**
   Agreement between passes that share preprocessing is one vote counted twice. Count a pass as
   independent only when its binarization or scale differs; require unanimity among independent
   passes; still fall through on disagreement.
4. **Refuse.** Emit the field as `Ambiguous` with both candidates named and the class stated
   (`residue 6: {6, G}`), exactly as `Resolution::Ambiguous` already does for width repair. A
   wrong pick is indistinguishable from a proof; a refusal is not.

Two things the rule must never do: let the check digit break a within-class tie (it cannot; the
class is defined as what it cannot see), and let a prior from glyph frequency break it silently
(that is a guess with a nice name).

**A runtime flag was declined.** #504 proposed marking every verifying field that contains a cell
in `{1, L, 6, G}` as *consistent but unresolved*. Its
[decision](https://github.com/ruledicaprio/SynthPass/issues/504#issuecomment-5848367123) declined
it. **Observed** over the hand-transcribed fixtures, the flag would fire on the document numbers of
44 of the 64 distinct zones and on 21 of the 34 personal numbers — the objection that already
rules out the full residue atlas, which flags `K`, `U`, `A` on nearly every document number — and
it catches none of the five blind events actually measured. A per-field state on `Checks` would
also reopen [ADR-0017](../decisions/ADR-0017-checks-distinguish-absent-from-verified.md). The
honest statement belongs in #421's table of what a passing parse guarantees.

## 5. Line 1: what could be proven with no digit at all

**Codes would be a detector.** Reproducer §D over `mrz::codes()` (280 distinct codes, `D` padded
to `D<<`, out of 27³ = 19,683 three-cell strings, density 1.42%):

| substitution of a listed code | lands on another listed code |
| :--- | ---: |
| any single cell, any of the 26 other symbols | 616 / 21,840 = **2.82%** |
| any two cells | 10,384 / 567,840 = 1.83% |
| single cell, `CONFUSABLES` letter alternatives only | 4 / 113 = 3.54% |

A registry check would flag a single misread of the issuer or nationality field 97.2% of the
time — more than a check digit's 92.2% — and this is *content-free* in the same sense as §2's
uniform row. The parser runs no such check on a reading that verifies: `valid()` is checksum
consistency only, [`countries.rs`](../../crates/mrz/src/countries.rs) records that an unlisted
code can still be legitimate, and the registry is used to choose among line-1 repairs and to
recognise text that is not an MRZ at all. An unknown code is unresolved evidence, never proof
(ADR-0021 Amendment 1 §5). The residual 2.82% is a named list (616 ordered (code, cell, symbol)
triples), so the dangerous neighbours of any given code can be enumerated and, for the issuer,
cross-checked against the document-code and document-number grammar. Both codes being listed
*and* mutually plausible is a further constraint the reproducer does not model.

**Names get soundness, not correctness.** Provable rejections, each a finite check:

- alphabet: `A–Z` and `<` only (Part 3 §4.6);
- structure: `PRIMARY<<SECONDARY`, single `<` between components, no leading filler, exactly one
  `<<` separator, trailing filler run to the line edge (Part 3 §4.6) — every violation is a
  detected error, and the 2026-09-21 note's finding that half the name lines cannot even be
  aligned is a *structural* failure this grammar would catch at zero cost;
- transliteration image: every component is a string over the output alphabet of the Part 3 §6
  tables, so a component containing a sequence no table produces (a digit, a `<` inside a
  component) is rejected;
- length: components non-empty, total ≤ the field width, truncation only at the end.

None of these can detect a substitution of one valid letter for another inside a name. That is
the honest bound: **line 1 guarantees are "well-formed under the ICAO grammar", never "correct"**,
and the report should say so in those words. A VIZ cross-read is corroboration, not a check.

## 6. How far exhaustive enumeration of the repair search goes

Reproducer §E. `CONFUSABLES` gives 23 of 37 glyphs a row (widest 4, mean 32 / 23 = 1.39); 14 glyphs
have no plausible confusion at all. Patterns with 1..k errors, ≤ 4 alternatives per cell:

| scope | k ≤ 1 | k ≤ 2 | k ≤ 3 | k ≤ 4 |
| :--- | ---: | ---: | ---: | ---: |
| document number + cd (10 cells) | 40 | 760 | 8,440 | 62,200 |
| date + cd (7) | 28 | 364 | 2,604 | 11,564 |
| personal number + cd (15) | 60 | 1,740 | 30,860 | 380,300 |
| TD3 line 2 (44) — not needed | 176 | 15,312 | 862,928 | 35,615,184 |

The per-field figures are the real budget, because both layers factor by field: a field's own
digit sees only its cells (§1), and `solve_field` / `solve_substitution` take a field, not a
line. Cross-field coupling exists only through the composite, and §3 shows it is a *second
filter*, so a statement proven per field is a lower bound on the whole.

What enumeration can establish, stated cleanly:

> **Prediction agreement under (C, k).** For every printed field f in population F and every
> error pattern e with |e| ≤ k drawn from confusion model C, the parser's outcome on e(f) — each
> `Checks` entry, and whether a repair is returned — equals the outcome predicted independently
> from `Σ wᵢΔᵢ − Δcd ≡ 0` and the repair rules. A blind pattern is predicted as *accepted as
> read*, never as a rejection.

Soundness in the stronger sense — *never a different accepted string* — is false under any C that
holds a blind pair or a shared shift of 5: `6`→`G` at k = 1 and two `2`↔`7` at k = 2 verify and
are accepted as read. Inside the repair search, uniqueness holds only within its declared
candidate set (ADR-0021 Amendment 1 §5); with `MAX_SUBSTITUTIONS = 1`, the truth is outside that
set for k ≥ 2.

Three layers, three kinds of evidence:

- **Check-digit layer: closed form, for all content.** Detectability depends on the Δ residues
  only (§1), so "a pattern passes iff Σ wᵢΔᵢ − Δcd ≡ 0" needs no enumeration and no F.
  `crates/mrz/tests/checkdigit_algebra.rs` pins it against `parse_td3` for every two-cell pattern
  in the document number and the date of birth, and for every shared shift at two cells of the
  personal number.
- **Bounded search layer: enumerable within its own bounds.** `MAX_UNKNOWNS = 2` and
  `MAX_SUBSTITUTIONS = 1` mean the search space `solve_field` / `solve_substitution` ever explore
  is finite and small (≤ 37² unknown fillings; ≤ 256 single-glyph candidates,
  `MAX_SUBSTITUTION_CANDIDATES`). The soundness of the *search within its candidate set* — it
  never returns a non-verifying string, never returns `Unique` when two candidates verify — is a
  property-based statement over all inputs, and `fuzz_props.rs` is the right home for it.
- **Repair and structural gates as a whole: enumerated over a stated F.** Content matters here
  (date plausibility, filler structure, country recognition), so F must be named: the
  hand-transcribed fixtures (`samples/ocr_fixtures`, count stated) plus generator output
  (`synthpass-gen`, seeds stated). Prediction agreement at k ≤ 2 (≤ 760 patterns per document
  number) is the CI test, and it extends #421's "the map is honest" test. The k ≤ 4 enumeration
  (62,200 per document number) measures how often a wrong reading still verifies and comes back
  unique. That is a measurement for ADR-0021's Phase 0 atlas (P0.3), not a gate.

The sentence to publish: *"The parser agrees with the closed-form prediction for all error
patterns of up to 2 substitutions per field drawn from confusion model C (table dated, n
alternatives), over F = N fixtures + M generated documents, verified by enumeration; the
check-digit layer is proven in closed form for all content."* Nothing about correctness is
available by enumeration, and nothing should imply it.

## 7. The external report

Every rate as `k / n` with a 95% interval, never a bare percentage. Computing the intervals waits
for the report itself; the sample sizes below are closed form (reproducer §F),
n = ⌈ln α / ln p⌉:

| zero failures needed for a lower bound of | one-sided 95% (α = 0.05) | two-sided 95% Clopper–Pearson (α = 0.025) |
| :--- | ---: | ---: |
| 99.0% | 299 | 368 |
| 99.5% | 598 | 736 |
| 99.9% | 2,995 | 3,688 |

The rule of three (300 / 600 / 3,000) approximates the one-sided column. A published bound states
its side and is computed, not approximated.

**The consistency/correctness ladder** — three rows, same denominator, each a subset of the one
above. Only the first exists without labels; the other two need the labelled subset and its own n:

| row | meaning | what it is evidence of |
| :--- | :--- | :--- |
| checksum-consistent | every printed digit agrees | the read is in the pass set of §1–§3, which includes the blind patterns |
| covered-fields-identical | every check-covered cell equals the transcription | the arithmetic was not fooled on this document |
| byte-identical | the whole zone equals the transcription | correct, including line 1 |

Today the first row is 139 / 151 (baseline 2026-09-24). The 2026-09-21 base-rates note gives the
shape of the other two on the labelled subset (33 labelled hits; 26 carried mismatches, 308 of 314
in uncovered cells, 5 under an own digit, 1 under the composite) — those 6 covered cells on
passing documents are the blind set of §1 appearing in the wild and are the number the report
must not hide.

**Rules for every number in the report.**

- State the model next to the rate: *uniform / same-Δ / measured-matrix (n events, date)*; §2
  shows the same field reading 7.03%, 25% and 3.57% under three defensible framings.
- State the population next to the denominator: real specimens / synthetic / labelled subset,
  with the counts of each miss bucket, exactly as `real-specimen-mrz-baseline.json` does.
- Never say "detection rate" for a check digit without saying *of what pattern class*; say
  "single-substitution", "k = 2 same-Δ", or "under matrix M".
- "Verified" means a check digit agreed. "Correct" means a transcription agreed. The report uses
  both words and never one for the other.

## What this does not claim

- **No new measurement of the recognizer.** Every observed input is taken from the two cited
  notes; the 126-event matrix is miss-conditioned and pooled across fields, and the reproducer
  treats it as a marginal Δ distribution, not as per-field truth. The one count over tracked
  files, §4's 44 of 64 fixture document numbers, is reproduced below.
- **Uniform-position assumption.** k cells are chosen uniformly; real damage clusters (a finger,
  a hole). Clustered patterns are a subset of the enumerated ones, so the rates are averages over
  a population the field may not see.
- **No claim about the direction of confusions.** The observed model uses each event's Δ only;
  the recognizer is directional (2026-09-21 §3). Pass rates depend on Δ alone, but any decision
  rule built from the matrix must keep direction.
- **Code-set numbers depend on the table.** 280 codes at MAIN `011e9f1`; ISO 3166-1 changes, and
  the crate documents that an unlisted code can be legitimate. #504's first draft counted 278 by
  scraping `countries.rs` with a regular expression, which missed three entries rustfmt wraps
  onto several lines (`UNK`, `XCO`, `XPO`) and caught a test's `ZZZ`; the reproducer reads
  `mrz::codes()` instead.
- **No parser behaviour changes.** #504's decision declined §4's flag and replaced a proposed
  soundness gate with §6's prediction agreement (#421) plus a P0.3 measurement. This change
  corrects two rustdoc passages the algebra contradicts (`CONFUSABLES` in `repair.rs`, the (7, 3)
  pairs in `blindspot.rs`) and pins the laws as tests.

## Reproducing

```bash
cargo run -p mrz --example checkdigit_blindspots_exact   # sections A–F, about a second
cargo test -p mrz --test checkdigit_algebra              # the laws, against parse_td3
```

The example reads `mrz::CONFUSABLES`, `mrz::CLASSES` and `mrz::codes()` directly; its only
hand-entered input is `OBSERVED`, the 126 events of the 2026-09-21 note. Replacing it with a
larger measured matrix recomputes §2's third row and nothing else.

§4's fixture count, from the repository root:

```bash
python3 - <<'EOF'
import glob, json
zones = {}
for path in glob.glob('samples/ocr_fixtures/*.json'):
    fixture = json.load(open(path, encoding='utf-8'))
    if fixture.get('mrz_line'):
        zones.setdefault(fixture['mrz_line'], fixture)
flag = set('1L6G')
for field in ('document_number', 'personal_number'):
    values = [z[field] for z in zones.values() if z.get(field)]
    print(field, sum(1 for v in values if set(v) & flag), '/', len(values))
EOF
```
