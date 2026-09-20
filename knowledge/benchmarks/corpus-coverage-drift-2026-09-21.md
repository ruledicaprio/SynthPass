# `CORPUS_COVERAGE.md` drift audit: 34 rows contradicted the committed ledger

**Date:** 2026-09-21 · **MAIN:** `2cbe5e4` · **DATA:** `396b22f3b97ee36275a5329667ee37cbd0449b40` · **Evidence:** Derived (script aggregation of `real-specimen-outcomes.jsonl` against `knowledge/CORPUS_COVERAGE.md`) · **Status:** current

## What was compared, against what, and when

[`knowledge/CORPUS_COVERAGE.md`](../CORPUS_COVERAGE.md) as committed at `2cbe5e4` (2026-09-20),
audited against [`real-specimen-outcomes.jsonl`](real-specimen-outcomes.jsonl) at the same commit
(`outcomes_sha256` ties it to `samples_data_sha` `396b22f…`, per
[`real-specimen-mrz-baseline.json`](real-specimen-mrz-baseline.json)). 261 asset rows, one per
document. [`coverage-ledger-rollup-2026-09-20.md`](coverage-ledger-rollup-2026-09-20.md) — the
per-country rollup handed to this audit — was used as a starting point and independently
re-derived from the same ledger with a fresh script (below); the two agree on every count except
the three `misc/` rows the rollup left un-attributed by construction (see "The three `misc/`
rows" below).

**Method.** A Python script (kept in the session scratchpad, not committed) grouped all 261
ledger rows by the same filename-splitting rule the rollup uses (`_Passport` / `_ID` /
`_Driving` / `_Residence`), summed `outcome` per group, then matched each group to a
`CORPUS_COVERAGE.md` row by ISO code (manual map for the handful of name mismatches: `PRK`/`KOR`
split by name, `BIH` merging the two Bosnia spellings, `D`/`DEU` split by the `_D00_` token in
the filename). For every matched row it compared:

- the leading Status word (HIT / MISS / Known MISS / No specimen yet) against whether the group
  had any `hit`;
- every parenthetical count claim (`(x2 specimens)`, `(+ N checksum-failed)`, `(+ N redacted)`,
  `(N of M)`) against the corresponding ledger sub-count;
- rows claiming "No specimen yet" against groups that had *any* ledger row at all.

Every disagreement below was checked by hand against the specific `asset_id`s before being acted
on — the script flags candidates, it does not decide them. Two independent passes were run (one
before, one after applying the fixes) to confirm the fixed file no longer disagrees with itself.

## The three `misc/` rows

Three assets match none of the rollup's filename markers: `misc/Croatia_BorderPass_Specimen_CB_HRV_2025_back_mrz.png`
(`hit`), `misc/Croatia_BorderPass_Specimen_CB_HRV_2025_front_no_mrz.png` (`no_mrz_expected`), and
`misc/Sweden_Visa_Specimen_VC_SWE_2024_mrz.jpg` (`hit`). All three are genuinely Croatian /
Swedish documents (not a naming defect); this audit attributed them by hand to HRV and SWE. The
Sweden row already accounted for the Visa specimen correctly (`HIT (x2, passport + visa) (+ 1
checksum-failed)` matches the ledger exactly — no fix needed). The Croatia row did not; see the
HRV entry below.

## Every disagreement found, classified

All 34 are **document-wrong** (CORPUS_COVERAGE asserted something the ledger, at this commit,
directly contradicts, with nothing in the document — no dated caveat, no pinned-evidence
citation — explaining the gap). None of the 2026-09-08 checksum-relabelling caveat's protected
rows are touched; that caveat covers *why* a `checksum_failed` document is a miss, not *whether*
a row's stated miss-type or hit-count matches the ledger. Sixteen of the eighteen redaction-group
rows share one root cause, described once below the table rather than eighteen times.

| Code | Country | Old status | New status | Asset(s) the fix rests on |
| :-- | :-- | :-- | :-- | :-- |
| AUT | Austria | `HIT (passport) + ID card front not wired (MRZ on card back)` | `HIT (passport) + negative control (ID card front)` | `id_cards/Austria_ID_Specimen_2021_front_no_mrz.png` = `no_mrz_expected` (the wired negative-control outcome, not absence from the corpus) |
| IRL | Ireland | `HIT (passport) + ID card front not wired` | `HIT (passport) + negative control (ID card front)` | `id_cards/Ireland_ID_Specimen_2015_front_no_mrz.jpg` = `no_mrz_expected` |
| NLD | Netherlands | `HIT (passport) + driving license not wired` | `HIT (passport) + negative control (driving license)` | `driving_licenses/Netherlands_Driving_License_Specimen_no_mrz.jpg` = `no_mrz_expected` |
| HKG | Hong Kong | `No specimen yet` | `MISS (checksum failed)` | `passports/Hong_Kong_Passport_Specimen_P0_HKG_2007_mrz.png`, `..._2019_mrz.png` = `checksum_failed` x2. The row's own Notes column already said "Both checksum-failed... not yet a HIT"; only the Status cell contradicted it |
| BIH | Bosnia and Herzegovina | `HIT (x2 specimens) (+ 1 checksum-failed)` | `HIT (x2 specimens) (+ 1 redacted)` | `passports/Bosnia_and_Herzegovina_Passport_Specimen_P0_BIH_2017_redacted_mrz.jpg` = `redacted_mrz`, not `checksum_failed` |
| ISR | Israel | `Known MISS (documented) + HIT (+ 1 checksum-failed)`, note claimed one specimen "kept local-only, not committed" | `HIT (+ 2 redacted)` | `P0_ISR_2003_redacted_mrz.jpg` and `P0_ISR_2013_redacted_mrz.jpg` are both `redacted_mrz` and both have real `asset_id`s in the committed ledger -- neither is "local-only"; `P0_ISR_2011_mrz.jpg` is the one `hit` |
| KOR | Korea (Republic of) | `HIT (+ 2 checksum-failed)` | `MISS (checksum failed)` | Both specimens (`PM_KOR_2020`, `PM_KOR_2022`) are `checksum_failed_specimen` -- zero `hit` in the ledger |
| UKR | Ukraine | `MISS (no MRZ found)` | `HIT (+ 1 redacted)` | `P0_UKR_2015_mrz.webp` = `hit`; `P0_UKR_2012_redacted_mrz.jpg` = `redacted_mrz`; zero `no_mrz_found` |
| USA | United States | `HIT (x4 specimens)` | `HIT (x3 specimens) (+ 2 redacted)` | ledger `hit`=3 (ID-card back, `2020_mrz`, `2020_mrz_highlight`), `redacted_mrz`=2 (`2005`, `XXXX`) -- the claimed count was wrong and the two redacted specimens were unmentioned |
| MYS | Malaysia | `HIT (x2 specimens)` | `HIT (+ 1 redacted)` | `P0_MYS_2017_mrz.jpg` = `hit`; `P0_MYS_2019_redacted_mrz_blur.png` = `redacted_mrz` (an open question per `denominator-correction-2026-09-09.md` -- not resolved here) |
| RUS | Russian Federation | `HIT (x3 specimens) (+ 1 checksum-failed, 2 redacted)`, doctype "Passport" only | `HIT (x4 specimens) (+ 1 checksum-failed, 2 redacted)`, doctype "Passport, ID card" | ledger `hit`=4 -- the three named passports plus `id_cards/Russian_Federation_ID_Specimen_2013_back_mrz.jpeg`, entirely unmentioned |
| ESP | Spain | `HIT (x2)` | `HIT (x4)` | all four Spain assets are `hit` |
| HRV | Croatia | `HIT (x2 specimens) (+ 1 checksum-failed)`, doctype "Passport, ID card" | `HIT (x3 specimens) (+ 1 checksum-failed)`, doctype adds "BorderPass" | ledger `hit`=3 once the `misc/Croatia_BorderPass...back_mrz.png` hit is attributed (see above) |
| DZA | Algeria | `MISS (checksum failed)` | `No specimen yet` | `passports/Algeria_Passport_Specimen_XX_XXX_XXXX_no_mrz.jpg` = `no_mrz_expected`; `checksum-failed-real-specimens-2026-09-08.md` independently documents this file's "MRZ" as OCR misreading VIZ boilerplate, not a zone |
| CHN | China | `HIT (x2 specimens) (+ 2 checksum-failed)` | `HIT (x3 specimens) (+ 2 redacted)` | ledger `hit`=3, `redacted_mrz`=2 (`2014`, `2024`), zero `checksum_failed` |
| JPN | Japan | `HIT (+ 2 checksum-failed)` | `HIT (+ 2 redacted)` | `P0_JPN_2009` and `P0_JPN_2021` are both `redacted_mrz` |
| KAZ | Kazakhstan | `HIT (+ 2 checksum-failed)` | `HIT (+ 1 redacted)` | one `hit`, one `redacted_mrz` (`P0_KAZ_2009`) -- count and type both wrong |
| TUR | Türkiye | `HIT (x5 specimens) (+ 5 checksum-failed)` | `HIT (x5 specimens) (+ 4 checksum-failed)` | ledger `checksum_failed_specimen`=4, not 5 (hit count itself was already right) |
| VNM | Viet Nam | `HIT (+ 1 no-MRZ specimen)`, note "the 2022 specimen carries no MRZ" | `HIT (+ 1 redacted)` | `P0_VNM_2022_redacted_mrz_blur.jpg` = `redacted_mrz` -- the file is redacted, not MRZ-less |
| FRA | France | `HIT (+ 2 checksum-failed)` | `HIT (+ 1 no-MRZ found, 1 redacted)` | zero `checksum_failed`; `id_cards/France_ID_Specimen_2020_back_mrz.png` = `no_mrz_found` (a real, still-open detection miss per `denominator-rebless-2026-09-19.md`), `passports/France_Passport_Specimen_P0_FRA_XXXX_redacted_mrz.jpg` = `redacted_mrz` |
| DEU | Germany | `HIT (+ 1 checksum-failed)` | `HIT` | zero `checksum_failed`/`checksum_failed_specimen` under the `DEU` (non-`D00`) code; the claim had no ledger basis at all |
| ITA | Italy | `HIT (+ 1 checksum-failed)` | `HIT (+ 1 no-MRZ found)` | `id_cards/Italy_ID_Specimen_2022_back_mrz.jpg` = `no_mrz_found` (real, still-open detection miss), zero `checksum_failed` |
| CHE | Switzerland | `HIT (+ 2 checksum-failed)` | `HIT (+ 1 checksum-failed)` | ledger `checksum_failed_specimen`=1 (`2023` ID-card back), not 2 |
| AUS | Australia | `HIT (+ 1 checksum-failed)` | `HIT (+ 1 redacted)` | `P0_AUS_2015_redacted_mrz.jpg` = `redacted_mrz`, zero `checksum_failed` |
| MEX | Mexico | `MISS (checksum failed)` | `Known MISS (documented)` | `P0_MEX_2016_redacted_mrz.jpg` = `redacted_mrz`, zero `checksum_failed` |
| URY | Uruguay | `MISS (checksum failed)` | `Known MISS (documented)` | `PP_URY_2025_redacted_mrz.jpeg` = `redacted_mrz` |
| IRN | Iran | `MISS (checksum failed)` | `Known MISS (documented)` | `P0_IRN_2017_redacted_mrz.jpg` = `redacted_mrz` |
| IRQ | Iraq | `MISS (checksum failed)` | `Known MISS (documented)` | `P0_IRQ_2009_redacted_mrz.jpg` = `redacted_mrz` |
| MMR | Myanmar | `MISS (no MRZ found)` | `Known MISS (documented)` | `PV_MMR_2014_redacted_mrz.jpg` = `redacted_mrz`, zero `no_mrz_found` |
| ALB | Albania | `MISS (checksum failed)` | `Known MISS (documented)` | `P0_ALB_2009_redacted_mrz.jpg` = `redacted_mrz` |
| BLR | Belarus | `MISS (checksum failed) (+ 1 no-MRZ)` | `Known MISS (documented)` | both `P0_BLR_2006` and `P0_BLR_2021` are `redacted_mrz`; zero `checksum_failed`, zero `no_mrz_found` |
| LUX | Luxembourg | `MISS (no MRZ found)` | `Known MISS (documented) + negative control (ID card front)` | `id_cards/Luxembourg_ID_Specimen_back_redacted_mrz.jpg` = `redacted_mrz`; `..._front_no_mrz.jpg` = `no_mrz_expected` |
| PER | Peru | `No specimen yet` (doctype/note both `--`) | `Known MISS (documented)` | `P0_PER_XXXX_redacted_mrz.jpg` = `redacted_mrz` -- a real specimen the row didn't mention at all |
| ARM | Armenia | `No specimen yet` (doctype/note both `--`) | `Known MISS (documented)` | `XX_XXX_XXXX_redacted_mrz.png` = `redacted_mrz` -- same gap |

## Why sixteen of these are one root cause, not sixteen separate bugs

`MEX`, `URY`, `IRN`, `IRQ`, `MMR`, `ALB`, `BLR`, `LUX`, `PER`, `ARM`, `CHN`, `JPN`, `KAZ`, `AUS`,
`VNM`, `USA` all trace to the same mechanism, documented in
[`denominator-correction-2026-09-09.md`](denominator-correction-2026-09-09.md): before
2026-09-09, the redaction check ran *after* the `mrz_found` gate, so a redacted specimen was
classified as `no_mrz_found` or (if the blackout bar OCR'd into noise that shaped like a zone)
`checksum_failed`, depending on OCR luck. The 2026-09-09 fix reordered the gates so a redacted
specimen is now classified `redacted_mrz` regardless of how its bar OCRs. `CORPUS_COVERAGE.md`'s
per-country notes for these rows were written from the **2026-08-17 real-OCR scan**, i.e. under
the *old* gate order, and were never revisited after the 2026-09-09 fix landed. This is distinct
from the 2026-09-08 checksum-relabelling caveat the document already carries (that one is about
*why* a `checksum_failed` document fails, not about a document being *reclassified out of*
`checksum_failed` into a different miss-type entirely) -- so it is not protected by that caveat,
and nothing else in the document flags the gap. `ISR`, `BIH`, `KOR`, `UKR`, `DZA` are separate
mechanisms (miscounts, a stale "kept local-only" claim, and a non-conforming-by-design pair with
zero real hits), addressed individually above.

## What changed in `CORPUS_COVERAGE.md`, and why

- **34 row edits** (table above) -- each replaces a status/count/label the ledger directly
  contradicts. No doctype, note, or status was changed beyond what the specific contradiction
  required; existing prose (dates, provenance, ADR references) was preserved verbatim wherever it
  did not conflict with the ledger.
- **Summary table** (top of the file): recomputed by re-parsing all 238 rows' Status column after
  the 34 edits and independently re-tallying the seven buckets with a second script pass. Every
  edit's bucket movement is accounted for:

  | Bucket | Before | After | Rows moved |
  | :-- | --: | --: | --: |
  | HIT | 75 | 76 | +ISR, +UKR, -KOR |
  | MISS | 15 | 7 | +HKG, -KOR, -UKR, -DZA, -MEX, -URY, -IRN, -IRQ, -MMR, -ALB, -BLR, -LUX |
  | Known MISS (documented) | 1 | 10 | -ISR, +MEX, +URY, +IRN, +IRQ, +MMR, +ALB, +BLR, +LUX, +PER, +ARM |
  | Candidate rejected | 1 | 1 | unchanged |
  | No legal source found | 0 | 0 | unchanged |
  | Not applicable | 1 | 1 | unchanged |
  | No specimen yet | 145 | 143 | -HKG (moves into MISS), +DZA, -PER, -ARM |
  | **Total** | **238** | **238** | closes |

  The recomputed 76/7/10/1/0/1/143 was verified by an independent script pass over the edited
  file, not just arithmetic on the diff -- see "Gates" below.
- **No narrative/dated bullets were touched.** The 2026-08-17, 2026-09-08, 2026-09-10,
  2026-09-12, and 2026-09-14 dated paragraphs under "## Summary" are pinned evidence of what was
  true on those dates and are untouched; only the row table and the (undated, living) Summary
  counts were corrected.

## Classified but not changed

- **DMA (Dominica) -- data-odd, not fixed.** The ledger's `id_cards/Dominica_ID_Specimen_2024_front_no_mrz.png`
  (`no_mrz_expected`) is attributed to Dominica by the filename-splitting rule this audit (and
  the rollup) uses, but [`cover-like-detector-2026-09-16.md`](cover-like-detector-2026-09-16.md)'s
  own visual review of the same file describes its content as a *Dominican Republic* cedula
  (`SANTO DOMINGO, R.D.`) -- a different country. Given that, CORPUS_COVERAGE's current "No
  specimen yet" for DMA is arguably still correct (no *genuine* Dominica specimen exists); editing
  it to claim Dominica coverage from a misattributed file would introduce a new error, not fix
  one. Left unchanged. **Recommendation (corpus/manifest, not acted on here):** the underlying
  file is misnamed -- `Dominica_ID_Specimen_2024_front_no_mrz.png` should be investigated and
  renamed/relocated to Dominican Republic if the visual read is correct.
- **BGD, LIE, ARE, GHA, MRT -- no fix, editorial terseness only.** Each has a real ledger asset
  (a negative control, an extra redacted specimen, or for ARE an entire untracked ID-card
  document type) that its row's plain `HIT`/`MISS` status doesn't itemize. None of these rows
  makes a *specific* count or type claim that the ledger contradicts -- they are simply less
  detailed than peer rows -- so this audit did not treat them as document-wrong per the "only fix
  a contradicted claim" line this report draws (see Unresolved). Reported, not edited.
- **SDN (Sudan), "Candidate rejected"** -- correctly has no ledger entry; rejected candidates are
  never ingested. No drift.
- **`Cetis Sample` and `Somaliland`** ledger entries (`P0_TRC_2022`, `P0_RSL_2023_mrz_non_ISO`)
  use non-ISO document codes not present in `crates/mrz/src/countries.rs`'s 238-code list, so they
  correctly have no `CORPUS_COVERAGE.md` row. Not a defect -- noted for transparency since they are
  part of the 261-asset ledger total.
- **Sweden (SWE) -- checked, already correct.** The row already accounted for the `misc/` Visa
  specimen (`HIT (x2, passport + visa) (+ 1 checksum-failed)` matches the ledger exactly). No fix
  needed; recorded here so the "handle the three `misc/` rows" instruction shows both outcomes
  (Croatia needed a fix, Sweden did not).

## Recommendations for code, corpus, or an ADR (not acted on here)

1. **Corpus/manifest:** `Dominica_ID_Specimen_2024_front_no_mrz.png` appears to depict a
   Dominican Republic document per `cover-like-detector-2026-09-16.md`'s own visual review, not a
   Dominica one. This is exactly the kind of error a filename-splitting country-aggregation rule
   (used by this audit, the rollup, and implicitly by any reader of `real-specimen-outcomes.jsonl`
   grouping by name) cannot catch on its own -- it silently attributes the asset to the wrong
   country. Worth a corpus-hygiene pass to confirm and relocate/rename.
2. **Corpus naming consistency:** `driving_licenses/Bosnia_Herzegovina_Driving_License_Specimen_{face,front}.gif`
   use the country name without "and", while every other Bosnia and Herzegovina asset uses
   `Bosnia_and_Herzegovina_*`. This audit's country-matching had to special-case both spellings by
   hand; a real reader (or `corpus_manifest.rs`'s own aggregation) would silently split one
   country into two. Both driving-license assets are `no_mrz_expected` negative controls and are
   not mentioned in BIH's `CORPUS_COVERAGE.md` row (doctype currently reads "ID card, Passport"
   only) -- worth a naming fix plus a doctype update once the manifest is consistent.
3. **ADR/manifest:** the D-vs-DEU split leaves `id_cards/Germany_ID_Specimen_2021_front_no_mrz.jpg`
   unmentioned by either the `D` or `DEU` row (only the 2024 ID-card front is discussed, under
   `D`), and that discussion is an editorial choice this audit did not have grounds to overturn
   (German ID cards do print the legacy code `D`, but this specific file carries no MRZ at all, so
   no code is actually printed on it). Whoever owns the `D`/`DEU` split should decide which row
   the 2021 card belongs under and mention it.
4. **CORPUS_COVERAGE completeness (optional, non-urgent):** BGD, LIE, ARE, GHA, and MRT each have
   at least one real ledger asset (a negative control or an extra redacted/non-conforming
   specimen) not itemized in their row. None of these are false claims -- see "Classified but not
   changed" above -- but a future documentation pass could bring them up to the same level of
   detail as peer rows (e.g. RUS, EGY) for consistency.

## Gates

- **`bash scripts/check-doc-links.sh`** -- ran once, in the background, against the repository
  state *after* all 34 CORPUS_COVERAGE.md edits and the Summary-table update, but *before* this
  report file existed. **Exit 0.** Output: "all documentation links resolve," covering relative
  Markdown links, `knowledge/` paths cited in prose, stale `docs/` references, Markdown anchor
  fragments, and `crates/`/`scripts/`/`tools/`/`.github/` paths cited anywhere. This confirms the
  three new benchmark links this audit added to CORPUS_COVERAGE.md
  (`checksum-failed-real-specimens-2026-09-08.md`, `denominator-correction-2026-09-09.md`,
  `denominator-rebless-2026-09-19.md`) resolve -- all three are already tracked, so the script's
  existence check sees them. **This run did not, and could not, examine this report file itself**
  -- it is untracked and did not exist yet when the script ran. It has not been re-run since; the
  calling session should re-run it after staging this file if its own links need checking (they
  are two ordinary bare-word filename mentions plus the standard citation style, no unusual
  paths).
- **`python tools/index_findings.py --write`** -- **not run.** This report matches the
  `**Date:** ... **MAIN:** ... **DATA:** ... **Evidence:** ... **Status:** ...` header standard
  `index_findings.py` requires of every `knowledge/benchmarks/*-YYYY-MM-DD.md` file, and by that
  standard it does belong in the findings index once committed. It is deliberately left unindexed
  here because the file is still untracked (`--write` reads it directly off disk by glob, so it
  would already pick this file up if run, but the calling session should decide when to run
  `--write` as part of committing this file, not this audit).
- **`bash scripts/check-headline-numbers.sh`** -- not run. It validates README/ROADMAP/benchmarks
  README against the baseline JSON; nothing in this audit touched any of those three files or the
  baseline, so it was out of scope for this pass.

## Unresolved

One methodological line, stated so it can be checked rather than assumed: this audit fixed a row
only when CORPUS_COVERAGE.md made a **specific, contradicted claim** (a status word, a count, or
a named miss-type) -- not when a row was merely less detailed than a ledger-perfect account would
be. Four rows (BGD, LIE, ARE, MRT/GHA) sit right on that line; they are listed under "Classified
but not changed" with the reasoning, but a reader who draws the line differently could reasonably
call them document-wrong too. Nothing else is unresolved: every disagreement this audit's method
surfaced was run down to a specific `asset_id` and either fixed (34) or classified with a
citation (DMA, SWE, SDN, Cetis Sample, Somaliland). The method independently found both of the
task's required known defects (AUT, HKG) without being told where they were, and along the way
found the same "not wired" mislabelling on two more rows (IRL, NLD) and the same
redaction-reclassification staleness on fourteen more rows beyond MEX -- which is why this report
is confident the sweep was not a spot-check.
