//! MRZ line assembly for generated documents (TD1, TD2, TD3).
//!
//! This is a thin adapter over the canonical emitters in the `mrz` crate:
//! - [`mrz::format_td1`] for TD1 (ID cards)
//! - [`mrz::format_td2`] for TD2 (official travel documents)  
//! - [`mrz::format_td3`] for TD3 (passports)
//!
//! Each function converts the crate's structured [`Passport`] into the
//! MRZ-native fields and splits the emitter's output into the lines the
//! renderer and labels need. All check-digit math lives in the `mrz` crate
//! — this module owns none of it.

use mrz::{format_td1, format_td2, format_td3, Td1Fields, Td2Fields, Td3Fields};

use crate::model::{DocumentType, Passport};

/// `mrz::Date` → the MRZ-native `YYMMDD` string [`mrz::Td3Fields`] expects.
fn yymmdd(date: &mrz::Date) -> String {
    format!(
        "{:02}{:02}{:02}",
        date.year.rem_euclid(100),
        date.month,
        date.day
    )
}

/// Assemble the two TD3 MRZ lines for `passport` via [`mrz::format_td3`]. The
/// result parses back through [`mrz::parse_td3`] as fully valid.
pub fn build_td3_lines(passport: &Passport) -> (String, String) {
    let fields = Td3Fields {
        document_code: passport.document_type.clone(),
        issuing_country: passport.issuing_country.clone(),
        document_number: passport.document_number.clone(),
        surname: passport.surname.clone(),
        given_names: passport.given_names.clone(),
        nationality: passport.nationality.clone(),
        date_of_birth: yymmdd(&passport.date_of_birth),
        sex: passport.sex.as_mrz_char().to_string(),
        date_of_expiry: yymmdd(&passport.date_of_expiry),
        personal_number: passport.personal_number.clone(),
    };
    let emitted = format_td3(&fields);
    let (line1, line2) = emitted
        .split_once('\n')
        .expect("format_td3 always returns two newline-joined lines");
    (line1.to_string(), line2.to_string())
}

/// Assemble the three TD1 MRZ lines for `passport` via [`mrz::format_td1`].
/// TD1 is used for ID cards and has 3 lines of 30 characters each.
pub fn build_td1_lines(passport: &Passport) -> (String, String, String) {
    let fields = Td1Fields {
        document_code: DocumentType::TD1.document_code().to_string(),
        issuing_country: passport.issuing_country.clone(),
        document_number: passport.document_number.clone(),
        optional_data_1: None, // Not used in basic TD1 generation
        surname: passport.surname.clone(),
        given_names: passport.given_names.clone(),
        nationality: passport.nationality.clone(),
        date_of_birth: yymmdd(&passport.date_of_birth),
        sex: passport.sex.as_mrz_char().to_string(),
        date_of_expiry: yymmdd(&passport.date_of_expiry),
        optional_data_2: passport.personal_number.clone(), // TD1 uses optional_data_2 for personal number
    };
    let emitted = format_td1(&fields);
    let lines: Vec<&str> = emitted.split('\n').collect();
    assert_eq!(lines.len(), 3, "format_td1 should return three lines");
    (lines[0].to_string(), lines[1].to_string(), lines[2].to_string())
}

/// Assemble the two TD2 MRZ lines for `passport` via [`mrz::format_td2`].
/// TD2 is used for official travel documents and has 2 lines of 36 characters each.
pub fn build_td2_lines(passport: &Passport) -> (String, String) {
    let fields = Td2Fields {
        document_code: DocumentType::TD2.document_code().to_string(),
        issuing_country: passport.issuing_country.clone(),
        document_number: passport.document_number.clone(),
        surname: passport.surname.clone(),
        given_names: passport.given_names.clone(),
        nationality: passport.nationality.clone(),
        date_of_birth: yymmdd(&passport.date_of_birth),
        sex: passport.sex.as_mrz_char().to_string(),
        date_of_expiry: yymmdd(&passport.date_of_expiry),
        optional_data: passport.personal_number.clone(), // TD2 uses optional_data instead of personal_number
    };
    let emitted = format_td2(&fields);
    let (line1, line2) = emitted
        .split_once('\n')
        .expect("format_td2 always returns two newline-joined lines");
    (line1.to_string(), line2.to_string())
}

/// Build MRZ lines for the specified document type.
pub fn build_mrz_lines(passport: &Passport, doc_type: DocumentType) -> Vec<String> {
    match doc_type {
        DocumentType::TD1 => {
            let (l1, l2, l3) = build_td1_lines(passport);
            vec![l1, l2, l3]
        }
        DocumentType::TD2 => {
            let (l1, l2) = build_td2_lines(passport);
            vec![l1, l2]
        }
        DocumentType::TD3 => {
            let (l1, l2) = build_td3_lines(passport);
            vec![l1, l2]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GeneratorConfig, Sex};
    use mrz::Date;

    #[test]
    fn assembles_44_char_lines() {
        let p = Passport {
            document_type: "P".to_string(),
            issuing_country: "UTO".to_string(),
            surname: "ERIKSSON".to_string(),
            given_names: "ANNA MARIA".to_string(),
            document_number: "L898902C3".to_string(),
            nationality: "UTO".to_string(),
            date_of_birth: Date::new(1974, 8, 12),
            sex: Sex::F,
            date_of_expiry: Date::new(2012, 4, 15),
            personal_number: Some("ZE184226B".to_string()),
        };
        let (l1, l2) = build_td3_lines(&p);
        assert_eq!(l1.len(), 44);
        assert_eq!(l2.len(), 44);
        // Matches the official ICAO 9303 part 4 specimen exactly.
        assert_eq!(l1, "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<");
        assert_eq!(l2, "L898902C36UTO7408122F1204159ZE184226B<<<<<10");
    }

    #[test]
    fn parses_back_valid_for_generated_identities() {
        for seed in 0..50u64 {
            let cfg = GeneratorConfig::new(seed);
            let p = crate::data::generate_passport(&cfg);
            let (l1, l2) = build_td3_lines(&p);
            let parsed = mrz::parse_td3(&l1, &l2).expect("parses");
            assert!(parsed.valid(), "seed {seed}: checks {:?}", parsed.checks);
        }
    }
}
