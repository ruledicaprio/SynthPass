# Tier-2 date misses were mostly normalizer gaps, 2026-09-04

**14 of the 70 date mismatches in the parity corpus were the model reading the
printed date correctly while `synthpass-core::normalize` discarded the reading.**
Fixing the two parsing gaps moves overall Tier-2 field accuracy from **48.1% to
52.5%** (156 → 170 of 324 scored comparisons) with no prompt change, no
`PROMPT_VERSION` bump, and no model involved.

## Confirmed by a full run

Predicted offline by replaying the previous run's raw answers through the new
normalizer, then **confirmed by a full parity run on the same corpus and model**
(`qwen2.5-1.5b-instruct-q4_k_m`, greedy, ~35 min):

| | baseline | with the fix | Δ |
| :-- | --: | --: | --: |
| reviewed (9 fields, 18 docs) | 78/162 — 48.1% | 85/162 — 52.5% | +7 |
| derived (3 fields, 54 docs) | 78/162 — 48.1% | 85/162 — 52.5% | +7 |
| legacy 7-field over reviewed | 54/126 — 42.9% | 61/126 — 48.4% | +7 |
| **overall** | **156/324 — 48.1%** | **170/324 — 52.5%** | **+14** |

The replay predicted 14 and the run produced 14, split evenly across two fixture
sets that share only three scored fields. Both halves moving by the same amount
is what a date-only change should look like — the derived set scores
`document_number` and the two dates, so two of its three fields are exactly the
ones touched.

This is also the fourth time this harness has reproduced a number exactly across
runs (the 2026-09-02 and 2026-09-04 baselines agreed at 43/162 before the
normalization fix rebased them). Treat a small delta from it as real.

## How this was found — and what it says about the next piece of work

This started as the first step of `VIZ_TIER2_DESIGN.md` §2.3 ("ask narrowly").
Before building anything, the per-field detail of the corrected 2026-09-04
parity run (`artifacts/parity2-baseline.log`, the run recorded in
`parity-mrz-holdout-2026-09-04.md`) was classified. Two findings, in order of
how much they changed the plan:

**1. The model almost never omits a field.** Only **6 of 168** misses are
`null`. §2.3's premise — tell the model which fields are still missing so it
stops leaving them out — therefore has almost no target on this corpus. The
model answers essentially always; it answers *wrong*, or answers in a form
nothing downstream accepts.

**2. Dates are 74 of the 168 misses**, the largest single block, and a large
share of them are not model failures at all:

| Raw Tier-2 answer | Ground truth | Why it was discarded |
| :-- | :-- | :-- |
| `01 10 1990` | 1990-10-01 | the numeric branch split on `-`, `.` and `/` only |
| `22 FEB 78` | 1978-02-22 | two-digit year: `22` and `78` are the same width, so the strict named-month parser saw two conflicting days and refused |
| `24 CEP/AUG 91` | 1991-08-24 | both gaps at once, plus a misread month name beside a good one |
| `25.12 1982` | 1982-12-25 | mixed separators |

Miss-kind classification over all 168:

| Class | Count | Share |
| :-- | --: | --: |
| different value | 122 | 73% |
| substring / format disagreement | 27 | 16% |
| MRZ fragment returned as a field | 8 | 5% |
| `null` | 6 | 4% |
| another field's value | 5 | 3% |

The "different value" class is the one that hides this: a date the normalizer
refused passes through raw, so `01 10 1990` is scored as a different value from
`1990-10-01` rather than as a formatting failure.

## The two rules

1. **Whitespace is a separator**, alongside `-`, `.` and `/`. Empty components
   are dropped, so a mixed or doubled separator (`25.12 1982`, `1 / 7 / 2031`)
   reads as one. The "neither end is a 4-digit year → don't guess" rule is
   untouched: `07 04 76` still passes through raw exactly as `07-04-76` did.

2. **Named months are read positionally when the strict parser declines** —
   digits before the month are the day, digits after are the year. This is the
   only way to resolve a two-digit year, where width cannot separate day from
   year. It runs *only* after the strict parser has refused, which is
   load-bearing: a bilingual line printing the day on both sides of the month
   (`01 JAN/JAN 01 2000`) has two leading digit groups and is refused
   positionally, but the strict parser reads it correctly by comparing day
   *values*.

A two-digit year is only resolvable if you know which field it came from —
`22 FEB 78` is 1978 as a birth date and 2078 as an expiry. So the century
decision needs the field, and it is routed through **`mrz::expand_date`**, whose
pivot is audited, documented as this workspace's own heuristic rather than an
ICAO rule, and CI-checked by `scripts/check-century-pivot.sh`. New entry points
`normalize::date_of_birth` / `normalize::date_of_expiry` carry that knowledge;
the field-agnostic `normalize::date` still refuses a two-digit year rather than
guessing.

## Why no currently-passing date can regress

Every date row scored `OK` in the parity log is already ISO `YYYY-MM-DD` by the
time it is compared (checked mechanically over the log: zero exceptions), and
ISO passes both parsers unchanged. The positional parser runs strictly after the
existing one, so it can only act on strings that previously produced nothing.

## The 56 date misses that remain

Genuinely wrong reads, and they are not a normalization problem: day/month
swaps (`2014-07-12` for 2014-12-07), century errors (`1911-01-11` for
2011-01-01), off-by-one days, and dates read from the wrong line of the
document. Those are the population §2.3's narrow ask and §2.2's per-line
geometry would have to address, and they are worth re-measuring against *this*
baseline rather than the old one.

## What this implies for §2.3

Not that §2.3 is wrong, but that its stated first lever is spent. Two of its
three parts are now on weaker ground than when the design was written:

- **"Tell it which fields are missing"** — 6 of 168 misses are nulls.
- **"Give it the verified MRZ fields"** (the `mrz_hint` from issue #102, built
  and gated off since) — **unmeasurable on this corpus by construction.** The
  hint emits only checksum-proven fields, and every parity fixture's ground
  truth *is* its checksum-valid MRZ, so a hint arm would be scored against the
  answer it was handed. Measuring it honestly needs a corpus of
  checksum-*partial* documents, which is the same synthetic-construction trick
  the MRZ-holdout mode already uses on the whole-MRZ case.
- **"Say which form you want"** — partly addressed already: normalization took
  `document_type` from 6% to 67% and `issuing_country` from 22% to 67%.

The 27 remaining format disagreements (16% of misses) are still the second
largest class after genuine misreads, and `nationality` — `CANADIAN/CANADIENNE`
where the truth is `CAN` — is where most of them sit.
