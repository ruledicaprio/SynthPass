//! The typed date and sex values of ADR-0019, and the text contract of
//! ADR-0020: `Display`, `FromStr` (and serde, through them) and the zone agree.
//!
//! Two properties here carry the migration, not just the types:
//!
//! - `MrzDate::from_field(f).to_string()` equals `expand_date_with_pivot(f)`
//!   for every six-character charset field, both roles and every pivot. That is
//!   the proof that swapping `MrzData`'s `String` dates for `MrzDate` leaves
//!   every date's text byte-identical.
//! - `from_str(to_string(x)) == x` for every value the parser can produce. That
//!   is what makes the text form lossless, so a consumer reading JSON loses
//!   nothing the type knows.

use mrz::{
    date_completeness, expand_date_with_pivot, Date, DateCompleteness, DateRole, MrzDate,
    RawDateField, Sex,
};
use proptest::prelude::*;

const MRZ_ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ<";

/// Six MRZ-charset characters, weighted towards the shapes that exercise each
/// variant: all digits (calendar and out-of-calendar), digits and fillers
/// (partially unknown and unknown), and the whole alphabet (malformed).
fn field_text() -> impl Strategy<Value = String> {
    prop_oneof![
        "[0-9]{6}",
        "[0-9<]{6}",
        "[0-9A-Z<]{6}",
        Just("<<<<<<".to_string()),
    ]
}

fn role() -> impl Strategy<Value = DateRole> {
    prop_oneof![Just(DateRole::Birth), Just(DateRole::Expiry)]
}

fn field(text: &str) -> RawDateField {
    RawDateField::try_from(text).expect("the strategy yields six charset characters")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4096))]

    #[test]
    fn display_is_byte_identical_to_expand_date_with_pivot(
        text in field_text(), role in role(), pivot in 0u32..=99,
    ) {
        let date = MrzDate::from_field(field(&text), role, pivot);
        let expected = expand_date_with_pivot(&text, role == DateRole::Birth, pivot);
        prop_assert_eq!(date.to_string(), expected);
    }

    #[test]
    fn text_form_round_trips(text in field_text(), role in role(), pivot in 0u32..=99) {
        let date = MrzDate::from_field(field(&text), role, pivot);
        prop_assert_eq!(date.to_string().parse::<MrzDate>(), Ok(date));
    }

    #[test]
    fn completeness_matches_the_raw_field(text in field_text(), role in role(), pivot in 0u32..=99) {
        let date = MrzDate::from_field(field(&text), role, pivot);
        prop_assert_eq!(date.completeness(), date_completeness(&text));
    }

    #[test]
    fn calendar_is_some_exactly_for_well_formed_six_digit_fields(
        text in "[0-9]{6}", role in role(), pivot in 0u32..=99,
    ) {
        let date = MrzDate::from_field(field(&text), role, pivot);
        match date.calendar() {
            Some(d) => {
                prop_assert!(d.is_well_formed());
                prop_assert!(matches!(date, MrzDate::Calendar(_)));
            }
            None => prop_assert!(matches!(date, MrzDate::OutOfCalendar(d) if !d.is_well_formed())),
        }
    }
}

#[test]
fn each_variant_from_its_field() {
    let birth = |text: &str| MrzDate::from_field(field(text), DateRole::Birth, 26);
    assert_eq!(birth("740812"), MrzDate::Calendar(Date::new(1974, 8, 12)));
    assert_eq!(birth("260101"), MrzDate::Calendar(Date::new(2026, 1, 1))); // at the pivot
    assert_eq!(birth("270101"), MrzDate::Calendar(Date::new(1927, 1, 1))); // past it
    assert_eq!(
        birth("000000"),
        MrzDate::OutOfCalendar(Date::new(2000, 0, 0))
    );
    assert_eq!(
        birth("110229"),
        MrzDate::OutOfCalendar(Date::new(2011, 2, 29))
    );
    assert_eq!(birth("74<<12"), MrzDate::PartiallyUnknown(field("74<<12")));
    assert_eq!(birth("<<<<<<"), MrzDate::Unknown);
    assert_eq!(birth("R38473"), MrzDate::Malformed(field("R38473")));

    // Expiry is always 20xx, whatever the pivot.
    let expiry = MrzDate::from_field(field("940623"), DateRole::Expiry, 26);
    assert_eq!(expiry, MrzDate::Calendar(Date::new(2094, 6, 23)));
}

#[test]
fn out_of_calendar_text_parses_back_instead_of_being_rejected() {
    // The crate's internal ISO parser rejects this by design; the text form
    // must not, or a date the parser produced could not be read back.
    assert_eq!(
        "2013-45-99".parse::<MrzDate>(),
        Ok(MrzDate::OutOfCalendar(Date::new(2013, 45, 99)))
    );
}

#[test]
fn text_that_no_date_renders_as_is_rejected() {
    for text in [
        "",
        "740812",      // six digits: no century without a pivot
        "1974-8-12",   // not zero-padded
        "1974/08/12",  // wrong separators
        "74<<1",       // five characters
        "74<<123",     // seven
        "74a812",      // lowercase is not the MRZ alphabet
        "74é812",      // nor is anything beyond ASCII
        "19740-08-12", // an ISO year is four digits
    ] {
        assert!(text.parse::<MrzDate>().is_err(), "{text:?} must not parse");
    }
}

#[test]
fn raw_field_admits_exactly_six_charset_characters() {
    assert_eq!(field("74<<12").as_str(), "74<<12");
    assert_eq!(field("74<<12").to_string(), "74<<12");
    for text in ["74<<1", "74<<123", "74a812", "74 812", "74é81"] {
        assert!(
            RawDateField::try_from(text).is_err(),
            "{text:?} must be rejected"
        );
    }
}

#[test]
fn completeness_of_each_variant() {
    let birth = |text: &str| MrzDate::from_field(field(text), DateRole::Birth, 26);
    assert_eq!(birth("740812").completeness(), DateCompleteness::Complete);
    assert_eq!(birth("000000").completeness(), DateCompleteness::Complete);
    assert_eq!(
        birth("74<<12").completeness(),
        DateCompleteness::PartiallyUnknown
    );
    assert_eq!(birth("<<<<<<").completeness(), DateCompleteness::Unknown);
    assert_eq!(birth("74O812").completeness(), DateCompleteness::Malformed);
}

#[test]
fn sex_round_trips_over_the_whole_alphabet() {
    for c in MRZ_ALPHABET.chars() {
        let sex = Sex::from_zone(c);
        assert_eq!(sex.zone_char(), c);
        assert_eq!(sex.to_string(), c.to_string());
        assert_eq!(sex.to_string().parse::<Sex>(), Ok(sex));
    }
}

#[test]
fn sex_keeps_the_three_icao_values_apart_from_everything_else() {
    assert_eq!(Sex::from_zone('M'), Sex::Male);
    assert_eq!(Sex::from_zone('F'), Sex::Female);
    assert_eq!(Sex::from_zone('<'), Sex::Unspecified);
    // `X` is the visual zone's letter for unspecified. In the MRZ it is not
    // conformant, so it is kept as read rather than reinterpreted.
    assert_eq!(Sex::from_zone('X'), Sex::NonConformant('X'));
    assert_eq!(Sex::from_zone('1'), Sex::NonConformant('1'));
    assert!("".parse::<Sex>().is_err());
    assert!("MF".parse::<Sex>().is_err());
    for text in ["a", "é", " "] {
        assert!(text.parse::<Sex>().is_err(), "{text:?}");
    }
}

#[cfg(feature = "serde")]
mod serde_form {
    use super::*;

    #[test]
    fn dates_serialise_as_their_display_string_and_back() {
        for text in ["740812", "000000", "74<<12", "<<<<<<", "R38473"] {
            let date = MrzDate::from_field(field(text), DateRole::Birth, 26);
            let json = serde_json::to_string(&date).unwrap();
            assert_eq!(json, format!("\"{date}\""));
            assert_eq!(serde_json::from_str::<MrzDate>(&json).unwrap(), date);
        }
    }

    #[test]
    fn sex_serialises_as_its_zone_character_and_back() {
        for (sex, json) in [
            (Sex::Male, "\"M\""),
            (Sex::Female, "\"F\""),
            (Sex::Unspecified, "\"<\""),
            (Sex::NonConformant('1'), "\"1\""),
        ] {
            assert_eq!(serde_json::to_string(&sex).unwrap(), json);
            assert_eq!(serde_json::from_str::<Sex>(json).unwrap(), sex);
        }
    }

    #[test]
    fn malformed_text_is_a_deserialisation_error_not_a_panic() {
        assert!(serde_json::from_str::<MrzDate>("\"740812\"").is_err());
        assert!(serde_json::from_str::<MrzDate>("{\"Calendar\":1}").is_err());
        assert!(serde_json::from_str::<Sex>("\"MF\"").is_err());
    }
}

#[cfg(feature = "zeroize")]
mod wipe {
    use super::*;
    use zeroize::Zeroize;

    // `ZeroizeOnDrop` cannot be observed without reading freed memory, but
    // `Zeroize` can: these assert the wipe the drop-time derive relies on.

    #[test]
    fn a_date_wipes_its_components() {
        let mut d = Date::new(1974, 8, 12);
        d.zeroize();
        assert_eq!(d, Date::new(0, 0, 0));
    }

    #[test]
    fn every_date_variant_wipes_its_payload_and_keeps_its_kind() {
        let mut calendar = MrzDate::Calendar(Date::new(1974, 8, 12));
        calendar.zeroize();
        assert_eq!(calendar, MrzDate::Calendar(Date::new(0, 0, 0)));

        let mut out = MrzDate::OutOfCalendar(Date::new(2013, 45, 99));
        out.zeroize();
        assert_eq!(out, MrzDate::OutOfCalendar(Date::new(0, 0, 0)));

        for text in ["74<<12", "R38473"] {
            let mut date = MrzDate::from_field(field(text), DateRole::Birth, 26);
            date.zeroize();
            match date {
                MrzDate::PartiallyUnknown(raw) | MrzDate::Malformed(raw) => {
                    assert_eq!(raw.as_str(), "\0\0\0\0\0\0");
                }
                other => panic!("zeroize changed the variant: {other:?}"),
            }
        }
    }

    #[test]
    fn a_kept_sex_character_is_wiped() {
        let mut sex = Sex::NonConformant('1');
        sex.zeroize();
        assert_eq!(sex, Sex::NonConformant('\0'));
    }

    #[test]
    fn every_sex_value_is_overwritten() {
        for mut sex in [
            Sex::Male,
            Sex::Female,
            Sex::Unspecified,
            Sex::NonConformant('1'),
        ] {
            sex.zeroize();
            assert_eq!(sex, Sex::NonConformant('\0'));
        }
    }
}
