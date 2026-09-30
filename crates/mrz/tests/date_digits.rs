//! #579 (C6): `ParseOptions::date_digits`, the rule that a date field holds
//! digits.
//!
//! Check digits are arithmetic and letters have values too, so a zone whose
//! date of birth is six letters can pass every check digit while being no date
//! at all. With the option on, a read whose date of birth or date of expiry
//! holds a character that is neither an ASCII digit nor the filler `<` is not
//! `valid()`. The check digits themselves do not change, and a partially
//! unknown date (digits and fillers only, Doc 9303 Part 3 §4.8) stays valid.
//!
//! Every zone here comes from `format_td1` / `format_td3`, which compute the
//! check digits over whatever the date field holds, letters included, so each
//! zone passes every check digit by construction. Invented names and numbers;
//! no specimen text.

mod support;

use mrz::{
    find_and_parse_with, format_td1, format_td3, parse_td1_with, parse_td3_with, DateCompleteness,
    MrzData, ParseOptions, Sex, Td1Fields, Td3Fields,
};

fn off() -> ParseOptions {
    ParseOptions::default()
}

fn on() -> ParseOptions {
    ParseOptions::default().with_date_digits(true)
}

/// A TD1 zone (three lines) with the given raw date fields.
fn td1_zone(dob: &str, expiry: &str) -> String {
    format_td1(&Td1Fields {
        document_code: "I".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "X12345678".to_string(),
        optional_data_1: None,
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth(dob),
        sex: Sex::Female,
        date_of_expiry: support::expiry(expiry),
        optional_data_2: None,
    })
}

/// A TD3 zone (two lines) with the given raw date fields.
fn td3_zone(dob: &str, expiry: &str) -> String {
    format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "X12345678".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth(dob),
        sex: Sex::Female,
        date_of_expiry: support::expiry(expiry),
        personal_number: None,
    })
}

fn parse_td1(zone: &str, opts: &ParseOptions) -> MrzData {
    let lines: Vec<&str> = zone.lines().collect();
    parse_td1_with(lines[0], lines[1], lines[2], opts).expect("an emitted TD1 zone parses")
}

fn parse_td3(zone: &str, opts: &ParseOptions) -> MrzData {
    let lines: Vec<&str> = zone.lines().collect();
    parse_td3_with(lines[0], lines[1], opts).expect("an emitted TD3 zone parses")
}

/// What the option changes about one read, checked against the option off:
/// the check digits are identical, and `valid()` is `expected_off` / `expected_on`.
fn assert_valid_off_and_on(
    parse: impl Fn(&ParseOptions) -> MrzData,
    expected_off: bool,
    expected_on: bool,
    what: &str,
) {
    let without = parse(&off());
    let with_option = parse(&on());
    assert_eq!(without.valid(), expected_off, "{what}: option off");
    assert_eq!(with_option.valid(), expected_on, "{what}: option on");
    // The arithmetic is untouched: the option adds a condition, it does not
    // re-score a check digit.
    assert_eq!(without.checks, with_option.checks, "{what}: check digits");
    assert!(with_option.checksum_consistent(), "{what}: arithmetic");
}

#[test]
fn the_option_is_off_by_default_and_has_a_builder() {
    assert!(!ParseOptions::default().date_digits);
    assert!(ParseOptions::default().with_date_digits(true).date_digits);
    assert!(
        !ParseOptions::default()
            .with_date_digits(true)
            .with_date_digits(false)
            .date_digits
    );
}

#[test]
fn a_birth_date_of_letters_is_valid_off_and_not_valid_on_td1() {
    for dob in ["ABCDEF", "74O812"] {
        let zone = td1_zone(dob, "301230");
        let birth = parse_td1(&zone, &off()).date_of_birth;
        assert_eq!(
            birth.completeness(),
            DateCompleteness::Malformed,
            "sanity: {dob} is not a date"
        );
        assert_valid_off_and_on(|o| parse_td1(&zone, o), true, false, dob);
    }
}

#[test]
fn an_expiry_date_of_letters_is_valid_off_and_not_valid_on_td1() {
    for expiry in ["GHJKLM", "30I230"] {
        let zone = td1_zone("800101", expiry);
        let read = parse_td1(&zone, &off());
        assert_eq!(
            read.date_of_expiry.completeness(),
            DateCompleteness::Malformed
        );
        assert_valid_off_and_on(|o| parse_td1(&zone, o), true, false, expiry);
    }
}

#[test]
fn both_dates_of_a_two_line_format_are_covered() {
    let birth = td3_zone("ABCDEF", "301230");
    assert_valid_off_and_on(|o| parse_td3(&birth, o), true, false, "TD3 birth");
    let expiry = td3_zone("800101", "GHJKLM");
    assert_valid_off_and_on(|o| parse_td3(&expiry, o), true, false, "TD3 expiry");
}

#[test]
fn a_partially_unknown_date_stays_valid_under_both() {
    // Doc 9303 Part 3 §4.8: unknown positions are fillers. Not letters, so not
    // this rule's business.
    for field in ["<<<<<<", "19<<<<", "<<0101"] {
        let td1_birth = td1_zone(field, "301230");
        assert_valid_off_and_on(|o| parse_td1(&td1_birth, o), true, true, field);
        let td1_expiry = td1_zone("800101", field);
        assert_valid_off_and_on(|o| parse_td1(&td1_expiry, o), true, true, field);
        let td3 = td3_zone(field, "301230");
        assert_valid_off_and_on(|o| parse_td3(&td3, o), true, true, field);
    }
}

#[test]
fn an_ordinary_read_is_unaffected_by_the_option() {
    let td1 = td1_zone("800101", "301230");
    assert_valid_off_and_on(|o| parse_td1(&td1, o), true, true, "TD1 ordinary");
    let td3 = td3_zone("800101", "301230");
    assert_valid_off_and_on(|o| parse_td3(&td3, o), true, true, "TD3 ordinary");
}

/// A read whose check digits fail stays not valid: the option only adds a
/// condition, it never rescues.
#[test]
fn the_option_never_makes_an_invalid_read_valid() {
    let zone = td1_zone("800101", "301230");
    let lines: Vec<&str> = zone.lines().collect();
    let tampered_l2 = lines[1].replacen("800101", "800102", 1);
    for opts in [off(), on()] {
        let read = parse_td1_with(lines[0], &tampered_l2, lines[2], &opts).unwrap();
        assert!(!read.valid());
    }
}

/// Through the scanner: the option reaches `find_and_parse_with`, not only the
/// direct parsers. With it on, no valid read may hold a non-digit date, whether
/// the scan returns the as-read zone as checksum-failed, repairs it, or refuses.
#[test]
fn the_scanner_honours_the_option() {
    let zone = td1_zone("ABCDEF", "301230");

    let without = find_and_parse_with(&zone, &off()).expect("the default scan reads the zone");
    assert!(without.valid());
    assert_eq!(without.date_of_birth.to_string(), "ABCDEF");

    assert!(
        !with_option_is_valid_with_letters(&zone),
        "the manufactured zone must not be returned as a valid read"
    );

    let unknown = td1_zone("19<<<<", "301230");
    let read = find_and_parse_with(&unknown, &on()).expect("a partially unknown date reads");
    assert!(read.valid());
}

fn with_option_is_valid_with_letters(zone: &str) -> bool {
    find_and_parse_with(zone, &on()).is_ok_and(|read| {
        read.valid() && read.date_of_birth.completeness() == DateCompleteness::Malformed
    })
}
