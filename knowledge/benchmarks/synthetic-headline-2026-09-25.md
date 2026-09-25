# The synthetic headline is 370 / 500 today, stack 451 moves no seed, and 207 of the 370 hits are wrong somewhere

**Date:** 2026-09-25 · **MAIN:** `1595bf9` and `919b5ff` (crates identical to `174366b`), each plus the report-only counter of #457 · **DATA:** none (generated corpus, no `samples-data` input) · **Evidence:** Observed (ten local release `synthpass-bench --profile clean --count 100 --seed 0` runs, five formats × two builds) plus Derived (per-seed field comparison against the generator's exact truth, check-digit arithmetic, generator and emitter code) · **Status:** current

**2026-09-25, runs 06:16–07:45 UTC, one machine, sequential.** This re-measures the published
synthetic row in [`README.md`](README.md#current-headline-numbers), last observed in CI on
2026-09-14 at `9c8f03d` as 377 / 500. The row had not been re-measured since #440 and the stack
#446 → #447 → #448 landed. Two builds ran. Arm **main** is `1595bf9`, `origin/main` after #440
and #454. Arm **s451** is `919b5ff`, the top of stack 451, which is now merged as
`aae8733`/`cec2c66`/`3bf1466`. `git diff 919b5ff 174366b -- crates/ Cargo.lock` is empty, so the
s451 arm is the reader on `main` at `174366b`. Both arms carry a cherry-pick of
[#457](https://github.com/ruledicaprio/SynthPass/pull/457), which is not merged. It adds only
`wrong_accept`, `wrong_fields` and `wrong_accepts` to the report: 148 inserted lines in
`src/bin/synthpass-bench.rs`, no deletions, and no change to `hit`. The document set is generated,
so every value below is synthetic and safe to publish.

## The answer

| Format | Hits (was, CI 2026-09-14/21) | Hits now | Strict (both names exact) | Wrong accepts | Exact on all 12 scored fields |
| :-- | --: | --: | --: | --: | --: |
| TD3 | 78 | **74** | 40 | 40 | 34 |
| TD2 | 73 | **72** | 36 | 38 | 34 |
| TD1 | 54 | **52** | 15 | 37 | 15 |
| MRV-A | 85 | **85** | 50 | 51 | 34 |
| MRV-B | 87 | **87** | 59 | 41 | 46 |
| **All five** | **377 / 500** | **370 / 500 = 74.0%** | **200 / 500** | **207 of 370 hits** | **163 / 500** |

- **Observed:** both arms give the same result on every seed of every format: the same `hit` bit,
  the same `wrong_accept` bit and `wrong_fields` list, and the same per-field CER and `got` string
  on all 500 documents. Stack 451 has **no effect on the synthetic corpus**.
- **Observed:** 370 / 500. Hits are the harness's definition: a checksum-valid record whose
  document number matches truth. The two denominators are the same thing here. Every generated
  document carries a conforming zone, so there is no off-denominator population and the scored
  count is the corpus count, 500.
- **Observed:** **207 of the 370 hits (55.9%) differ from truth in at least one of the twelve
  scored fields.** 170 of them carry a wrong name. Only **163 / 500** documents are hits that
  are right on every scored field.
- **Derived:** MRV-A's `optional_data_1` column (31 of 85 hits wrong) is **genuine OCR misreads,
  not a truth-label defect**. 30 of the 31 are an `O`↔`0` swap in a field that MRV-A covers with
  no check digit. See the section below.

## What the 377 → 370 drop is, and what is not attributable

The before figure is CI (`bench-charts.yml`, rows on `bench-data` at `9c8f03d` on 2026-09-14 and at
`b0337e1` on 2026-09-21, both 78/73/54/85/87). The after figure is local. The CI rows are
aggregates, so there is no per-seed before-run at either CI commit.

| Change | Seeds | Attribution |
| :-- | :-- | :-- |
| TD3 −3 | 0–49 | **#440, Observed.** The overnight TD3 arm is byte-identical per seed (hit bit, every field CER, every `got`) on seeds 0–49 to the after arm of [`m4-gate-440-wrong-reads-refused-2026-09-25.md`](m4-gate-440-wrong-reads-refused-2026-09-25.md) (`69d5dac`, report SHA-256 `44f1d6f3…`). Against its before arm (`4a027cd`, `f5df922d…`) it differs in the hit bit on exactly seeds 7, 46 and 48, and in field output on those three plus 12 and 20. That finding showed all three refused reads were wrong. So 42 → 39 on seeds 0–49 is three wrong reads refused. |
| TD3 −1 | 50–99 | **Not attributable.** The −1 is inferred as the remainder of −4, and it rests on the assumption that CI's seeds 0–49 at `b0337e1` also read 42, which was measured locally at `4a027cd` and not in CI. |
| TD2 −1, TD1 −2 | all | **Not attributable.** No before-run exists at either CI commit. |
| MRV-A 0, MRV-B 0 | all | Unchanged in count. Per-seed stability is not known. |

Between `b0337e1` and `1595bf9`, 20 commits touched `crates/` or `Cargo.lock`. The candidates for
the unattributed −4 include the OCR-stack bump to `ocrs` 0.13.1 / `rten` 0.26
([#390](https://github.com/ruledicaprio/SynthPass/pull/390)), which measured zero outcome changes
on real specimens but was never measured on synthetic. They also include #440 itself, whose
damaged-pass rule applies to every format, not only TD3, and the ADR-0019 typed-value series. A
platform change is a candidate too: CI runs on Linux, this run on Windows. Count-level agreement
between local and CI was observed on 2026-09-16 at `c617254`, and it was not re-checked here.
**Hypothesis only; none of these is measured.** A CI `bench-charts.yml` run on current `main`
would settle the platform question, and a per-seed local run at `b0337e1` would settle the rest.

## Stack 451 is a null on this corpus, and why that is expected

**Observed:** zero seeds moved in any format. **Observed** from the build logs: each arm compiled
`mrz`, `synthpass-die` and `synthpass-bench` from its own worktree (`overnight-arm-main\crates\…`,
`overnight-arm-s451\crates\…`), so the null is not two builds of the same tree. It is not a
same-binary A/B. A null between two builds rules out an effect on these 500 seeds only, and says
nothing about the real-specimen gate, which is where the stack was measured.

**Derived**, why no synthetic seed can move:

- **#446** keeps a genuine Part 4 §4.4 document code (`PP`, `PS`, …). The generator always emits
  `P<` on TD3 (`DocumentType::document_code`), so no synthetic document carries one.
- **#447** acts only on a 45-cell TD3 line 1, a prefix **insertion**, whose as-read issuer does
  not resolve.
- **#448** stops the damaged pass inventing the format letter at cell 0. Not examined per seed.

The TD3 line-1 prefix errors that survive are **deletions**, the mirror image of #447's case.
Seven TD3 hits carry one, identical in both arms. In each, the filler at cell 1 is lost and the
line is re-padded to 44 cells, so the document code, the issuer and the surname are all wrong,
and no check digit covers any of them:

| Seed | Truth line 1 (prefix) | Accepted line 1 (prefix) | Wrong fields |
| --: | :-- | :-- | :-- |
| 18 | `P<RUSPETROV<<IVAN` | `PRUSPETROV<<IVAN` | document_type, issuing_country, surname |
| 26 | `P<UKRTKACHENKO<<NADIIA` | `PUKRTKACHENKO<NADIIA` | document_type, issuing_country, surname |
| 37 | `P<GBRNYSTROM<<LEILANI` | `PGBRNYSTROMLEILANI` | + given_names |
| 60 | `P<BGRZHUKOV<<MIKHAILO` | `PBGRZHUKOV<MIKHAILO` | document_type, issuing_country, surname |
| 66 | `P<BGRTKACHENKO<<IURII` | `PBGRTKACHENKO<<IURII` | document_type, issuing_country, surname |
| 72 | `P<JPNSTRAND<<DMITRI` | `PJPNSTRANDDMITRI` | + given_names |
| 86 | `P<AUSESKANDARI<<MILENA` | `PAUSESKANDARI<MILENA` | document_type, issuing_country, surname |

Seeds 18, 26 and 37 are the three the #440 finding named. 60, 66, 72 and 86 are in the seeds
50–99 that finding did not run. `line1_flagged` is `true` on all seven.

A repair for exactly this shape exists already: `unshift_if_country_resolves` puts `<` back at
cell 1 when the unshifted issuer resolves, and it would resolve for all seven (`RUS`, `UKR`,
`GBR`, `BGR`, `BGR`, `JPN`, `AUS`). It did not act on these reads. #446's description says
"`class_sweep_pass` and `damaged_pass` never reach the unshift", which would explain it if these
reads come from one of those passes. **Hypothesis:** which pass produced each read was not
instrumented. A deletion at cell 1 is among the mechanisms
[ADR-0021](../decisions/ADR-0021-fixed-grid-mrz-strips.md) lists as subsumed by the aligner. #446
also names a planned follow-up "#429 PR-b" in this area. Issue
[#429](https://github.com/ruledicaprio/SynthPass/issues/429) is closed, and its text is about a
right-shifted `PP` line, so whether that follow-up covers a deletion is not verified here.

The same shape is not TD3-only. On TD2, 13 of the 38 wrong accepts have a wrong `document_type`,
and TD2's line 1 has no check digit either. The per-seed TD2 shapes were not read out.

## The wrong accepts, by field

**Observed**, counting each field once per wrong-accept document. A document can carry several.

| Field | TD3 | TD2 | TD1 | MRV-A | MRV-B | All |
| :-- | --: | --: | --: | --: | --: | --: |
| `given_names` | 28 | 25 | 37 | 33 | 26 | **149** |
| `surname` | 25 | 33 | 36 | 17 | 25 | **136** |
| `optional_data_1` | — | 3 | 1 | 31 | 20 | **55** |
| `issuing_country` | 8 | 13 | 0 | 0 | 2 | **23** |
| `document_type` | 7 | 13 | 0 | 0 | 2 | **22** |
| `personal_number` | 7 | 0 | 0 | 0 | 0 | **7** |
| `optional_data_2` | 0 | 0 | 2 | 0 | 0 | **2** |
| `nationality` | 1 | 0 | 0 | 0 | 0 | **1** |
| **Wrong-accept documents** | **40** | **38** | **37** | **51** | **41** | **207** |

`document_number`, both dates and `sex` are never wrong on a hit. The harness requires the
document number to match, and the three check digits cover the rest. Almost all the wrong fields
are ones no check digit covers: line 1 on every format, and the optional data on MRV-A and
MRV-B. The 13 wrong accepts on covered fields are all check-digit collisions. The misread zone
validates, recomputed by hand, because its substitutions cancel mod 10:

- TD3 `personal_number`, 7. Each has 2–3 substitutions that pass both the personal-number digit
  and the composite (seeds 3, 15, 16, 36, 49, 56, 93).
- TD2 `optional_data_1`, 3. They pass the composite (table below).
- TD1 `optional_data_1`/`_2`, 3. They pass the composite (seeds 11, 55, 91).

Seed 55 of TD1 needs only one substitution, `<`→`K`. In the check-digit arithmetic `<` is 0 and
`K` is 20, so the difference is 0 mod 10 at every weight. `0`, `<`, `A` (10), `K` (20) and `U`
(30) are all worth 0 mod 10, and a single swap among them is invisible to every ICAO check
digit. On TD1, 34 of
the 37 wrong accepts are wrong in names only.

## MRV-A `optional_data_1`: OCR misreads, not a label defect

The question was whether 31 of MRV-A's 85 hits come from a truth value the zone cannot carry,
like [#410](https://github.com/ruledicaprio/SynthPass/issues/410)'s truncated personal number,
or from genuine OCR misreads.

- **Derived, from the code: the truth label is the printed zone.** `synthpass-bench` builds truth
  by parsing the generator's own emitted MRZ lines through the matching per-format parser
  (`parse_ground_truth_mrz`, `crates/synthpass-bench/src/lib.rs`), not from the `Passport`
  struct. A value the zone truncated cannot reach the truth side. #410's VIZ/MRZ mismatch is a
  visual-zone problem and does not enter this bench. Separately, the generator draws a
  14-character value (`random_personal_number`, `crates/synthpass-gen/src/data.rs`) and MRV-A's
  optional-data element is 16 cells wide. MRV-A never truncates it, which is why #410 names only
  TD1, TD2 and MRV-B.
- **Observed, from the report: the misreads are small and of one kind.** The report stores the
  full expected and read zone (`mrz_lines.expected` / `.got`) whenever the zone read is not
  exact. On all 31 documents the read line 2 has the same length as the truth line. The
  optional-data CER is **1/14** on 25 documents, **2/14** on 5 and **3/14** on 1. None is near 1,
  which is the signature a label mismatch would leave. The 38 substituted cells are
  **`0`→`O` 18, `O`→`0` 17**, `I`→`1` 1, `Q`→`G` 1, `7`→`Z` 1. **30 of the 31 documents contain
  an `O`↔`0` swap.** Seed 99's single `7`→`Z` is the only document without one.
- **Derived, from `mrz::emit` and Doc 9303 Part 7: nothing can catch it.** MRV-A has no check
  digit over its optional data and no composite check digit. A misread there always survives
  into an accepted hit. 16 of MRV-A's 51 wrong accepts are wrong in `optional_data_1` alone.
- **Modelled, why MRV-A leads MRV-B (31 against 20):** the generator draws uniformly from 36
  symbols, two of which are `O` and `0`. A 14-cell value contains at least one of them with
  probability 1 − (34/36)^14 ≈ 0.55, and MRV-B's 8-cell truncation with ≈ 0.37. That is a ratio
  of 1.5, against an observed ratio of wrong rates of 1.6 (31/85 against 20/87). MRV-B's 20 have
  the same shape: 18 contain an `O`↔`0` swap, and the other substitutions are `I`→`T` 3,
  `Q`→`G` 3, `Z`→`2` 1 and `1`→`<` 1.

**Conclusion (Derived):** genuine OCR misreads, dominated by the `O`/`0` confusion already
measured on real specimens ([`observed-ocrb-confusions-2026-09-21.md`](observed-ocrb-confusions-2026-09-21.md)),
landing in a field no check digit covers. It is not a label defect. The generator does make the
exposure large, because it fills a 14-cell field with uniform random alphanumerics. How often
real visas carry letter/digit-ambiguous optional data is not measured.

**The three TD2 `optional_data_1` wrong accepts are composite collisions (Derived, recomputed).**
TD2's composite does cover the optional data. In each of the three, two substitutions cancel
mod 10, and the printed composite digit validates both the truth and the misread line 2:

| Seed | Truth `optional_data` | Read | Composite (printed = computed, both lines) |
| --: | :-- | :-- | :-- |
| 11 | `4MO9LO0` | `4M09L0O` | 6 |
| 12 | `XDOZ4GY` | `XD074GY` | 3 |
| 93 | `X0U0IF2` | `XOUOIF2` | 4 |

`O` is worth 24 and `0` is worth 0, so a swap adds a multiple of 24 times the weight. Two swaps at
weights 7 and 3 add 240, which is 0 mod 10 (seed 93). This is the "a check digit is consistency,
not proof" case, occurring naturally on clean synthetic data.

## What this does not claim

- **Not a CI measurement of the arms.** Every "now" figure is one local Windows run per arm.
  **Update, 2026-09-25:** `bench-charts.yml` [run 36111444345](https://github.com/ruledicaprio/SynthPass/actions/runs/36111444345) at `f80877b` (v1.6.1, crates equal to
  `174366b`'s apart from the mrz version and changelog) observed the same counts on Linux, format
  by format: TD1 52, TD2 72, TD3 74, MRV-A 85, MRV-B 87. That settles the platform question
  raised above: the −7 is not a Windows/Linux difference. CI compares counts only, not seeds.
- **Not that stack 451 has no effect.** It has none on these 500 generated documents. Its
  measurement is the real-specimen gate.
- **Not an attribution of TD3 seeds 50–99 (−1), TD2 (−1) or TD1 (−2)** to any change. The
  −3 on TD3 seeds 0–49 is attributed to #440 only by per-seed identity with that finding's arms.
- **Not run-to-run stability.** Each arm ran once, although the two builds agreeing on all 500
  documents, `got` strings included, bounds inference noise on this corpus.
- **Not that the 163 fully exact hits are all the correct reads the reader can make.** The count
  is exact-on-twelve-fields among hits. A miss can still carry correct fields.
- **Not a gate proposal.** `wrong_accepts` is report-only in #457, and whether to gate on it is
  [#453](https://github.com/ruledicaprio/SynthPass/issues/453).
- **Timing is not a result.** No contention check was made. Per-format wall times in `run.log`
  (TD1 about 13 min, the others about 5–7 min) include OCR and are not comparable across machines.

## Invocation

Runner `overnight-synth.ps1` (local, not committed), once per arm in its own worktree
(`overnight-arm-main` at `07a4ca3` = `1595bf9` + #457's commit, `overnight-arm-s451` at `f8d69b1`
= `919b5ff` + the same commit), for `<fmt>` in `td3 td2 td1 mrva mrvb`:

```
cargo build -p synthpass-bench --release --bin synthpass-bench
cargo run -p synthpass-bench --release --bin synthpass-bench -- \
  --document-type <fmt> --profile clean --count 100 --seed 0 --out <arm>-<fmt>.json
```

Every run exited 0. Report SHA-256 prefixes: main `8191c816` (td3), `f725500e` (td2), `ad6c75cd`
(td1), `5cdbc245` (mrva), `c57aaa90` (mrvb); s451 `4102729e`, `06da858d`, `fb6273ef`,
`ae3a5c2c`, `f1e11eb6`. The reports are local and not committed. The per-seed comparisons are
standard-library Python over `results[]`, kept beside this finding's working copy as
`artifacts/synth-headline-verify-2026-09-25.py`, `artifacts/synth-mrva-od1-2026-09-25.py`,
`artifacts/synth-od1-confusions-2026-09-25.py`, `artifacts/synth-td3-vs-m4-2026-09-25.py` and
`artifacts/synth-covered-field-collisions-2026-09-25.py` (not committed).

## Rejected on the way

- **Reading 377 → 370 as a regression.** Rejected: three of the seven are #440 refusing wrong
  reads (Observed per seed), and the other four have no before-run to compare against.
- **Reading MRV-A's 31 as a #410-style label defect.** Rejected: truth is parsed from the printed
  zone, MRV-A does not truncate the value, line lengths match, and the CER is 1–3 cells of 14.
- **Crediting stack 451 with a synthetic effect, or citing its null as evidence about real
  specimens.** Rejected: nothing moved, and none of its three preconditions occurs in the
  generator's output by construction, except possibly #448's, which was not examined.
- **Counting `strict_hits` or exact-on-all-fields as the new headline.** Rejected: the headline
  stays the harness's `hit`. The stricter counts are published beside it.
