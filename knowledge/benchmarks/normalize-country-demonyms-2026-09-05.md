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
- **Not confirmed by a full run yet** at the time of writing. The replay is a prediction until
  a real parity run agrees with it; expected 180/324.

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
