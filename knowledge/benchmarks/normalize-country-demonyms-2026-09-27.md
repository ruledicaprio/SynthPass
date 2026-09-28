# Country demonyms and alternate names, round two: 28 forms recover 38 fields, zero regressions

**Date:** 2026-09-27 · **MAIN:** `1988bbc` (branch `claude/539-normalizer-demonyms`) · **DATA:** none (fixtures unchanged; source is a CI log, not `samples/`) · **Evidence:** Observed (CI `native-llm` run 36342651928 on `1988bbc`, the real parity harness) and Derived (the same captured CI log re-scored by `vocab_replay` first; it predicted the run exactly) · **Status:** current

**19 demonym entries and 9 alternate-name entries recover 38 Tier-2 fields: 385/744 → 423/744,
51.7% → 56.9%, zero regressions.** No prompt change, no `PROMPT_VERSION` bump, no model run —
the same `vocab_replay` technique the 2026-09-05 table was proved with, replayed against a newer
log (issue #539).

## Observed in CI

The real parity harness, not a replay: `ci.yml` workflow_dispatch run
[36342651928](https://github.com/ruledicaprio/SynthPass/actions/runs/36342651928), `native-llm` job,
on this branch at `1988bbc`, with prompt v3, the same 118 fixtures and vocabulary `8feb315a58cdae3e`
(v3's was `b6bd1f9a5fdd108e`).

| Population | prompt v3 (run 36323412121) | this branch (run 36342651928) |
| :-- | --: | --: |
| Reviewed fixtures, all nine fields | 297 / 585 = 50.8% | **335 / 585 = 57.3%** |
| `issuing_country` (reviewed) | 41 / 65 | **53 / 65** |
| `nationality` (reviewed) | 25 / 65 | **51 / 65** |
| Derived fixtures, check-digited fields | 88 / 159 = 55.3% | 88 / 159 = 55.3% |
| Overall | 385 / 744 = 51.7% | **423 / 744 = 56.9%** |
| Legacy seven-field rate (reviewed) | 219 / 455 = 48.1% | 245 / 455 = 53.8% |

- **The replay's projection held exactly:** +38 / −0 overall, `issuing_country` +12,
  `nationality` +26.
- **No other field moved.** Every other per-field line in the two job logs is identical.

## Where the evidence came from

Issue #539 flagged the prompt-v3 parity record
([`parity-prompt-v3-2026-09-27.md`](parity-prompt-v3-2026-09-27.md), CI run 36323412121): 40 of 65
reviewed `nationality` fields miss, most of them alphabetic demonyms (`ROMANIAN`, `INDIAN`,
`PAKISTANI`...); `issuing_country` shows the country-*name* form of the same gap
(`REPUBLIC OF KOREA`, `TURKEY`, `CZECH REPUBLIC`, the UK's full official name).

The CI job log (744 scored `actual=` lines, `normalizer vocabulary: b6bd1f9a5fdd108e   log-format:
1   prompt: qwen2.5-fields v3   n_ctx: 2048   fixtures: 118`) was stripped of its GitHub Actions
timestamp prefix (`sed -E 's/^[^ ]+ //'`) so `vocab_replay` could parse the
`--- [n/m] <stem> ... ---` / `  field  expected=... actual=... OK|MISMATCH` shape it expects — the
raw Actions log carries an extra `<timestamp>Z ` before every line, which the parser has never
seen before and correctly does not recognize.

```
before: 385/744 (51.7%)
```

matches this directory's live headline for the same run (see `README.md`'s `Tier-2 per-field
exact match, 118-fixture parity corpus` row), confirming the stripped log replays byte-for-byte
the same as CI scored it.

## What was admitted, and where

Mid-task, the brief for this table changed: alternate *names* of a country — not the adjective
describing a person's nationality — now belong in `mrz`'s own `code_for_name`, not in
`synthpass-core`'s `DEMONYMS`. Every candidate below was re-sorted under that rule and admitted
into whichever table it actually is.

**`synthpass-core::normalize::DEMONYMS`** (19 entries; `crates/synthpass-core/src/normalize.rs`)
— adjectives describing a person's nationality:

| token | code | flips |
| :-- | :-- | --: |
| `AFGHAN` | AFG | 1 |
| `ANGOLANA` | AGO | 1 |
| `BANGLADESHI` | BGD | 1 |
| `CHINESE` | CHN | 1 |
| `COLOMBIANA` | COL | 1 |
| `DOMINICANA` | DOM | 2 |
| `GERMAN` | D | 1 |
| `GHANAIAN` | GHA | 1 |
| `INDIAN` | IND | 3 (two case variants) |
| `MAURITANIENNE` | MRT | 1 |
| `NEPALESE` | NPL | 1 |
| `NIGERIAN` | NGA | 1 |
| `PAKISTANI` | PAK | 1 |
| `POLISH` | POL | 1 |
| `ROMANIAN` | ROU | 1 |
| `RUSSIAN` | RUS | 1 |
| `SOMALI` | SOM | 1 |
| `SWISS` | CHE | 1 |
| `TURKISH` | TUR | 1 |

**`mrz::countries::ALTERNATE_NAMES`** (9 entries, whole-string, checked by `code_for_name` after
`CODES` itself declines; `crates/mrz/src/countries.rs`) — genuine alternate **names**: a former
official English name, a UN long/short form, or the same name with `CODES`'s diacritic dropped:

| name | code | flips |
| :-- | :-- | --: |
| `Turkiye` | TUR | 2 (accent-dropped `Türkiye`) |
| `Turkey` | TUR | 2 (pre-2022 English name) |
| `Republic of Turkey` | TUR | 1 |
| `Republic of Korea` | KOR | 4 (two documents × two fields) |
| `Czech Republic` | CZE | 2 (pre-2016 official name) |
| `United Kingdom of Great Britain and Northern Ireland` | GBR | 1 |
| `Islamic Republic of Afghanistan` | AFG | 1 |
| `Azerbaycan` | AZE | 2 (Turkish/Azeri spelling) |
| `Polska` | POL | 1 (Polish endonym) |

19 + 9 = 28 entries admitted. The per-field breakdown `vocab_replay` reports is the authoritative
count: **issuing_country +12 / -0, nationality +26 / -0, total +38 / -0.** (The per-entry "flips"
column above is attributed separately, by re-running the replay with each candidate present vs.
absent; some fixtures score the same string under both `nationality` and `issuing_country`, which
is why the per-entry column and the tool's own per-field totals don't sum the same way.)

### The rule that moved `CZECH` out of `DEMONYMS`

`CZECH` was first admitted as a `DEMONYMS` token — the same move that let the 2026-09-05 table
extract `SLOVAK` from the observed `"SLOVAK REPUBLIC"` phrase. But the corpus here never prints a
bare `CZECH`: the only observed form is the full former official name, `"CZECH REPUBLIC"`. Once
alternate country *names* got their own home in `mrz`, `"Czech Republic"` resolves there directly
— a name a name table can hold, unlike an adjective — and the `DEMONYMS` token became redundant
for the one form actually observed. Removed from `DEMONYMS`, added to `mrz::ALTERNATE_NAMES`
instead.

### Why token-based demonym matching cannot cover the multi-word official names

Before the scope changed, this document tried resolving `"REPUBLIC OF KOREA"`,
`"UNITED KINGDOM OF GREAT BRITAIN AND NORTHERN IRELAND"`, `"Republic of Turkey"`, and
`"Islamic Republic of Afghanistan"` by adding a single distinguishing token
(`KOREA`, `BRITAIN`, `TURKEY`) to `DEMONYMS` and letting `code_from_demonym_tokens`'s whole-token
splitting find it inside the longer phrase — the same mechanism that already resolves
`"SLOVENSK? REPUBLIKA / SLOVAK REPUBLIC/"` via its lone `SLOVAK` token.

It mostly worked (`KOREA`, `TURKEY` both flipped their misses this way), but
**`"United Kingdom of Great Britain and Northern Ireland"` did not**, and the reason is the exact
safety net this table depends on: the phrase's tokens are `UNITED, KINGDOM, OF, GREAT, BRITAIN,
AND, NORTHERN, IRELAND`, and `IRELAND` is itself a real country name
(`mrz::code_for_name("Ireland") == Some("IRL")`). `code_from_demonym_tokens`'s conflict check
correctly refuses to resolve a string whose tokens name two different countries — `GBR` from
`BRITAIN`, `IRL` from `IRELAND` — exactly as it should for `"CANADIAN FRANCE"`. There is no way to
special-case this within the token mechanism without weakening the conflict check for everyone.

Also: `"Islamic Republic of Afghanistan"` could never resolve this way even without the `IRELAND`
collision, because `AFGHANISTAN` is *already* the exact `mrz::country_name` for `AFG`, and
`code_from_demonym_tokens` deliberately returns `None` for a lone plain-country-name token with no
demonym present (see that function's doc comment) — adding `AFGHANISTAN` itself as a `DEMONYMS`
entry would fail `demonyms_are_unique_and_do_not_shadow_the_country_name_table`, rightly, since it
is dead weight the moment `mrz` already resolves it.

Both problems disappear once the whole phrase is a **whole-string** entry in `mrz::ALTERNATE_NAMES`
instead of a token inside `DEMONYMS`: `code_for_name` compares the entire trimmed field against
`"United Kingdom of Great Britain and Northern Ireland"` and `"Islamic Republic of Afghanistan"`
directly, with no token splitting and therefore no `IRELAND`/`AFGHANISTAN` collision to trip over.
This is the concrete case for keeping "names" and "demonyms" in two tables rather than one: the
resolution *mechanism* legitimately differs, not just the taxonomy.

## Rejected candidates

| candidate | why rejected |
| :-- | :-- |
| `SOMALILAND` / `SOMALILANDER` (expected `RSL`) | `RSL` is not in `mrz::CODES` at all — Somaliland has no ICAO/ISO code this workspace recognizes. Adding a `DEMONYMS` or `ALTERNATE_NAMES` entry for it would fail the shadow test's `country_name(code).is_some()` check. Fixing this needs a `mrz` code addition, a different and larger change than a name/demonym synonym — out of scope here. |
| `TURKAYA`, `CYPRIO`, `ESPANOEA`, `TISH`, `CHIN`, `D.JI`, `SLOVENSKA REPUBLIK.A` | OCR corruption, not a real word in any language. Approximate matching is the "different, riskier mechanism" both country-demonym notes have already declined. |
| `HR`, `FR` (expected `HRV`/`FRA`) | ISO 3166-1 **alpha-2** codes, not names or demonyms — a different code system this table's mechanism does not touch. |
| `DEU` (expected `D`) | Not a demonym gap: `DEU` is Germany's *own* ISO alpha-3 code, already fully resolved by `country_code`'s `looks_like_a_code` fast path — the miss is `code_for_name`'s Germany alias returning the primary `DEU` where this one fixture's ground truth is the legacy MRZ `D`. A code-alias policy question, not a vocabulary gap; flagged, not fixed, here. |
| `Swedish`, `IDBEL`, `Meurowonuires pobenk`, `ID`, `B`, `ENG`, `IITOPIA` | Either a demonym for the *wrong* country relative to that fixture's expected code (model error, not a normalizer gap) or outright garbage. Neither is fixable by vocabulary. |
| `I.C./TUR` (expected `TUR`) | Contains a valid ISO code (`TUR`) embedded in punctuation-joined noise, not a name or demonym token. Would need a mechanism that recognizes an embedded already-valid code inside a string — a different feature than this table. |

## What a replay cannot tell you

Unchanged from the 2026-09-05 note: nothing about a prompt change, and nothing about a value a
normalizer already resolved *wrongly* (only values that passed through unresolved replay
faithfully). See that note's "What a replay cannot tell you" section for the full reasoning —
still true here, same harness, same caveats.

## Confirmed by `vocab_replay`, this branch

```
replayed 744 comparison(s)

before: 385/744 (51.7%)
after:  423/744 (56.9%)   +38

per field (gained / lost):
  issuing_country  +12 / -0
  nationality      +26 / -0
```

Zero losses. Every one of the 28 admitted entries independently flips at least one miss to a hit
— confirmed by adding/removing each one and re-running the replay, mirroring the "admit one
observed form at a time" discipline the 2026-09-05 table was built under.

## Vocabulary fingerprint

`synthpass_core::normalize::vocabulary_fingerprint()` now also hashes
`mrz::alternate_names()` alongside `mrz::codes()`, `DEMONYMS`, `MONTH_NAMES`, `SEX_FORMS` and
`DOCUMENT_TYPE_FORMS` — the same reasoning as the 2026-09-05 note's stale-binary guard: a change to
either country-name table is exactly as capable of silently moving parity numbers as a change to
`DEMONYMS` itself.
