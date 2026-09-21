# Corpus coverage — comprehensive world passport-check backlog

Tracks Tier-1 MRZ corpus coverage against every ISO/ICAO country/entity code in
`crates/mrz/src/countries.rs` (238 codes). This is the concrete backlog behind the "wider
real-world corpus is the natural next accuracy milestone" note in
`knowledge/ARCHITECTURE.md` §8 — grown one individually-vetted specimen at a time, per the
checklist in `CONTRIBUTING.md`. **PRADO (`consilium.europa.eu/prado`) is never a source
here** — its copyright notice prohibits harvesting/redistributing its material outside
official, non-commercial use; it's consulted only as a manual human reference, never
scraped or stored. Which sources *are* allowed, and the review gates an agent-found candidate
passes, are in [`SPECIMEN_SOURCES.md`](SPECIMEN_SOURCES.md).

Tier-1 MRZ checksum/parsing logic itself is ICAO-9303-generic and not special-cased per
country — a HIT here reflects real-world OCR/format validation on an actual specimen,
not new per-country code. `scripts/watch-samples.ps1` + `synthpass-ocr`'s `check_sample`
example give an instant first-pass check when a new candidate specimen is dropped into
`samples/` — see CONTRIBUTING.md.

**Specimen images are not tracked in git** — they live on the orphan `samples-data` branch (see
`samples/README.md`), so a per-country row below reflects a contributor's locally-verified corpus
at the time it was checked, not something a fresh clone can reproduce without first running
`./scripts/sync-samples.ps1`. What *does* survive a clone is `samples/corpus.jsonl`, the tracked
manifest: it records every image's ICAO document code, issuing state, provenance and
ground-truth link, so the shape of the corpus is reviewable even when the pictures are absent. This repo already treats prose-recorded verification
as legitimate without the backing artifact being committed (`knowledge/benchmarks/README.md`'s
"a constant with no measurement behind it does not ship" principle is the same idea, applied
here to corpus coverage instead of a benchmark number).

## Summary

| Status | Countries |
|---|---|
| HIT (checksum-valid real specimen, `mrz_corpus.rs` and/or the 2026-08-17 real-OCR scan) | 76 |
| MISS (checksum failed or no MRZ found, real-OCR scan, 2026-08-17) | 7 |
| Known MISS (documented, e.g. physically redacted specimen) | 10 |
| Candidate specimen rejected per the vetting checklist | 1 |
| No legal source found (searched, dated) | 0 |
| Not applicable by definition (no document can carry the code) | 1 |
| No specimen yet | 143 |
| **Total tracked codes** | **238** |

**What "covered" means.** A code is covered when it appears *in its proper field* of a
checksum-valid specimen. For states and organisations that is the issuing-state field (line 1).
For the nationality-only codes — `GBD`, `GBN`, `GBO`, `GBP`, `GBS`, `XXA`, `XXB`, `XXC`, `XXX` — it
is the nationality field, since a document records these about its holder, never as its issuer.
`IAO` appears on no document at all (ICAO uses it only to sign a master list), so it is *not
applicable by definition*. It stays in the table so the denominator is visibly every code.

*No legal source found* is a searched status, recorded with the date of the search. It is
distinct from *No specimen yet*, which means nobody has looked. What counts as a legal source is
in [`SPECIMEN_SOURCES.md`](SPECIMEN_SOURCES.md).

Grown substantially 2026-08-17: contributor additions plus a full real-OCR pass over
`samples/passports/` and `samples/id_cards/` (`integrity_survey.rs --mrz-only`, added this
session — see `knowledge/benchmarks/README.md`'s dated finding) replaced most of the
previous "specimen present, not yet wired" placeholders with a real, measured HIT/MISS
status. `Passport`/`ID card`/etc. counts inside a single country's `Note` column (e.g.
"HIT (x2 specimens) (+ 1 checksum-failed)") are per-specimen, not per-country — see each
row for the exact breakdown.

**2026-09-08 — the 23 `checksum_failed` specimens are now labelled.** The
[checksum_failed root-cause track](benchmarks/checksum-failed-real-specimens-2026-09-08.md)
hand-transcribed the true printed MRZ for every real specimen whose zone lands in
`checksum_failed`, into `samples/ocr_fixtures/<stem>.json` (hand-verified fixtures: 18 → 41).
Of the 23: **7 carry a checksum-valid printed MRZ** (Afghanistan `P0_AFG_2016`, Belgium ID
2021, Croatia ID 2021, Czechia `P0_CZE_2005`, Romania `PE_ROU_2024`, Russia `P0_RUS_2019`,
Sweden ID 2022 — a `checksum_failed` on these is an OCR error, not a bad specimen, and each
keeps its `expected_document_number`), **16 are non-conforming by design** and carry a
`ground_truth_stem` but no `expected_document_number`. The per-country rows below still show
the HIT/MISS status; this changes what a MISS on those rows *means*, not the count.

**2026-09-08 — Nicaragua added.** Two passport specimens (`P0_NIC_2001`, `P0_NIC_2015`)
ingested via manifest regeneration; both read checksum-valid. NIC moves from "No specimen
yet" to HIT (157 codes still uncovered).

**2026-09-10 — 15 specimens ingested for the ADR-0008 orientation track.** Real-specimen
CI baseline `documents` 238 → 254, `tier1_hits` 118 → 128. Six new codes: **DJI, NGA, SOM,
UZB → HIT**; **DOM, PAK → MISS (`no_mrz_found`** — DOM is a low-signal scan, PAK is
photographed sideways; both are chunk-2 targets). **IND and IDN flip MISS → HIT** on the new
2023 / 2024-rotated books. AGO, AZE, BGD already HIT (new specimens don't change the row).
KEN's front page is in the corpus but carries no MRZ — a bio-page book specimen is still
needed. 151 codes still uncovered.

**2026-09-12 — ADR-0008 chunk 2 read fifteen more specimens.** The orientation fix
([`orientation-fix-2026-09-12.md`](benchmarks/orientation-fix-2026-09-12.md)) took the CI gate's
`tier1_hits` 128 → 143 with nothing lost. **DOM, KWT, NPL, OMN, PAK and VNM flip MISS → HIT**, and
CAN, FIN, IND, MCO and PRT gain specimens that had found no MRZ. OMN's and VNM's "stale, not yet
root-caused" rows were the page-orientation vote ADR-0008 chunk 1 attributed. **RUS moves from
"No specimen yet" to HIT**: that row was wrong, and Russian passport specimens were already in the
corpus. 150 codes still uncovered.

**2026-09-14 — cohort c01 (scout loop) added three specimens.** LVA and SMR each get a first
specimen, LTU its first HIT. **LTU flips No specimen yet → HIT**: a checksum-valid TD3 passport
(`Lithuania_Passport_Specimen_P0_LTU_2019_mrz.jpg`, CC BY-SA, self-published Commons upload).
LVA and SMR stay *No specimen yet* — both new images are ID-card sides carrying no
checksum-valid MRZ (an ID-card front and a blank-template back respectively), the same pattern
as Kenya's front-only passport row above. Full provenance and review reasoning:
`work/scouting/c01/packet-c01.md`. 149 codes still uncovered.

**2026-09-14 — cohorts c03/c07/c09 (scout loop) added eight specimens.** `D` moves *No specimen
yet* → **MISS (checksum failed)**: two `D<<` passports (2018, already in corpus; 2024, added
here) both fail checksum on this OCR pass, not yet a HIT. **SGP flips No specimen yet → HIT**: a
checksum-valid TD3 passport (`Singapore_Passport_Specimen_PA_SGP_2017_mrz.jpg`, attributed to the
Immigration & Checkpoints Authority), confirmed by both `check_sample` and the manifest's own OCR
pass — the first clean HIT this loop produced outside Germany's partial case. `HKG`, `PHL` and
`ZAF` each get a first specimen but stay *No specimen yet*: HKG's two passports both fail
checksum (same single-character-misread pattern as `D`), PHL's PhilSys card and ZAF's DHA smart
ID both carry no MRZ zone at all by design. Full provenance and review reasoning:
`work/scouting/c03/packet-c03.md`, `work/scouting/c07/packet-c07.md`,
`work/scouting/c09/packet-c09.md`. 145 codes still uncovered.

## Full table

| Code | Country/Entity | Document type(s) | Status | Note |
|---|---|---|---|---|
| DZA | Algeria | Passport | No specimen yet | The corpus specimen carries no MRZ -- OCR recovered VIZ boilerplate text, not a zone (see [`checksum-failed-real-specimens-2026-09-08.md`](benchmarks/checksum-failed-real-specimens-2026-09-08.md)); not a coverage HIT, same pattern as Kenya's front-only row above |
| AGO | Angola | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| BEN | Benin | Passport (cover, 2 series) | No specimen yet | Covers only in `samples/covers/` (c13, 2026-09-15, Commons CC-BY-SA 4.0 / CC0): 2018 and 2026 series; no data page, never a coverage claim (ADR-0012) |
| BWA | Botswana | -- | No specimen yet | -- |
| BFA | Burkina Faso | Passport (cover, 2 series) | No specimen yet | Covers only in `samples/covers/` (c13, 2026-09-15): the 2018 ECOWAS-era book from police.gov.bf (none-stated licence, kept public on the user's call) and the 2025 AES book from Commons (CC-BY-SA 4.0); no data page, never a coverage claim (ADR-0012) |
| BDI | Burundi | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c13, 2026-09-15, Commons CC-BY-SA 4.0); no data page, never a coverage claim (ADR-0012) |
| CPV | Cabo Verde | Passport (cover, 2 series) | No specimen yet | Covers only in `samples/covers/` (c13, 2026-09-15, Commons CC-BY-SA 4.0 / CC0): 2015 and 2012 series; no data page, never a coverage claim (ADR-0012) |
| CMR | Cameroon | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c13, 2026-09-15, passcam.cm -- the DGSN enrolment portal run by AUGENTIC PassCam, none-stated licence, kept public on the user's call); no data page, never a coverage claim (ADR-0012) |
| CAF | Central African Republic | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c13, 2026-09-15, Commons public domain); no data page, never a coverage claim (ADR-0012) |
| TCD | Chad | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c13, 2026-09-15, Commons CC-BY-SA 3.0, very low-res); no data page, never a coverage claim (ADR-0012) |
| COM | Comoros | -- | No specimen yet | -- |
| COG | Congo | -- | No specimen yet | -- |
| COD | Congo (Democratic Republic of the) | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c13, 2026-09-15, Commons public domain under the DRC official-acts exemption; the Commons page names PRADO as its copy path -- kept on the user's explicit call, not a precedent for H1); no data page, never a coverage claim (ADR-0012) |
| CIV | Côte d'Ivoire | Passport (cover, 2 series) | No specimen yet | Covers only in `samples/covers/` (c13, 2026-09-15, Commons CC-BY-SA 4.0): 2014 (open-flat shot) and 2018 (low-res); no data page, never a coverage claim (ADR-0012) |
| DJI | Djibouti | Passport | HIT (x2) | Ingested 2026-09-10; both books read checksum-valid |
| EGY | Egypt | Passport | HIT (x2) (+ 2 redacted) | Real-specimen CI gate, 2026-09-13: the 2017 and 2022 specimens read; the 2012 and an undated specimen have their zones masked by the publisher (2012 renamed `_redacted_mrz` 2026-09-13) |
| GNQ | Equatorial Guinea | -- | No specimen yet | -- |
| ERI | Eritrea | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c13, 2026-09-15, Commons CC-BY-SA 3.0 / GFDL); no data page, never a coverage claim (ADR-0012) |
| SWZ | Eswatini | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c13, 2026-09-15, Commons CC-BY-SA 3.0); no data page, never a coverage claim (ADR-0012) |
| ETH | Ethiopia | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c13, 2026-09-15, Commons public domain, credited to the Government of Ethiopia); no data page, never a coverage claim (ADR-0012) |
| GAB | Gabon | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c13, 2026-09-15, Commons CC-BY-SA 4.0); no data page, never a coverage claim (ADR-0012) |
| GMB | Gambia | -- | No specimen yet | -- |
| GHA | Ghana | Passport | MISS (checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| GIN | Guinea | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c13, 2026-09-15, Commons CC-BY-SA 4.0, dark interior/back-cover shot); no data page, never a coverage claim (ADR-0012) |
| GNB | Guinea-Bissau | Passport (cover); cover | No specimen yet | Cover only in `samples/covers/` (c14, 2026-09-16, Commons public domain); no data page, never a coverage claim (ADR-0012) |
| KEN | Kenya | Passport (front) | No specimen yet | Front page in corpus 2026-09-10, no MRZ; bio-page book specimen still needed |
| LSO | Lesotho | -- | No specimen yet | -- |
| LBR | Liberia | Passport (cover); cover | No specimen yet | Cover only in `samples/covers/` (c14, 2026-09-16, Commons CC-BY-SA); no data page, never a coverage claim (ADR-0012) |
| LBY | Libya | Passport (cover); cover | No specimen yet | Cover only in `samples/covers/` (c14, 2026-09-16, Commons public domain); no data page, never a coverage claim (ADR-0012) |
| MDG | Madagascar | -- | No specimen yet | -- |
| MWI | Malawi | -- | No specimen yet | -- |
| MLI | Mali | -- | No specimen yet | -- |
| MRT | Mauritania | Passport | MISS (checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| MUS | Mauritius | -- | No specimen yet | -- |
| MAR | Morocco | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| MOZ | Mozambique | Passport (cover); cover | No specimen yet | Cover only in `samples/covers/` (c14, 2026-09-16, Commons CC-BY-SA); no data page, never a coverage claim (ADR-0012) |
| NAM | Namibia | -- | No specimen yet | -- |
| NER | Niger | -- | No specimen yet | -- |
| NGA | Nigeria | Passport | HIT | Ingested 2026-09-10; reads checksum-valid |
| RWA | Rwanda | Passport (cover); cover | No specimen yet | Cover only in `samples/covers/` (c14, 2026-09-16, Commons CC-BY-SA); no data page, never a coverage claim (ADR-0012) |
| STP | Sao Tome and Principe | -- | No specimen yet | -- |
| SEN | Senegal | -- | No specimen yet | -- |
| SYC | Seychelles | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| SLE | Sierra Leone | Passport (cover); cover | No specimen yet | Cover only in `samples/covers/` (c14, 2026-09-16, Commons public domain); no data page, never a coverage claim (ADR-0012) |
| SOM | Somalia | Passport | HIT | Ingested 2026-09-10; reads checksum-valid |
| ZAF | South Africa | ID card (front+back) | No specimen yet | DHA smart ID card front+back added 2026-09-14 (public, public-domain, Commons, author credited as the Department of Home Affairs). Illustrative template card (alphabet-placeholder name, sequential ID number, generic silhouette photo), not a real person. Card carries no printed MRZ (chip + 2D barcode instead) -- not a coverage HIT, same as Kenya's front-only row above. |
| SSD | South Sudan | Passport (cover); cover | No specimen yet | Cover only in `samples/covers/` (c14, 2026-09-16, Commons CC-BY-SA); no data page, never a coverage claim (ADR-0012) |
| SDN | Sudan | Passport | Candidate rejected | No SPECIMEN watermark, read as real personal data -- excluded per vetting checklist |
| TZA | Tanzania | Passport (cover); cover | No specimen yet | Cover only in `samples/covers/` (c14, 2026-09-16, Commons CC-BY-SA); no data page, never a coverage claim (ADR-0012) |
| TGO | Togo | Passport (cover); cover | No specimen yet | Cover only in `samples/covers/` (c14, 2026-09-16, dgdn.gouv.tg, none-stated licence); no data page, never a coverage claim (ADR-0012) |
| TUN | Tunisia | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| UGA | Uganda | Passport (cover); cover | No specimen yet | Cover only in `samples/covers/` (c14, 2026-09-16, Commons CC-BY-SA); no data page, never a coverage claim (ADR-0012) |
| ZMB | Zambia | -- | No specimen yet | -- |
| ZWE | Zimbabwe | -- | No specimen yet | -- |
| ESH | Western Sahara | -- | No specimen yet | -- |
| ATG | Antigua and Barbuda | -- | No specimen yet | -- |
| ARG | Argentina | Passport | HIT (emergency passport) (+ 3 non-conforming) | Real-specimen CI gate, 2026-09-13: the 2015 emergency passport reads; the 2026 pair and the 2021 child passport print zones that fail their own check digits and are scored out as `checksum_failed_specimen` — see [`denominator-bucket-a-2026-09-10.md`](benchmarks/denominator-bucket-a-2026-09-10.md) and [`manifest-review-no-mrz-found-2026-09-13.md`](benchmarks/manifest-review-no-mrz-found-2026-09-13.md) |
| BHS | Bahamas | -- | No specimen yet | -- |
| BRB | Barbados | -- | No specimen yet | -- |
| BLZ | Belize | -- | No specimen yet | -- |
| BOL | Bolivia | -- | No specimen yet | -- |
| BRA | Brazil | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| CAN | Canada | Passport | HIT (x4 specimens) | Contributor-supplied specimens (SPECIMEN watermark); the 2023-issue book, which found no MRZ in the 2026-08-17 real-OCR scan, reads since ADR-0008 chunk 2 (CI gate, 2026-09-11) |
| CHL | Chile | -- | No specimen yet | -- |
| COL | Colombia | Passport | MISS (x2, checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| CRI | Costa Rica | -- | No specimen yet | -- |
| CUB | Cuba | -- | No specimen yet | -- |
| DMA | Dominica | -- | No specimen yet | -- |
| DOM | Dominican Republic | Passport | HIT | Ingested 2026-09-10 as a low-signal `no_mrz_found`; reads since ADR-0008 chunk 2 (CI gate, 2026-09-11) |
| ECU | Ecuador | -- | No specimen yet | -- |
| SLV | El Salvador | -- | No specimen yet | -- |
| GRD | Grenada | -- | No specimen yet | -- |
| GTM | Guatemala | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c12, 2026-09-15, Commons CC0); no data page, never a coverage claim (ADR-0012) |
| GUY | Guyana | -- | No specimen yet | -- |
| HTI | Haiti | -- | No specimen yet | -- |
| HND | Honduras | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c12, 2026-09-15, Commons CC-BY-SA); no data page, never a coverage claim (ADR-0012) |
| JAM | Jamaica | -- | No specimen yet | -- |
| MEX | Mexico | Passport | Known MISS (documented) | The specimen's MRZ zone is physically redacted by the publisher; the 2026-08-17 scan's `checksum failed` label predates the redaction-aware classification fix (see [`denominator-correction-2026-09-09.md`](benchmarks/denominator-correction-2026-09-09.md)) |
| NIC | Nicaragua | Passport | HIT (x2 specimens) | Manifest regeneration, 2026-09-08. Both specimens (2001 and 2015 issues) read checksum-valid (`P<NIC` Td3). |
| PAN | Panama | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c12, 2026-09-15, Commons CC-BY-SA); no data page, never a coverage claim (ADR-0012) |
| PRY | Paraguay | -- | No specimen yet | -- |
| PER | Peru | Passport | Known MISS (documented) | A real specimen is in the corpus; its MRZ zone is physically redacted by the publisher |
| KNA | Saint Kitts and Nevis | -- | No specimen yet | -- |
| LCA | Saint Lucia | -- | No specimen yet | -- |
| VCT | Saint Vincent and the Grenadines | -- | No specimen yet | -- |
| SUR | Suriname | -- | No specimen yet | -- |
| TTO | Trinidad and Tobago | -- | No specimen yet | -- |
| USA | United States of America | Passport, ID card | HIT (x3 specimens) (+ 2 redacted) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`); two further passport specimens have their MRZ zone physically redacted by the publisher |
| URY | Uruguay | Passport | Known MISS (documented) | The specimen's MRZ zone is physically redacted by the publisher; the 2026-08-17 scan's `checksum failed` label predates the redaction-aware classification fix (see [`denominator-correction-2026-09-09.md`](benchmarks/denominator-correction-2026-09-09.md)) |
| VEN | Venezuela | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| AFG | Afghanistan | Passport | MISS (checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| ARM | Armenia | Passport | Known MISS (documented) | A real specimen is in the corpus; its MRZ zone is physically redacted by the publisher |
| AZE | Azerbaijan | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| BHR | Bahrain | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| BGD | Bangladesh | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| BTN | Bhutan | -- | No specimen yet | -- |
| BRN | Brunei Darussalam | -- | No specimen yet | -- |
| KHM | Cambodia | Passport | No specimen yet | Current-series passport bio-page added 2026-09-14 (public, none-stated licence, attributed to the Royal Embassy of Cambodia in Washington D.C.). A real person's document, admitted only because every personal field including the MRZ band is fully redacted (revised H5 standard, 2026-09-14) -- no MRZ present at all, so not a coverage HIT. |
| CHN | China | Passport | HIT (x3 specimens) (+ 2 redacted) | Contributor-supplied specimen (SPECIMEN watermark); corpus grew in the real-OCR scan, 2026-08-17; two further specimens (2014, 2024) have their MRZ zone physically redacted by the publisher |
| CYP | Cyprus | Passport | HIT (x3 specimens) | Contributor-supplied specimens (2010/2020/2026-issue, SPECIMEN watermark); the 2026 one is the first corpus specimen with Cyprus's new "PP" document-type code (effective 15 December 2025) |
| GEO | Georgia | -- | No specimen yet | -- |
| HKG | Hong Kong | Passport (x2 specimens) | MISS (checksum failed) | 2019 + 2007 e-Passport design illustrations added 2026-09-14 (public, attributed to Hong Kong Immigration Department, none-stated licence). Both checksum-failed on this OCR pass (single-character misread pattern) -- not yet a HIT. |
| IND | India | Passport | HIT (3 of 7 specimens; rest non-conforming or redacted) | The 2022, 2023 and 2024 books read checksum-valid — 2022 (was `checksum_failed`) and 2024 (was `no_mrz_found`) since ADR-0008 chunk 2; the other four are the 2013 specimen and its boxed copy, whose printed zones fail their own check digits, and 2 redacted (CI gate, 2026-09-11) |
| IDN | Indonesia | Passport | HIT (1 of 3 specimens) | The 2024 specimen added 2026-09-10 (photographed sideways) reads checksum-valid; the 2011 specimen is non-conforming, the 2023 one redacted |
| IRN | Iran | Passport | Known MISS (documented) | The specimen's MRZ zone is physically redacted by the publisher; the 2026-08-17 scan's `checksum failed` label predates the redaction-aware classification fix (see [`denominator-correction-2026-09-09.md`](benchmarks/denominator-correction-2026-09-09.md)) |
| IRQ | Iraq | Passport | Known MISS (documented) | The specimen's MRZ zone is physically redacted by the publisher; the 2026-08-17 scan's `checksum failed` label predates the redaction-aware classification fix (see [`denominator-correction-2026-09-09.md`](benchmarks/denominator-correction-2026-09-09.md)) |
| ISR | Israel | Passport | HIT (+ 2 redacted) | Public specimens; the 2011 specimen reads checksum-valid in the real-OCR scan, 2026-08-17. The 2003 and 2013 specimens have their MRZ zone physically redacted by the publisher -- both are in the corpus, not local-only |
| JPN | Japan | Passport | HIT (+ 2 redacted) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`); the two other specimens (2009, 2021) have their MRZ zone physically redacted by the publisher |
| JOR | Jordan | -- | No specimen yet | -- |
| KAZ | Kazakhstan | Passport | HIT (+ 1 redacted) | Public-domain specimens; real-OCR scan, 2026-08-17; the second specimen's MRZ zone is physically redacted by the publisher |
| PRK | Korea (Democratic People's Republic of) | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| KOR | Korea (Republic of) | Passport | MISS (checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`); neither specimen (2020, 2022) validates -- both print non-conforming check digits by design, not an OCR misread (see [`checksum-failed-real-specimens-2026-09-08.md`](benchmarks/checksum-failed-real-specimens-2026-09-08.md)) |
| KWT | Kuwait | Passport | HIT | Found no MRZ in the 2026-08-17 real-OCR scan; reads since ADR-0008 chunk 2 (CI gate, 2026-09-11) |
| KGZ | Kyrgyzstan | -- | No specimen yet | -- |
| LAO | Lao People's Democratic Republic | -- | No specimen yet | -- |
| LBN | Lebanon | -- | No specimen yet | -- |
| MAC | Macao | -- | No specimen yet | -- |
| MYS | Malaysia | Passport | HIT (+ 1 redacted) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`); the second specimen's zone is scored `redacted` though its printed check digits validate on the unredacted line -- an open question, not resolved here (see [`denominator-correction-2026-09-09.md`](benchmarks/denominator-correction-2026-09-09.md)) |
| MDV | Maldives | -- | No specimen yet | -- |
| MNG | Mongolia | -- | No specimen yet | -- |
| MMR | Myanmar | Passport | Known MISS (documented) | The specimen's MRZ zone is physically redacted by the publisher; the 2026-08-17 scan's `no MRZ found` label predates the redaction-aware classification fix (see [`denominator-correction-2026-09-09.md`](benchmarks/denominator-correction-2026-09-09.md)) |
| NPL | Nepal | Passport | HIT (1 of 3 specimens; 2 redacted) | The 2019 book reads since ADR-0008 chunk 2 (CI gate, 2026-09-11); the 2011 specimen and its rotated copy carry a redacted zone and are scored out |
| OMN | Oman | Passport | HIT | Reads since ADR-0008 chunk 2 (CI gate, 2026-09-11). The "no MRZ found" recorded here from 2026-08-17 was root-caused by ADR-0008 chunk 1: the page-orientation vote turned the page sideways before OCR |
| PAK | Pakistan | Passport | HIT | Ingested 2026-09-10; photographed sideways, reads since ADR-0008 chunk 2 moved quarter-turns into the retry chain (CI gate, 2026-09-11) |
| PSE | Palestine | -- | No specimen yet | -- |
| PHL | Philippines | ID card (front) | No specimen yet | PhilSys national ID sample added 2026-09-14 (public, public-domain, PSA-attributed). No MRZ zone -- PhilSys is a national ID, not a travel document. |
| QAT | Qatar | -- | No specimen yet | -- |
| SAU | Saudi Arabia | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| SGP | Singapore | Passport | HIT | 2017 biometric-passport design illustration added 2026-09-14 (public, attributed to Singapore's Immigration & Checkpoints Authority, none-stated licence), checksum-valid TD3, confirmed by both `check_sample` and this manifest's OCR pass. |
| LKA | Sri Lanka | Passport (cover, 2 series) | No specimen yet | Covers only in `samples/covers/` (c12, 2026-09-15, Commons CC0 / public domain): 2024 P series and the earlier N series; no data page, never a coverage claim (ADR-0012) |
| SYR | Syrian Arab Republic | -- | No specimen yet | -- |
| TWN | Taiwan | -- | No specimen yet | -- |
| TJK | Tajikistan | -- | No specimen yet | -- |
| THA | Thailand | -- | No specimen yet | -- |
| TLS | Timor-Leste | -- | No specimen yet | -- |
| TUR | Türkiye | Passport, ID card | HIT (x5 specimens) (+ 4 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| TKM | Turkmenistan | -- | No specimen yet | -- |
| ARE | United Arab Emirates | Passport | HIT | Contributor-supplied specimen (watermarked reference) |
| UZB | Uzbekistan | Passport | HIT | Ingested 2026-09-10; reads checksum-valid |
| VNM | Viet Nam | Passport | HIT (+ 1 redacted) | The 2023 book reads since ADR-0008 chunk 2 (CI gate, 2026-09-11); its "no MRZ found" from 2026-08-17 was the page-orientation vote ADR-0008 chunk 1 root-caused. The 2022 specimen's MRZ zone is redacted |
| YEM | Yemen | -- | No specimen yet | -- |
| ALB | Albania | Passport | Known MISS (documented) | The specimen's MRZ zone is physically redacted by the publisher; the 2026-08-17 scan's `checksum failed` label predates the redaction-aware classification fix (see [`denominator-correction-2026-09-09.md`](benchmarks/denominator-correction-2026-09-09.md)) |
| AND | Andorra | -- | No specimen yet | -- |
| AUT | Austria | ID card front, Passport | HIT (passport) + negative control (ID card front) | Public-domain specimens; passport HIT in real-OCR scan, 2026-08-17. The ID card front carries no MRZ (it is on the card back, not sourced here) |
| BLR | Belarus | Passport | Known MISS (documented) | Both specimens (2006, 2021) have their MRZ zone physically redacted by the publisher; the 2026-08-17 scan's `checksum failed`/`no MRZ` labels predate the redaction-aware classification fix (see [`denominator-correction-2026-09-09.md`](benchmarks/denominator-correction-2026-09-09.md)) |
| BEL | Belgium | Passport, ID card | HIT (x3 specimens) (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| BIH | Bosnia and Herzegovina | ID card, Passport | HIT (x2 specimens) (+ 1 redacted) | Public-domain specimens; real-OCR scan, 2026-08-17. The third specimen's MRZ zone is physically redacted, not a checksum failure |
| BGR | Bulgaria | ID card front, Passport | HIT (passport) + negative control (ID card front) | Public-domain specimen; passport HIT in real-OCR scan, 2026-08-17 |
| HRV | Croatia | Passport, ID card, BorderPass | HIT (x3 specimens) (+ 1 checksum-failed) | Public-domain specimen; ID card added in real-OCR scan, 2026-08-17. A BorderPass specimen (`CB_HRV_2025`) is also in the corpus: its back reads checksum-valid, its front carries no MRZ |
| CZE | Czechia | Passport | HIT (x2 specimens) (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| DNK | Denmark | Passport | HIT (x3 specimens) | Public-domain specimens; real-OCR scan, 2026-08-17 |
| EST | Estonia | Passport | HIT | Public-domain specimen |
| FIN | Finland | Passport | HIT (x5 specimens) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`); the 2007 and 2023 books, which found no MRZ there, read since ADR-0008 chunk 2 (CI gate, 2026-09-11) |
| FRA | France | Passport, ID card | HIT (+ 1 no-MRZ found, 1 redacted) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`); the ID-card back is a real, still-open detection miss (TD1, see [`denominator-rebless-2026-09-19.md`](benchmarks/denominator-rebless-2026-09-19.md)); the redacted passport's MRZ zone is physically redacted by the publisher; the ID-card front carries no MRZ by design |
| DEU | Germany | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| D | Germany | Passport (x2), ID card front | MISS (checksum failed) | Legacy single-letter code, Doc 9303 Part 3 §5 Part A. Both passport specimens (`Germany_Passport_Specimen_P0_D00_2018_mrz.webp`, pre-existing; `Germany_Passport_Specimen_P0_D00_2024_mrz.jpg`, added 2026-09-14, public, official Bundesgesetzblatt `PassV` Anlage 2a) read an MRZ but fail checksum on this OCR pass -- the single-character-misread pattern (see `knowledge/benchmarks/README.md`), not confirmed evidence either specimen itself is invalid. Not yet a HIT for `D`. ID card front (`Germany_ID_Specimen_2024_front_no_mrz.jpg`, added 2026-09-14, public, official `PAuswV` Anlage 1) carries no MRZ -- `Personalausweis` MRZ is on the back, not sourced here; same front-only gap as Latvia's and Ireland's ID card rows. |
| GRC | Greece | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| HUN | Hungary | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| ISL | Iceland | Passport | HIT (x2 specimens) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| IRL | Ireland | ID card front, Passport | HIT (passport) + negative control (ID card front) | Public-domain specimen; passport HIT in real-OCR scan, 2026-08-17 |
| ITA | Italy | Passport, ID card | HIT (+ 1 no-MRZ found) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`); the ID-card back is a real, still-open detection miss (TD1, see [`denominator-rebless-2026-09-19.md`](benchmarks/denominator-rebless-2026-09-19.md)); the front carries no MRZ by design |
| XKX | Kosovo | Passport | HIT (x3 specimens) | Public-domain specimens; real-OCR scan, 2026-08-17 |
| LVA | Latvia | ID card (front) | No specimen yet | ID card front added 2026-09-14 (public, `Latvia_ID_Specimen_2021_front_no_mrz.png`); carries no MRZ (TD1 MRZ is on the back, not yet sourced) — same pattern as Kenya's front-only passport row |
| LIE | Liechtenstein | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| LTU | Lithuania | Passport | HIT | Added 2026-09-14 (public, `Lithuania_Passport_Specimen_P0_LTU_2019_mrz.jpg`, CC BY-SA); checksum-valid TD3, self-published Commons upload, not the issuing authority |
| LUX | Luxembourg | ID card | Known MISS (documented) + negative control (ID card front) | The back's MRZ zone is physically redacted by the publisher; the 2026-08-17 scan's `no MRZ found` label predates the redaction-aware classification fix (see [`denominator-correction-2026-09-09.md`](benchmarks/denominator-correction-2026-09-09.md)). The front carries no MRZ by design |
| MLT | Malta | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| MDA | Moldova | Passport | HIT (x2 of 3) | Manifest regeneration, 2026-09-04. 2023 and the 2014 `wide` crop read checksum-valid; the tighter 2014 crop of the same document does not. |
| MCO | Monaco | ID card, Passport | HIT (passport, ID-card back) | Public-domain specimens; real-OCR scan, 2026-08-17. The ID-card back reads since ADR-0008 chunk 2 (CI gate, 2026-09-11); the front carries no MRZ |
| MNE | Montenegro | -- | No specimen yet | -- |
| NLD | Netherlands | Driving license (no MRZ), Passport | HIT (passport) + negative control (driving license) | Public-domain specimen; passport HIT in real-OCR scan, 2026-08-17 |
| MKD | North Macedonia | Passport | HIT (x2 specimens) | Public-domain specimens; real-OCR scan, 2026-08-17 |
| NOR | Norway | Passport, ID card | HIT (x2 specimens) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| POL | Poland | Passport, ID card | HIT (x2 specimens) (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| PRT | Portugal | Passport, ID card | HIT (x3: 2 passports, ID-card back) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`); the `PX_PRT_2017` book, which found no MRZ there, reads since ADR-0008 chunk 2 (CI gate, 2026-09-11). The ID-card front carries no MRZ |
| ROU | Romania | Passport, ID card | HIT (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| RUS | Russian Federation | Passport, ID card | HIT (x4 specimens) (+ 1 checksum-failed, 2 redacted) | `PD_RUS_2004` and `P0_RUS_2025` read; `P0_RUS_2014` since ADR-0008 chunk 2 (CI gate, 2026-09-11). `P0_RUS_2019` carries a checksum-valid zone that OCR misreads. An ID-card back also reads checksum-valid; its front carries no MRZ. This row said "No specimen yet" until 2026-09-12, though Russian specimens were already in the corpus |
| SMR | San Marino | ID card (back) | No specimen yet | ID card back added 2026-09-14 (public, `San_Marino_ID_Specimen_2017_back_mrz.jpg`, public-domain); blank facsimile template, all-filler MRZ zone, OCR reads nothing parseable — not yet a HIT |
| SRB | Serbia | Passport, ID card (TD1) | HIT (x2 specimens) (+ negative control, 1 no-MRZ) | Public-domain specimens; real-OCR scan, 2026-08-17 |
| SVK | Slovakia | Passport, Service Passport | HIT (x2) + negative control | Contributor-supplied specimens (Specimen/Vzorka placeholder name); corpus includes additional unlabelled specimens per real-OCR scan, 2026-08-17 |
| SVN | Slovenia | ID card (TD1), Passport | HIT (x2, ID card + passport) (+ negative control) | Public-domain specimen; passport HIT in real-OCR scan, 2026-08-17 |
| ESP | Spain | Passport | HIT (x4) | Contributor-supplied specimens (ESPECIMEN watermark / placeholder name); confirmed still HIT in real-OCR scan, 2026-08-17 |
| SWE | Sweden | Passport, ID card, Visa | HIT (x2, passport + visa) (+ 1 checksum-failed) | Real-specimen CI gate, 2026-09-13. The 2022 ID-card back is a named `checksum_failed` with a hand-verified fixture; the 2027 ID-card front carries no MRZ (renamed `_front_no_mrz` 2026-09-13 — see [`manifest-review-no-mrz-found-2026-09-13.md`](benchmarks/manifest-review-no-mrz-found-2026-09-13.md)) |
| CHE | Switzerland | Passport, ID card | HIT (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| UKR | Ukraine | Passport | HIT (+ 1 redacted) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`); the 2015 specimen reads checksum-valid, the 2012 specimen's MRZ zone is redacted |
| GBR | United Kingdom | Passport | HIT (x3 specimens) (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| VAT | Holy See (Vatican City State) | -- | No specimen yet | -- |
| AUS | Australia | Passport | HIT (+ 1 redacted) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`); the second specimen's MRZ zone is physically redacted by the publisher |
| FJI | Fiji | -- | No specimen yet | -- |
| KIR | Kiribati | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| MHL | Marshall Islands | -- | No specimen yet | -- |
| FSM | Micronesia (Federated States of) | -- | No specimen yet | -- |
| NRU | Nauru | -- | No specimen yet | -- |
| NZL | New Zealand | -- | No specimen yet | -- |
| PLW | Palau | -- | No specimen yet | -- |
| PNG | Papua New Guinea | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (c12, 2026-09-15, Commons public domain, low-res); no data page, never a coverage claim (ADR-0012) |
| WSM | Samoa | -- | No specimen yet | -- |
| SLB | Solomon Islands | -- | No specimen yet | -- |
| TON | Tonga | -- | No specimen yet | -- |
| TUV | Tuvalu | -- | No specimen yet | -- |
| VUT | Vanuatu | -- | No specimen yet | -- |
| ABW | Aruba | -- | No specimen yet | -- |
| BMU | Bermuda | -- | No specimen yet | -- |
| CYM | Cayman Islands | -- | No specimen yet | -- |
| CUW | Curaçao | -- | No specimen yet | -- |
| FRO | Faroe Islands | -- | No specimen yet | -- |
| GIB | Gibraltar | -- | No specimen yet | -- |
| GRL | Greenland | -- | No specimen yet | -- |
| SXM | Sint Maarten (Dutch part) | -- | No specimen yet | -- |
| UTO | Utopia (ICAO specimen) | -- | No specimen yet | -- |
| EUE | European Union | -- | No specimen yet | -- |
| RKS | Kosovo | -- | No specimen yet | -- |
| GBD | British Overseas Territories Citizen | -- | No specimen yet | Nationality-field code; covered by a specimen whose nationality field carries it |
| GBN | British National (Overseas) | -- | No specimen yet | Nationality-field code; covered by a specimen whose nationality field carries it |
| GBO | British Overseas Citizen | -- | No specimen yet | Nationality-field code; covered by a specimen whose nationality field carries it |
| GBP | British Protected Person | -- | No specimen yet | Nationality-field code; covered by a specimen whose nationality field carries it |
| GBS | British Subject | -- | No specimen yet | Nationality-field code; covered by a specimen whose nationality field carries it |
| XXA | Stateless person (1954 Convention) | -- | No specimen yet | Nationality-field code; covered by a specimen whose nationality field carries it |
| XXB | Refugee (1951 Convention) | -- | No specimen yet | Nationality-field code; covered by a specimen whose nationality field carries it |
| XXC | Refugee (other) | -- | No specimen yet | Nationality-field code; covered by a specimen whose nationality field carries it |
| XXX | Unspecified nationality | -- | No specimen yet | Nationality-field code; covered by a specimen whose nationality field carries it |
| UNO | United Nations Organization | -- | No specimen yet | -- |
| UNA | United Nations specialized agency | -- | No specimen yet | -- |
| UNK | United Nations Interim Administration Mission in Kosovo | -- | No specimen yet | -- |
| XOM | Sovereign Military Order of Malta | -- | No specimen yet | -- |
| XBA | African Development Bank | -- | No specimen yet | -- |
| XIM | African Export-Import Bank | -- | No specimen yet | -- |
| XCC | Caribbean Community (CARICOM) | -- | No specimen yet | -- |
| XCO | Common Market for Eastern and Southern Africa (COMESA) | -- | No specimen yet | -- |
| XEC | Economic Community of West African States (ECOWAS) | -- | No specimen yet | -- |
| XPO | International Criminal Police Organization (INTERPOL) | -- | No specimen yet | -- |
| XCE | Council of Europe | -- | No specimen yet | -- |
| XES | Organization of Eastern Caribbean States (OECS) | -- | No specimen yet | -- |
| XMP | Parliamentary Assembly of the Mediterranean (PAM) | -- | No specimen yet | -- |
| XDC | Southern African Development Community | -- | No specimen yet | -- |
| ANT | Netherlands Antilles | -- | No specimen yet | Deprecated in ISO 3166; valid on documents issued before withdrawal |
| NTZ | Neutral Zone | -- | No specimen yet | Deprecated in ISO 3166; valid on documents issued before withdrawal |
| IAO | International Civil Aviation Organization | -- | Not applicable by definition | Used only when ICAO digitally signs a master list; no document carries it |
