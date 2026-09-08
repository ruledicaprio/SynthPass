# Real-specimen `checksum_failed` misses, categorised — 2026-09-08

`checksum_failed` is the largest real-specimen Tier-1 miss category, and the
[2026-08-16 weak-spot finding](README.md#2026-08-16--first-genuine-real-specimen-tier-1-numbers)
framed it as the clean signal: *"OCR found MRZ-shaped text and got at least one
character wrong inside it."* This note pulls the raw OCR text for every one of
those misses — the follow-up that finding asked for — and the framing does not
hold. **`checksum_failed` on the current corpus is not a usable OCR-accuracy
signal.** Most of it is two other things.

## Method

- `provider-bench --real-specimens --dump-ocr` (the full pre-parse OCR dump added
  in [#236](https://github.com/ruledicaprio/SynthPass/pull/236)), `mrz` provider,
  whole corpus, no `--limit`.
- Repo `320ef00`; `samples/corpus.jsonl` sha256 `e86a8646…`, 236 rows.
- The `llm` provider pass was not run to completion (its CPU inference dominates
  runtime and adds nothing here — `checksum_failed` is the deterministic provider's
  verdict). The 71 `mrz`-provider dump blocks are in
  `artifacts/checksum-dump/mrz-pass.log`.
- Each of the 71 blocks was read by hand; a handful were re-checked by feeding the
  OCR's own cleanest MRZ lines back through `mrz::find_and_parse`.

## Run totals

| | count | share of 236 |
|---|---|---|
| Tier-1 HIT | 118 | 50.0% |
| MISS `checksum_failed` | 71 | 30.1% |
| MISS `no_mrz_found` | 47 | 19.9% |

## The 71 `checksum_failed` misses break into three populations

| population | count | what it actually is |
|---|---:|---|
| **A — no MRZ exists** (`*_no_mrz` specimens) | 19 | The document has no machine-readable zone. OCR read VIZ text / card labels / the guilloché as two `[A-Z0-9<]` lines ≥ 28 chars, and `mrz::find_and_parse` accepted them. |
| **B — MRZ deliberately redacted** (`*_redacted_mrz`) | 25 | The MRZ is blacked out, X-ed out, or scrambled in the specimen itself. The information is not in the image. |
| **C — MRZ present, no ground truth** (`*_mrz`) | 27 | A real MRZ is on the page. But **no specimen in this set has verified ground truth**, and spot-checks show several carry deliberately non-conforming MRZs (template / "SPECIMEN" / "ÖRNEK" / "VZOR" data with wrong check digits by design). |

Only population C could contain a genuine OCR misread, and it cannot currently be
told apart from a non-conforming specimen.

### A — `find_and_parse` accepts VIZ text as an MRZ (19 of 71)

The "recovered MRZ zone" for these is card boilerplate:

```
Czechia_Passport_..._no_mrz     ->  Td2   "CESKAREPUBLIKACZECHREPUBLIC<<<" / "CE5KAREPU811KACZECHREPUBLIC<<<"
Algeria_Passport_..._no_mrz     ->  Td2   "CEPASSERORTCONTIENT28PAGES..."  / "THISPASSP0RTC0NTA1N528PA6E5..."
Croatia_BorderPass_..._no_mrz   ->  MrvB  "VRSTADOZVOLEITYPEOFPERMIT..."   / "POGRANICNAPROPU5N1CA323002100..."
Liechtenstein_ID_..._no_mrz     ->  MrvB  "VORNAMEINIFIRSTNAMEISI..."      / "GEBURTSDATUMDATE0F81RTH..."
Luxembourg_ID_..._no_mrz        ->  Td2   "CHIEFOFPROTOCO1SIGNATURE..."    / "M1N15TERE0E5AFFAIRES..."
```

`mrz::find_and_parse` requires two lines of the MRZ character set at a plausible
width and nothing more — no gate on line 1 carrying a valid document code, on the
issuing-country field being a code in the table, or on the date fields being
digits. So a "document number" that is all letters and a "date of birth" that is
`REPUBL` parse fine and then fail their check digits. Every one of these 19
documents *should* be `no_mrz_found`.

This is also the one item here with a **correctness** stake, not just a metric one:
a hallucinated MRZ that happened to satisfy its check digits would be a silent
wrong extraction, which is worse than any miss.

### B — redacted MRZ (25 of 71)

`Armenia_..._redacted_mrz` OCRs as `XXXXXXXXXXXXXXXXXX`. `Albania_..._redacted_mrz`
reads the redaction bar as `AASFLKJAFLAKGSDF…`. A few (`Australia`, `Nepal …
blur_rotated`, `Russia 2016`) keep a readable name line but the number/date zone is
gone. Nothing to recover — the pixels don't carry it. These should be excluded from
the accuracy denominator or tracked as their own outcome, not scored as OCR misses.

### C — real MRZ, unverifiable (27 of 71)

All 27 have a real MRZ and ≥ 2 MRZ-shaped OCR lines. But feeding the OCR's own
cleanest lines back through `mrz::find_and_parse` still fails for the ones checked
(UK, Poland, Korea ×1, Türkiye ×1, Mauritania) — e.g. the UK specimen's
consistently-read `8248122645GBR8912126M3107054<<<<<<<<<<<<<<00` has a
document-number check digit of `5` where the 7-3-1 rule wants `3`. These are
`…TEMPLATE<<FAKE…` / `…ORNEK…` documents whose printed MRZ is not conformant.

Genuine OCR/parser problems *are* visible in this group, they just can't be counted
without labels:

- **character confusion** — `0↔O` (`Ghana`: `H00000014` vs `HOODOD14`), `S↔5`
  (`India 2013`: `L5733700` read `LS733700`), `2↔7` (`India boxed`: leading `2`
  read `7`), `M↔N` (`Ghana` sex `M`→`N`).
- **line-2 left-truncation** — `Afghanistan` drops the leading `44` of the document
  number, shifting every field left.
- **candidate selection picks a garbled duplicate line-1 for line 2** — `UK`,
  `Poland`, `Korea 2022`, `Türkiye 2025`, `Russia 2019`, `Mauritania`: the recovered
  zone is two line-1 reads, or a mangled line-1 in the line-2 slot, while a cleaner
  line-2 sits elsewhere in the same OCR output. Matches the ROADMAP note that
  line-1 prefix-shift repair is *"hit-rate-safe but not complete."*
- **personal-number-only near-miss** — `Germany D00 2018` fails **only** the
  optional-data check digit; `Colombia 2026`, `Czechia 2005`, `Romania 2024` fail
  `personal_number` + `composite` and nothing else. One digit wrong in the 14-char
  optional field.

## Ranked next steps

1. **Gate MRZ acceptance on line-1 structure** (`crates/mrz`, `find_and_parse` /
   the per-format detectors + `damaged_pass`). Reject a candidate whose line 1 has
   no valid document code, whose issuing-country field is not a code in the table,
   or whose date fields are non-numeric. Measurable: ~19 documents move
   `checksum_failed → no_mrz_found`, and the silent-wrong-extraction risk shrinks.
   Must re-run the full real corpus to confirm zero HIT regression (the 118 HITs
   have structurally valid MRZs, so a structural gate should not touch them) — this
   is the gate to watch. Related open item: `UnrecognizedIssuingCountry`
   (`ROADMAP.md`).
2. **Sub-classify `checksum_failed` in the bench.** Split "line-1 structurally
   invalid" (population A) from "MRZ structurally valid, check digits fail"
   (populations B + C) in `MissReason::ChecksumFailed`. Cheap; makes the metric
   honest going forward and lets the trend chart show the split. Also add a
   `redacted` outcome (or filename-driven exclusion) so population B stops inflating
   the miss count.
3. **Grow ground truth for population C** (`samples/ocr_fixtures/*.json` or at least
   `expected_document_number` in `corpus.jsonl` for the 27 `*_mrz` names in
   `artifacts/checksum-dump/cf-names.txt`). Until these have labels, an OCR misread
   and a non-conforming specimen are indistinguishable, and no `mrz` character-level
   fix can be measured. This is the `ROADMAP.md` "grow labelled ground truth" item
   with a concrete priority list.
4. **Then** the character-confusion / line-selection `mrz` fixes (wider
   `CONFUSABLES` / wiring `solve_substitution` into the checksum-invalid path;
   line-2 left-anchor repair; candidate ranking that prefers a valid line-1+line-2
   pair over two line-1s). Each pinned by a regression test built from the specimen
   that motivated it — once (3) makes "motivated" mean something.

`blindspot_seq` (sequence-level check-digit blind spot,
[2026-08-05 note](checksum-blindspots-measured-2026-08-05.md)) is unrelated and
stays deferred — it is about mismatches that *pass* the checksum, the opposite end.
