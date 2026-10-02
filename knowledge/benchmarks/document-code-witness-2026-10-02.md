# Document code on real specimens: 15 of 46 printed letter-filler codes are read with a letter in the filler cell, 8 of them in accepted hits; 20 of 21 printed two-letter codes are read as two letters

**Date:** 2026-10-02 · **MAIN:** `dc8288d` (#677's merge; measured at its pre-merge head `ca01153`, the head of [#677](https://github.com/ruledicaprio/SynthPass/pull/677): `origin/main` `78dd666` plus the archive's `truth.code_cells` and `archive_query.py codes`; the tracked tree was clean, the run header's `working_tree_dirty: true` is the untracked files only) · **DATA:** the public corpus, `samples-data` `65a5040` (the baseline's pinned `samples_data_sha`, synced with `scripts/sync-samples.ps1 -DataRef`), 261 documents, `samples/corpus.jsonl` sha256 `00218001…` · **Evidence:** Observed on public real specimens, default arms, one local release `provider-bench --real-specimens --mrz-only --progress` run (win11-i7-1255U, `RTEN_NUM_THREADS=8`, no `--dump-ocr-passes`, no `--include-private`, no `--include-local`), run id `ab80f5069392`, 15:21:20Z to 15:52:38Z, exit 0 · **Status:** current

**2026-10-02.** This is the real-specimen witness that [#664](https://github.com/ruledicaprio/SynthPass/issues/664)
asks for before any `P<` repair is proposed: for every labelled real document, the printed class of the
document code, the observed class, the pass that produced the read, and whether the code was read exactly. The
synthetic glyph atlas ([#660](https://github.com/ruledicaprio/SynthPass/pull/660)) is calibration evidence only;
this is the real measurement. No OCR zone text, line or field value appears here: classes, counts and public asset
ids only. No code, ADR, baseline or default changes with this note.

**How the classes are read.** The printed class is `truth.code_cells`, the classes of the first two cells of the
hand-transcribed zone's line 1 (`A` letter, `<` filler): `A<` is a letter then a filler (`P<`, `I<`, `V<`), `AA` is
two letters (`PS`, `PO`, `ID`). Only the 67 of 261 documents that have a hand-transcribed zone carry it; the other
194 are skipped and counted. The observed class is the first two cells of the Tier-1 read's line 1 after repair
(`tier1_read`), whether or not the read was accepted; `unread` is no Tier-1 read at all. The code's `exact` /
`wrong` / `unread` is the `document_type` entry of `field_correctness`, which is about the **accepted** read, so a
checksum-failed read is `unread` there even when its class is shown.

## Matrix, per format

| Format | Printed | Observed `A<` | Observed `AA` | Unread | Documents |
| --- | --- | ---: | ---: | ---: | ---: |
| TD3 | `A<` | 27 | 15 | 1 | 43 |
| TD3 | `AA` | 0 | 12 | 0 | 12 |
| TD1 | `A<` | 2 | 0 | 1 | 3 |
| TD1 | `AA` | 0 | 7 | 1 | 8 |
| MRV-B | `AA` | 0 | 1 | 0 | 1 |
| **All** | `A<` | **29** | **15** | **2** | **46** |
| **All** | `AA` | **0** | **20** | **1** | **21** |

The format is the one the ledger row records for the document.

- No printed `AA` was read as `A<`. Every class change is a printed filler read as a letter (15) or no read (3).
- The 15 misreads are all in TD3. Of them, **8 are accepted hits** and 7 are checksum-failed reads (never
  accepted).

## The code, exact / wrong / unread, per printed class

| Printed | `exact` | `wrong` | `unread` |
| --- | ---: | ---: | ---: |
| `A<` (46) | 17 | 8 | 21 |
| `AA` (21) | 8 | 0 | 13 |

`unread` here is "no accepted read": for `A<` it is 12 reads of the right class that failed their checksums (never
accepted), 7 misread-class reads that failed, and 2 documents with no read; for `AA` it is 12 right-class reads that
failed their checksums and 1 with no read. The 8 `wrong` are exactly the 8 accepted hits whose observed class is
`AA` for a printed `A<`.

## Provenance of the 18 class changes

| Read | Documents | `retry_variant_id` | `retry_stop` |
| --- | ---: | --- | --- |
| `A<` read as `AA`, accepted hit | 8 | `general` 4, `pass-01` 2, `pass-04` 1, `pass-05` 1 | `general_valid` 4, `variant_valid` 4 |
| `A<` read as `AA`, checksum failed | 7 | none | `exhausted` 7 |
| unread | 3 | none | `exhausted` 3 |

Half of the accepted wrong codes (4 of 8) come from the general pass. Public asset ids of the 15 misreads: the TD3 specimens named
`Argentina_..._2026_mrz_blur`, `Cetis_Sample_..._2022_mrz_inner_page`, Cyprus 2010, Dominican Republic 2020,
Germany 2024, Ghana 2019, Hong Kong 2007 and 2019, India 2022, Russian Federation 2019, Slovakia 2005, Somaliland
2023, Spain 2015, United Kingdom 2021 and Uzbekistan 2013; the three unread are the Argentina 2021 child page, the
France 2020 ID back and the Italy 2022 ID back.

## The named specimens

| Asset id | Truth-backed | Printed | Observed | Outcome | Variant, stop | `document_type` |
| --- | --- | --- | --- | --- | --- | --- |
| `Slovakia_Passport_Specimen_PS_SVK_2014_mrz.jpg` | yes | `AA` | `AA` | hit | `general`, `general_valid` | exact |
| `Afghanistan_Passport_Specimen_PO_AFG_2016_mrz.webp` | yes | `AA` | `AA` | checksum failed | none, `exhausted` | unread |
| `China_Passport_Specimen_PO_CHN_2012_mrz.png` | no | none | `AA` | hit | `general`, `general_valid` | none |
| `China_Passport_Specimen_PO_CHN_2018_mrz.png` | no | none | `AA` | hit | `pass-00`, `variant_valid` | none |
| `Korea_Democratic_Peoples_Republic_Passport_Specimen_PO_PRK_2005_mrz.jpg` | no | none | `AA` | hit | `general`, `general_valid` | none |

The genuine `PS` and the three `PO` passports that [#658](https://github.com/ruledicaprio/SynthPass/pull/658) renamed
(Afghanistan 2016, China 2012, China 2018) were all read, and so was the older Korea 2005 `PO`: all five are
observed as two letters. **Only two of the five are truth-backed**: the China and Korea passports have no
hand-transcribed fixture, so they have no printed class and are outside the matrix above. #664's requirement that
the matrix include all three `PO` specimens is therefore met for one of them; the other two need a fixture
before they can count.

## What this does not show

- One run of the default arm: no repeat, so the run-to-run spread of a budget-sensitive document is unknown (the
  three unread documents are all `exhausted`, and the France 2020 ID back among them is one the corpus is known
  to leave budget-limited). A candidate arm is compared with the two-run form
  (`archive_query.py codes RUN RUN2`) later.
- 67 labelled documents, 43 of them `A<` TD3; formats other than TD3 have 1 to 11 each, so a per-format rate for
  TD1 or MRV-B is not supported.
- The printed class says two letters or a letter and a filler, not which letters: nothing here separates `PS`
  from `PO` or `P<` from `I<`.
- It does not say why a filler cell reads as a letter, only where: no recommendation and no repair follow from it.
