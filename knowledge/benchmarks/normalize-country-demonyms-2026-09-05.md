# Country demonyms, and a replay harness that gates vocabulary, 2026-09-05

**Six table entries recover 10 Tier-2 fields: 170/324 → 180/324, 52.5% → 55.6%,
zero regressions.** No prompt change, no `PROMPT_VERSION` bump, no model run.

The more durable half is the tool that proved it:
`crates/synthpass-bench/examples/vocab_replay.rs`.

## What was measured first

After the date normalizer ([normalize-date-forms-2026-09-04.md](normalize-date-forms-2026-09-04.md))
the obvious follow-up was "is any other normalizer missing vocabulary?" Classifying the
remaining 154 misses per field answered it, and **refuted two of the three candidates**:

| field | misses | vocabulary helps? |
| :-- | --: | :-- |
| `sex` | 13 | **No.** 6 are `M` where truth is `F`, 2 are `None`, 5 are OCR garbage (`N?`, `HIIIEU`, `OTTANA CAN`). Not one foreign-language word. |
| `document_type` | 6 | **Barely.** Five are `expected=PP actual=P` — the fixture holds the 2-char MRZ code and the normalizer emits the single letter. A representation disagreement, not a missing word. |
| `nationality` + `issuing_country` | 17 | **Yes.** |

Worth keeping from the refuted half: **6 of 13 `sex` misses are the model answering `M` where
the truth is `F`.** That is a directional bias, not a vocabulary gap, and no normalizer can fix
it.

## The gap

`normalize::country_code` defers to `mrz::code_for_name`, which is an exact case-insensitive
match over 235 country **names** (`crates/mrz/src/countries.rs:323`). It holds `Canada`,
`Spain`, `Croatia` — never `CANADIAN`, `ESPAÑOLA`, `HRVATSKO`.

Documents print the demonym, usually bilingually, and every one fell through unresolved.

## Result

```
replayed 324 comparison(s)
before: 170/324 (52.5%)
after:  180/324 (55.6%)   +10

per field (gained / lost):
  issuing_country  +3 / -0
  nationality      +7 / -0
```

Six entries do it: `CANADIAN`, `CANADIENNE`, `HRVATSKO`, `SERBIAN`, `SLOVAK`, `ESPANOLA`.

Resolution is **whole-token over the whole string**, mirroring `month_from_named_tokens`:
`CANADIAN/CANADIENNE` is two tokens naming one country and resolves; a string whose tokens
name two *different* countries returns `None` rather than picking one. That is what lets
`SLOVENSK? REPUBLIKA / SLOVAK REPUBLIC/` resolve on its one clean token.

The table lives in `synthpass-core::normalize`, **not** in `mrz`, and is consulted only after
`mrz::code_for_name` declines. `mrz` is a published crate and `code_for_name("Canadian")`
returning `CAN` would be wrong for it: an MRZ carries codes, never demonyms.

## The accept rule, and why it makes a word list safe

`vocab_replay` re-scores a finished parity log against the current normalizers. It works
because the harness normalizes before comparing and `normalize` passes anything it cannot
resolve through **verbatim** — so every mismatch line carries the model's own string, and a
change confined to normalization can be scored without the model.

**A candidate ships only if it flips ≥1 miss→hit and 0 hit→miss.** A single hit→miss is a
blocker and exits non-zero — mirroring the texture A/B, and for the reason recorded there: an
aggregate that nets a gain against an unnoticed loss is a failure this project has already hit.

The rule has a property that matters more than the 10 fields:

> **An entry no document exercises flips nothing, so nothing supports shipping it.**

That is what makes the harness safe to point at a *generated* word list. Hallucinated
vocabulary cannot get in, because acceptance requires evidence from a real document, whatever
produced the candidate.

### What a replay cannot tell you

- **Nothing about a prompt change.** Different prompt, different answers; every recorded string
  is stale. Any edit to the prompt, grammar, or sampling invalidates a replay completely.
- **Nothing about a value the normalizer already resolved *wrongly*.** The log records the
  normalized value, so a raw answer that was converted into the wrong thing is gone. Only
  values that passed through unresolved replay faithfully — which is exactly the population a
  vocabulary change addresses, and exactly why this technique suits this class of change and no
  other.

The technique has been checked against reality once: before PR #206 it predicted 14 recovered
date fields and the full run produced exactly 14, split +7/+7.

## What was deliberately left out

**The obvious sibling forms.** `CROATIAN`, `SPANISH`, `SLOVAQUE`, `SRPSKO` are all surely
correct, and all absent. No specimen here prints them, so nothing measures them. A table that
grows by guessing is one nobody can audit later, and an unresolved string costs a caller less
than a plausible-looking wrong code.

**`SLOVENSKA`.** Slovakia's `SLOVENSKÁ` and Slovenia's `SLOVENSKA` differ by exactly the accent
OCR discards, so the two collapse onto one token that would name the wrong country half the
time. The corpus already contains the cousin of that failure — a Slovenian passport where the
model answered `Serbian`. `slovenska_is_deliberately_absent` asserts the omission is a decision
rather than an oversight.

**Fuzzy matching.** Six misses stay unresolved and should stay that way: `CYPRIO`, `ESPANOEA`
and `SLOVENSKA REPUBLIK.A` are OCR corruption, and approximate country matching is a different,
riskier mechanism that belongs behind its own measurement.

## Honest caveats

- **A resolved wrong answer looks more confident than an unresolved one.** On that Slovenian
  document, `Serbian` currently survives as visible raw text; with this table it becomes a
  well-formed `SRB`. The score is unchanged — it was a miss and stays a miss — but the product
  now emits a plausible wrong code where it used to emit obvious garbage. That is an argument
  about confidence handling, not against the table, and it is the reason the entry bar is
  "a document prints it" rather than "it is true".
- **The accept rule optimises against a fixed 72-fixture set.** Run it long enough and it fits
  the test set. Vocabulary is the safest thing to fit — *"is ESPAÑOLA Spanish for Spanish"* is
  checkable by a person, unlike a tuned threshold — but the hazard is real, and the MRZ-holdout
  arm is the guard.
## Confirmed by a full run, both arms, 2026-09-05

`scripts/measure-parity.sh`, one binary (`26ee6171…`), vocabulary fingerprint
**`74aee114001a9ed2`** printed in both logs.

| arm | before | after | Δ |
| :-- | --: | --: | --: |
| baseline reviewed (9 fields, 18 docs) | 85/162 (52.5%) | **95/162 (58.6%)** | **+10** |
| baseline derived (3 fields, 54 docs) | 85/162 (52.5%) | 85/162 (52.5%) | **0** |
| **baseline overall** | **170/324 (52.5%)** | **180/324 (55.6%)** | **+10** |
| holdout reviewed | 70/162 (43.2%) | **78/162 (48.1%)** | +8 |
| holdout derived | 83/162 (51.2%) | 83/162 (51.2%) | **0** |
| holdout legacy 7-field | 51/126 (40.5%) | **56/126 (44.4%)** | +5 |

**The replay predicted 180/324 and the run produced 180/324** — the same
technique that predicted 14 for PR #206 and got 14. For a change confined to the
normalizers, a sub-second replay is now a trustworthy stand-in for a 35-minute
run. That is the more valuable result here than the ten fields.

**The derived set is the built-in control.** Derived fixtures score only the
three check-digited fields, and neither `nationality` nor `issuing_country` is
among them — so a demonym change *cannot* move that number. It did not, in
either arm. Any movement there would have meant the table was reaching fields it
has no business touching, exactly as #204's four untouched fields proved that
change was a measurement fix rather than a flattering number.

**Both arms moved.** Holdout reviewed is now 48.1% — what the *baseline* was two
days ago. That a demonym helps the escalation case nearly as much as the easy one
is the expected shape: the adjectival form is printed in the visual zone, which
is precisely what survives when the MRZ is stripped.

## The stale-binary incident, and the guard it produced

The first attempt to confirm the 180/324 **measured the wrong code and said nothing about it.**
Recorded here in full, per this directory's "record the rejections" rule, because the failure is
reusable and the fix is now load-bearing.

An ad-hoc runner did this:

```bash
cargo test -p synthpass-llm --test parity --release --no-run   # exited 101
BIN=$(ls -t target/release/deps/parity-*.exe | head -1)        # picked the OLD binary
```

The build had failed with `LNK1104: cannot open parity-9bc350efd6f4cd45.exe` — a previous parity
process still held its own executable open — and the script carried on. Two complete arms ran, 55
minutes, and produced numbers that were real, reproducible, internally consistent, and measured
**pre-demonym code**.

Three properties made it invisible:

1. **The binary filename is identical across builds.** Only the content differs, so nothing in
   any path or log distinguished the stale one. Caught only by diffing two `sha256` files by hand.
2. **The baseline arm reproduced 52.5% exactly**, which read as reassuring determinism rather
   than as the alarm it actually was.
3. **No fingerprint covered it.** `PROMPT_VERSION`/`prompt_digest` is computed as
   `fill_template("", HINT_PREFIX)` — the prompt template with the content empty — so it
   fingerprints what the model is *asked* and nothing about the normalizers applied to what it
   answers. Both accuracy changes this week landed in exactly that uncovered half.

This is the third time on this project that a measurement has silently scored the wrong thing:
the parity harness skipping `normalize::extraction` for ~20 points (PR #204), `MAX_RETRY_VARIANTS`
truncating the variant under test, and now a stale binary.

**The guard:** `normalize::vocabulary_fingerprint()` hashes every table the module dispatches on,
and `parity.rs` prints it in its run header:

```
normalizer vocabulary: 74aee114001a9ed2   prompt: qwen2.5-fields v2   fixtures: 72
```

A run whose fingerprint matches an earlier run used the same vocabulary, whatever the working
tree claims. **Any log without that line predates the guard and cannot prove which code produced
it** — which includes every measurement in this directory written before 2026-09-05.

**The runner:** `scripts/measure-parity.sh` takes the binary path from cargo's own
`--message-format=json` `executable` field rather than a timestamp glob, aborts on a failed
build, and holds a lock so two cargo invocations cannot overlap. That last point is its own
lesson: concurrent cargo against one `target/` directory has produced both `LNK1103`
("debugging information corrupt" in a cached rlib) and `STATUS_DLL_INIT_FAILED` on this project.
Both looked like flakes. Both were contention.

**What the bad run was still good for.** The stale binary was post-#206/pre-demonym — precisely
the state needed to answer whether the date normalizer moved the MRZ-holdout arm. It did, and
both arms moved together; see
[parity-mrz-holdout-2026-09-04.md](parity-mrz-holdout-2026-09-04.md). A void measurement of one
change was a valid measurement of another, which is luck, not method.

## On generating the table with the local LLM

Considered and deferred. The harness is source-agnostic, so wiring a proposal file in later is
not a redesign.

For *this* table the model adds nothing the corpus does not already prove: six validated entries
are hand-writable in minutes, and a 1.5B instruct model is an unreliable multilingual
lexicographer precisely on the uncommon countries where a human would also want to check. Its
only real use is generalising beyond the corpus — and by the accept rule above, those are
exactly the entries no measurement can support.

## The scaling question a review raised, and where the answer already lives

An ultra code review of this work (2026-09-05, after the demonym table shipped) flagged `DEMONYMS`
as a hand-grown, one-entry-per-specimen list with no mechanism to grow it beyond linear human
attention per new corpus country. That is a fair description of the *table*, but not of the
tooling around it: `crates/synthpass-ocr/examples/mine_country_vocab.rs` already exists, was built
alongside this work, and is exactly the missing mechanism — a deterministic, non-LLM candidate
generator over the corpus itself, gated by the same `vocab_replay` accept rule this document
describes above.

It works by exclusivity, not similarity: a token proposed for code `C` must appear on at least two
specimens of `C` and on **no** specimen of any other country, which is what separates `ESPANOLA`
(Spanish documents only) from `PASSPORT` (everywhere) without any edit-distance heuristic that
would also have to explain why `HRVATSKO` and `Croatia` are the same country despite sharing no
characters. It proposes; a person still disposes — exclusivity alone also selects `MADRID → ESP`
and `BUNDESDRUCKEREI → DEU`, which are real, exclusive, and not country names.

**It has not been run against the full corpus yet.** The six entries in `DEMONYMS` today were
found by hand, before this tool existed. The next concrete step is not "invent a scaling
mechanism" — it is: run `mine_country_vocab.rs` over the full corpus, read its candidate list,
and prove whatever looks like a real demonym through `vocab_replay` exactly as the six existing
entries were proved. That is a data-collection task, not a design one.

## Running it: 194 candidates, zero shippable

Ran `mine_country_vocab.rs` over all 168 corpus specimens with a resolvable issuing state
(release build, single pass) → `artifacts/country-vocab-candidates.tsv`, 194 proposals.

**Most are exactly the false positives the tool's own doc comment warns about.** Exclusivity
selects a city or a generic word just as readily as a demonym: `MADRID`, `ANKARA`, `BEOGRAD`,
`BRATISLAVA`, `GATINEAU`, `BEIJING`/`SHANDONG`/`GUANGDONG`, `PYONGYANG`, `LEFKOSIA` are cities;
`ORNEK` (Turkish "specimen"), `PASPOR`/`PASZPORT` (Indonesian/Polish "passport"), `IMZA` (Turkish
"signature"), `NAAM`/`HOUDER`/`AFGIFTE` (Dutch "name"/"holder"/"issuance"), `TARIHI`/
`GECERLILIK`/`KIMLIK` (Turkish "date"/"validity"/"identity"), `RESPUBLIKASI`/`NAZIRLIYI`
(Azerbaijani "republic"/"ministry"), `VENCIMIENTO` (Spanish "expiry") are generic document
vocabulary; `SARAH`, `HANNE`, `GEORGIOS`, `ORKHAN`/`ORXAN`, `VIRTANEN`, `ZINONAS`/`ZHNONAS` are
personal names the corpus happens to concentrate in one country; a cluster of Serbia-only,
`n=2` tokens (`BARSNON`, `BAXKN`, `CPBIJA`, and a dozen more) is single-template OCR corruption,
plus `TEST`/`SRBTEST`/`STRNAME` — placeholder text burned into a synthetic specimen, not language
at all.

The linguistically defensible remainder — a genuine demonym or the country's own name in some
language, not a city/org/generic noun — comes to 23 tokens across 16 countries: `RUSSIAN` (RUS),
`CZECH` (CZE), `INDIAN` (IND), `BRITISH` (GBR), `DANSK`/`DANISH`/`DANMARK`/`DANEMARK`/`DANOISE`
(DNK), `FINLANDE`/`SUOMI` (FIN), `MACEDONIA`/`MACEDOINE`/`MACEDONIAN` (MKD), `CYPRIOT` (CYP),
`EGYPTIAN` (EGY), `ELLINAS`/`HELLAS`/`HELLENIC` (GRC), `FRANCAISE` (FRA), `BOSNA`/`HERZEGOVINA`
(BIH), `ISLANDE` (ISL), `KOSOVAR` (RKS), `NEDERLANDEN`/`NEDERLANDSE` (NLD), `POLSKA`/
`RZECZPOSPOLITA` (POL), and `SRPSKO` (SRB) — the exact sibling form this document's own "what
was deliberately left out" section named as absent two days ago, because no specimen printed it
at the time.

**Staged all 23 in `DEMONYMS` and replayed both confirmed post-#208 logs (fingerprint
`74aee114001a9ed2`): 0/324 change, both arms.** Not a near-miss — none of the 23 tokens appear
anywhere in either log's text, because the *reviewed* fixture set behind these logs only carries
ground truth for 11 countries/entities (Canada, Cetis, China, Croatia, Cyprus, Estonia, Serbia,
Slovakia, Slovenia, Spain, UAE), and on the three where a candidate's country does overlap
(`SRB`, `CYP`, `ARE`), the model's actual recorded miss is different text entirely: the Cyprus
fixture's nationality miss is the OCR-corrupted `CYPRIO`, not `CYPRIOT`; the Serbia fixture's
misses are unrelated garbage strings, not `SRPSKO`; the UAE fixture has no country-field miss to
flip at all. This is the same "no document exercises it" case the six existing entries were
already gated by — a candidate this pass found in the *mining* corpus (168 specimens, OCR only)
is simply invisible to the *measurement* corpus (18 reviewed fixtures) most of the time, because
the two pools barely overlap. **Nothing shipped from this pass.** The mining tool and the accept
rule both did their job: they refused 23 plausible-looking entries for lack of evidence rather
than admit them on vibes.

**Byproduct: a real, unrelated `vocab_replay` bug, found only because this pass replayed the
holdout log before touching `DEMONYMS`.** `parity.rs`'s `fields_match` folds every field through
`.trim().to_uppercase()`, but `renormalized()` skipped that fold for `document_number`/`surname`
(the two fields with no crate normalizer), so a case-only difference (`"Martin"` vs `"MARTIN"`)
replayed as a false regression with zero vocabulary change involved. Fixed in
`fix/parity-tooling-hardening`, independent of this document's own findings.

**The actual next step to grow `DEMONYMS` further** is not another mining pass — it is
broadening the *reviewed* fixture set (the 18 hand-labeled documents that score `nationality`/
`issuing_country` at all) to carry ground truth for the countries this pass found real candidates
for: Russia, Czechia, India, the UK, Denmark, Finland, North Macedonia, Egypt, Greece, France,
Bosnia and Herzegovina, Iceland, Kosovo, the Netherlands, Poland. That is fixture-authoring work
— finding and reviewing real ground truth for those specimens — not a tooling gap; the tooling
already proved it can't do anything useful without that ground truth to check candidates against.
