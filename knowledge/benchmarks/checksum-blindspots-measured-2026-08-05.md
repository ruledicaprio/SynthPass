# Check-digit blind spots, measured — 2026-08-05

What actually gets past an ICAO 9303 check digit in practice, as opposed to what
the closed-form law says can. `crates/mrz/src/blindspot.rs` proves the law; this
note is the empirical distribution, and the two disagree about which substitutions
matter.

## Method

Source: the `bench-data` branch's `dataset.jsonl`, which
`.github/workflows/bench-data-collection.yml` appends to on each scheduled run
(one row per generated document: `seed`, `profile`, `hit`, `reason`).

Rows analysed: every row whose `reason` is a `document number mismatch`, across the
branch's full history — **76 rows, seeds 4131219–4134499**, spanning all five
capture profiles. Each row records the OCR's read and the generator's ground truth,
so the comparison is against labels that are correct by construction (M2's DoD),
not against another model's opinion.

For each row, both strings were scored with the 7-3-1 mod-10 check digit
(`mrz::checksum::check_digit`'s algorithm) to ask: *would a perfectly-read check
digit have caught this?*

**Selection bias, stated up front:** these rows are conditioned on having **passed**
the checksum gate — a read the check digit rejected is recorded as `checksum
invalid` instead and never appears here. In the 2026-08-05 06:15 run, 52 of 57
misses were `checksum invalid` and 5 were mismatches. So this is not "the checksum
fails 76 times"; it is "here is what the residue is made of, once the checksum has
done its job."

## Results

| | count | share |
|---|---|---|
| Total document-number mismatches | 76 | |
| `cd(got) == cd(expected)` — **no check digit could catch it** | 74 | 97% |
| `cd(got) != cd(expected)` — got through because the check digit character was *itself* misread | 2 | 3% |

Characters differing per row:

| differing chars | 1 | 2 | 3 | 4+ |
|---|---|---|---|---|
| rows | 25 | 41 | 7 | 3 |

**Only a third of real misreads are single-character.** That is the finding
everything below follows from.

### Substitution pairs inside the undetectable set

| pair | Δ ICAO value | count | pairwise verdict |
|---|---|---|---|
| 0↔O | 24 | 44 | *caught* |
| G↔Q | 10 | 16 | blind |
| 7↔Z | 28 | 9 | *caught* |
| 0↔U | 30 | 8 | blind |
| 5↔S | 23 | 6 | *caught* |
| 1↔L | 20 | 5 | blind |
| O↔Q | 2 | 5 | *caught* |
| I↔T | 11 | 5 | *caught* |
| 1↔I | 17 | 4 | *caught* |
| 2↔Z | 33 | 4 | *caught* |

(Tail omitted; 28 distinct pairs in all.)

## What this means

`blindspot.rs`'s law is correct: a **single** substitution is invisible iff the two
ICAO values are congruent mod 10. Its practical guidance — that O↔0, I↔1, B↔8, S↔5,
Z↔2 are "all caught" and the quiet pairs K↔`<`, I↔S, B↔L, A↔K are the real danger —
does not survive contact with this data. The pairs it reassures you about dominate
the undetectable set, and of the quiet pairs it names, **exactly one** K↔`<` appears
in 76 rows.

The mechanism is multi-character cancellation. A misread spanning k positions shifts
the weighted sum by `Σ Δᵢwᵢ mod 10`, so individually-caught substitutions cancel
each other. The common case is two 0↔O at weights 7 and 3: `24 × (7 + 3) = 240 ≡ 0`.
Each alone would have been caught — `24 × 7 ≡ 8`, `24 × 3 ≡ 2` — but together they
are invisible. For a 9-character document number the weights run 7,3,1,7,3,1,7,3,1,
so the (7,3) pairing recurs at positions (0,1), (3,4) and (6,7). This is structural,
not a coincidence of one seed.

Practical consequence: **the check digit is a strong filter and a weak oracle.** It
rejects most misreads (52 of 57 in the sampled run), and what it passes is not a
random remainder — it is specifically the set arithmetic cannot see. In the bench we
notice because ground-truth labels exist. In production they do not, so a read in
this class is reported as a confident Tier-1 success. That is the argument for the
structural guards `mrz` already layers on top (recognised country codes, date
plausibility, name charset), and against treating checksum validity alone as proof
of a faithful read.

## Follow-up, not done here

`blindspot(a: char, b: char)` is pairwise by signature, so it cannot express the
case this note is about. A sequence-level companion — something like
`blindspot_seq(got, expected)` computing `Σ Δᵢwᵢ mod 10` position-aware — would let
callers ask the question the data actually poses. Deliberately not added with this
note: it is public API on a published crate (`mrz` 0.6.3), so it needs a minor
version bump, semver CI and its own tests, and it should be designed against more
than one corpus.

## Reproducing

```bash
git fetch origin bench-data
git show origin/bench-data:dataset.jsonl > dataset.jsonl
# filter rows whose .reason matches 'document number mismatch',
# then score both strings with the 7-3-1 mod-10 weighting
```
