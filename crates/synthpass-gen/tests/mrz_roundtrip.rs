//! Keystone correctness test: the MRZ extracted from `Labels` must parse back
//! through the appropriate mrz parser as fully checksum-valid, and every parsed field
//! must equal the generated `Passport` field it came from.

use synthpass_gen::{data::generate_passport, generate, DocumentType, GeneratorConfig};

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
            format!(
                "{:04}-{:02}-{:02}",
                passport.date_of_birth.year,
                passport.date_of_birth.month,
                passport.date_of_birth.day
            ),
            "seed {seed}"
        );
        assert_eq!(
            parsed.sex,
            passport.sex.as_mrz_char().to_string(),
            "seed {seed}"
        );
        assert_eq!(
            parsed.date_of_expiry,
            format!(
                "{:04}-{:02}-{:02}",
                passport.date_of_expiry.year,
                passport.date_of_expiry.month,
                passport.date_of_expiry.day
            ),
            "seed {seed}"
        );
        assert_eq!(
            parsed.personal_number, passport.personal_number,
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
    assert_eq!(parsed.personal_number, None);
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
