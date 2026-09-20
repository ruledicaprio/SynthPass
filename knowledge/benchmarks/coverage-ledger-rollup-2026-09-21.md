# Per-country rollup of the real-specimen outcome ledger

**Date:** 2026-09-21 · **MAIN:** `8480772` · **DATA:** `396b22f3b97ee36275a5329667ee37cbd0449b40` · **Evidence:** Derived (aggregation of `real-specimen-outcomes.jsonl`) · **Status:** current

Mechanical aggregation of [`real-specimen-outcomes.jsonl`](real-specimen-outcomes.jsonl) (261
assets), one row per country. Country is the asset filename prefix before the first of
`_Passport`, `_ID`, `_Driving`, `_Residence`, `_BorderPass` or `_Visa`; every asset in the ledger
matches one of those, so nothing is dropped and no row is a filename fragment. Counts only — no
zone text, holder name or character value appears here.

This is the derived input to
[`corpus-coverage-drift-2026-09-21.md`](corpus-coverage-drift-2026-09-21.md); it exists so that
audit's per-country claims can be checked without re-deriving them.

| country | assets | hit | checksum_failed | checksum_failed_specimen | no_mrz_found | no_mrz_expected | redacted_mrz | formats_seen |
|---|---|---|---|---|---|---|---|---|
| Afghanistan | 1 | 0 | 1 | 0 | 0 | 0 | 0 | TD3 |
| Albania | 1 | 0 | 0 | 0 | 0 | 0 | 1 |  |
| Algeria | 1 | 0 | 0 | 0 | 0 | 1 | 0 |  |
| Angola | 2 | 2 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Argentina | 4 | 1 | 0 | 3 | 0 | 0 | 0 | MRVA, TD3 |
| Armenia | 1 | 0 | 0 | 0 | 0 | 0 | 1 |  |
| Australia | 2 | 1 | 0 | 0 | 0 | 0 | 1 | TD3 |
| Austria | 2 | 1 | 0 | 0 | 0 | 1 | 0 | TD3 |
| Azerbaijan | 3 | 3 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Bahrain | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Bangladesh | 3 | 2 | 0 | 0 | 0 | 1 | 0 | TD3 |
| Belarus | 2 | 0 | 0 | 0 | 0 | 0 | 2 |  |
| Belgium | 6 | 3 | 1 | 0 | 0 | 2 | 0 | TD1, TD3 |
| Bosnia Herzegovina | 2 | 0 | 0 | 0 | 0 | 2 | 0 |  |
| Bosnia and Herzegovina | 5 | 2 | 0 | 0 | 0 | 2 | 1 | TD1, TD3 |
| Brazil | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Bulgaria | 2 | 1 | 0 | 0 | 0 | 1 | 0 | TD3 |
| Cambodia | 1 | 0 | 0 | 0 | 0 | 1 | 0 |  |
| Canada | 4 | 4 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Cetis Sample | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| China | 5 | 3 | 0 | 0 | 0 | 0 | 2 | TD3 |
| Colombia | 2 | 0 | 0 | 1 | 0 | 0 | 1 | TD2, TD3 |
| Croatia | 8 | 3 | 1 | 0 | 0 | 4 | 0 | TD1, TD2, TD3 |
| Cyprus | 3 | 3 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Czechia | 4 | 2 | 1 | 0 | 0 | 1 | 0 | TD3 |
| Denmark | 3 | 3 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Djibouti | 2 | 2 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Dominica | 1 | 0 | 0 | 0 | 0 | 1 | 0 |  |
| Dominican Republic | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Egypt | 4 | 2 | 0 | 0 | 0 | 0 | 2 | TD3 |
| Estonia | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Finland | 5 | 5 | 0 | 0 | 0 | 0 | 0 | TD3 |
| France | 4 | 1 | 0 | 0 | 1 | 1 | 1 | MRVB, TD1, TD3 |
| Germany | 5 | 1 | 1 | 1 | 0 | 2 | 0 | TD3 |
| Ghana | 2 | 0 | 0 | 1 | 0 | 0 | 1 | TD3 |
| Greece | 2 | 2 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Hong Kong | 2 | 0 | 2 | 0 | 0 | 0 | 0 | TD3 |
| Hungary | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Iceland | 2 | 2 | 0 | 0 | 0 | 0 | 0 | TD3 |
| India | 7 | 3 | 0 | 2 | 0 | 0 | 2 | TD3 |
| Indonesia | 3 | 1 | 0 | 1 | 0 | 0 | 1 | TD1, TD3 |
| Iran | 1 | 0 | 0 | 0 | 0 | 0 | 1 | TD1 |
| Iraq | 1 | 0 | 0 | 0 | 0 | 0 | 1 |  |
| Ireland | 2 | 1 | 0 | 0 | 0 | 1 | 0 | TD3 |
| Israel | 3 | 1 | 0 | 0 | 0 | 0 | 2 | TD3 |
| Italy | 3 | 1 | 0 | 0 | 1 | 1 | 0 | TD1, TD3 |
| Japan | 3 | 1 | 0 | 0 | 0 | 0 | 2 | TD3 |
| Kazakhstan | 2 | 1 | 0 | 0 | 0 | 0 | 1 | TD3 |
| Kenya | 1 | 0 | 0 | 0 | 0 | 1 | 0 |  |
| Kiribati | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Korea Democratic Peoples Republic | 2 | 2 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Korea Republic of Korea | 2 | 0 | 0 | 2 | 0 | 0 | 0 | TD3 |
| Kosovo | 3 | 3 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Kuwait | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Latvia | 1 | 0 | 0 | 0 | 0 | 1 | 0 |  |
| Liechtenstein | 2 | 1 | 0 | 0 | 0 | 1 | 0 | TD3 |
| Lithuania | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Luxembourg | 2 | 0 | 0 | 0 | 0 | 1 | 1 |  |
| Malaysia | 2 | 1 | 0 | 0 | 0 | 0 | 1 | TD3 |
| Malta | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Mauritania | 1 | 0 | 0 | 1 | 0 | 0 | 0 | TD3 |
| Mexico | 1 | 0 | 0 | 0 | 0 | 0 | 1 |  |
| Moldova | 3 | 2 | 0 | 0 | 0 | 1 | 0 | TD3 |
| Monaco | 3 | 2 | 0 | 0 | 0 | 1 | 0 | TD1, TD3 |
| Morocco | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Myanmar | 1 | 0 | 0 | 0 | 0 | 0 | 1 | TD3 |
| Nepal | 3 | 1 | 0 | 0 | 0 | 0 | 2 | TD3 |
| Netherlands | 5 | 3 | 0 | 0 | 0 | 2 | 0 | TD3 |
| Nicaragua | 2 | 2 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Nigeria | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| North Macedonia | 3 | 2 | 0 | 0 | 0 | 0 | 1 | TD3 |
| Norway | 3 | 2 | 0 | 0 | 0 | 1 | 0 | TD1, TD3 |
| Oman | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Pakistan | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Peru | 1 | 0 | 0 | 0 | 0 | 0 | 1 |  |
| Philippines | 1 | 0 | 0 | 0 | 0 | 1 | 0 |  |
| Poland | 5 | 2 | 0 | 1 | 0 | 2 | 0 | TD1, TD3 |
| Portugal | 4 | 3 | 0 | 0 | 0 | 1 | 0 | TD1, TD3 |
| Romania | 2 | 1 | 1 | 0 | 0 | 0 | 0 | TD1, TD3 |
| Russian Federation | 8 | 4 | 1 | 0 | 0 | 1 | 2 | TD1, TD3 |
| San Marino | 1 | 0 | 0 | 0 | 0 | 1 | 0 |  |
| Saudi Arabia | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Serbia | 3 | 2 | 0 | 0 | 0 | 1 | 0 | TD1, TD3 |
| Seychelles | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Singapore | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Slovakia | 3 | 2 | 0 | 0 | 0 | 1 | 0 | TD3 |
| Slovenia | 3 | 2 | 0 | 0 | 0 | 1 | 0 | TD1, TD3 |
| Somalia | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Somaliland | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| South Africa | 2 | 0 | 0 | 0 | 0 | 2 | 0 |  |
| Spain | 4 | 4 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Sweden | 4 | 2 | 1 | 0 | 0 | 1 | 0 | MRVB, TD1, TD3 |
| Switzerland | 5 | 2 | 0 | 1 | 0 | 2 | 0 | MRVB, TD1, TD3 |
| Tunisia | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Turkiye | 14 | 5 | 0 | 4 | 0 | 5 | 0 | TD1, TD2, TD3 |
| Ukraine | 2 | 1 | 0 | 0 | 0 | 0 | 1 | TD3 |
| United Arab Emirates | 4 | 2 | 0 | 0 | 0 | 1 | 1 | TD3 |
| United Kingdom | 4 | 3 | 0 | 1 | 0 | 0 | 0 | TD3 |
| United States of America | 5 | 3 | 0 | 0 | 0 | 0 | 2 | TD1, TD3 |
| Uruguay | 1 | 0 | 0 | 0 | 0 | 0 | 1 |  |
| Uzbekistan | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Venezuela | 1 | 1 | 0 | 0 | 0 | 0 | 0 | TD3 |
| Vietnam | 2 | 1 | 0 | 0 | 0 | 0 | 1 | TD3 |

Total countries: 103. Total assets: 261.
