//! #425: every ISO 3166-1 alpha-3 code resolves to a name.
//!
//! The list below is the ISO-alpha3 column of the UN Statistics Division's M49
//! "Standard country or area codes for statistical use", retrieved 2026-09-25 from
//! <https://unstats.un.org/unsd/methodology/m49/overview/> (248 codes). M49 omits
//! one ISO 3166-1 code, `TWN`, which the table carries anyway and which
//! `countries.rs`'s own tests pin. When ISO 3166-1 changes, refresh this list from the
//! same source, never from memory.

const M49_ISO_ALPHA3: [&str; 248] = [
    "ABW", "AFG", "AGO", "AIA", "ALA", "ALB", "AND", "ARE", "ARG", "ARM", "ASM", "ATA", "ATF",
    "ATG", "AUS", "AUT", "AZE", "BDI", "BEL", "BEN", "BES", "BFA", "BGD", "BGR", "BHR", "BHS",
    "BIH", "BLM", "BLR", "BLZ", "BMU", "BOL", "BRA", "BRB", "BRN", "BTN", "BVT", "BWA", "CAF",
    "CAN", "CCK", "CHE", "CHL", "CHN", "CIV", "CMR", "COD", "COG", "COK", "COL", "COM", "CPV",
    "CRI", "CUB", "CUW", "CXR", "CYM", "CYP", "CZE", "DEU", "DJI", "DMA", "DNK", "DOM", "DZA",
    "ECU", "EGY", "ERI", "ESH", "ESP", "EST", "ETH", "FIN", "FJI", "FLK", "FRA", "FRO", "FSM",
    "GAB", "GBR", "GEO", "GGY", "GHA", "GIB", "GIN", "GLP", "GMB", "GNB", "GNQ", "GRC", "GRD",
    "GRL", "GTM", "GUF", "GUM", "GUY", "HKG", "HMD", "HND", "HRV", "HTI", "HUN", "IDN", "IMN",
    "IND", "IOT", "IRL", "IRN", "IRQ", "ISL", "ISR", "ITA", "JAM", "JEY", "JOR", "JPN", "KAZ",
    "KEN", "KGZ", "KHM", "KIR", "KNA", "KOR", "KWT", "LAO", "LBN", "LBR", "LBY", "LCA", "LIE",
    "LKA", "LSO", "LTU", "LUX", "LVA", "MAC", "MAF", "MAR", "MCO", "MDA", "MDG", "MDV", "MEX",
    "MHL", "MKD", "MLI", "MLT", "MMR", "MNE", "MNG", "MNP", "MOZ", "MRT", "MSR", "MTQ", "MUS",
    "MWI", "MYS", "MYT", "NAM", "NCL", "NER", "NFK", "NGA", "NIC", "NIU", "NLD", "NOR", "NPL",
    "NRU", "NZL", "OMN", "PAK", "PAN", "PCN", "PER", "PHL", "PLW", "PNG", "POL", "PRI", "PRK",
    "PRT", "PRY", "PSE", "PYF", "QAT", "REU", "ROU", "RUS", "RWA", "SAU", "SDN", "SEN", "SGP",
    "SGS", "SHN", "SJM", "SLB", "SLE", "SLV", "SMR", "SOM", "SPM", "SRB", "SSD", "STP", "SUR",
    "SVK", "SVN", "SWE", "SWZ", "SXM", "SYC", "SYR", "TCA", "TCD", "TGO", "THA", "TJK", "TKL",
    "TKM", "TLS", "TON", "TTO", "TUN", "TUR", "TUV", "TZA", "UGA", "UKR", "UMI", "URY", "USA",
    "UZB", "VAT", "VCT", "VEN", "VGB", "VIR", "VNM", "VUT", "WLF", "WSM", "YEM", "ZAF", "ZMB",
    "ZWE",
];

#[test]
fn every_m49_iso_alpha3_code_has_a_name() {
    let missing: Vec<&str> = M49_ISO_ALPHA3
        .iter()
        .copied()
        .filter(|c| mrz::country_name(c).is_none())
        .collect();
    assert!(missing.is_empty(), "codes with no name: {missing:?}");
}

#[test]
fn every_name_maps_back_to_a_code_that_names_it() {
    // code_for_name returns the first code in table order; for Germany and Kosovo that is
    // a different, equivalent code, so compare through codes_equivalent.
    for code in M49_ISO_ALPHA3 {
        let name = mrz::country_name(code).expect("covered by the test above");
        let back = mrz::code_for_name(name).expect("a name in the table maps back");
        assert!(
            mrz::codes_equivalent(code, back),
            "{code} -> {name} -> {back}"
        );
    }
}

#[test]
fn the_new_territory_codes_are_named_as_m49_prints_them() {
    assert_eq!(mrz::country_name("ALA"), Some("Åland Islands"));
    assert_eq!(
        mrz::country_name("FLK"),
        Some("Falkland Islands (Malvinas)")
    );
    assert_eq!(mrz::country_name("PRI"), Some("Puerto Rico"));
    assert_eq!(mrz::code_for_name("REUNION"), None); // names match case-insensitively, not accent-insensitively
    assert_eq!(mrz::code_for_name("Réunion"), Some("REU"));
}
