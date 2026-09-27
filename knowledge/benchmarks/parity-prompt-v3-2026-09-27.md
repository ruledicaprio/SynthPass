# Prompt v3 moves no parity field on the 84 fixtures v2 reached, and completes all 118: 50.8% reviewed, 55.3% derived

**Date:** 2026-09-27 · **MAIN:** `39345b2` (PR #533, branch `claude/506-bound-tier2-prompt`); v2 arm `f1072c4` · **DATA:** none (the parity fixtures are tracked on `main` under `samples/ocr_fixtures/`, byte-identical at both SHAs) · **Evidence:** Observed (two CI `native-llm` job logs, same pinned GGUF, greedy decoding) plus Derived (per-fixture, per-field pairing of the two logs; a Python mirror of the prompt's content cleanup over the 118 tracked fixtures) · **Status:** current

**2026-09-27.** The measurement record for prompt v3
([#506](https://github.com/ruledicaprio/SynthPass/issues/506),
[PR #533](https://github.com/ruledicaprio/SynthPass/pull/533)). v3 changes only what happens to
the OCR text before it reaches `{content}`: a second line filter, `drop_long_run_noise`, drops any
line holding a whitespace-free run longer than 64 characters (guilloche microprint read as one
long Latin string), and `build_prompt_within_budget` caps the content so the prompt plus the
500-token output budget fits `n_ctx`, always keeping MRZ-shaped lines. `SYSTEM`, `FIELDS`,
`TEMPLATE` and `HINT_PREFIX` are unchanged, so the digest is too; `PROMPT_VERSION` goes 2 → 3.

The v2 run is the one that opened #506. It panicked on fixture 85 of 118,
`Romania_Passport_Specimen_PE_ROU_2024_mrz`: `prompt (2257 tokens) does not fit in context window
(2048)`. So the only honest before/after is the paired one on fixtures 1–84, and fixtures 85–118
have a v3 number and no v2 number.

## The answer

- **Observed: zero field verdicts flip on fixtures 1–84.** 281 / 522 fields OK in both arms. Two
  field *values* changed, both wrong in both arms, and both on fixtures whose prompt v3 changed.
- **Observed: on the 81 fixtures whose prompt v3 leaves byte-identical, all 501 scored field
  values are byte-identical across the two runs.** Greedy decoding on this pinned GGUF reproduced
  exactly across two CI runners a day apart. The noise floor this pair shows is zero, so a
  one-field delta between two such runs is attributable, not noise.
- **Observed: v3 completes the corpus and passes.** 118 / 118 fixtures, no `EXTRACTION FAILED`,
  0 repair fallbacks. Reviewed **297 / 585 = 50.8%** over 65 documents; derived **88 / 159 =
  55.3%** over 53; overall 385 / 744 = 51.7%. Both 15% floors clear by 35–40 points.
- **Derived: only 5 of the 118 fixtures have a different v3 prompt** (the mirror below). Romania
  loses 15 microprint lines and fits. The other four are Czechia 2005, Germany 2018, Portugal ID
  2024 back and Switzerland ID 2023 back.
- **The 2026-09-05 baseline (58.6% / 52.5%) is not comparable** to either arm here: a different
  fixture set (72 → 118, reviewed 18 → 65), a different normalizer vocabulary, and edited
  fixtures. See [Comparability](#comparability-with-the-2026-09-05-baseline).

## Headline, v3, all 118 fixtures

`crates/synthpass-llm/tests/parity.rs`, `qwen2.5-1.5b-instruct-q4_k_m`, `n_ctx` 2048, grammar on,
vocabulary `b6bd1f9a5fdd108e`:

| Set | Documents | Fields OK / scored | Rate | 15% floor needs | Margin |
| :-- | --: | --: | --: | --: | --: |
| reviewed (all nine prompt fields) | 65 | **297 / 585** | **50.8%** | 88 | 209 fields, 35.8 pp |
| derived (check-digited fields only) | 53 | **88 / 159** | **55.3%** | 24 | 64 fields, 40.3 pp |
| overall | 118 | 385 / 744 | 51.7% | — | — |
| legacy seven-field, reviewed only | 65 | 219 / 455 | 48.1% | — | — |

Per field, reviewed: `issuing_country` 41 / 65, `date_of_birth` 39, `document_type` 37,
`date_of_expiry` 36, `sex` 34, `document_number` 33, `given_names` 31, `nationality` 25,
`surname` 21. Derived: `date_of_birth` 31 / 53, `document_number` 29, `date_of_expiry` 28.

**The legacy 48.1% equals the 48.1% in `parity.rs`'s climb table by coincidence.** That row is the
2026-09-04 *overall* rate, 156 / 324 over 72 fixtures; this is the seven-field rate over 65
reviewed documents, 219 / 455. Different metric, different corpus.

## Paired comparison, fixtures 1–84

Both runs walk the fixtures in the same sorted order; the first 84 stems are identical, as is
every `expected=` value.

| Set, fixtures 1–84 | Documents | v2 (`f1072c4`) | v3 (`39345b2`) | Δ |
| :-- | --: | --: | --: | --: |
| reviewed | 45 | 218 / 405 = 53.8% | 218 / 405 = 53.8% | 0 |
| derived | 39 | 63 / 117 = 53.8% | 63 / 117 = 53.8% | 0 |
| overall | 84 | 281 / 522 = 53.8% | 281 / 522 = 53.8% | 0 |
| legacy seven-field | 45 | 165 / 315 = 52.4% | 165 / 315 = 52.4% | 0 |

Per-field verdicts: **0 OK → MISMATCH, 0 MISMATCH → OK.** The two values that moved:

| # | Fixture | Set | Field | Expected | v2 | v3 |
| --: | :-- | :-- | :-- | :-- | :-- | :-- |
| 41 | `Germany_Passport_Specimen_P0_D00_2018_mrz` | reviewed | `date_of_expiry` | `2028-12-01` | `2028-12-11` ✗ | `2028-01-12` ✗ |
| 83 | `Portugal_ID_Specimen_2024_back_mrz` | derived | `date_of_birth` | `1950-01-01` | `2001013F3404012` ✗ | `2001-04-01` ✗ |

Both are on fixtures whose prompt v3 changed (Germany lost 6 lines, Portugal 1), which is the only
place a greedy decode can move. The third prompt-changed fixture in this range, Czechia 2005 (2
lines dropped), returned the same value on all nine fields.

## Which prompts v3 changed

`drop_long_run_noise` re-implemented in Python over `samples/ocr_fixtures/**/*.md` (mirror at the
end). Line and character counts are of the cleaned content that fills `{content}`:

| # | Fixture | Set | v2 content | v3 content | Lines dropped | Runs dropped (chars) |
| --: | :-- | :-- | --: | --: | --: | :-- |
| 29 | `Czechia_Passport_Specimen_P0_CZE_2005_mrz` | reviewed | 2 015 / 78 | 1 871 / 76 | 2 | 74, 68 |
| 41 | `Germany_Passport_Specimen_P0_D00_2018_mrz` | reviewed | 2 265 / 86 | 1 594 / 80 | 6 | 74–171 |
| 83 | `Portugal_ID_Specimen_2024_back_mrz` | derived | 1 264 / 65 | 1 155 / 64 | 1 | 95 |
| 85 | `Romania_Passport_Specimen_PE_ROU_2024_mrz` | reviewed | 4 148 / 96 | 2 495 / 81 | 15 | 69–206 |
| 102 | `Switzerland_ID_Specimen_2023_back_mrz` | reviewed | 1 703 / 53 | 820 / 47 | 6 | 73–186 |

The other 113 fixtures produce byte-identical content under v2 and v3; none ends in a blank line,
which is the one other way the second `lines().join("\n")` pass could have changed a prompt.

The token cap never engaged, per the engineer's offline count with the Qwen2.5 tokenizer: Romania
goes from 2 257 to 1 496 tokens against a budget of 1 548, and no other fixture reaches the
budget. Not re-verified here (no tokenizer). Consistent with it (Derived): Romania is still the
longest v3 content by 432 characters, next Indonesia 2011 at 2 063.

## Run-to-run nondeterminism

**None observed.** 81 of the 84 fixtures fed a byte-identical prompt to the same GGUF (SHA-256
`6a1a2eb6d15622bf3c96857206351ba97e1af16c30d7a74ee38970e434e9407e`, verified by `sha256sum -c` in
both jobs) through the same `llama-cpp-2` 0.1.156 on the same runner image (`ubuntu-24.04`
20260920.314.1), and every one of their 501 scored fields printed the same `actual=` string in both
runs, OK and MISMATCH alike. That is a stronger check than verdict agreement: a wrong answer that
changed to a different wrong answer would have shown here, and none did.

So for this configuration the delta bound is the prompt, not the sampler: any field that moves
between two CI runs of this harness on unchanged fixtures is a code or data change. This is one
pair of runs; see [what this does not claim](#what-this-does-not-claim).

## The 34 fixtures v2 never reached

v3 only; v2 panicked before any of them. 20 reviewed, **79 / 180 = 43.9%**; 14 derived, **25 / 42
= 59.5%**; legacy seven-field 54 / 140 = 38.6%.

**Romania `PE_ROU_2024`, the overflow: 3 / 9.** It now extracts; it does not extract well.

| Field | Expected | v3 | |
| :-- | :-- | :-- | :-- |
| `document_type` | `PE` | `P` | ✗ |
| `issuing_country` | `ROU` | `ROU` | ✓ |
| `document_number` | `040021039` | `6 040021039` | ✗ |
| `surname` | `POPESCU` | `PEROUPOPESCU` | ✗ |
| `given_names` | `MIHAELA STEFANIA` | `MIHAELA STEFANIA` | ✓ |
| `nationality` | `ROU` | `ROMANIAN` | ✗ |
| `date_of_birth` | `1980-01-01` | `1980-01-01` | ✓ |
| `sex` | `F` | `M` | ✗ |
| `date_of_expiry` | `2034-09-02` | `2024-09-02` | ✗ |

`PEROUPOPESCU` is the model reading the MRZ's `PEROUPOPESCU<<…` line-1 prefix into the surname;
`ROMANIAN` is a demonym the normalizer does not hold (see below). Its v3 content still carries five
microprint lines with runs of 50–63 characters, under the 64 threshold by design.

| # | Fixture | Set | OK |
| --: | :-- | :-- | --: |
| 85 | `Romania_Passport_Specimen_PE_ROU_2024_mrz` | reviewed | 3 / 9 |
| 86 | `Russian_Federation_Passport_Specimen_P0_RUS_2019_mrz` | reviewed | 5 / 9 |
| 87 | `Russian_Federation_Passport_Specimen_PD_RUS_2004_mrz` | derived | 2 / 3 |
| 88 | `Saudi_Arabia_Passport_Specimen_P0_SAU_2021_mrz` | derived | 3 / 3 |
| 89 | `Serbia_ID_Specimen_2008_back_with_mrz` | reviewed | 1 / 9 |
| 90 | `Serbia_Passport_Specimen_P0_SRB_2012_mrz` | reviewed | 8 / 9 |
| 91 | `Seychelles_Passport_Specimen_P0_SYC_2022_mrz` | derived | 2 / 3 |
| 92 | `Slovakia_Passport_Specimen_P0_SVK_2005_mrz` | reviewed | 4 / 9 |
| 93 | `Slovakia_Passport_Specimen_PS_SVK_2014_mrz` | reviewed | 3 / 9 |
| 94 | `Slovenia_ID_Specimen_2022_back_mrz` | reviewed | 1 / 9 |
| 95 | `Slovenia_Passport_Specimen_P0_SVN_2016_mrz` | derived | 1 / 3 |
| 96 | `Somalia_Passport_Specimen_P0_SOM_2023_mrz` | reviewed | 7 / 9 |
| 97 | `Somaliland_Passport_Specimen_P0_RSL_2023_mrz_non_ISO` | reviewed | 6 / 9 |
| 98 | `Spain_Passport_Specimen_P0_ESP_2013_mrz` | reviewed | 7 / 9 |
| 99 | `Spain_Passport_Specimen_P0_ESP_2015_mrz_highlight` | reviewed | 5 / 9 |
| 100 | `Spain_Passport_Specimen_P0_ESP_2015_mrz_pixelated` | derived | 2 / 3 |
| 101 | `Sweden_ID_Specimen_2022_back_mrz` | reviewed | 2 / 9 |
| 102 | `Switzerland_ID_Specimen_2023_back_mrz` | reviewed | 1 / 9 |
| 103 | `Tunisia_Passport_Specimen_P0_TUN_2019_mrz_rotated` | derived | 2 / 3 |
| 104 | `Turkiye_ID_Specimen_2020_back_mrz` | reviewed | 3 / 9 |
| 105 | `Turkiye_ID_Specimen_2023_back_mrz` | derived | 0 / 3 |
| 106 | `Turkiye_Passport_Specimen_P0_TUR_2010_mrz` | reviewed | 1 / 9 |
| 107 | `Turkiye_Passport_Specimen_P0_TUR_2022_mrz` | derived | 1 / 3 |
| 108 | `Turkiye_Passport_Specimen_P0_TUR_2024_mrz` | reviewed | 1 / 9 |
| 109 | `Turkiye_Passport_Specimen_P0_TUR_2025_mrz` | reviewed | 3 / 9 |
| 110 | `Ukraine_Passport_Specimen_P0_UKR_2015_mrz` | derived | 3 / 3 |
| 111 | `United_Arab_Emirates_Passport_Specimen_P0_ARE_2011_mrz` | reviewed | 7 / 9 |
| 112 | `United_Arab_Emirates_Passport_Specimen_P0_ARE_2019_mrz` | derived | 3 / 3 |
| 113 | `United_Kingdom_Passport_Specimen_P0_GBR_2005_mrz` | derived | 1 / 3 |
| 114 | `United_Kingdom_Passport_Specimen_P0_GBR_2015_mrz` | derived | 3 / 3 |
| 115 | `United_Kingdom_Passport_Specimen_P0_GBR_2021_mrz` | reviewed | 2 / 9 |
| 116 | `United_States_of_America_ID_Specimen_back_mrz` | derived | 0 / 3 |
| 117 | `Uzbekistan_Passport_Specimen_P0_UZB_2013_mrz` | reviewed | 9 / 9 |
| 118 | `Venezuela_Passport_Specimen_P0_VEN_2014_mrz` | derived | 2 / 3 |

Switzerland ID 2023 back (1 / 9) is the other prompt-changed fixture in this range. It has no v2
number to compare against.

The tail scores lower on reviewed fields than the head (43.9% against 53.8%). Inferred, not
measured: that is which documents sort late (six Türkiye fixtures, seven card backs), not v3,
since v3 left the head unchanged.

## Comparability with the 2026-09-05 baseline

`knowledge/prompts/README.md` and `parity.rs`'s module doc quote 2026-09-05: 72 fixtures (18
reviewed + 54 derived), vocabulary `74aee114001a9ed2`, reviewed 95 / 162 = 58.6%, derived 85 / 162
= 52.5%, 180 / 324 = 55.6% overall. Today's 50.8% / 55.3% cannot be read as a move from it:

1. **Different documents.** 118 fixtures against 72, and the reviewed set is 65 against 18: at
   least 47 reviewed fixtures that 2026-09-05 did not score on nine fields, some promoted out of
   `derived/` and some new. The derived set is 53 against 54, so it lost promotions and gained new
   fixtures. Neither denominator is a superset of the old one.
2. **Different normalizers.** The vocabulary fingerprint is `b6bd1f9a5fdd108e` against
   `74aee114001a9ed2`; `parity.rs` prints it because a changed fingerprint means the same model
   answer can score differently.
3. **Edited fixtures.** #439 changed Belgium ID 2021 back's label and Somaliland 2023's label
   and `.md`. The label edits are to fields this harness does not score, but Somaliland's OCR
   text, the model's input, changed. Earlier edits cannot be traced from this clone (shallow
   history).
4. **Different prompt preprocessing.** v3's filter changes five of today's prompts; measured
   score-neutral on the three it reaches in 1–84, unmeasured on Switzerland.

A like-for-like trend needs a same-binary run over one fixture list. The 84-fixture pair above is
that for v2 → v3; nothing today is that for 2026-09-05 → now.

## Invocation

CI, `ci.yml` `workflow_dispatch`, job `native-llm`, step `cargo test -p synthpass-llm --test parity
-- --ignored --nocapture` (debug test profile), `SYNTHPASS_MODEL_N_CTX` unset (the harness prints
`n_ctx: 2048`):

| Arm | Run / job | Head | Date | Header line | Result |
| :-- | :-- | :-- | :-- | :-- | :-- |
| v2 | 36252511121 / 108433062104 | `f1072c4` | 2026-09-26 | `normalizer vocabulary: b6bd1f9a5fdd108e   log-format: 1   prompt: qwen2.5-fields v2   fixtures: 118` | panicked at fixture 85 after 1 263 s |
| v3 | 36323412121 / 108631494688 | `39345b2` | 2026-09-27 | `normalizer vocabulary: b6bd1f9a5fdd108e   log-format: 1   prompt: qwen2.5-fields v3   n_ctx: 2048   fixtures: 118` | passed, 118 / 118, 2 106 s |

Between the two heads, `samples/ocr_fixtures/`, `crates/synthpass-core` and the scoring path of
`parity.rs` are unchanged; `crates/synthpass-llm/src/lib.rs` changes only the prompt builder call
and the context check. The pairing parsed every `--- [n/118] <stem> (<set>)` header and the
`<field> expected=… actual=… OK|MISMATCH` lines under it from the two job logs, and re-derived the
v3 summary exactly (297 / 585, 88 / 159, 219 / 455).

The prompt mirror, Python, over the tracked fixtures:

```python
WS = set("\t\n\v\f\r \x85\xa0         "
         "       　")   # Rust char::is_whitespace
PUNCT = set("<>/:.,-_()'\"*#|=+&%!?;")
def unescape(s):                       # unescape_html, same order, &amp; last
    for a, b in [("&lt;", "<"), ("&gt;", ">"), ("&quot;", '"'), ("&#39;", "'"),
                 ("&apos;", "'"), ("&amp;", "&")]:
        s = s.replace(a, b)
    return s
def lines(s):                          # str::lines: \n split, one trailing \r stripped
    p = s.split("\n") if s else []
    if p and p[-1] == "": p = p[:-1]
    return [x[:-1] if x.endswith("\r") else x for x in p]
def non_latin(l):                      # is_non_latin_noise
    cs = [c for c in l if c not in WS]
    bad = [c for c in cs if not ((c.isascii() and c.isalnum()) or c in PUNCT)]
    return len(cs) > 0 and 2 * len(bad) > len(cs)
def runs(l):                           # split_whitespace
    out, cur = [], ""
    for c in l:
        if c in WS:
            if cur: out.append(cur); cur = ""
        else: cur += c
    return out + ([cur] if cur else [])
def v2(md): return "\n".join(l for l in lines(unescape(md)) if not non_latin(l))
def v3(md): return "\n".join(l for l in lines(v2(md)) if not any(len(r) > 64 for r in runs(l)))
```

Where it approximates: `char::is_whitespace` is spelled out as the Unicode `White_Space` set, and
Python's `str.isspace` disagrees with it on no character in the fixtures (checked); no fixture has
a bare `\r`, where Rust's `lines()` edge cases would matter. The mirror's sort order matches the
order both logs walked. The token cap is not mirrored.

## Also in the data

- **Observed: most `nationality` misses are words the normalizer does not resolve.** 40 of 65
  reviewed `nationality` fields miss; 34 of those 40 are an alphabetic answer of four or more
  letters, among them `ROMANIAN`, `German`, `INDIAN` (twice), `CHINESE`, `PAKISTANI` and `Polish`.
  `issuing_country` shows the country-name form of the same gap: `REPUBLIC OF KOREA` (twice),
  `TURKEY` (twice), `CZECH REPUBLIC`, `UNITED KINGDOM OF GREAT BRITAIN AND NORTHERN IRELAND`.
  `normalize::country_code`'s demonym table was admitted on the 72-fixture corpus, one observed
  entry at a time; the 118-fixture log is new evidence for the same accept rule, and
  `vocab_replay` can score candidates against this log without a model run. Which of these
  `mrz::code_for_name` already resolves under another spelling is not checked here.
- **Observed: Germany 2018's `issuing_country` misses on representation, not reading.** Truth is
  the MRZ's `D`, the answer `DEU`: the same kind of disagreement as the `PP` / `P`
  `document_type` misses recorded in
  [`normalize-country-demonyms-2026-09-05.md`](normalize-country-demonyms-2026-09-05.md).

## What this does not claim

- **Not that v3 improves extraction.** On the 84 fixtures both arms scored it moved nothing. Its
  gain is that fixture 85 onward now runs, and that no fixture in this corpus overflows.
- **Not that the harness is deterministic everywhere.** One pair of runs on one runner image. The
  CPU model is not logged, and `llama.cpp` picks kernels by CPU feature; a different host could
  decode differently. A second pair, or two runs of one head, would extend the bound.
- **Not a Tier-2 escalation accuracy.** Every fixture has a checksum-valid MRZ; production runs
  Tier 2 only when Tier 1 fails (see `parity.rs`, "A known bias in this corpus").
- **Not a timing result.** The v3 run spent 1 485 s on fixtures 1–84 against v2's 1 261 s (+18%)
  while 81 of those prompts were identical, so the difference is the runner.
- **Not the token counts.** 2 257 → 1 496 against a budget of 1 548 is the engineer's offline
  measurement, and "the cap engages on no fixture" rests on it.

## Rejected on the way

- **Reading v3 against the whole v2 run.** v2 has no 118-fixture total; comparing 50.8% to v2's
  53.8% over 1–84 would credit v3 with the tail's composition. Paired on 1–84 instead.
- **Reading v3 against 2026-09-05.** A 7.8-point "drop" on reviewed and a 2.8-point "rise" on
  derived, neither of which is a change: see Comparability.
- **Using wall time as evidence the shorter prompts are cheaper.** Identical prompts ran 18% slower
  in the v3 job.
- **Counting dropped lines by hand from the Romania image.** The mirror found four more changed
  prompts that nobody had named, two of which carry the only two moved values.
