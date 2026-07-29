# Nightly corpus, first read — 2026-07-29

First analysis of the `bench-data` corpus since collection started on 2026-07-21.
Recorded because the dataset had been accumulating for eight days with nothing
ever looking at it.

## Invocation

Not a sweep. This is the accumulated output of
`.github/workflows/bench-data-collection.yml`, which runs

```
cargo run -p synthpass-bench --release --bin synthpass-bench -- \
  --count 200 --seed <window> --profile all --out bench-report.json
```

once a day and appends to `dataset.jsonl` on the orphan `bench-data` branch.

```
git fetch origin bench-data
git show origin/bench-data:dataset.jsonl
```

**n = 1610** rows (322 per profile), 2026-07-21 → 2026-07-29, across seven
`main` commits. Deduplicated: see "Corpus integrity" below.

## Hit rate by profile

"Hit" is `synthpass-bench`'s definition — the MRZ provider produced a
checksum-valid record whose document number matches ground truth — not "the
extraction was correct". Tier 2 is not exercised here.

| Profile | n | Hit rate | Median ms |
|---|---:|---:|---:|
| `mobile` | 322 | 68.3% | 3143 |
| `scanner` | 322 | 65.8% | 3810 |
| `clean` | 322 | 63.0% | 3899 |
| `worn` | 322 | 58.7% | 4342 |
| `border-kiosk` | 322 | 42.2% | 4603 |
| **overall** | **1610** | **59.6%** | — |

Consistent with the 55–56% sequential figure `benchmarks/README.md` records as
the honest number.

## The finding: only `border-kiosk` separates

`clean` scores *below* `mobile` and `scanner`, which reads as a result until you
check it. At n = 322 per profile it is not one:

| Comparison | Difference | z | Verdict |
|---|---:|---:|---|
| `clean` vs `mobile` | +5.3pp | +1.41 | not significant |
| `clean` vs `scanner` | +2.8pp | +0.74 | not significant |
| `clean` vs `worn` | −4.3pp | −1.13 | not significant |
| `clean` vs `border-kiosk` | −20.8pp | **−5.41** | significant |

So the honest statement is: **four of the five profiles are statistically
indistinguishable, and only `border-kiosk` degrades the pipeline measurably.**

That is worth knowing in both directions. It may mean the OCR path is genuinely
robust to the mobile/scanner/worn degradations — or that those profiles are not
degrading hard enough to separate from `clean`, in which case four fifths of
every nightly run is measuring the same thing four times. `ADVERSARIAL.md`
describes what each profile intends to simulate; nothing here says the intent is
being realised. Answering that needs a run with the degrade parameters recorded
per row, which the current schema deliberately omits (they are recoverable from
`seed`, but not present).

**Do not tune anything on the clean/mobile/scanner/worn ordering.** It is noise
at this sample size. The `border-kiosk` gap is the only signal in this table.

## Failure modes

650 misses. `checksum invalid` dominates every profile (591, 91%). `no MRZ
found: NotFound` is rare (25) and, notably, ordered backwards: most common on
`clean` (10), absent entirely on `border-kiosk` (0). Whatever costs
`border-kiosk` its 20pp is not the detector failing to find a zone.

34 misses are document-number mismatches where the MRZ read cleanly but got a
character wrong, and the pairs are almost all the classic OCR confusions:

```
got "0094XETJ5", expected "OO94XETJ5"    0 ↔ O
got "ZJ400C9HZ", expected "ZJ4OOC9HZ"    0 ↔ O
got "MIAFLBLSR", expected "MIAF4BL5R"    L ↔ 4, S ↔ 5
got "IT2KIUNFS", expected "JT2K1ONFS"    I ↔ J, I ↔ 1, U ↔ O
```

These are the substitutions the ICAO check-digit arithmetic is *sometimes* blind
to — `mrz::collisions` and the `blind_positions` evidence field already model
this. Cross-referencing the mismatch set against `blind_positions` is the
obvious next question and is not answered here.

## Corpus integrity

The dataset had 1810 rows until 2026-07-29; 200 were duplicates. The nightly
seed window was derived from `days_since_epoch * count`, so a manual dispatch on
2026-07-25 alongside the scheduled run re-measured the same 200 documents.
Removed (1810 → 1610), and the workflow now continues from the dataset's own highest seed and
refuses an append that would duplicate a `(seed, profile)` pair.

Every duplicate pair agreed on `hit` — an unplanned confirmation that
`synthpass-gen` is deterministic from `seed` across runners and commits, which
is the assumption the whole "the seed is enough to regenerate the document"
design rests on. It had not previously been tested.
