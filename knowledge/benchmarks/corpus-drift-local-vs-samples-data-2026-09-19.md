# Local corpus drift against `samples-data`

**Date:** 2026-09-19 · **MAIN:** `82087f4` · **DATA:** `396b22f` (was `5988c7b`) · **Evidence:** Observed (file-level comparison + sha256) · **Status:** current

The main checkout's `samples/` had drifted **20 files** from `origin/samples-data`. Every
local-versus-CI benchmark discrepancy that had accumulated as an open question turns out to be this
drift, and both resolve to exact arithmetic rather than to anything in the extraction path.

Nothing was regressed. The local corpus simply was not the corpus CI measures.

## The arithmetic

```
local files                 263
  minus 6 non-images        257   <- what a local run loaded
samples-data images         261   <- what the committed CI baseline measured
```

Two questions close on this:

- **`documents` 261 (CI) vs 257 (local).** Six files under `samples/passports/` were not images at
  all — a stray PDF and five ground-truth `.json`/`.md` sidecars sitting beside the images instead
  of in `samples/ocr_fixtures/` — and the walker skips them. The remainder is the missing
  `Singapore_Passport_Specimen_PA_SGP_2017_mrz.jpg`, absent locally, plus name-only differences.
- **`no_mrz_found` 3 (CI) vs 4 (local).** `San_Marino_ID_Specimen_2017_back` is `_no_mrz` on
  `samples-data` and was still `_mrz` locally. CI scored a correct refusal out of the denominator;
  the local run scored it as a detection miss. This had been carried as an unexplained regression.

## What had drifted

| Class | Files | Effect |
| :--- | ---: | :--- |
| Stale names (byte-identical) | 3 | Misclassified: San Marino `_mrz`→`_no_mrz`, UAE 2018 `_no_mrz`→`_redacted_mrz`, Vietnam 2022 `_no_mrz`→`_redacted_mrz_blur` |
| Absent locally | 6 | Not measured at all, incl. one whole document (Singapore 2017) |
| Stray non-images | 6 | Skipped by the walker; inflated the local file count |
| Orphans of an unfinished rename | 2 | `China …P0_CHN_2012_mrz.webp`, `Serbia …SRB_2009_mrz.jpg` — in neither the manifest nor upstream |

The three renames were verified **byte-identical** before being applied, and all 261 files were
sha256-compared against `samples-data` afterwards: **0 mismatches**.

The two orphans were quarantined rather than deleted — and the subsequent fast-forward of the local
`samples-data` branch deleted exactly those two upstream, independently confirming they were
residue of the unfinished corpus-wide rename and not unpushed work.

## The mechanism that spread it

[#340](https://github.com/ruledicaprio/SynthPass/pull/340) and
[#345](https://github.com/ruledicaprio/SynthPass/pull/345) give every new worktree the corpus
automatically, via `git-wt step copy-ignored --require-include`. That copies **from the main
checkout**.

So the guarantee it provides is *"this worktree matches the main checkout"* — **not** *"this
worktree matches the reference corpus"*. Those are different claims, and here they differed by 20
files:

```
2026-09-14   local San Marino file last touched
2026-09-17   samples-data relabels it _no_mrz        <- main checkout goes stale
2026-09-18   #340 merges; every new worktree now inherits the stale copy
```

The copy mechanism worked exactly as designed and faithfully propagated drift. **A worktree having
its assets is not the same as having the right ones**, and the existing rule — *state the N, and
check it against the full set* — catches the first but not the second. The stronger check is to
compare the corpus against `samples-data` itself, which is what the arithmetic above does.

## Moldova PA 2014 — reclassified

Found while investigating the fourth `no_mrz_found`. The strip printed along the bottom edge of
`Moldova_Passport_Specimen_PA_MDA_2014` is **not an ICAO zone**: line 1 does not begin with a
document code, neither line carries a character in any check-digit position, and line 2 spells an
English phrase where the holder and document fields belong. It is novelty artwork on a template.
Its band scored **0.18**, the lowest in that bucket — consistent with a detector shown decorative
text.

It is also **not the same document as its `_wide` sibling**: different holder, different date of
birth, different photograph. The two had been treated as one document framed two ways, which made
the sibling's clean validation look like evidence that the narrow crop was a framing failure. It
was not evidence about that file at all.

Renamed `_no_mrz` on `samples-data` (`396b22f`), following San Marino 2017 (`5988c7b`): one rename,
bytes unchanged, nothing deleted. Returning no zone for a document that has none is a correct
refusal, not a detection miss — the same class the
[2026-09-09 denominator correction](denominator-correction-2026-09-09.md) closed for MRZ-less
fronts.

This reverses a maintainer judgement recorded in `corpus.jsonl` on 2026-09-04. That judgement —
that the page is a genuine older Moldovan passport format — is a claim about the printed layout,
not about the strip, and may well hold. Only the strip is reclassified.

**Provenance follow-up, not acted on here:** an image carrying joke text in place of an MRZ is
unlikely to be an official issuing-authority specimen. Where the file came from is worth checking
on the corpus-provenance track.

## What this does not change

Per-document results are unaffected: the six M6 `checksum_failed` attributions were derived from
files identical in both corpora, and the
[field-attribution comparison](checksum-failed-miss-mechanisms-2026-09-18.md) stands on all six.
What was provisional was every **aggregate** measured locally, because its denominator was a
different population.

**No MRZ content, holder value or zone text is transcribed here or in any commit message**, on
either branch.
