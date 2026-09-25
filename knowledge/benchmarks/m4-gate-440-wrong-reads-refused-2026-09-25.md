# #440 on the M4 synthetic gate: 42 / 50 → 39 / 50 is three wrong reads refused, none lost

**Date:** 2026-09-25 · **MAIN:** `69d5dac` (after, #440 merged) against `4a027cd` (before, its parent) · **DATA:** none (generated corpus, no `samples-data` input) · **Evidence:** Observed (two local release `synthpass-bench --count 50 --seed 0 --profile clean` runs, one per commit, plus single-seed re-runs of the before arm and an instrumented replay of the pre-#440 parser) plus Derived (per-seed field comparison against the generator's exact truth) · **Status:** current

**2026-09-24, finished 22:03 UTC; written up 2026-09-25.**
[#440](https://github.com/ruledicaprio/SynthPass/pull/440) (issue [#431](https://github.com/ruledicaprio/SynthPass/issues/431)) changed `single()` in
`crates/mrz/src/parser.rs`, the unanimity gate behind `find_and_parse`'s damaged-zone recovery,
from comparing six fields to comparing the whole `MrzData` except `mrz_lines`. On real specimens it
refused one manufactured hit (Bosnia 2013, the
[2026-09-24 `FINDINGS.md` entry](FINDINGS.md#2026-09-24--a-manufactured-hit-refused-bosnia-2013-card-back-leaves-tier-1-140--152--139--151)).
This is the same change measured on the synthetic corpus that `ci.yml`'s `m4-hit-rate` job gates
every PR on: 50 clean TD3 documents, seeds 0–49. The corpus is generated, so every field of every
document has exact truth, and all values below are synthetic and safe to publish.

## The answer

- **Observed:** the gate's count went **42 / 50 = 84% → 39 / 50 = 78%**. Exactly three seeds
  changed outcome — **7, 46 and 48** — each `hit` → `checksum_failed` (`personal_number,
  composite`). The seed sets are identical, and no seed moved the other way. Both runs exited 0:
  `m4-hit-rate` is a floor at 0.30, not a no-regression check, so this drop could not fail CI.
- **Derived:** all three old hits were **wrong reads**. Each had a wrong `document_type`, a wrong
  `issuing_country` and a wrong `surname` (table below), and two of the three also a wrong
  personal number. No old accepted read matched truth on every field. The drop is **three wrong
  reads refused and zero correct reads lost**.
- **Derived:** by the gate's own definition, net of those three wrong accepts, the pre-#440 count
  was already **39 / 50**. The 42 was 39 plus three reads that the checksum arithmetic accepted and
  the truth rejects. #440 moved what the gate counts, not what the reader reads correctly.
- **Derived:** the stricter measures moved the other way. `strict_hits` (both names exact) went
  **20 / 50 → 21 / 50**, and hits exact on all twelve scored fields went **15 / 50 → 16 / 50**.
  Both gains are seed 20, which stayed a hit but changed its read (see "Two hits changed their
  read" below).

The 39 is not "39 correct reads". After #440, 34 of the 39 hits still differ from truth in at least
one field or in `mrz_lines`, and 23 of them in at least one of the twelve scored fields. That is the
subject of "The gate cannot see this" below.

## The three reads #440 refused

Values are the before arm's accepted read against the generator's truth. The document number,
both dates, the nationality and the sex on line 2 are correct on all three.

| Seed | Field | Truth | Accepted before #440 |
| ---: | --- | --- | --- |
| 7 | `document_type` / `issuing_country` | `P` / `GBR` | `PG` / `BRC` |
| 7 | `surname` / `given_names` | `CASTELLANO` / `MATEO` | `ASTELLANO` / `MATEOCCCCCCCKCC` |
| 7 | line 1 | `P<GBRCASTELLANO<<MATEO<<<…` | `PGBRCASTELLANO<MATEOCCCCCCCKCC<<<…` |
| 46 | `document_type` / `issuing_country` | `P` / `SWE` | `PS` / `WES` |
| 46 | `surname` / personal number | `STRAND` / `TFSQ68OYV81RJE` | `TRAND` / `TFS0680YV81RJE` |
| 46 | line 1 | `P<SWESTRAND<<ANIKA<<<…` | `PSWESTRAND<ANIKA<<<…` |
| 48 | `document_type` / `issuing_country` | `P` / `FRA` | `PF` / `RAA` |
| 48 | `surname` / personal number | `ADEYEMI` / `2MSUV5TH7AB8IO` | `DEYEMI` / `2MSUVSTH74B810` |
| 48 | line 1 | `P<FRAADEYEMI<<MATEO<<<…` | `PFRAADEYEMI<<MATEO<<<…` |

All five check states were `true` on each old read. After #440 each seed returns the invalid
fallback parse, which fails its personal-number and composite digits and is not counted. The
fallback's line-1 prefix is correct on all three (`P<GBR`, `P<SWE`, `P<FRA`).

## Mechanism: two candidates that differ only in the personal number

**Observed**, from a replay of each seed's synthetic raw OCR through an instrumented copy of the
pre-#440 `mrz` crate that logged `single()`'s input (run locally by the A/B's author, not committed
and not reproduced here). Each replay's damaged pass produced exactly **two** distinct candidates.
Both were checksum-valid on all five TD3 check digits. They agreed on every `MrzData` field except
`optional_data_1`, which on TD3 is line 2 positions 29–42, the personal-number slot.
`synthpass-bench` reports that slot under its `personal_number` column, which is why the bench's
own `optional_data_1` column is empty on both sides.

| Seed | Candidate 0 (old accepted) | Candidate 1 | Truth |
| ---: | --- | --- | --- |
| 7 | `QH0NTS6JNPIS3K` | `QHON1S6JNPIS3K` | `QH0NTS6JNPIS3K` |
| 46 | `TFS0680YV81RJE` | `TFSQ68OYV81RJE` | `TFSQ68OYV81RJE` |
| 48 | `2MSUVSTH74B810` | `2MSUVSTH7AB81Q` | `2MSUV5TH7AB8IO` |

The old rule compared the document number, both dates, both names and the nationality. The
candidates agree on all six, so the rule counted them as unanimous and returned candidate 0.
#440 compares the whole record, sees the personal numbers disagree, and refuses. Candidate 0 is
the truth for seed 7's personal number, and candidate 1 for seed 46's. Neither matches seed 48.
Both candidates of each pair share the same wrong line 1, so no pick between them would have been
a correct read.

## Every refused read also had a wrong line 1, and no check digit could see it

**Derived**, from the table above: in all three reads the filler at line-1 cell 1 is lost. Every
later cell shifts one place left, and the line is re-padded at the end. Cell 1 then holds the
issuer's first letter (`PG`, `PS`, `PF`), the issuer field holds the issuer's last two letters plus
the surname's first (`BRC`, `WES`, `RAA`), and the surname starts one letter late (`ASTELLANO`,
`TRAND`, `DEYEMI`). This is a line-1 prefix error. TD3 line 1 carries no check digit, so every
check state can be `true` on a read whose document code, issuer and surname are all wrong.
`synthpass-bench`'s report-only `line1_flagged` (the `synthpass_core::fusion` line-1 integrity
check) was `true` on all three, and they still counted as hits.

The same shape survives #440 on three seeds that stay hits, **Observed** in the after arm: 18
(`PR` / `USP` / `ETROV`), 26 (`PU` / `KRT` / `KACHENKO`) and 37 (`PG` / `BRN` / `YSTROMLEILANI`).
Before #440 there were eight such hits (7, 12, 18, 20, 26, 37, 46, 48); `line1_flagged` was `true`
on all eight.

**How this relates to open work, without a measured effect.** Two open items reason about a TD3
line-1 prefix through whether the issuer resolves in `countries.rs`:

- Issue [#445](https://github.com/ruledicaprio/SynthPass/issues/445) is about
  `unshift_if_country_resolves`, which re-inserts `<` at cell 1 when cell 1 is a letter and the
  unshifted issuer resolves. The three refused reads have exactly that shape: the as-read issuers
  (`BRC`, `WES`, `RAA`) do not resolve and the unshifted ones (`GBR`, `SWE`, `FRA`) do. The
  accepted reads still came out shifted. Which path produced them without that repair is not
  verified here.
- Draft PR [#447](https://github.com/ruledicaprio/SynthPass/pull/447) refuses or repairs a TD3
  line-1 prefix *insertion* (a 45-cell line) when the as-read issuer does not resolve. These reads
  are the mirror image, a lost cell rather than an inserted one. Whether #447's precondition ever
  holds on them is not verified.

Neither PR was run against this corpus, and nothing here claims either would change a seed.
Seed 37 is a limit on any rule of this kind: `GBR` shifted one cell left reads `BRN`, which
resolves (Brunei Darussalam), so its wrong read passes an issuer-resolution test. The #445
collision class occurs naturally on synthetic data.

## The gate cannot see this

**Derived**, from `crates/synthpass-bench/src/lib.rs` at `69d5dac`: `synthpass-bench` counts a
document as a hit when `find_and_parse` returns a checksum-valid record whose document number
matches truth (`hit: reason.is_none()`). The other fields are compared and reported (per-field CER,
`strict_hits`, `line1_flagged`), but none of them decides `hit`. The generator's truth is exact,
so the harness already holds everything needed to tell a correct read from a checksum-consistent
wrong one. The `m4-hit-rate` gate still counts both the same.

This is the blind spot that issue [#443](https://github.com/ruledicaprio/SynthPass/issues/443)
names on real specimens: the gate trusts checksum consistency, and a checksum-consistent wrong
read passes. The M4 gate is where it is cheapest to see, because truth is exact and free. Before
#440 the gate's 42 held eight hits with a wrong line-1 prefix. #440 refused three of them and
changed the read of two more (seeds 12 and 20), and three remain in the 39. What to do about it is
a separate decision and is not proposed here; issue
[#453](https://github.com/ruledicaprio/SynthPass/issues/453) tracks it.

## The Bosnia parallel

Bosnia 2013's card back on real specimens failed the same way. Its damaged pass produced 40
candidates that agreed on the old six fields and differed only in optional data, and the old
rule returned the first one. The difference is where the wrong characters came from. Bosnia's
printed zone fails its own composite digit, so every checksum-valid reading of it was manufactured.
Here the printed zone is conforming, and the two candidates are two OCR readings of an ambiguous
personal number (`0`/`O`/`Q`, `5`/`S`, `A`/`4`, `I`/`1`, `T`/`1`), both sitting on a shifted
line 1. In both cases the refusal is a correctness gain and the lower count is the honest one.

## Two hits changed their read

**Observed:** outside the three moved seeds, 45 of 47 seeds have byte-identical per-field output in
both arms. Two stayed hits with a different read:

| Seed | Before #440 | After #440 |
| ---: | --- | --- |
| 12 | `PB` / `LRT` / `KACHENKOPETRO` / `""` | `P` / `BLR` / `TKACHENKOPETROC` / `""` |
| 20 | `PB` / `GRP` / `ETROV`, personal number `093479NR4CTAMH` | every scored field exact |

(document type / issuer / surname / given names; truth is `P` / `BLR` / `TKACHENKO` / `PETRO` and
`P` / `BGR` / `PETROV` / `IULIIA`, personal number `93L479NR4CIAMH`.)

**Hypothesized:** #440 refused a damaged-pass read on an early OCR attempt and a later attempt
returned a different checksum-valid read. Both seeds took about twice as long in the after arm
(3.6 s → 7.6 s and 3.7 s → 7.8 s), which fits extra attempts. The timing was not taken on a
checked-clean machine, so it is weak support. A replay of these two seeds would settle it. The
hit bit did not move, so the gate records no change for either seed; the improvement is visible
only in per-field output.

## Why a two-build comparison is enough here

The arms are two builds, `4a027cd` and `69d5dac`, adjacent on `main`, not one build with an
environment toggle. Three things make the delta attributable anyway:

- **Observed:** single-seed re-runs of the before arm (`--count 1 --seed 7`, `46`, `48`)
  reproduced its per-field output for all three seeds exactly.
- **Observed:** 45 of the 47 unmoved seeds have identical per-field output across the two builds,
  so inference noise between builds did not show up anywhere else.
- **Observed:** the replay above explains each of the three moves by the one rule #440 changed.

## What this does not claim

- It does not claim #440 costs no recall. On this corpus it cost none, because all three refused
  reads were wrong. A future corpus could hold a correct read that the damaged pass reaches only
  alongside a disagreeing twin, and #440 would refuse it.
- It does not claim the 39 remaining hits are correct reads. 23 of them are wrong in at least one
  scored field, most often a name (`given_names` 16, `surname` 15).
- It does not measure seeds 50–99 or any format other than TD3. The published synthetic row in
  [`README.md`](README.md#current-headline-numbers) (TD3 78% at 100 seeds, MAIN `9c8f03d`) predates
  #440 and was not re-measured.
- It does not claim #445's fix or #447 would change any seed here. Neither was run.
- It does not attribute the seed 12 and seed 20 read changes to #440 beyond a hypothesis.
- Timing is not a result. The arms ran on a developer machine with no contention check, and the
  unmoved seeds' median `elapsed_ms` ratio (after / before) is 1.21, which is machine state rather
  than the change.

## Invocation

Both arms, local, release, from the worktree `codex-m4-ab-440` checked out at each commit in turn
(`--out` differs per arm):

```
cargo run -p synthpass-bench --release --bin synthpass-bench -- \
  --count 50 --seed 0 --profile clean --min-hit-rate 0.30 --out <report>.json
```

`after.json` is `69d5dac` (report timestamp 2026-09-24 21:52 UTC, exit 0); `before.json` is
`4a027cd` (22:02 UTC, exit 0). The three single-seed files use the same command with
`--count 1 --seed <n>` at `4a027cd`. The reports are local and not committed; their SHA-256
prefixes are `f5df922d…` (before), `44f1d6f3…` (after), `e4d3ec3e…` (seed 7), `74f24847…` (seed 46)
and `288b02f6…` (seed 48). The per-seed comparison is a standard-library Python script over the two
reports' `results[]`. The `single()` replay used a temporary instrumented copy of the pre-#440 crate.
It is not reproduced by any committed tool.

## Rejected on the way

- **Reading the drop as a recall regression.** Rejected: the three moved seeds were field-checked
  against exact truth, and none of the three refused reads was correct.
- **Crediting #440 with the seed 12 and seed 20 improvements as Observed.** Rejected: the arms are
  two builds and no replay covers those seeds, so it stays a hypothesis.
- **Counting `strict_hits` or field-exact hits as the gate's new number.** Rejected: that is a
  proposal to change the gate, which is out of scope for a measurement record.
