//! Keystone correctness test: the MRZ extracted from `Labels` must parse back
//! through the appropriate mrz parser as fully checksum-valid, and every parsed field
//! must equal the generated `Passport` field it came from.

use synthpass_gen::{data::generate_passport, generate, DocumentType, GeneratorConfig};

/// What the zone can carry of a painted value: `mrz::emit` pads or truncates
/// every field to its exact width, and the generator draws a 14-character
/// personal number for every format (`data::random_personal_number`), so on
/// the formats whose optional-data slot is narrower than 14 the zone holds a
/// prefix of the label — TD1's second element (11), TD2's element (7),
/// MRV-B's (8). TD3 (14) and MRV-A (16) carry it whole. Pinned as a fact
/// about the generator, not fixed here: shortening the value would change
/// every generated corpus and export.
fn as_emitted(value: Option<&str>, width: usize) -> Option<String> {
    value.map(|v| v.chars().take(width).collect())
}

#[test]
fn generated_mrz_round_trips_through_mrz_crate() {
    for seed in 0..100u64 {
        let cfg = GeneratorConfig::new(seed);
        let passport = generate_passport(&cfg);
        let (_image, labels) = generate(&passport, &cfg);

        let mrz_string = labels.mrz_string();
        let mut lines = mrz_string.lines();
        let line1 = lines.next().expect("line1");
        let line2 = lines.next().expect("line2");

        let parsed = mrz::parse_td3(line1, line2)
            .unwrap_or_else(|e| panic!("seed {seed}: MRZ failed to parse: {e}"));
        assert!(
            parsed.valid(),
            "seed {seed}: MRZ parsed but not checksum-valid: {:?}",
            parsed.checks
        );
        assert_eq!(parsed.document_type, passport.document_type, "seed {seed}");
        assert_eq!(
            parsed.issuing_country, passport.issuing_country,
            "seed {seed}"
        );
        assert_eq!(parsed.surname, passport.surname, "seed {seed}");
        assert_eq!(parsed.given_names, passport.given_names, "seed {seed}");
        assert_eq!(
            parsed.document_number, passport.document_number,
            "seed {seed}"
        );
        assert_eq!(parsed.nationality, passport.nationality, "seed {seed}");
        assert_eq!(
            parsed.date_of_birth,
            mrz::MrzDate::Calendar(passport.date_of_birth),
            "seed {seed}"
        );
        // The emitter writes `<` for the generator's X.
        let expected_sex = match passport.sex {
            synthpass_gen::Sex::M => mrz::Sex::Male,
            synthpass_gen::Sex::F => mrz::Sex::Female,
            synthpass_gen::Sex::X => mrz::Sex::Unspecified,
        };
        assert_eq!(parsed.sex, expected_sex, "seed {seed}");
        assert_eq!(
            parsed.date_of_expiry,
            mrz::MrzDate::Calendar(passport.date_of_expiry),
            "seed {seed}"
        );
        // TD3 is the one format that prints a personal number; the accessor
        // names the primary optional-data slot on it (ADR-0018).
        assert_eq!(
            parsed.personal_number(),
            passport.personal_number.as_deref(),
            "seed {seed}"
        );
    }
}

#[test]
fn generated_td1_mrz_round_trips() {
    for seed in 0..50u64 {
        let cfg = GeneratorConfig::with_document_type(seed, DocumentType::TD1);
        let passport = generate_passport(&cfg);
        let (_image, labels) = generate(&passport, &cfg);

        let mrz_string = labels.mrz_string();
        let mut lines = mrz_string.lines();
        let line1 = lines.next().expect("line1");
        let line2 = lines.next().expect("line2");
        let line3 = lines.next().expect("line3");

        let parsed = mrz::parse_td1(line1, line2, line3)
            .unwrap_or_else(|e| panic!("seed {seed}: TD1 MRZ failed to parse: {e}"));
        assert!(
            parsed.valid(),
            "seed {seed}: TD1 MRZ parsed but not checksum-valid: {:?}",
            parsed.checks
        );
        assert_eq!(parsed.surname, passport.surname, "seed {seed}");
        assert_eq!(parsed.given_names, passport.given_names, "seed {seed}");
        // The generator writes its personal number into TD1's *second*
        // optional-data element (`mrz_line.rs`), truncated to its 11
        // characters, and a TD1 prints no personal number, so the accessor is
        // `None` (ADR-0018).
        assert_eq!(
            parsed.optional_data_2,
            as_emitted(passport.personal_number.as_deref(), 11),
            "seed {seed}"
        );
        assert_eq!(parsed.personal_number(), None, "seed {seed}");
    }
}

#[test]
fn generated_td2_mrz_round_trips() {
    for seed in 0..50u64 {
        let cfg = GeneratorConfig::with_document_type(seed, DocumentType::TD2);
        let passport = generate_passport(&cfg);
        let (_image, labels) = generate(&passport, &cfg);

        let mrz_string = labels.mrz_string();
        let mut lines = mrz_string.lines();
        let line1 = lines.next().expect("line1");
        let line2 = lines.next().expect("line2");

        let parsed = mrz::parse_td2(line1, line2)
            .unwrap_or_else(|e| panic!("seed {seed}: TD2 MRZ failed to parse: {e}"));
        assert!(
            parsed.valid(),
            "seed {seed}: TD2 MRZ parsed but not checksum-valid: {:?}",
            parsed.checks
        );
        assert_eq!(parsed.surname, passport.surname, "seed {seed}");
        assert_eq!(parsed.given_names, passport.given_names, "seed {seed}");
        // TD2 prints one optional-data element, the primary slot, 7 wide; it
        // is not a personal number (ADR-0018).
        assert_eq!(
            parsed.optional_data_1,
            as_emitted(passport.personal_number.as_deref(), 7),
            "seed {seed}"
        );
        assert_eq!(parsed.optional_data_2, None, "seed {seed}");
        assert_eq!(parsed.personal_number(), None, "seed {seed}");
    }
}

#[test]
fn generated_mrva_mrz_round_trips() {
    for seed in 0..50u64 {
        let cfg = GeneratorConfig::with_document_type(seed, DocumentType::MrvA);
        let passport = generate_passport(&cfg);
        let (_image, labels) = generate(&passport, &cfg);

        let mrz_string = labels.mrz_string();
        let mut lines = mrz_string.lines();
        let line1 = lines.next().expect("line1");
        let line2 = lines.next().expect("line2");

        let parsed = mrz::parse_mrv_a(line1, line2)
            .unwrap_or_else(|e| panic!("seed {seed}: MRV-A MRZ failed to parse: {e}"));
        assert!(
            parsed.valid(),
            "seed {seed}: MRV-A MRZ parsed but not checksum-valid: {:?}",
            parsed.checks
        );
        assert_eq!(parsed.surname, passport.surname, "seed {seed}");
        assert_eq!(parsed.given_names, passport.given_names, "seed {seed}");
        // MRV-A's one optional-data element is the primary slot, 16 wide, so
        // it carries the whole value (ADR-0018).
        assert_eq!(
            parsed.optional_data_1.as_deref(),
            passport.personal_number.as_deref(),
            "seed {seed}"
        );
        assert_eq!(parsed.optional_data_2, None, "seed {seed}");
        assert_eq!(parsed.personal_number(), None, "seed {seed}");
    }
}

#[test]
fn generated_mrvb_mrz_round_trips() {
    for seed in 0..50u64 {
        let cfg = GeneratorConfig::with_document_type(seed, DocumentType::MrvB);
        let passport = generate_passport(&cfg);
        let (_image, labels) = generate(&passport, &cfg);

        let mrz_string = labels.mrz_string();
        let mut lines = mrz_string.lines();
        let line1 = lines.next().expect("line1");
        let line2 = lines.next().expect("line2");

        let parsed = mrz::parse_mrv_b(line1, line2)
            .unwrap_or_else(|e| panic!("seed {seed}: MRV-B MRZ failed to parse: {e}"));
        assert!(
            parsed.valid(),
            "seed {seed}: MRV-B MRZ parsed but not checksum-valid: {:?}",
            parsed.checks
        );
        assert_eq!(parsed.surname, passport.surname, "seed {seed}");
        assert_eq!(parsed.given_names, passport.given_names, "seed {seed}");
        // MRV-B's one optional-data element is the primary slot, 8 wide
        // (ADR-0018).
        assert_eq!(
            parsed.optional_data_1,
            as_emitted(passport.personal_number.as_deref(), 8),
            "seed {seed}"
        );
        assert_eq!(parsed.optional_data_2, None, "seed {seed}");
        assert_eq!(parsed.personal_number(), None, "seed {seed}");
    }
}

#[test]
fn generated_mrz_round_trips_without_personal_number() {
    let cfg = GeneratorConfig {
        seed: 999,
        document_type: DocumentType::TD3,
        include_personal_number: false,
    };
    let passport = generate_passport(&cfg);
    assert!(passport.personal_number.is_none());
    let (_image, labels) = generate(&passport, &cfg);
    let mrz_string = labels.mrz_string();
    let mut lines = mrz_string.lines();
    let parsed = mrz::parse_td3(lines.next().unwrap(), lines.next().unwrap()).unwrap();
    assert!(parsed.valid(), "checks: {:?}", parsed.checks);
    assert_eq!(parsed.personal_number(), None);
    assert_eq!(parsed.optional_data_1, None);
}

/// Cyrillic-script identities: the generator draws a native-script name, stores
/// its ICAO 9303 Part 3 §6 B transliteration (computed with the issuing
/// state's language) as the Latin `surname`/`given_names`, and the MRZ carries
/// that Latin form checksum-valid. This is the round-trip that makes §6 B
/// measurable — before it, nothing exercised `transliterate_cyrillic` end to
/// end.
#[test]
fn cyrillic_identities_transliterate_and_round_trip() {
    let lang_for = |code: &str| -> Option<mrz::CyrillicLanguage> {
        use mrz::CyrillicLanguage::*;
        Some(match code {
            "RUS" => Russian,
            "BLR" => Belarusian,
            "BGR" => Bulgarian,
            "SRB" => Serbian,
            "UKR" => Ukrainian,
            "MKD" => Macedonian,
            _ => return None,
        })
    };

    let mut seen_cyrillic = 0;
    let mut seen_languages = std::collections::BTreeSet::new();

    for seed in 0..400u64 {
        let cfg = GeneratorConfig::new(seed);
        let passport = generate_passport(&cfg);
        let (_image, labels) = generate(&passport, &cfg);

        let Some(lang) = lang_for(&passport.issuing_country) else {
            // Latin-script identity: no native name, `surname` is the printed form.
            assert!(passport.surname_native.is_none(), "seed {seed}");
            assert!(passport.given_names_native.is_none(), "seed {seed}");
            continue;
        };
        seen_cyrillic += 1;
        seen_languages.insert(passport.issuing_country.clone());

        let sn_native = passport.surname_native.as_ref().expect("native surname");
        let gn_native = passport
            .given_names_native
            .as_ref()
            .expect("native given names");

        // The Latin fields are exactly the §6 B transliteration of the native
        // ones, and pure [A-Z].
        assert_eq!(
            passport.surname,
            mrz::transliterate_cyrillic(sn_native, lang),
            "seed {seed} ({})",
            passport.issuing_country
        );
        assert_eq!(
            passport.given_names,
            mrz::transliterate_cyrillic(gn_native, lang),
            "seed {seed}"
        );
        assert!(
            passport.surname.chars().all(|c| c.is_ascii_uppercase())
                && passport.given_names.chars().all(|c| c.is_ascii_uppercase()),
            "seed {seed}: romanized name is not [A-Z]: {:?} / {:?}",
            passport.surname,
            passport.given_names
        );

        // Labels carry the native strings on the same VIZ rects.
        assert_eq!(labels.surname_native.as_ref().unwrap().value, *sn_native);
        assert_eq!(
            labels.surname_native.as_ref().unwrap().rect,
            labels.surname.rect
        );

        // The MRZ round-trips checksum-valid and reads back the Latin surname.
        let mrz_string = labels.mrz_string();
        let mut ls = mrz_string.lines();
        let parsed = mrz::parse_td3(ls.next().unwrap(), ls.next().unwrap())
            .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        assert!(parsed.valid(), "seed {seed}: checks {:?}", parsed.checks);
        assert_eq!(parsed.surname, passport.surname, "seed {seed}");
        assert_eq!(parsed.given_names, passport.given_names, "seed {seed}");
    }

    assert!(
        seen_cyrillic >= 20,
        "expected a meaningful share of Cyrillic identities in 400 seeds, got {seen_cyrillic}"
    );
    assert_eq!(
        seen_languages.len(),
        6,
        "all six Cyrillic issuing states should appear in 400 seeds: {seen_languages:?}"
    );
}
