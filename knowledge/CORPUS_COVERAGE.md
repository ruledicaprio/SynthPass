# Corpus coverage — comprehensive world passport-check backlog

Tracks Tier-1 MRZ corpus coverage against every ISO/ICAO country/entity code in
`crates/mrz/src/countries.rs` (238 codes). This is the concrete backlog behind the "wider
real-world corpus is the natural next accuracy milestone" note in
`knowledge/ARCHITECTURE.md` §8 — grown one individually-vetted specimen at a time, per the
checklist in `CONTRIBUTING.md`. **PRADO (`consilium.europa.eu/prado`) is never a source
here** — its copyright notice prohibits harvesting/redistributing its material outside
official, non-commercial use; it's consulted only as a manual human reference, never
scraped or stored.

Tier-1 MRZ checksum/parsing logic itself is ICAO-9303-generic and not special-cased per
country — a HIT here reflects real-world OCR/format validation on an actual specimen,
not new per-country code. `scripts/watch-samples.ps1` + `synthpass-ocr`'s `check_sample`
example give an instant first-pass check when a new candidate specimen is dropped into
`samples/` — see CONTRIBUTING.md.

**Only `samples/ocr_fixtures/` is tracked in git.** `samples/passports/`, `samples/id_cards/`,
and `samples/driving_licenses/` are gitignored local mirrors (see `samples/README.md`), so a
per-country row below reflects a contributor's locally-verified corpus at the time it was
checked, not something a fresh clone of this repository can reproduce on its own — only the
`ocr_fixtures/` subset survives a clone. This repo already treats prose-recorded verification
as legitimate without the backing artifact being committed (`knowledge/benchmarks/README.md`'s
"a constant with no measurement behind it does not ship" principle is the same idea, applied
here to corpus coverage instead of a benchmark number).

## Summary

| Status | Countries |
|---|---|
| HIT (checksum-valid real specimen, `mrz_corpus.rs` and/or the 2026-08-17 real-OCR scan) | 56 |
| MISS (checksum failed or no MRZ found, real-OCR scan, 2026-08-17) | 19 |
| Stale (was HIT, now reproducibly MISS against current OCR/parser — 2026-08-17) | 2 |
| Known MISS (documented, e.g. physically redacted specimen) | 1 |
| Candidate specimen rejected per the vetting checklist | 1 |
| No specimen yet | 159 |
| **Total tracked codes** | **238** |

Grown substantially 2026-08-17: contributor additions plus a full real-OCR pass over
`samples/passports/` and `samples/id_cards/` (`integrity_survey.rs --mrz-only`, added this
session — see `knowledge/benchmarks/README.md`'s dated finding) replaced most of the
previous "specimen present, not yet wired" placeholders with a real, measured HIT/MISS
status. `Passport`/`ID card`/etc. counts inside a single country's `Note` column (e.g.
"HIT (x2 specimens) (+ 1 checksum-failed)") are per-specimen, not per-country — see each
row for the exact breakdown.

## Full table

| Code | Country/Entity | Document type(s) | Status | Note |
|---|---|---|---|---|
| DZA | Algeria | Passport | MISS (checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| AGO | Angola | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| BEN | Benin | -- | No specimen yet | -- |
| BWA | Botswana | -- | No specimen yet | -- |
| BFA | Burkina Faso | -- | No specimen yet | -- |
| BDI | Burundi | -- | No specimen yet | -- |
| CPV | Cabo Verde | -- | No specimen yet | -- |
| CMR | Cameroon | -- | No specimen yet | -- |
| CAF | Central African Republic | -- | No specimen yet | -- |
| TCD | Chad | -- | No specimen yet | -- |
| COM | Comoros | -- | No specimen yet | -- |
| COG | Congo | -- | No specimen yet | -- |
| COD | Congo (Democratic Republic of the) | -- | No specimen yet | -- |
| CIV | Côte d'Ivoire | -- | No specimen yet | -- |
| DJI | Djibouti | -- | No specimen yet | -- |
| EGY | Egypt | Passport | HIT (+ 2 no-MRZ) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| GNQ | Equatorial Guinea | -- | No specimen yet | -- |
| ERI | Eritrea | -- | No specimen yet | -- |
| SWZ | Eswatini | -- | No specimen yet | -- |
| ETH | Ethiopia | -- | No specimen yet | -- |
| GAB | Gabon | -- | No specimen yet | -- |
| GMB | Gambia | -- | No specimen yet | -- |
| GHA | Ghana | Passport | MISS (checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| GIN | Guinea | -- | No specimen yet | -- |
| GNB | Guinea-Bissau | -- | No specimen yet | -- |
| KEN | Kenya | -- | No specimen yet | -- |
| LSO | Lesotho | -- | No specimen yet | -- |
| LBR | Liberia | -- | No specimen yet | -- |
| LBY | Libya | -- | No specimen yet | -- |
| MDG | Madagascar | -- | No specimen yet | -- |
| MWI | Malawi | -- | No specimen yet | -- |
| MLI | Mali | -- | No specimen yet | -- |
| MRT | Mauritania | Passport | MISS (checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| MUS | Mauritius | -- | No specimen yet | -- |
| MAR | Morocco | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| MOZ | Mozambique | -- | No specimen yet | -- |
| NAM | Namibia | -- | No specimen yet | -- |
| NER | Niger | -- | No specimen yet | -- |
| NGA | Nigeria | -- | No specimen yet | -- |
| RWA | Rwanda | -- | No specimen yet | -- |
| STP | Sao Tome and Principe | -- | No specimen yet | -- |
| SEN | Senegal | -- | No specimen yet | -- |
| SYC | Seychelles | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| SLE | Sierra Leone | -- | No specimen yet | -- |
| SOM | Somalia | -- | No specimen yet | -- |
| ZAF | South Africa | -- | No specimen yet | -- |
| SSD | South Sudan | -- | No specimen yet | -- |
| SDN | Sudan | Passport | Candidate rejected | No SPECIMEN watermark, read as real personal data -- excluded per vetting checklist |
| TZA | Tanzania | -- | No specimen yet | -- |
| TGO | Togo | -- | No specimen yet | -- |
| TUN | Tunisia | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| UGA | Uganda | -- | No specimen yet | -- |
| ZMB | Zambia | -- | No specimen yet | -- |
| ZWE | Zimbabwe | -- | No specimen yet | -- |
| ESH | Western Sahara | -- | No specimen yet | -- |
| ATG | Antigua and Barbuda | -- | No specimen yet | -- |
| ARG | Argentina | Passport | MISS (checksum failed) (+ 1 no-MRZ) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| BHS | Bahamas | -- | No specimen yet | -- |
| BRB | Barbados | -- | No specimen yet | -- |
| BLZ | Belize | -- | No specimen yet | -- |
| BOL | Bolivia | -- | No specimen yet | -- |
| BRA | Brazil | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| CAN | Canada | Passport | HIT (x3 specimens) | Contributor-supplied specimens (SPECIMEN watermark); a 4th specimen (2023-issue) found no MRZ in the real-OCR scan, 2026-08-17 |
| CHL | Chile | -- | No specimen yet | -- |
| COL | Colombia | Passport | MISS (x2, checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| CRI | Costa Rica | -- | No specimen yet | -- |
| CUB | Cuba | -- | No specimen yet | -- |
| DMA | Dominica | -- | No specimen yet | -- |
| DOM | Dominican Republic | -- | No specimen yet | -- |
| ECU | Ecuador | -- | No specimen yet | -- |
| SLV | El Salvador | -- | No specimen yet | -- |
| GRD | Grenada | -- | No specimen yet | -- |
| GTM | Guatemala | -- | No specimen yet | -- |
| GUY | Guyana | -- | No specimen yet | -- |
| HTI | Haiti | -- | No specimen yet | -- |
| HND | Honduras | -- | No specimen yet | -- |
| JAM | Jamaica | -- | No specimen yet | -- |
| MEX | Mexico | Passport | MISS (checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| NIC | Nicaragua | -- | No specimen yet | -- |
| PAN | Panama | -- | No specimen yet | -- |
| PRY | Paraguay | -- | No specimen yet | -- |
| PER | Peru | -- | No specimen yet | -- |
| KNA | Saint Kitts and Nevis | -- | No specimen yet | -- |
| LCA | Saint Lucia | -- | No specimen yet | -- |
| VCT | Saint Vincent and the Grenadines | -- | No specimen yet | -- |
| SUR | Suriname | -- | No specimen yet | -- |
| TTO | Trinidad and Tobago | -- | No specimen yet | -- |
| USA | United States of America | Passport, ID card | HIT (x4 specimens) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| URY | Uruguay | Passport | MISS (checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| VEN | Venezuela | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| AFG | Afghanistan | Passport | MISS (checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| ARM | Armenia | -- | No specimen yet | -- |
| AZE | Azerbaijan | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| BHR | Bahrain | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| BGD | Bangladesh | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| BTN | Bhutan | -- | No specimen yet | -- |
| BRN | Brunei Darussalam | -- | No specimen yet | -- |
| KHM | Cambodia | -- | No specimen yet | -- |
| CHN | China | Passport | HIT (x2 specimens) (+ 2 checksum-failed) | Contributor-supplied specimen (SPECIMEN watermark); corpus grew to 4 total in the real-OCR scan, 2026-08-17 |
| CYP | Cyprus | Passport | HIT (x3 specimens) | Contributor-supplied specimens (2010/2020/2026-issue, SPECIMEN watermark); the 2026 one is the first corpus specimen with Cyprus's new "PP" document-type code (effective 15 December 2025) |
| GEO | Georgia | -- | No specimen yet | -- |
| HKG | Hong Kong | -- | No specimen yet | -- |
| IND | India | Passport | MISS (x2, no MRZ found) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| IDN | Indonesia | Passport | MISS (x2, checksum failed) (+ 1 no-MRZ) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| IRN | Iran | Passport | MISS (checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| IRQ | Iraq | Passport | MISS (checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| ISR | Israel | Passport | Known MISS (documented) + HIT (+ 1 checksum-failed) | Public specimen, physically redacted MRZ -- kept local-only, not committed; 2 more (unrelated, unredacted) specimens found in the real-OCR scan, 2026-08-17, one a clean HIT |
| JPN | Japan | Passport | HIT (+ 2 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| JOR | Jordan | -- | No specimen yet | -- |
| KAZ | Kazakhstan | Passport | HIT (+ 2 checksum-failed) | Public-domain specimens; real-OCR scan, 2026-08-17 |
| PRK | Korea (Democratic People's Republic of) | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| KOR | Korea (Republic of) | Passport | HIT (+ 2 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| KWT | Kuwait | Passport | MISS (no MRZ found) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| KGZ | Kyrgyzstan | -- | No specimen yet | -- |
| LAO | Lao People's Democratic Republic | -- | No specimen yet | -- |
| LBN | Lebanon | -- | No specimen yet | -- |
| MAC | Macao | -- | No specimen yet | -- |
| MYS | Malaysia | Passport | HIT (x2 specimens) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| MDV | Maldives | -- | No specimen yet | -- |
| MNG | Mongolia | -- | No specimen yet | -- |
| MMR | Myanmar | Passport | MISS (no MRZ found) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| NPL | Nepal | Passport | MISS (x2, checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| OMN | Oman | Passport | MISS (stale, 2026-08-17) | Specimen present in `samples/`; reproducibly returns "no MRZ found" against the current OCR/parser despite the recorded doc number — regression or flaky original hit, not yet root-caused |
| PAK | Pakistan | -- | No specimen yet | -- |
| PSE | Palestine | -- | No specimen yet | -- |
| PHL | Philippines | -- | No specimen yet | -- |
| QAT | Qatar | -- | No specimen yet | -- |
| SAU | Saudi Arabia | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| SGP | Singapore | -- | No specimen yet | -- |
| LKA | Sri Lanka | -- | No specimen yet | -- |
| SYR | Syrian Arab Republic | -- | No specimen yet | -- |
| TWN | Taiwan | -- | No specimen yet | -- |
| TJK | Tajikistan | -- | No specimen yet | -- |
| THA | Thailand | -- | No specimen yet | -- |
| TLS | Timor-Leste | -- | No specimen yet | -- |
| TUR | Türkiye | Passport, ID card | HIT (x5 specimens) (+ 5 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| TKM | Turkmenistan | -- | No specimen yet | -- |
| ARE | United Arab Emirates | Passport | HIT | Contributor-supplied specimen (watermarked reference) |
| UZB | Uzbekistan | -- | No specimen yet | -- |
| VNM | Viet Nam | Passport | MISS (stale, 2026-08-17) | Specimen present in `samples/`; reproducibly returns "no MRZ found" against the current OCR/parser despite the recorded doc number — regression or flaky original hit, not yet root-caused |
| YEM | Yemen | -- | No specimen yet | -- |
| ALB | Albania | Passport | MISS (checksum failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| AND | Andorra | -- | No specimen yet | -- |
| AUT | Austria | ID card front, Passport | HIT (passport) + ID card front not wired (MRZ on card back) | Public-domain specimens; passport HIT in real-OCR scan, 2026-08-17 |
| BLR | Belarus | Passport | MISS (checksum failed) (+ 1 no-MRZ) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| BEL | Belgium | Passport, ID card | HIT (x3 specimens) (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| BIH | Bosnia and Herzegovina | ID card, Passport | HIT (x2 specimens) (+ 1 checksum-failed) | Public-domain specimens; real-OCR scan, 2026-08-17 |
| BGR | Bulgaria | ID card front, Passport | HIT (passport) + negative control (ID card front) | Public-domain specimen; passport HIT in real-OCR scan, 2026-08-17 |
| HRV | Croatia | Passport, ID card | HIT (x2 specimens) (+ 1 checksum-failed) | Public-domain specimen; ID card added in real-OCR scan, 2026-08-17 |
| CZE | Czechia | Passport | HIT (x2 specimens) (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| DNK | Denmark | Passport | HIT (x3 specimens) | Public-domain specimens; real-OCR scan, 2026-08-17 |
| EST | Estonia | Passport | HIT | Public-domain specimen |
| FIN | Finland | Passport | HIT (x2 specimens) (+ 2 no-MRZ) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| FRA | France | Passport, ID card | HIT (+ 2 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| DEU | Germany | Passport | HIT (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| D | Germany | -- | No specimen yet | Legacy single-letter code, Doc 9303 Part 3 §5 Part A |
| GRC | Greece | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| HUN | Hungary | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| ISL | Iceland | Passport | HIT (x2 specimens) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| IRL | Ireland | ID card front, Passport | HIT (passport) + ID card front not wired | Public-domain specimen; passport HIT in real-OCR scan, 2026-08-17 |
| ITA | Italy | Passport, ID card | HIT (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| XKX | Kosovo | Passport | HIT (x3 specimens) | Public-domain specimens; real-OCR scan, 2026-08-17 |
| LVA | Latvia | -- | No specimen yet | -- |
| LIE | Liechtenstein | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| LTU | Lithuania | -- | No specimen yet | -- |
| LUX | Luxembourg | ID card | MISS (no MRZ found) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| MLT | Malta | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| MDA | Moldova | -- | No specimen yet | -- |
| MCO | Monaco | ID card, Passport | HIT (passport) + ID card no MRZ found | Public-domain specimens; real-OCR scan, 2026-08-17 |
| MNE | Montenegro | -- | No specimen yet | -- |
| NLD | Netherlands | Driving license (no MRZ), Passport | HIT (passport) + driving license not wired | Public-domain specimen; passport HIT in real-OCR scan, 2026-08-17 |
| MKD | North Macedonia | Passport | HIT (x2 specimens) | Public-domain specimens; real-OCR scan, 2026-08-17 |
| NOR | Norway | Passport, ID card | HIT (x2 specimens) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| POL | Poland | Passport, ID card | HIT (x2 specimens) (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| PRT | Portugal | Passport, ID card | HIT (+ 1 checksum-failed, 1 no-MRZ) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| ROU | Romania | Passport, ID card | HIT (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| RUS | Russian Federation | -- | No specimen yet | -- |
| SMR | San Marino | -- | No specimen yet | -- |
| SRB | Serbia | Passport, ID card (TD1) | HIT (x2 specimens) (+ negative control, 1 no-MRZ) | Public-domain specimens; real-OCR scan, 2026-08-17 |
| SVK | Slovakia | Passport, Service Passport | HIT (x2) + negative control | Contributor-supplied specimens (Specimen/Vzorka placeholder name); corpus includes additional unlabelled specimens per real-OCR scan, 2026-08-17 |
| SVN | Slovenia | ID card (TD1), Passport | HIT (x2, ID card + passport) (+ negative control) | Public-domain specimen; passport HIT in real-OCR scan, 2026-08-17 |
| ESP | Spain | Passport | HIT (x2) | Contributor-supplied specimens (ESPECIMEN watermark / placeholder name); confirmed still HIT in real-OCR scan, 2026-08-17 |
| SWE | Sweden | -- | No specimen yet | -- |
| CHE | Switzerland | Passport, ID card | HIT (+ 2 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| UKR | Ukraine | Passport | MISS (no MRZ found) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| GBR | United Kingdom | Passport | HIT (x3 specimens) (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| VAT | Holy See (Vatican City State) | -- | No specimen yet | -- |
| AUS | Australia | Passport | HIT (+ 1 checksum-failed) | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| FJI | Fiji | -- | No specimen yet | -- |
| KIR | Kiribati | Passport | HIT | Real-OCR scan, 2026-08-17 (`integrity_survey.rs --mrz-only`) |
| MHL | Marshall Islands | -- | No specimen yet | -- |
| FSM | Micronesia (Federated States of) | -- | No specimen yet | -- |
| NRU | Nauru | -- | No specimen yet | -- |
| NZL | New Zealand | -- | No specimen yet | -- |
| PLW | Palau | -- | No specimen yet | -- |
| PNG | Papua New Guinea | -- | No specimen yet | -- |
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
| GBD | British Overseas Territories Citizen | -- | No specimen yet | -- |
| GBN | British National (Overseas) | -- | No specimen yet | -- |
| GBO | British Overseas Citizen | -- | No specimen yet | -- |
| GBP | British Protected Person | -- | No specimen yet | -- |
| GBS | British Subject | -- | No specimen yet | -- |
| XXA | Stateless person (1954 Convention) | -- | No specimen yet | -- |
| XXB | Refugee (1951 Convention) | -- | No specimen yet | -- |
| XXC | Refugee (other) | -- | No specimen yet | -- |
| XXX | Unspecified nationality | -- | No specimen yet | -- |
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
| IAO | International Civil Aviation Organization | -- | No specimen yet | Used only when ICAO digitally signs a master list |
