# #410's VIZ personal number: no hit count moves on TD1, TD2 or MRV-B, but TD1 churns ten seeds

**Date:** 2026-09-26 · **MAIN:** `f73fb6d` (before) and `101313e` (after: #410's generator commit on `f73fb6d`, since rebased onto `8efda99` as `6079165`; the commits in between change `synthpass-bench`'s reporting and gates (#511, #516) and `mrz`'s unwired `strip` module, but not `synthpass-gen`, the OCR path or the definition of `hit`); `f80877b` for the platform control · **DATA:** none (generated corpus, no `samples-data` input) · **Evidence:** Observed (seven local release `synthpass-bench --profile clean --count 100 --seed 0` runs on one Linux container: TD1, TD2 and MRV-B before and after, one repeat of the after TD1 arm, one TD1 run at `f80877b`) plus Derived (per-seed comparison of the JSON reports) plus Hypothesized (the mechanism of the TD1 churn, marked where used) · **Status:** current

**2026-09-26, one cloud Linux container (4 vCPU, Intel Xeon @ 2.10 GHz), sequential runs, release
builds sharing one target directory.** [#410](https://github.com/ruledicaprio/SynthPass/issues/410)
option 2 keeps the generator's 14-character personal-number draw, so the RNG sequence and every MRZ
line are unchanged. It paints the value the zone carries instead of the whole draw: 11 characters
on TD1, 7 on TD2 and 8 on MRV-B. That changes VIZ pixels and the exported `personal_number` label
on those three formats and nothing else. `synthpass-bench` scores against truth parsed from the MRZ
lines, which the change does not touch, so any movement below is the reader reacting to different
VIZ pixels, not a change in what is scored. TD3 and MRV-A carry the whole draw, cannot change, and
were not run.

## The answer

| Format | Hits before → after | Strict (both names exact) | Wrong accepts | Seeds whose outcome changed |
| :-- | --: | --: | --: | --: |
| TD1 | **21 → 21** | 15 → 12 | 6 → 9 | 20 (5 hits gained, 5 lost) |
| TD2 | **52 → 52** | 39 → 39 | 16 → 16 | 1 (seed 69, a wrong accept stays one) |
| MRV-B | **68 → 68** | 48 → 48 | 32 → 32 | 0 |

- **Observed: no hit count moves on any of the three formats.** MRV-B is identical on every seed,
  on every field's `got` string. TD2 differs on one seed only.
- **Observed: TD1 is not a null result.** Twenty of 100 seeds change outcome. Hits
  gained: seeds 0, 15, 39, 58, 90. Hits lost: seeds 5, 21, 30, 50, 95. Four of the five gained
  hits (0, 15, 58, 90) are wrong accepts, wrong on both names, and four of the five lost hits
  (5, 21, 30, 50) were strict hits. So the net on this machine is zero hits, **−3 strict** and
  **+3 wrong accepts**. Among misses, `no_mrz_found` goes 33 → 30, `checksum_failed` 44 → 46 and
  `document_number_mismatch` 2 → 3.
- **Observed: the TD1 churn is the change, not run-to-run noise.** A second run of the after
  binary on TD1 reproduced every seed's `hit` and every field's `got` string.
- **Observed: TD2 seed 69** was a wrong accept before (given names read `IEVHENII`) and is one
  after (the name separator `<<` is lost, so the surname reads `HRIHORENKOIEVHENIIA` and the given
  names empty). Line 2 reads identically in both arms.
- **Hypothesized: why TD1 alone churns.** TD1's personal-number row (VIZ row 6) is the last VIZ
  row above the MRZ band on the smallest canvas. Shortening its painted text from 14 to 11
  characters changes the ink next to the band, which is enough to move the band detection and
  retry path on a fifth of seeds. The other two formats lose 7 and 6 characters of VIZ ink with
  almost no effect. This was not traced pass by pass.

## The absolute counts here are not the headline's

- **Observed: this container reads far fewer synthetic hits than the published headline**, on
  both arms: TD1 21, TD2 52, MRV-B 68, against the headline's 52, 72 and 87
  ([`README.md`](README.md#current-headline-numbers)).
- **Observed: that gap is the platform, not a regression on `main`.** TD1 at `f80877b`, the
  commit `bench-charts.yml`
  [run 36111444345](https://github.com/ruledicaprio/SynthPass/actions/runs/36111444345)
  measured at 52 / 100 in CI, reads **20 / 100** on this container. No per-document time budget was
  hit: documents took about 1 s each against `SYNTHPASS_OCR_MAX_SECONDS`'s 52 s default. The cause
  (CPU feature set, thread count, float paths in `rten`) is unattributed.
- **Derived: so this note measures a delta, not a level.** Before and after ran on the same machine
  with the same models and the same flags. That holds for the comparison, but the headline row's
  per-format counts cannot be re-read from these runs.

## What this does and does not claim

- It claims that on one machine, #410 moves no hit count on TD1, TD2 or MRV-B, and that on TD1 it
  moves outcomes on 20 seeds with a net of −3 strict and +3 wrong accepts.
- It does **not** claim the headline row is unchanged on the reference platforms (Windows and
  `bench-charts.yml` on Linux CI). TD1's per-seed churn means the reference TD1 count may move in
  either direction, and TD2's single-seed difference could be larger there. The headline row in
  [`README.md`](README.md#current-headline-numbers) is left as it stands. It should be re-measured
  by the next `bench-charts.yml` run after #410 lands. That run can be compared per seed against
  run 36111444345, because no other change between them touches the generator.
- It does not claim that the new labels are better for the reader. The change fixes a label
  defect: before it, VIZ and MRZ disagreed on every TD1, TD2 and MRV-B document. The reader's
  score is a side effect.

## Reproduce

```bash
# at f73fb6d (before) and at #410's generator commit (after), with the two .rten models in the repo root
cargo build --release -p synthpass-bench --bin synthpass-bench
for t in td1 td2 mrvb; do
  target/release/synthpass-bench --document-type $t --profile clean --count 100 --seed 0 \
    --out artifacts/$t.json
done
```

The per-seed comparison keys on `hit`, `wrong_accept`, `wrong_fields`, `names_exact`, `reason` and
each field's `got` string in the JSON reports. The reports themselves were not committed.
