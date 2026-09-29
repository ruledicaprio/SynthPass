# The line-1 selector over three replayed arms: the veto passes, two real names go wrong → exact, and no outcome or correct field moves

**Date:** 2026-09-29 · **MAIN:** `cb334d0` (#609's selector, not merged; the same tree as #609's `f863407`) replaying a capture made at `5001160` (#608's branch head, the same tree as `main`'s `53c283f`) · **DATA:** the local `samples-data` checkout; all 261 source hashes match `samples/corpus.jsonl` (sha256 `37274376fff2`), and the capture's outcomes equal the committed ledger's (CI, DATA `396b22f`) on all 261 documents; none for the synthetic runs (generated corpus) · **Evidence:** Observed (one live local release `provider-bench --real-specimens --mrz-only` capture, three replays of it with one binary, fifteen `synthpass-bench --profile clean --count 100 --seed 0` runs; compared with #609's `tools/bench_ab_diff.py`) plus Observed by eye (the four changed zones against their specimen images) plus Derived (per-document comparison of the three reports) · **Status:** current

**2026-09-29.** This is the A/B note that
[#574](https://github.com/ruledicaprio/SynthPass/issues/574) PR 4c asks for.
[#609](https://github.com/ruledicaprio/SynthPass/pull/609) adds a shadow line-1 selector, off by
default, under `SYNTHPASS_MRZ_LINE1_SELECT` = `off` / `control` / `on`. It runs
only after Tier 1 has accepted a valid TD3, TD2, MRV-A or MRV-B zone whose line-1 name field breaks
the name grammar. It then looks in the same OCR text for other lines that have exactly the format's
width, only MRZ characters, the same document code and issuer byte for byte, and a grammatical name
field, and that parse with the accepted line 2 to the same data in every non-name field. Exactly one
distinct name field is a proposal: `control` records it and `on` applies it. More than one is
`ambiguous`, and nothing changes. A zone that the existing wrong-line checks flag stays
`unresolved`. There is no ranking and no constant to tune. The three arms replay one captured run
([ADR-0024, amendment 3](../decisions/ADR-0024-per-document-benchmark-archive.md);
[README, "Replaying a captured run"](README.md#replaying-a-captured-run)), so every arm reads
byte-identical OCR text. No code, ADR, baseline or headline changes with this note.

## The answer

- **The veto passes on every item** (Observed). Under `on`, real Tier-1 hits hold at **139 / 151
  = 92.1%** of scored documents and **139 / 261 = 53.3%** of the corpus. 0 documents change
  outcome, and every outcome bucket is unchanged. 0 fields go exact → wrong or exact →
  unread. None of the 13 exact-name documents changes. All five synthetic formats are identical
  to `off`, seed for seed.
- **Benefit: two names go wrong → exact among the documents with name truth** (Observed; both
  checked by eye against the image). They are Djibouti 2019 and Somalia 2023. Strict names go
  **13 / 45 = 28.9% → 15 / 45 = 33.3%**, and names exact among hits go **13 / 33 = 39.4% → 15 /
  33 = 45.5%**.
- **Two more documents without name truth change.** Russian Federation `PD` 2004 goes wrong →
  exact by eye. Bosnia and Herzegovina 2014 goes wrong → less wrong: the surname becomes right,
  but the given names stay wrong (`AZRACMARINA`). The gate cannot see either, so both are
  inspected by eye.
- **`control` proposes exactly the four documents that `on` applies**, and the placebo arm changes
  nothing but its own recorded verdicts.
- **The default stays `off`.** Promotion is a separate decision for the maintainer. Per the
  maintainer's decision on #609, it must include routing the v1 record's parse.

## The veto, item by item

Checks as the #574 PR 4 plan sets them, `off` → `on`, over the same replayed capture (Observed):

| Veto item | Passes when | Measured | Verdict |
| --- | --- | --- | --- |
| No lost hit | No outcome changes from `hit`, and the Tier-1 hit count is equal | 139 → 139; 0 outcome changes | pass |
| No correct → wrong in any field | `field_correctness` regressions empty, over the 65 labelled documents | 0 exact → wrong, 0 exact → unread; the only transitions are surname wrong → exact 1 and given names wrong → exact 2 | pass |
| No change to the exact-name documents | Each is absent from the outcome, names and zone change lists, and its verdict is not `applied` | 13 on this capture: 12 `kept`, and Serbia ID 2008 (TD1) `out_of_scope`; none in any change list | pass |
| No new document-number mismatch | No asset newly `document_number_mismatch`; `document_number` exact → wrong = 0 | 0 outcome changes; 0 `document_number` transitions | pass |
| No synthetic per-format regression | Per format, hits and correct fields under `on` ≥ `off` | Every `results[]` row identical apart from `elapsed_ms`, in all five formats | pass |

The histogram, identical in both arms: `hit` 139, `checksum_failed` 10, `no_mrz_found` 2 (the
scored 151), and `checksum_failed_specimen` 20, `redacted_mrz` 39, `no_mrz_expected` 51 (the 110
scored out). `false_positive_mrz` is 0 and `checksum_valid_on_failed_specimen` is 0 in both.
Hits by format: TD3 123, TD1 13, MRV-B 2, TD2 1.

## What moved: the four applied zones

Only the line-1 name field changes. Line 2, the document code and the issuer are the same in both
arms by construction, and the dump confirms it: 4 zone changes out of 171 dumped zones.

| Document | Name truth | Line 1 name field, `off` | Line 1 name field, `on` | Names | By eye |
| --- | --- | --- | --- | --- | --- |
| Somalia 2023 (`P0_SOM_2023`) | yes | `IBRAHIM<<ABDIAZIZ<MOHAMUD<<<<<<Z<…` | `IBRAHIM<<ABDIAZIZ<MOHAMUD<<<…` | given names wrong → exact | equals the printed line 1 |
| Djibouti 2019 (`P0_DJI_2019`) | yes | `HASSANSFARID<RAGUEK<C<<<<<KK<…` | `HASSAN<FARID<RAGUE<<<…` | surname and given names wrong → exact | equals the printed line 1 |
| Russian Federation 2004 (`PD_RUS_2004`) | no | `SMIRNOVA<<<…<VALENTINAC<…` | `SMIRNOVA<<VALENTINA<<<…` | given names wrong → exact | equals the printed line 1 |
| Bosnia and Herzegovina 2014 (`P0_BIH_2014_mrz_highlight`) | no | `KOVACEVIC<AZRA<MARINA<<<…<K` | `KOVACEVIC<<AZRACMARINA<<<…` | surname right; given names still wrong | printed `KOVACEVIC<<AZRA<MARINA<<<…` |

- **Observed:** the fixtures agree on Somalia 2023 (IBRAHIM / ABDIAZIZ MOHAMUD) and Djibouti 2019
  (HASSAN / FARID RAGUE). The dump marks both zones' fixture match as exact under `on`.
- **Observed, by eye:** Djibouti's printed line 1 has no `<<`. The applied line keeps its single
  `<` separators, as printed.
- **Bosnia and Herzegovina 2014:** the incumbent parses as surname "KOVACEVIC AZRA MARINA" and
  given name "K". The applied line gets the surname right and reads the `<` between the given
  names as `C`. The name grammar cannot see a filler read as a letter, which is the risk the plan
  names.
- **Where the proposal came from** (Observed, the report's `source_passes`): Somalia had 3 eligible
  readings with 1 distinct name field, from the general pass, `pass-02` (`mrz_variants:2`) and
  `pass-07` (`geometry_band:0`). The other three had 1 eligible reading each, from the general
  pass.

**Both denominators** (Observed; strict names per ADR-0013 are report-only):

| Metric | `off` | `on` |
| --- | --- | --- |
| Tier-1 hits, scored | 139 / 151 = 92.1% | 139 / 151 = 92.1% |
| Tier-1 hits, corpus | 139 / 261 = 53.3% | 139 / 261 = 53.3% |
| Strict names (hit and both names exact), of name-scorable scored documents | 13 / 45 = 28.9% | **15 / 45 = 33.3%** |
| Names exact among name-scorable hits | 13 / 33 = 39.4% | **15 / 33 = 45.5%** |
| Accepted reads, field match (33 documents) | 276 / 319 (Derived from 86.52%) | 279 / 319 (Derived from 87.46%) |

## The verdicts, per document

The report records a verdict on the 182 documents with an MRZ anchor. The other 79 have none, and
the report omits the key there. Under `control` and `on` (Observed):

| Verdict | Reason | Count | Documents |
| --- | --- | ---: | --- |
| `proposed` (`control`) / `applied` (`on`) | — | 4 | the four above |
| `unresolved` | `repeated_line` | 1 | Croatia ID 2002 back (TD2 hit, no name truth) |
| `unresolved` | `issuer_unresolved` | 2 | Switzerland ID 2003 back (MRV-B hit, no name truth); Dominican Republic 2020 (TD3 hit, issuer read `DOR`, names wrong) |
| `ambiguous` | — | 0 | — |
| `none` | `kept` | 108 | the incumbent name field is grammatical |
| `none` | `out_of_scope` | 55 | 13 TD1 hits, and 42 reads that are not a valid zone |
| `none` | `no_candidate` | 12 | ungrammatical, but no other line is eligible |

**The 18 name-scorable hits still wrong under `on`** (Observed), by why the selector leaves them:

- **14 `kept`:** the wrong name field is grammatical, so the rule does not run. Angola 2026,
  Bangladesh 2023, Canada 2013, Canada `PP` 2023 highlight, the CETIS sample 2022, Cyprus 2010,
  Djibouti 2017, Estonia 2020, India 2022, Indonesia 2024, Nigeria 2022, Slovakia `PS` 2014, Spain
  2013 and Spain 2015 highlight.
- **2 `no_candidate`:** Canada `PP` 2023 wide and United Arab Emirates 2011.
- **1 `unresolved`:** Dominican Republic 2020.
- **1 `out_of_scope`:** Slovenia ID 2022 (TD1).

## Against what the plan modelled

The plan modelled the rule in Python over #595's capture at `76f914e`, before #602's whitespace
rejoin. Its figures were Derived; the ones below are measured on a newer capture.

| Document or figure | Plan (Derived, `76f914e` capture) | Measured (this capture) |
| --- | --- | --- |
| Somalia 2023 | applied, exact | applied, exact |
| Djibouti 2019 | applied, exact | applied, exact |
| Dominican Republic 2020 | `unresolved`, issuer `DOR` | `unresolved/issuer_unresolved` |
| Slovakia `P0` 2005 | `unresolved`; out of reach | **`kept`: its names are already exact on this capture** |
| Bosnia and Herzegovina 2014 | wrong → wrong (`<` read as `C`) | the same |
| Russian Federation 2004 | a change, a fix by eye | applied; exact by eye |
| DPRK `PO` 2005 | a change, a fix by eye | `kept`: its line 1 is already grammatical on this capture |
| Exact-name documents untouched | 12 | 13 |
| Strict names | 12 / 45 → 14 / 45 | 13 / 45 → 15 / 45 |

**Slovakia 2005 is capture-dependent.** The committed ledger (CI, 2026-09-24) records its names
as wrong. On this capture, at `main`'s tree, they are exact without the selector, which is why
`off` reads 13 / 45 and not the baseline's 12 / 45. The capture's `--assert-baseline` run printed
it as `strict_hits 12 -> 13`, report-only. The selector adds the same +2 that the plan modelled,
on a different starting point.

## Fidelity and placebo

- **Fidelity, capture ~ `off`** (Observed; `--expect-identical --check-report`): **NEUTRAL**. That
  is 261 / 261 documents, 0 ledger rows differ, 171 / 171 dump rows with 0 differing (raw OCR
  text compared by sha256), and 0 of 261 report rows differ. In default mode it shows 0 outcome
  changes and no field-correctness transitions. The runs differ only in flags, `replay_of`,
  `mrz_line1_select_arm` and the model paths, which a replay does not load. Two documents stopped
  on the retry budget in the capture, France ID 2020 back and front. A replay inherits that, so
  both arms have them.
- **Placebo, `off` ~ `control`** (Observed):
  - **Synthetic:** all five formats are identical: `results[]` 0 differ and dump blocks 0 differ.
  - **Real:** 0 ledger rows and 0 dump rows differ. 182 report rows differ, and only in
    `control`'s recorded `line1_selection`, which is the design. With `--ignore
    line1_selection` the pair is NEUTRAL.
  - **Assets proposed in one arm and neither applied nor proposed in the other:** 0 (placebo)
    and 0 (veto).

## Synthetic

Live `synthpass-bench --profile clean --count 100 --seed 0 --dump-ocr`, five formats × three arms,
one binary (Observed). Every count is identical in all three arms. `bench_ab_diff` reports 0 reads
changed and 0 flag-only changes per format, `off` → `on`:

| Format | Hits | Strict | Correct | Wrong accepts | Accepted reads | Accepted with a wrong code or issuer |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| TD1 | 59 | 32 | 31 | 28 | 66 | seed 0 |
| TD2 | 81 | 48 | 48 | 33 | 84 | none |
| TD3 | 74 | 48 | 44 | 30 | 78 | none |
| MRV-A | 87 | 49 | 27 | 60 | 90 | none |
| MRV-B | 90 | 61 | 48 | 42 | 93 | none |

- **No synthetic benefit and no synthetic regression.**
- **The synthetic report records no per-seed verdict** (#609 routes the read through
  `read_tier1` but keeps only the parsed zone).
- **Derived:** no synthetic zone was swapped. A swap replaces an ungrammatical line 1, which
  therefore differs from the generator's truth. Each such row records the zone it read (`got`),
  and no row differs between `off` and `on`.
- **Not measured:** whether `control` proposed anything on synthetic that `on` would then apply
  identically. It cannot happen, by the same argument.

## How this was measured

- **One slot, one machine, 2026-09-29, in order** (UTC, from the run log):
  - **Capture** at `5001160`, 17:41–18:16, 34.4 min of OCR, exit 0. 0 documents changed outcome
    against the committed ledger. Its run manifest is
    `provider-bench-ocr-run-6d1cb2c7c24ab77664c8b5ba71fe8a3b7e828f189528d8f2ec67360a96a28455.json`.
    Its same-binary replay was NEUTRAL: #608's acceptance.
  - **Arms:** a release build of `cb334d0`, with `provider-bench` sha256 prefix
    `d621bf08c1ec7b50` and `synthpass-bench` `b76e3f126a929245`. Three replays took about 11 s
    each, 18:19. The synthetic runs took 18:19–19:25.
- **Why the capture serves `cb334d0`** (Observed): `git diff 5001160 cb334d0 --
  crates/synthpass-ocr crates/synthpass-imageprep` is empty. The `mrz` diff is the selector, its
  `rank` helper and the `candidate_lines` extract. `cb334d0` and `f863407` have the same tree.
  #609's later `8ec70da` adds one test file only.
- **Environment:** `SYNTHPASS_OCR_MODEL_DIR=/opt/synthpass/models`, and every other
  `SYNTHPASS_OCR_*` variable and `SYNTHPASS_MRZ_CLASS_SWEEP` unset. OCR arms: chargrid `off`,
  texture `on`, order, rotate and skew `default`. Each report records its own arm as
  `mrz_line1_select_arm`, and each replay manifest records it under `mrz_arms`: `off`, `control`
  and `on` as intended.

  ```
  # capture (5001160)
  provider-bench --real-specimens --mrz-only --progress --dump-ocr --dump-ocr-hits \
    --dump-ocr-passes --assert-baseline knowledge/benchmarks/real-specimen-mrz-baseline.json \
    --out capture/real/report.json
  # per arm (cb334d0): <v> = off | control | on
  SYNTHPASS_MRZ_LINE1_SELECT=<v> provider-bench --real-specimens --mrz-only --progress \
    --replay-ocr-passes capture/real --dump-ocr --dump-ocr-hits --out <arm>/real/report.json
  SYNTHPASS_MRZ_LINE1_SELECT=<v> synthpass-bench --document-type <td1|td2|td3|mrva|mrvb> \
    --profile clean --count 100 --seed 0 --dump-ocr --out <arm>/<fmt>.json
  # comparisons (#609's tools/bench_ab_diff.py)
  bench_ab_diff.py capture off --expect-identical --check-report   # fidelity (real only)
  bench_ab_diff.py off ctl --expect-identical --check-report       # placebo
  bench_ab_diff.py off ctl --expect-identical --check-report --ignore line1_selection  # real
  bench_ab_diff.py off on [--json]                                 # veto
  ```

- **Not committed:** the reports, ledgers and dumps. They hold OCR text. The run used no
  `--include-private`.

## What this does not claim

- **Not a CI measurement, and nothing to re-bless.** It is one local capture. A replay is refused
  `--write-baseline` and `--assert-baseline`, and the arms are not defaults. No headline changes.
- **Nothing upstream of the OCR text.** A replay measures only code downstream of `OcrPage::text`.
  The passes, their order and the retry stop are the capture's. The selector runs no OCR, so this
  is the whole of its effect on Tier 1 for this capture, and says nothing about another capture's
  text.
- **Capture-dependent reach.** Slovakia 2005 and DPRK 2005 show that another capture can put a
  document on either side of the rule. The two wrong → exact are measured on this text only.
- **Not the product path.** The arms exercise `read_tier1` as #609 routes it, in both benches.
  Promotion must also route the v1 record's parse (the maintainer's decision on #609), and this
  note measures nothing there.
- **The no-truth documents rest on one reader's eye.** Russian Federation 2004 and Bosnia and
  Herzegovina 2014 have no fixture. No gate or fixture checks them.
- **No timing claim.** Replay timings are not OCR timings, and the synthetic runs were not timed
  for comparison.

## Candidates rejected on the way

Rejected in the #574 PR 4 plan before any code, on its Derived Python model over the `76f914e`
capture. They are recorded here so they are not proposed again. None is re-measured by this note.

- **A ranking of line-1 candidates.** Every observed line 1 has the same checksum backing, so a
  ranking needs tie-break constants that nothing has measured (principle 6). The rule is a filter
  plus a uniqueness test instead.
- **A tail-fitted variant** that pads or trims a candidate to the format width. It applied to 9
  documents and was ambiguous on 2. It pulled in Croatia ID 2002's garbage line and three more
  documents without truth, and it tied Somalia 3–3 between two different name fields. Exact width
  only.
- **Pass provenance as eligibility** (the same pass family as the incumbent). A pass carries no
  evidence of correctness, and `mrz` stays pass-agnostic. Provenance is reported, never used.
- **Three live arms instead of replays.** The retry loop is wall-clock budgeted, so live arms can
  read different text, and a budget stop in one arm only is not an A/B.
- **Earlier selection schemes cost accuracy.** A candidate search that kept looking for a
  plausible line 1 lost accuracy. A later reading superseding an earlier one cost Azerbaijan 2022
  its exact name ([`retry-stop-rerun-2026-09-27.md`](retry-stop-rerun-2026-09-27.md)). This rule
  never replaces a grammatical incumbent. Azerbaijan 2022 is `kept` here and stays exact.

## Open

- **Promotion** of the default from `off`, for the maintainer. It is a separate decision, and it
  includes routing the v1 record's parse.
- **The 14 grammatical wrong names** are outside this rule by design, and they are most of what
  keeps strict names at 15 / 45. The mechanisms are in
  [`wrong-physical-line1-2026-09-28.md`](wrong-physical-line1-2026-09-28.md) and
  [`ocr-filler-unknown-trace-2026-09-29.md`](ocr-filler-unknown-trace-2026-09-29.md).
- **Bosnia and Herzegovina 2014's `AZRACMARINA`.** A `<` read as a letter inside a grammatical
  field is invisible to the grammar. This note does not propose a check for it.
