# A wrong physical line accepted as line 1: nine real hits and one synthetic TD1 read, and ranking fixes seven at no measured cost

**Date:** 2026-09-28 · **MAIN:** `c21606b` (#577's code, on `main` as `aee7b20`) and `bf7894c`; `55c03a2` (#566, not merged) for the synthetic TD1 case · **DATA:** the local `samples-data` checkout, whose `bf7894c` run reproduces the CI ledger measured at DATA `396b22f` on all 261 outcomes; none for the synthetic runs (generated corpus) · **Evidence:** Observed (two local release `provider-bench --real-specimens --mrz-only --dump-ocr --dump-ocr-hits` runs, five `synthpass-bench --profile clean --count 100 --seed 0` runs, sixteen single-seed `--dump-ocr` dumps, and a replay of every dumped text through the `mrz` crate and an instrumented copy of it) plus Derived (check-digit arithmetic, per-zone census) plus Modelled (Tier-1 effect of refusals and reclassification) · **Status:** current

**2026-09-28; replays finished 2026-09-29.** This is the write-up that
[#574](https://github.com/ruledicaprio/SynthPass/issues/574) PR 7 asks for before any code. #574
lists four accepted reads whose line 1 is a different physical line: a name line, a second
reading of line 2, or a visual-zone line. Line 1 of every two-line format carries no check digit,
and TD1's line 3 carries none either. So a checksum-valid zone can hold a line that is not the one
it claims to be. This note works each case out from the dumped OCR text. It then applies six
candidate structural checks to every accepted zone, in two modes: **refuse** (the zone is dropped)
and **rank** (the zone is kept only when no alternative passes). It names every correct read that
each check would cost. No code, ADR or baseline changes with it.

## The answer

- **Observed: 9 of the 139 real hits** hold a wrong physical line in some slot, not 3. They are the
  three #574 names, three more cases of line 2 read twice, one TD1 line 3 that repeats line 1, and
  two zones assembled entirely from repeated lines. Rates: 9 / 139 hits; the hits are 139 / 151
  scored (92.1%) and 139 / 261 corpus-wide (53.3%).
- **Observed: three of the nine are manufactured hits, not wrong-line versions of a right read.**
  - Russian Federation ID 2013 and Switzerland ID 2003 print zones that fail their own check
    digits.
  - Croatia ID 2002's accepted TD2 zone is its TD1 line 1, read twice.
  - No structural check can rank any of the three to a correct reading, because the text holds
    none.
- **Observed: ranking costs no correct read** on any population measured. Ranking uses three
  checks: the issuer resolves, no slot repeats another, and there is no digit in a two-line
  format's name field.
  - **Real:** 7 hits get a better line (Kosovo 2023's becomes exact), and 0 hits are lost.
  - **Synthetic at `c21606b`:** 4 zones change, the hit count stays 389 / 500, and 0 correct reads
    are lost.
  - **At `55c03a2`:** seed 57 becomes an exact correct read.
- **Observed: refusing costs reads.**
  - The TD1 name-shape check refuses 2 correct synthetic TD1 reads at `c21606b` (seeds 92 and 95,
    whose document numbers are all letters). It flags 4 at `55c03a2`.
  - The issuer check refuses 9 real hits, among them Somaliland 2023 (a non-ISO issuer) and
    Germany 2018.
  - The repeat and visual-zone checks refuse only the two manufactured hits (Croatia 2002 and
    Switzerland 2003).
- **Recommendation:**
  - **Rank** for all three mechanisms.
  - **Refuse** none of them now.
  - The three manufactured hits are a corpus and gate question for the maintainer (see
    [Open questions](#open-questions-for-the-maintainer)).

## Why a wrong line passes: what each format's check digits cover

**Derived**, from ICAO 9303 Parts 4–7 and the arithmetic below:

| Format | Line 1 | Line 2 | Line 3 |
| --- | --- | --- | --- |
| TD3, TD2 | code, issuer, names: **no check digit** | number, DOB, expiry, (TD3) personal number, composite | — |
| MRV-A, MRV-B | code, issuer, names: **no check digit** | number, DOB, expiry; no composite | — |
| TD1 | number (cells 5–13, check 14) and optional data, both inside the composite | DOB, expiry, composite (cell 29) | names: **no check digit** |

On TD3, TD2 and MRV, any string of the right width that starts with the format's code letter
passes as line 1. That includes a second reading of line 2, which starts with a document number
such as Kosovo's `P000000005`. On TD1, a wrong line 3 is free. A wrong line 1 must pass two
digits, the document number's and the composite. **Modelled:** that is about 1 in 100 for
uniformly random digits. **Observed:** the scan makes hundreds of TD1 parse attempts per document
(241 on Russia 2013 and 481 on Croatia 2002 before a zone is returned), so a 1-in-100 coincidence
is reachable. Tier-1 validity also accepts a filler as a check digit worth 0. It does not require
a date field to hold digits: Russia's accepted date of birth is `IPRUS2`.

## Where the zones are assembled

Every case below is the ordinary scan in `find_and_parse_with`
([`parser.rs`](../../crates/mrz/src/parser.rs)), not the damaged pass: `damaged_recovery` is
`false` on all ten. The scan pairs each candidate line 1 with any of the next three lines
(`take(3)`). The OCR retry loop appends every pass's MRZ-shaped lines to one text
([`lib.rs`](../../crates/synthpass-ocr/src/lib.rs), the `text.push_str(&candidates)` in the retry
loop). The scan therefore sees no pass boundaries. The per-pass stop test in that loop parses one
pass's lines alone, so none of these stitches is what stopped the loop. They form later, in
Tier 1's parse of the concatenation. **Observed** from an instrumented copy of the crate that
logged the raw lines behind each returned zone.

## The four cases in #574

### 1. Russian Federation ID 2013, card back (TD1): the zone is rotated by one row

- **Printed zone** (read off the image, and matching the OCR's clean lines):
  `IPRUS2414567891<<<<<<<<<<<<<<<` / `8703021F2208138RUS<<<<<<<<<<<5` /
  `ALEKSANDROVA<<ALEKSANDRA<<<<<<`.
- **Accepted zone:** `ALEKSANDROVATA1EKSANDRA<<<<<<<` / `IPRUS2414567891<<<<<<<<<7<<<<<` /
  `8703021F2208138RUS<<<<<<<<<<<5`. That is printed line 3, then line 1, then line 2.
- **Assembly (Observed):** raw lines 20–22 of the concatenated text. Line 20 is the name line
  (`ALEKSANDROVATALEKSANDRA<`) that closes the three MRZ lines at raw 18–20, which are inferred
  from the line order to be one pass. Lines 21–22 are the next pass's line 1
  (`IPRUS2414567891<<<<<<<<<7`, with an OCR `7` at cell 24) and line 2. The three are adjacent, so
  no gap tolerance was needed. The pass boundary is invisible to the scan.
- **Check digits (Derived):**

| Field | Accepted zone (value, digit) | Holds? | Printed zone (value, digit) | Holds? |
| --- | --- | --- | --- | --- |
| Document number | `ANDROVATA`, `1` | yes | `241456789`, `1` | **no** (computes 8) |
| Date of birth | `IPRUS2`, `4` | yes, over letter values | `870302`, `1` | **no** |
| Date of expiry | `456789`, `1` | yes | `220813`, `8` | **no** |
| Composite | `<` = 0 | yes | `5` | **no** |

- **What the printed zone implies:** it fails all four of its own digits. This specimen is
  non-conforming. The checksum-valid reading is a coincidence of the rotation plus one spurious
  cell inside the composite's range. It is not a wrong-line version of a correct read. There is
  none to rank toward.
- **Ledger:** the manifest's `observed` block records this reading as the specimen's zone
  (`document_code` `AL`, `issuing_state` `EKS`).

### 2. Kosovo passport 2023 (TD3): line 2 read twice, from two passes

- **Accepted zone:** `POOOO000O5RKS0108308F33073081001234567<<<<14` /
  `P000000005RKS0108308F33073081001234567<<<<14`.
- **Printed line 1** (image): `P<RKSBERISHA<<VLORA<<<…`.
- **Assembly (Observed):** #574's reading is confirmed, with one refinement. The two line-2
  readings come from **different passes**.
  - The scan's line 1 is one pass's line-2 reading (`PO000000O5RKS…`).
  - Its line 2 is the next pass's line 2, three raw lines later, reached through `take(3)`.
  - The earlier pass's own line 1 was a 27-character fragment that paired with nothing.
  - The next pass's own pair (`P<RKSBERISHA<<VLORA…` with its line 2) validates on its own when
    replayed alone. The scan reached the earlier line first.
- **Check digits (Derived):** all five hold, because all five are on line 2. The line in the
  line-1 slot enters none of them.

### 3. Sweden visa 2024 (MRV-B): a visual-zone header as line 1

- **Accepted zone:** `VISERINGVISASNE987654321<<<<<<<<<<<<` / `0202217109XXX6704045F2501154<1901001`.
- **Printed line 1** (image): `VCSWETESTSSON<<ANNIKA<<<…`.
- **Assembly (Observed):** four consecutive raw lines.
  - `VISERINGVISASNE987654321` is the header "VISERING/VISA … SWE 987654321", 24 characters.
  - `SVERIGESWFDENISUEDESWE` comes next.
  - Then the true line 1 (`VCSWETESTSSONANNIKA<<<<<<<KK<<<K`) and line 2.
  - The header starts with `V`, and line 2 is three lines below it, inside `take(3)`. The scan
    tried the header first and padded it to 36.
- **Check digits (Derived):** the document number (`020221710`, `9`), birth (`670404`, `5`) and
  expiry (`250115`, `4`) digits hold on line 2. MRV-B has no composite, so line 1 is never
  consulted.

### 4. Synthetic TD1 seed 57: two different failures on two generators

The generator differs between the two commits (#566 changes TD1's line pitch), so the OCR text
differs. Keep the two apart.

- **At `55c03a2` (#566):** the name line is taken as line 1.
  - **Accepted zone:** `ADEYEMIYUKI<<<<<<<<<<<<<<<<<<<` / `6507110F3311226USAT8C7EXTQH531` /
    `ADEYEMYUKI<<<<<<<<<<<<<<<<<<<<`.
  - **Truth:** `I<USAK2HPBKQ2L0<<<…` / the same line 2 / `ADEYEMI<<YUKI<<<…`.
  - **Assembly (Observed):** a cross-pass stitch. The first pass's name line
    (`ADEYEMIYUKI<<<<<<`, raw line 11) is followed by the VIZ watermark and then by pass-00's
    lines. The scan pairs the name line with pass-00's line 2 (two lines later) and line 3. It
    skips pass-00's own line 1, `IUSAK2HPBKQ2L0<<<…`.
  - **Check digits (Derived):** DOB and expiry hold, as on truth.
    - **Document number:** `MIYUKI<<<`, check `<`. It computes 0, and the filler counts as 0.
    - **Composite:** the name line's cells 5–29 sum to 0 mod 10 under the weights, and so do
      truth's. That is a second 1-in-10 coincidence.
- **On `main` (`c21606b`):** the true line 1, shifted one cell left.
  - **Accepted line 1:** `IUSAK2HPBKQ210<<<…`, giving code `IU`, issuer `SAK` and document number
    `2HPBKQ210`. Its check is `<` = 0, and it computes 0.
  - **Assembly (Observed):** ordinary scan, one pass (raw lines 8–10).
  - This is #574 PR 6's case, and it is not a wrong physical line.
  - **Derived:** even with the prefix restored (`I<USAK2HPBKQ210`), the document number reads
    `K2HPBKQ21` where truth is `K2HPBKQ2L`. `L` is worth 21 and `1` is worth 1, at weight 1, so
    the check digit cannot tell them apart
    ([`checkdigit-blindspots-exact-2026-09-26.md`](checkdigit-blindspots-exact-2026-09-26.md)).
    The read stays `document_number_mismatch` after any line-1 fix.

## The other five real hits with a wrong line

All five are **Observed**, from the same traces.

| Document | Format | Mechanism | Accepted line(s) | Assembly |
| --- | --- | --- | --- | --- |
| Kosovo passport 2011 | TD3 | line 2 twice | `P0OOO00005RKS9101092F2106308<<<…04` as line 1 | pass N's line 2 + pass N+1's line 2, two raw lines apart |
| Slovakia passport 2005 | TD3 | line 2 twice | `P0OOO0005SVK1111112M1501043<<<…3<00` as line 1 | pass N's 35-character line 2 + pass N+1's line 2, three lines apart |
| Canada passport `PP` 2023 | TD3 | line 2 twice | `P1Z3456AA0CAN9008010F330114<<<…06` as line 1 | pass N's line 2 + pass N+1's line 2, three raw lines apart |
| Belgium ID 2018, back | TD1 | line 1 twice | `IDBEL0005906961015<<<<<<<<<<<<` as line 3 | three consecutive raw lines |
| Switzerland ID 2003, back | read as MRV-B | name line twice | `VADIS3QUOCE<<<…KK<<` / `VADISC<QU0E<<<…` | two name-line readings three raw lines apart, across a pass |
| Croatia ID 2002, back | read as TD2 | line 1 twice | `IOHRVOOOOOOOOOO<K<<<…K` / `TOHRVO0O00OOO00<<<…` | two consecutive raw lines |

- **Switzerland 2003 is a manufactured hit.**
  - The card is TD1. Its printed zone (`IDCHES0002568<5<<<…` / `8102288M1301013CHE<<<<<<<<<<<5` /
    `VADIS<<QUOCE<<<…`, from the image and matching several OCR passes) fails all four of its own
    digits.
  - The accepted MRV-B zone is two readings of the name line. Its document-number digit holds by
    a 1-in-10 coincidence (`VADISC<QU`, `0`).
  - Its date fields are all filler, which the filler-as-zero rule satisfies trivially.
- **Croatia 2002 is a wrong read counted as a hit.**
  - The printed TD1 zone is conforming: the replay parses `IOHRV0000000000<<<…` /
    `7701018F0212126HRV<<<<<<<<<<<0` / `SPECIMEN<<SPECIMEN<<<…` as valid.
  - The accepted zone is TD2, built from two readings of line 1. The document number of zeros
    matches only because both readings are zeros. The birth date is `00<<<<` and the expiry is
    all filler.
  - No valid TD1 reading of the OCR text exists. Refusing the TD2 zone leaves an invalid TD1
    parse.

## Candidate checks and their measured cost

The checks, each applied to every accepted (checksum-valid) zone:

| Check | Refuses or demotes a zone when |
| --- | --- |
| **C1 shape** | line 1 cell 0 is not the format's code letter (TD3 `P`, MRV `V`, TD1/TD2 `I`/`A`/`C`), or cells 1–4 are not letters or filler |
| **C2 issuer** | line 1 cells 2–4 do not resolve in the 278-code registry (`countries.rs`) |
| **C3 repeat** | any two slots are near-identical: similarity ≥ 0.6 after dropping fillers and folding `O/Q/D→0`, `I/L→1`, `Z→2`, `S→5`, `B→8`, `G→6`, `T→7` (similarity = 1 − Levenshtein / longer length) |
| **C4 TD1 name shape** | TD1 line 1 has no digit outside cell 14 |
| **C5 VIZ digit** | a two-line format has a digit anywhere in line 1 cells 5+ (the names) |
| **C6 dates** | birth or expiry field holds anything but digits or filler |

**Refuse** drops the zone, and the scan continues (then the damaged pass). **Rank** does the same,
but falls back to the first dropped zone when nothing else passes, so a document that validated
still validates.

Populations:

- **Real:** the 139 hits at `c21606b`. Each flag is replayed over all 171 dumped texts.
- **Synthetic:** the 410 accepted zones (389 hits) at `c21606b`, across the five formats at 100
  seeds each. Every flagged seed is replayed from its own dump.
- **#566:** TD1 at `55c03a2`, 67 accepted zones and 57 hits.

**Observed** unless marked. Unflagged zones cannot change under either mode: the first valid
unflagged zone is still returned first (Derived).

| Check | Real hits flagged | Real hits lost, refuse | Real hits lost, rank | Synthetic flagged (`c21606b`) | Correct synthetic lost, refuse / rank | `55c03a2` TD1 flagged |
| --- | --- | --- | --- | --- | --- | --- |
| C1 shape | 3: Canada `PP` 2023, Kosovo 2011, Slovakia 2005 | 0 (all 3 re-read) | 0 | 1: TD3 95 | 0 / 0 | 0 |
| C2 issuer | 15 | **9** | 0 | 4: TD1 52, 57, 75 and TD3 95, all wrong reads | 0 / 0 | 5: 0, 28, 39, 52, 57, all wrong |
| C3 repeat | 7: Belgium 2018, Croatia 2002, Switzerland 2003, Canada `PP` 2023, Kosovo 2011 and 2023, Slovakia 2005 | **2**: Croatia, Switzerland | 0 | 3: TD1 51, 71 and TD3 95, all hits holding a repeated line | 0 / 0 | 3: 0, 28, 57, all wrong |
| C4 TD1 name shape | 1: Russia 2013 | **1**: Russia | 0 | 2: TD1 92, 95, **both correct reads** | **2** / 0 | 6: 22, 57 wrong; 66, 68, 92, 95 correct |
| C5 VIZ digit | 6: Switzerland 2003, Sweden visa 2024, Canada `PP` 2023, Kosovo 2011 and 2023, Slovakia 2005 | **1**: Switzerland | 0 | 1: TD3 95 | 0 / 0 | 0 |
| C6 dates | 1: Russia 2013 | **1**: Russia | 0 | 0 | 0 / 0 | 0 |

- **C2's nine refusals:** Russia 2013 and Switzerland 2003 return nothing. Argentina 2015
  (emergency), Cetis 2022, Germany 2018, Somaliland 2023 and Spain 2022 are left with an incomplete
  sequence. Dominican Republic 2020 and Sweden 2012 (boxed) fall to an invalid parse.
- **C2 in rank mode** re-reads six zones and loses none: Sweden visa, Canada `PP` 2023, Kosovo
  2011 and 2023, Netherlands 2014 (blur) and Slovakia 2005.
- **Similarity gap (C3's 0.6 threshold):**
  - **Correct zones** score at most 0.30 on real hits (Slovenia ID 2022) and 0.28 on synthetic.
  - **Flagged zones** score at least 0.69 (Switzerland 2003); the next is Croatia at 0.82.
  - The threshold sits in a measured gap, so P6 holds: the constant has a measurement behind it.
- **Negative results:**
  - C1 misses Kosovo 2023, because `POOOO` reads its zeros as letters.
  - C4 rank changes nothing, because Russia has no alternative.
  - C4 refusal costs correct reads: all-letter document numbers are legal, and the generator
    produces them.

**The recommended set, C2 + C3 + C5 in rank mode, together** (Observed):

| | Before (`c21606b`) | Rank C2 + C3 + C5 |
| :-- | --: | --: |
| Real Tier-1 hits, scored | 139 / 151 = 92.1% | 139 / 151 = 92.1% |
| Real Tier-1 hits, whole corpus | 139 / 261 = 53.3% | 139 / 261 = 53.3% |
| Real hits whose read changes | — | 7 |
| Synthetic hits, five formats × 100 | 389 / 500 | 389 / 500 |
| Synthetic accepted zones whose read changes | — | 4 |
| `55c03a2` TD1 seed 57 | `document_number_mismatch` | exact on every line (Modelled: TD1 57 → 58 / 100 at `55c03a2`) |

**The seven real re-reads** are all Observed on replay; "Right" means it agrees with the printed
zone, the fixture, or the issuer in the filename:

| Document | Slot | New read | Right | Still wrong |
| --- | --- | --- | --- | --- |
| Kosovo 2023 | line 1 | `P<RKSBERISHA<<VLORA<<<…` | exact to the printed line | nothing |
| Kosovo 2011 | line 1 | `P<RKSKOSOVA<MARIGONA<<<…K<<<<<<` | code and issuer | not verified against the image |
| Canada `PP` 2023 | line 1 | `PPCANMARTIN<SARAH<<<…` | code and issuer | one name separator |
| Slovakia 2005 | line 1 | `PSSVKSPECIMEN<VZOR<<<…` | issuer | code (`PS`, fixture `P<`) and one separator |
| Netherlands 2014 (blur) | line 1 | `PSNLDSEMERELS<SHARDYONESULISESSGIRIGORIOS<<<` | issuer | code and several names |
| Sweden visa 2024 | line 1 | `VCSWETESTSSONANNIKA<<<…` | code and issuer | one name separator |
| Belgium 2018 | line 3 | `SPECIMEN<SPECIMEN<<<<<<<<<<<KK` | — | not verified against the image |

**The four synthetic re-reads at `c21606b`:**

- **TD1 51, line 3:** was a repeat of line 2, now `ESKANDARIPRIYA<<<…` (truth `ESKANDARI<<PRIYA`).
- **TD1 57, line 1:** prefix now `I<USA`, but the document number is still check-blind wrong, as
  explained above.
- **TD1 71, line 3:** was a repeat of line 1, now the watermark `SYNTHETIESPECIMENSYNTHETIC`.
  Still wrong.
- **TD3 95, line 1:** was a repeat of line 2, now `P<BLRTKACHENKO<OLENA<<<…`, one separator short
  of truth.

None of the four changes the hit bit.

## Decision per mechanism

- **Name line as TD1 line 1 → rank by C2 (issuer resolves); do not refuse by C4.**
  - C2 rank loses nothing on any population and turns `55c03a2` seed 57 into a correct read.
  - This is PR 6's rule ("prefer the candidate whose issuer resolves; never refuse when none
    does"), applied where these zones form: the ordinary scan, not only the class sweep.
    Whether PR 6's diff already covers these zones is not verified here.
  - C4 as a refusal costs 2 correct synthetic reads at `c21606b`, and it flags 4 at `55c03a2`.
  - Russia 2013 cannot be ranked away, because no alternative exists. Only C4 or C6 in refuse
    mode remove it (next question).
- **Line 2 read twice → rank by C3 (no slot repeats another).**
  - It is the only check that catches all four real two-line cases, Belgium's line 3, and
    synthetic TD1 51, 71 and TD3 95, at zero rank cost.
  - C1 catches three of the four and is not needed alongside C3.
  - Refusing costs exactly the two manufactured hits (Croatia 2002, Switzerland 2003). That
    trade is the maintainer's; see the open questions.
- **Visual-zone line as line 1 → rank by C5 (and C2, which flags the same zone).**
  - Sweden visa 2024 is the only real instance. Rank fixes it at zero cost.
  - Refusing costs Switzerland 2003 only.
  - C5 cannot see a header line with no digits whose cells 2–4 happen to resolve. How often such a
    line occurs is not measured.
- **Precedent.** Ranking is line-1 attempt selection, the narrow task ADR-0021's maintainer
  decision of 2026-09-24 sanctions
  ([ADR-0021](../decisions/ADR-0021-fixed-grid-mrz-strips.md), "Decision (maintainer,
  2026-09-24)"). It needs no aligner.
  - #440 refuses when line 1 alone cannot decide between candidates
    ([`m4-gate-440-wrong-reads-refused-2026-09-25.md`](m4-gate-440-wrong-reads-refused-2026-09-25.md)).
    Here a refusal would be warranted only where no alternative exists, and on these corpora
    those are the three manufactured hits.

## How this was measured

- **Real specimens** (Observed): release `provider-bench` built at `bf7894c` and at `c21606b`,
  every OCR arm at its default except chargrid off, run from the repository root:

  ```
  provider-bench --real-specimens --mrz-only --progress --dump-ocr --dump-ocr-hits \
    --assert-baseline knowledge/benchmarks/real-specimen-mrz-baseline.json --out <report>.json
  ```

  - The `bf7894c` run changed 0 of 261 outcomes against the CI ledger and scored 139 hits.
  - The `c21606b` run's 261 outcomes have the same bucket counts. Its zones are identical except
    Canada 2013 and Djibouti 2017, which do not affect any count here.
- **Synthetic:** release `synthpass-bench --document-type <td1|td2|td3|mrva|mrvb> --profile clean
  --count 100 --seed 0` at `c21606b`, and `--document-type td1` at `55c03a2`. Raw text came from
  `--count 1 --seed <n> --dump-ocr` on each flagged seed: TD1 51, 52, 57, 71, 75, 92, 95 and
  TD3 95 at `c21606b`, and TD1 0, 28, 39, 52, 57 at `55c03a2`.
- **Replay** (Observed):
  - Each dumped `raw_ocr_text` goes through `mrz::find_and_parse` from a release build of
    `crates/mrz` at `c21606b`. The replay reproduces all 139 dumped hit zones exactly.
  - The checks ran in an uncommitted copy of that crate. It filters inside
    `find_and_parse_with`'s `consider` closure and after `damaged_pass`, switched by environment
    variable. With no check enabled it reproduces the unmodified crate on every text.
  - The same copy logged the raw lines behind each returned zone and counted TD1 parse attempts.
- **Why a text replay models the pipeline in rank mode (Derived):** rank never turns a valid parse
  into an invalid one. The retry loop stops on validity (`find_and_parse(&candidates)` per pass),
  so it runs the same passes and builds the same text. In refuse mode the loop could run further,
  and the replay under-states what refusal would see.
- **Check-digit arithmetic** (Derived): ICAO 7-3-1 weights, letters worth 10–35, filler worth 0.

## What this does not claim

- **No pipeline A/B was run.** The costs are from a text replay of one run per population. The
  replay is faithful for rank mode (see above) but not for refuse mode. A same-binary A/B on the
  full real corpus is still needed before code.
- **Nothing about time.** The machine was running other cargo work throughout, so no timing was
  taken. The only cost figure is Russia's TD1 parse attempts: 241 before its zone is returned,
  and 19,314 in refuse mode, which exhausts the damaged pass first. A flagged document with no
  alternative pays the same under rank mode.
- **The seven real re-reads are better lines, not correct ones.** Only Kosovo 2023's line 1 is
  exact against the printed zone. The others fix the issuer, and not always the code or the name
  separators.
- **Synthetic `55c03a2` covers TD1 only.** #566 is not merged, and its other formats were not
  examined.
- **Flagged seeds only.** The TD1 seeds 22, 66 and 68 at `55c03a2` were counted by the census, not
  replayed.
- **PR 6 was not run.** Nothing here says whether its diff reproduces the C2 rank result.
- **The three manufactured hits are not proposed for any edit here.** Reclassifying a specimen
  needs a reviewed fixture, and a Tier-1 change needs a CI re-bless in the same PR.

## Candidates rejected

- **C4, refuse a TD1 line 1 with the name-line shape.** It costs 2 correct synthetic TD1 reads at
  `c21606b` (seeds 92 and 95, document numbers `GPGRTOVDJ` and `PFNTZPGCI`) and flags 4 at
  `55c03a2`. As a rank it changes nothing that C2 does not.
- **C2 as a refusal.** It costs 9 real hits. The issuer registry is not a validity test for
  non-ISO issuers (Somaliland `RSL`) or for issuers the OCR mangles on a correct zone.
- **C1 as the repeat detector.** It misses Kosovo 2023.
- **A position-wise repeat score (best of −1/0/+1 cell offsets, fillers ignored).** It scored
  Croatia 2002 at 0.82 but Switzerland 2003 at only 0.46. The Levenshtein form over
  filler-stripped lines separates both from every correct zone.
- **Raw `difflib` similarity with trailing fillers stripped.** It scored Switzerland 2003 at 0.40,
  and it caught the four TD3 cases and Belgium but not the two MRV-B/TD2 zones.

## Open questions for the maintainer

1. **Rank now?** C2 + C3 + C5 in rank mode, with one PR per mechanism or one for all three. The
   measured cost is zero on 139 real hits and 410 synthetic zones. Should the gate be a
   same-binary pipeline A/B on the full real corpus plus the five synthetic formats, with a
   correct→refused crosstab as ADR-0021 requires for these narrow fixes?
2. **The three manufactured hits.**
   - Russia 2013 and Switzerland 2003 print non-conforming zones. Should each get a reviewed
     fixture (`mrz_checksums_valid=false`), which moves them to `checksum_failed_specimen`?
   - Croatia 2002's accepted TD2 zone is wrong in format and dates. Should it be refused, which
     takes C3 in refuse mode?
   - **Modelled** if all three are done: Tier-1 139 / 151 = 92.1% → 136 / 149 = 91.3% scored, and
     139 / 261 = 53.3% → 136 / 261 = 52.1% corpus-wide.
   - Precedent: Bosnia 2013, refused by #440 and recorded in `FINDINGS.md` on 2026-09-24.
3. **C6 (date fields must be digits).** Tier-1 validity does not require it today, so Russia's
   birth date `IPRUS2` passes. C6 flags only Russia on every population measured. Is it a
   validity rule (refuse) or out of scope for #574?
4. **Should Tier-1 stop reading a filler as a check digit of 0 when the field it checks is not all
   filler?** Every case above leans on it once: `MIYUKI<<<` with `<`, and composites of `<`. This
   is a question, not a measurement; its cost has not been counted.
5. **The manifest's `observed` block** records the wrong reading as the specimen's zone for
   Russia 2013 (`AL`/`EKS`), Switzerland 2003 (`VA`/`DIS`, MRV-B) and Croatia 2002 (TD2). Should it
   be regenerated once any fix lands?
