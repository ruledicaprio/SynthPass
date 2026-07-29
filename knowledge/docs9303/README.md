# docs9303/

ICAO Doc 9303 (*Machine Readable Travel Documents*, Eighth Edition, 2021),
Parts 1–13, converted to clean Markdown. This is the source of truth behind
`crates/mrz` and the M7 `synthpass-die` provider work, and supersedes the
informal, partial ICAO references that used to be scattered across
`crates/mrz/README.md` and the constitution's §10 "MRZ Standards" — those
stay as the project's *policy* on MRZ handling; this tree is the *spec* they
implement.

## Contents

| File | Part | Relevance to SynthPass |
|---|---|---|
| `Doc_9303_Part1_Introduction.md` | 1 — Introduction | Glossary and acronyms; §5.1 is a one-paragraph map of all 13 parts and the canonical cross-reference hub. |
| `Doc_9303_Part2_Security_of_MRTDs.md` | 2 — Security of the Design, Manufacture and Issuance of MRTDs | Physical/procedural document security and machine-assisted authentication (Appendix C). Not implemented in code; background for document-authenticity claims. |
| `Doc_9303_Part3_Specs_Common_to_all_MRTDs.md` | 3 — Specifications Common to all MRTDs | The check-digit algorithm (§4.9, 7-3-1 weighting) — `crates/mrz/src/checksum.rs`. OCR-B character set, ICAO/ISO 3166-1 country codes — `crates/mrz/src/countries.rs`. |
| `Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md` | 4 — MRPs and other TD3 Size MRTDs | The TD3 (passport) format — `crates/mrz/src/parser.rs::parse_td3`, `emit.rs::format_td3`. |
| `Doc_9303_Part5_Specs_for_TD1_MROTDs.md` | 5 — TD1 Size MROTDs | The TD1 (ID card, 3-line MRZ) format — `parse_td1` / `format_td1`. |
| `Doc_9303_Part6_Specs_for_TD2_MROTDs.md` | 6 — TD2 Size MROTDs | The TD2 (2-line MRZ) format — `parse_td2` / `format_td2`. |
| `Doc_9303_Part7_Machine_Readable_Visas_MRVs.md` | 7 — Machine Readable Visas | MRV-A/MRV-B visa formats — `crates/mrz`'s MRV-A/B parse and emit support. |
| `Doc_9303_Part8_Emergency_Travel_Documents.md` | 8 — Emergency Travel Documents | Guidance material, not a normative format; informs which document types `synthpass-gen` does and doesn't need to cover. |
| `Doc_9303_Part9_Biometric_identification_and_Electronic_Data_storage_in_MRTDs.md` | 9 — Biometric Identification and Electronic Data Storage | eMRTD chip inspection flow. Relevant to future vision/chip-data providers in `synthpass-die`, not implemented yet. |
| `Doc_9303_Part10_LDS_for_Storage_of_Biometrics_and_Other_Data_in_the_Contactless_IC.md` | 10 — Logical Data Structure (LDS) | DG1–DG16 data group layout for the contactless chip. Relevant to future electronic-chip-data support; no code depends on it yet. |
| `Doc_9303_Part11_Security_Mechanisms_for_MRTDs.md` | 11 — Security Mechanisms for MRTDs | BAC, PACE, Chip/Terminal/Active/Passive Authentication. Future PKI and chip-authentication work. |
| `Doc_9303_Part12_Public_Key_Infrastructure_for_MRTDs.md` | 12 — Public Key Infrastructure for MRTDs | CSCA/Document Signer certificate chains. Future Passive Authentication verification. |
| `Doc_9303_Part13_Visible_Digital_Seals.md` | 13 — Visible Digital Seals | Barcode-based digital seal structure, used in Part 7's and Part 8's worked examples. Per `knowledge/ROADMAP.md`, barcode documents are their own project, not folded into the MRZ roadmap — kept here for reference regardless. |

## Figures

`figures/` holds diagrams rendered from the source PDFs, prefixed with the
filename of the part they came from and the PDF page they were rendered
from: `<PartFileStem>_p<PageNumber>.png` (matching the naming convention in
`../papers/figures/`, adapted since these are full-page renders rather than
individually cropped images — several figures on the same PDF page share one
file). Wired into the Markdown as `<img src="./figures/...">` with real alt
text, immediately under the figure's caption; the caption's own bracketed
`[Diagram: ...]` description (where present) is left in place below the
image rather than removed, since it was independently verified against the
source PDF and is what a text-only reader gets.

Only the highest-value diagrams were extracted — MRZ zone-layout and
dimensional diagrams (Parts 4–7, the geometric basis for `crates/mrz`'s field
offsets), Part 2's Appendix C authentication-architecture diagrams (which had
been OCR'd into unreadable prose before this tree was cleaned up), the Chip
Inside symbol (Part 9), and the Digital Seal structure diagrams (Part 13).
Many bracketed `[Diagram: ...]` placeholders remain un-illustrated by design
— the text description was judged sufficient, or the diagram is decorative
(e.g. Part 2 Appendix C's GUI screenshot mock-ups, already described in
detail in prose). Extract more if a specific diagram turns out to matter for
the redesign; don't do it speculatively.

## Note on provenance

Converted from the official ICAO consolidated PDFs (Eighth Edition, 2021),
one per part. The raw conversion carried real damage that was cleaned up
here: three incompatible TOC/table styles were unified onto the cleanest one
found in the corpus (matching the original Parts 10–13); Part 2 had
duplicated front matter, PDF page header/footer text bled into the body, two
appendices missing their heading markup entirely, and diagrams OCR'd into
unreadable run-on prose; Part 9 opened with a leftover LLM chat preamble;
Part 13 had an editorial note embedded in its own TOC; several files were
also found, only after a cleanup script bug, to be missing real content at
the very end (a page-counting mistake, not a research one — caught by
diffing every file's ending against its source PDF rather than trusting the
script). Every part genuinely ends with `-- END --` in the official ICAO
text; that surprised the first pass at this cleanup enough to be worth
saying plainly here.

Every `Doc 9303-N` mention in the corpus is now a real Markdown link to that
part's file — 247 of them. The source documents cite each other only at
whole-part granularity in prose (`Doc 9303-11`, `see Part 4`); they do not
cite specific section numbers across parts, so there was nothing finer to
link at that level. Headings do carry their section number in the heading
text itself (e.g. `## 4.2.2 Document Number`), so a GitHub-style anchor is
always derivable — `Doc_9303_Part4_....md#422-document-number` — for anyone
who wants to cite a specific section precisely, the way `crates/mrz/src/*.rs`
already does in prose (`ICAO 9303 part 4 §4.2.2.2`). Nothing wires those
Rust doc comments to real links yet; that's a natural follow-up once this
tree has settled, not done here.

Part 8's Visible Digital Seal byte-dump example (Appendix B) is short about
13 of the 134 bytes the source PDF states it should have — a two-column hex
layout that doesn't extract into a reliable byte order from text alone.
Documented in place with a comment rather than guessed at.

## What does not belong here

PDFs. The point of this tree is that it is greppable, diffable text a model
can read in context. The source PDFs live outside the repository at
`pdf_docs9303/` on the machine this was converted on — convert on the way
in, don't commit the PDF itself.
