//! Regressions for `shift_line1_right_at_country` (`crates/mrz/src/parser.rs`),
//! wired into `repair_td3_line1`/`repair_mrv_line1` as a third line-1
//! candidate alongside `unshift_line1_prefix`.
//!
//! `crates/synthpass-ocr/examples/integrity_survey.rs --dir passports
//! --mrz-only` (2026-08-17) found 34 of 57 `UnrecognizedIssuingCountry`
//! findings across real specimens sharing one exact shape — a single
//! garbage character inserted right after line 1's position-1 filler,
//! pushing `issuing_country`'s real third letter into the name field. The
//! mirror image of `line1_prefix_shift.rs`'s dropped-filler case; see
//! `shift_line1_right_at_country`'s doc comment for why this one needs a
//! content-based (`country_name`) guard instead of a shape-only one.
//!
//! **Fixtures are emitted, not transcribed**, same discipline
//! `line1_prefix_shift.rs` follows.

use mrz::{
    find_and_parse, format_mrv_a, format_mrv_b, format_td3, Format, MrvAFields, MrvBFields,
    Td3Fields,
};

/// Insert one spurious character right after line 1's position-1 filler,
/// then truncate back to the original width — reproducing the measured OCR
/// failure shape: the line stays the same length overall (the real third
/// `issuing_country` letter and everything after it shifts one position
/// right, and the last character, ordinarily a name-field filler, falls off
/// the end), same as `variants()`/`fit_length` sees it in production.
fn insert_after_position_1(line1: &str) -> String {
    let mut damaged = line1.to_string();
    damaged.insert(2, 'S');
    damaged.truncate(line1.len());
    damaged
}

#[test]
fn td3_recovers_issuing_country_after_an_inserted_position_2_character() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E000000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");

    let damaged = insert_after_position_1(l1);
    let text = format!("{damaged}\n{l2}");
    let data = find_and_parse(&text).expect("an inserted position-2 character must still parse");

    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.document_type, "P");
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid(), "line 2's real check digits are untouched");
}

#[test]
fn mrv_a_recovers_issuing_country_after_an_inserted_position_2_character() {
    let mrz = format_mrv_a(&MrvAFields {
        document_code: "V".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E00000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_mrv_a emits two lines");

    let damaged = insert_after_position_1(l1);
    let text = format!("{damaged}\n{l2}");
    let data = find_and_parse(&text).expect("an inserted position-2 character must still parse");

    assert_eq!(data.format, Format::MrvA);
    assert_eq!(data.document_type, "V");
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid());
}

#[test]
fn mrv_b_recovers_issuing_country_after_an_inserted_position_2_character() {
    let mrz = format_mrv_b(&MrvBFields {
        document_code: "V".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E0000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_mrv_b emits two lines");

    let damaged = insert_after_position_1(l1);
    let text = format!("{damaged}\n{l2}");
    let data = find_and_parse(&text).expect("an inserted position-2 character must still parse");

    assert_eq!(data.format, Format::MrvB);
    assert_eq!(data.document_type, "V");
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid());
}

/// The guard case: an undamaged zone (a real, resolvable `issuing_country`
/// already at position 2) must round-trip unchanged —
/// `shift_line1_right_at_country` must never fire when there's nothing to
/// shift. This is the test that would catch a discriminator false-positive,
/// since (unlike the dropped-filler case) this corruption leaves position 1
/// looking identical to an undamaged line.
#[test]
fn an_undamaged_td3_zone_is_not_spuriously_shifted() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E000000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");

    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("emitted TD3 parses");
    assert_eq!(data.document_type, "P");
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid());
}

/// Canada and — effective 15 December 2025 — Cyprus print an ordinary
/// citizen passport as `PP`, a genuine two-real-letter document code (see
/// `synthpass_core::fusion::document_types_agree`'s doc comment). This pins
/// that `shift_line1_right_at_country`'s position-1-is-`<` guard correctly
/// leaves it alone: position 1 is `P`, not `<`, so there's no filler
/// boundary for a spurious insertion to hide behind here in the first
/// place.
#[test]
fn a_genuine_two_letter_document_code_is_unaffected() {
    let mrz = format_td3(&Td3Fields {
        document_code: "PP".to_string(),
        issuing_country: "CAN".to_string(),
        document_number: "E000000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "CAN".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");

    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("emitted TD3 parses");
    assert_eq!(
        data.document_type, "PP",
        "a genuine two-letter document code must survive untouched"
    );
    assert_eq!(data.issuing_country, "CAN");
}
