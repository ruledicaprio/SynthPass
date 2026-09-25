# LIBRARY.md — the knowledge tree, and its private twin

A card catalog: what lives where, in this repository and in the private `ocr-b` repository, and
why each piece is public or private. `README.md` is the entry point for a new reader (start
there); this file is the map for someone about to file a new standard, a new benchmark note, or a
new provider write-up and unsure where it goes.

## Why a separate file

`README.md` already curates "read these five first" and the current/forward-looking document
list — it is the *reading order*. This file is the *inventory*: every directory, its file count,
and its one-line purpose, plus the cross-repository half of the picture `README.md` has no reason
to carry (a public repo's README should not be the place a reader learns what a private repo
holds). Put the two together only if `README.md`'s own length stops being "an hour of reading" —
it is not there yet.

## `knowledge/` inventory

Fifteen subdirectories, all under `knowledge/`, plus the sixteen root-level living documents
`README.md` already indexes under "Current & forward-looking" (not repeated here). File counts
are top-level files (a `figures/`, `incoming/` or similar subfolder is called out separately, not
folded into the count).

| Directory | Files (top-level) | Purpose, per its own README |
| --- | --- | --- |
| [`decisions/`](decisions/) | 21 ADRs + README | Architecture Decision Records — why, not what. Never deleted, only superseded. |
| [`benchmarks/`](benchmarks/) | 44 dated `.md` reports + `FINDINGS.md`, `PIPELINE.md`, README, 10 raw-data files (`real-specimen-mrz-baseline.json`, `real-specimen-outcomes.jsonl` and 8 older sweep-output JSONs), plus 2 sweep-output subfolders (`ocr-order-ab/`, `mrz-matrix-probe-2026-09-21-run/`) | Methodology, metric definitions, dated sweep results including rejected candidates. The only place in the repo carrying live accuracy numbers. |
| [`docs9303/`](docs9303/) | 13 ICAO Doc 9303 Parts + `CONFORMANCE_BASIS.md`, `Mrz_Field_Layout.md`, README, plus a `figures/` subfolder | ICAO Doc 9303, Parts 1–13, the spec behind `crates/mrz` and `synthpass-die`. Kept as reference; audited against ICAO's own PDFs 2026-09-24. |
| [`papers/`](papers/) | 7 source papers + README, plus a `figures/` subfolder | Source academic material (six open-set-recognition papers, one end-to-end OCR paper), kept as raw reference. Distillation happens in `research/`, not here. |
| [`ecma/`](ecma/) | 5 ECMA transcriptions + README | The freely-published Ecma OCR standards (ECMA-11, 15, 18, 21, 30) behind the MRZ typeface, transcribed in full — public, unlike their ISO twins. |
| [`ocrb/`](ocrb/) | 8 distilled-fact notes + README | OCR-B glyph geometry, print quality, pitch and spacing — cited facts drawn from ISO 1073-2 / ISO 1831 / ISO 2033 (via the private `ocr-b` repo) and from ECMA, tied to SynthPass code and measurements. The public face of `ocr-b`. |
| [`archive/`](archive/) | 7 superseded documents + README | Historical record — reasoning that led to the current state, not a description of it. Never rewritten once archived. |
| [`research/`](research/) | 4 distilled notes + README | Actionable summaries drawn from `papers/`, filtered by "can this become a trait, struct, algorithm, pipeline stage, benchmark, configuration knob, or heuristic?" |
| [`ocr/`](ocr/) | 1 note + README | Recognition, preprocessing, layout analysis, confidence scoring — the Tier-1 input path. |
| [`img/`](img/) | 13 assets, no README before this pass | Generated benchmark-chart SVGs and a handful of hand-taken screenshots, embedded by other documents. |
| [`evaluation/`](evaluation/) | README only | How we decide something is good enough to ship — gate definitions and their rationale, not yet holding a dedicated note beyond the README's own worked example (the M4 CI gate). |
| [`hardware/`](hardware/) | README only | Memory budgets, CPU-only and consumer-GPU targets, deployment shapes — a living scope statement; no dedicated measurement note filed yet. |
| [`prompts/`](prompts/) | README only | Prompt philosophy and per-version evaluation results. The prompts themselves are compiled into `crates/synthpass-llm/prompts/`, not here. |
| [`providers/`](providers/) | README only | One note per intelligence provider (capability, measured behaviour, weights licence) — the template exists; no per-provider note has been filed yet. |
| [`vision/`](vision/) | README only | Vision-language models and the multimodal track. Explicitly "not implemented" as of v1.4.0 — the README states the target shape, not a shipped capability. |

Five of the fifteen (`evaluation/`, `hardware/`, `prompts/`, `providers/`, `vision/`) are
README-only by design, not by neglect — each README says so itself (see `providers/README.md`'s
"what does not belong here" and `vision/README.md`'s "Status: not implemented"). A README-only
directory is not a gap to fill speculatively; principle 6 (`benchmarks/README.md`) rules out
writing a note before there is a measurement or a shipped capability behind it.

## The private twin: `D:\Projects\ocr-b`

**`ocr-b` (`ruledicaprio/ocr-b`) is a separate, private repository — read-only from here, never
edited as part of this repo's work.** It holds the ISO/ECMA source library: the original PDF
scans, faithful Markdown transcriptions made by reading page images (not by OCR-ing the PDF's own
text layer, which is either absent or unusable for most of these), the OCR-B font files measured
against, and the measurement tools. ISO material is copyrighted and **never reproduced here** —
what crosses into this public repo is distilled facts, paraphrased, cited by clause and page, in
[`knowledge/ocrb/`](ocrb/README.md).

### `ocr-b/standards/` — what is transcribed there today

Per `ocr-b/README.md`'s own "Transcriptions" table (2026-09-24):

| Standard | Owner | Transcription status in `ocr-b` | Public distillation |
| --- | --- | --- | --- |
| ISO 1073-2:1976 (OCR-B shapes) | © ISO | Complete (44 pages, vision transcription) | Partially distilled in `knowledge/ocrb/glyph-dimensions.md`, `filler-and-symbols.md`, `line-and-pitch.md` — body pp. 1-21 read in full, Annexes A-C skimmed |
| ISO 1831:1980 (print quality) | © ISO | Complete (46 pages; the PDF's own OCR layer is unusable) | Partially distilled in `knowledge/ocrb/print-quality-iso1831.md` — §1-§6 and Annexes B/D read |
| ISO 2033:1983 (MICR/OCR coding) | © ISO | Complete (13 pages) | Partially distilled in `knowledge/ocrb/code-positions.md` — Table 8 (p. 8) read |
| ECMA-11 3rd ed. (OCR-B alphanumeric set) | © Ecma, free to download | Complete (40 pages) | **Fully public**, `knowledge/ecma/ECMA-11_OCR-B_Alphanumeric_Character_Set_1976.md` |
| ECMA-15 (print specs for OCR) | © Ecma | Complete (52 pages) | **Fully public**, `knowledge/ecma/ECMA-15_Printing_Specifications_for_OCR_1968.md` |
| ECMA-18 (printing-line position) | © Ecma | Complete (9 pages) | **Fully public**, `knowledge/ecma/ECMA-18_Printing_Line_Position_on_OCR_Single-Line_Documents_1977.md` |
| ECMA-21 (journal-tape positioning) | © Ecma | Complete (12 pages) | **Fully public**, `knowledge/ecma/ECMA-21_Character_Positioning_on_OCR_Journal_Tape_1969.md` |
| ECMA-30 (OCR-B numeric subsets) | © Ecma | Complete (7 pages) | **Fully public**, `knowledge/ecma/ECMA-30_OCR-B_Sub-Sets_for_Numeric_Applications_1976.md` |
| ECMA-6/-35/-43/-48 (code-set family) | © Ecma | Complete for born-digital editions and the 1974 ECMA-43 scan; the 1973/1985 ECMA-6 and 1984/1986 ECMA-48 scans not yet transcribed | Not mirrored publicly — cited only for the character-repertoire question in `knowledge/ocrb/code-positions.md` |
| ECMA-19 (MICR/OCR coding, ISO 2033's ancestor) | © Ecma | Not yet transcribed (PDF only) | Not distilled — `knowledge/ocrb/README.md`'s Sources table records only "history and scope read" |

**Correction made in this pass.** `knowledge/ocrb/README.md` previously stated ECMA-11, ISO
1073-2, ECMA-15, ECMA-18 and ECMA-21 were "in progress" full transcriptions. `ocr-b/README.md`'s
own table says all five (plus ISO 1831, ISO 2033 and ECMA-30) are complete. Fixed in this pass —
see the report for the diff.

### Why the ECMA family is public and the ISO family is not

ISO sells its standards; ECMA gives its old editions away free (each edition says so on its own
cover: "available free of charge"), but they are still copyrighted, so `ocr-b/README.md` treats
them like ISO for redistribution purposes — **the free-download fact licenses reading, not
republishing.** `knowledge/ecma/` nonetheless carries full ECMA transcriptions publicly, on the
judgment already recorded in `knowledge/ecma/README.md`'s "Provenance" section: Ecma's own
publication choice ("available free of charge") is treated as licence enough for a faithful,
attributed transcription, the way a freely-published RFC or W3C spec would be. That judgment
applies to ECMA-11/15/18/21/30 specifically; it is not a blanket rule for every future ECMA
standard, and a maintainer decision should be sought before mirroring a new one at full length
(this is `ocr-b/README.md`'s own hedge: "these old editions carry no redistribution licence, so
treat them like ISO unless the maintainer decides otherwise").

**ISO material never crosses at full length, regardless of how it was obtained.** `knowledge/ocrb/`
is the only public artifact ISO's OCR-B/OCR-print/MRZ-coding family produces here: paraphrased
facts, each with a clause-and-page citation and a verification tag ([R]/[M]/[T]/[P]/[H], defined
in `knowledge/ocrb/README.md`).

### Landing zones for what the maintainer supplies next

See [`knowledge/ocrb/README.md`](ocrb/README.md#standards-awaiting-source) for the per-standard
distillation template and the standards genuinely awaiting a first pass (ISO/IEC 7501-1, ISO/IEC
30116, ISO/IEC 7810, ISO/IEC 18013-1 — none yet held in `ocr-b`, per its own
`standards/incoming/INVENTORY.md`).

## Public-vs-private, in one paragraph

If it is a *fact drawn from* a standard — a dimension, a tolerance, a governance date — with a
clause citation and a tie to SynthPass code or a measurement, it belongs in `knowledge/ocrb/`
(ISO-sourced) or `knowledge/ecma/` (Ecma-sourced, transcribed in full). If it is the standard
*itself* — a scan, a page-faithful transcription, an outline-coordinate table, a reproduced
figure — it belongs only in the private `ocr-b` repository, and never crosses into this repo for
the ISO-owned documents. `knowledge/docs9303/` follows the same split one level up: ICAO's Doc
9303 is converted to Markdown *in full* and kept public, because ICAO publishes it without ISO's
paywall and Doc 9303 is this project's own normative spec (see
[ADR-0003](decisions/ADR-0003-docs9303-source-of-truth.md)) — a different provenance and a
different licensing posture from the ISO/ECMA font-and-print family this file maps, not an
inconsistency with it.
