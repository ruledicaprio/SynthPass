//! `format_td3`/`format_td2`/`format_td1` round-trip tests: each emitter is
//! correct iff it is the exact inverse of its matching `parse_*` function.

use mrz::{
    check_digit, format_mrv_a, format_mrv_b, format_td1, format_td2, format_td3, parse_mrv_a,
    parse_mrv_b, parse_td1, parse_td2, parse_td3, MrvAFields, MrvBFields, Td1Fields, Td2Fields,
    Td3Fields,
};
use proptest::prelude::*;

// ---- Document-number overflow sweeps (ICAO 9303 Part 5/6 note j; TD3 by
// analogy — see `parser::read_overflow`'s doc comment) ----
//
// The overflow encoding fires when `document_number` exceeds the 9-char
// field AND `remainder (len - 9) + check digit + terminating filler` fits
// the format's optional-data field. `optional_width` below is that field's
// width for each format (TD3 personal_number=14, TD2 optional_data=7, TD1
// optional_data_1=15) — the same widths `src/emit.rs::doc_number` uses, so a
// length overflows iff `len > 9 && len <= optional_width + 7`.
fn overflows(len: usize, optional_width: usize) -> bool {
    len > 9 && len <= optional_width + 7
}

/// A document number of exactly `len` MRZ-charset characters, letters and
/// digits alternating so it isn't accidentally palindromic or degenerate.
fn doc_number_of_len(len: usize) -> String {
    (0..len)
        .map(|i| if i % 2 == 0 { b'A' + (i as u8 % 26) } else { b'0' + (i as u8 % 10) } as char)
        .collect()
}

// ICAO 9303 specimen identity (Utopia / Anna Maria Eriksson) — same constants
// as the ones pinned in `src/lib.rs`'s test module, which has the full
// provenance note (Part 4's own copy is a figure, not extracted text;
// corroborated via Part 6's literal TD2 specimen).
const TD3_L1: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
const TD3_L2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";

#[test]
fn specimen_byte_for_byte() {
    let fields = Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "L898902C3".to_string(),
        surname: "ERIKSSON".to_string(),
        given_names: "ANNA MARIA".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: "740812".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "120415".to_string(),
        personal_number: Some("ZE184226B".to_string()),
    };

    let expected = format!("{TD3_L1}\n{TD3_L2}");
    assert_eq!(format_td3(&fields), expected);
}

#[test]
fn specimen_round_trips_as_valid() {
    let fields = Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "L898902C3".to_string(),
        surname: "ERIKSSON".to_string(),
        given_names: "ANNA MARIA".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: "740812".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "120415".to_string(),
        personal_number: Some("ZE184226B".to_string()),
    };

    let mrz = format_td3(&fields);
    let (l1, l2) = mrz.split_once('\n').unwrap();
    let d = parse_td3(l1, l2).unwrap();
    assert!(d.valid(), "checks: {:?}", d.checks);
}

fn doc_number_strategy() -> impl Strategy<Value = String> {
    "[A-Z0-9]{1,9}"
}

fn name_strategy() -> impl Strategy<Value = String> {
    // Capped so surname + "<<" + given_names never exceeds the 39-char name
    // field (18 + 2 + 18 = 38 <= 39) — an over-long name is a truncation
    // concern, not a check-digit concern, and out of scope for M1.
    "[A-Z]{1,18}"
}

fn yymmdd_strategy() -> impl Strategy<Value = String> {
    // Keep month/day within always-valid ranges so `expand_date` accepts
    // them cleanly (no plausibility rejection to work around here).
    (0u32..100, 1u32..=12, 1u32..=28).prop_map(|(yy, mm, dd)| format!("{yy:02}{mm:02}{dd:02}"))
}

fn sex_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("M".to_string()),
        Just("F".to_string()),
        Just("X".to_string())
    ]
}

fn personal_number_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "[A-Z0-9]{1,14}".prop_map(Some),]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn arbitrary_fields_round_trip(
        document_number in doc_number_strategy(),
        surname in name_strategy(),
        given_names in name_strategy(),
        date_of_birth in yymmdd_strategy(),
        sex in sex_strategy(),
        date_of_expiry in yymmdd_strategy(),
        personal_number in personal_number_strategy(),
    ) {
        let fields = Td3Fields {
            document_code: "P".to_string(),
            issuing_country: "UTO".to_string(),
            document_number: document_number.clone(),
            surname: surname.clone(),
            given_names: given_names.clone(),
            nationality: "UTO".to_string(),
            date_of_birth: date_of_birth.clone(),
            sex: sex.clone(),
            date_of_expiry: date_of_expiry.clone(),
            personal_number: personal_number.clone(),
        };

        let mrz = format_td3(&fields);
        let (l1, l2) = mrz.split_once('\n').unwrap();
        prop_assert_eq!(l1.len(), 44);
        prop_assert_eq!(l2.len(), 44);

        let parsed = parse_td3(l1, l2).unwrap();
        prop_assert!(parsed.valid(), "checks: {:?}", parsed.checks);

        prop_assert_eq!(&parsed.document_number, &document_number);
        prop_assert_eq!(&parsed.surname, &surname);
        prop_assert_eq!(&parsed.given_names, &given_names);
        prop_assert_eq!(&parsed.sex, &sex);

        let expected_personal = personal_number.filter(|s| !s.is_empty());
        // Clone rather than move: `MrzData` derives `ZeroizeOnDrop` when the
        // workspace unifies `mrz`'s `zeroize` feature on (e.g. via
        // `synthpass-pipeline`), and a `Drop` type forbids partial moves out of it.
        prop_assert_eq!(parsed.personal_number.clone(), expected_personal);

        // `expand_date` turns YYMMDD into ISO YYYY-MM-DD; check the tail
        // (MM-DD) and that the parsed year's last two digits match.
        let expected_mmdd = &date_of_birth[2..6];
        prop_assert_eq!(&parsed.date_of_birth[5..7], &expected_mmdd[0..2]);
        prop_assert_eq!(&parsed.date_of_birth[8..10], &expected_mmdd[2..4]);
        prop_assert_eq!(&parsed.date_of_birth[2..4], &date_of_birth[0..2]);

        let expected_mmdd_exp = &date_of_expiry[2..6];
        prop_assert_eq!(&parsed.date_of_expiry[5..7], &expected_mmdd_exp[0..2]);
        prop_assert_eq!(&parsed.date_of_expiry[8..10], &expected_mmdd_exp[2..4]);
        prop_assert_eq!(&parsed.date_of_expiry[2..4], &date_of_expiry[0..2]);
    }
}

// Official ICAO 9303 part 6 (TD2) specimen (Utopia / Anna Maria Eriksson),
// published verbatim as text
// (`knowledge/docs9303/Doc_9303_Part6_Specs_for_TD2_MROTDs.md:487-488`) —
// same constants as the ones pinned in `src/lib.rs`'s test module.
const TD2_L1: &str = "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<";
const TD2_L2: &str = "D231458907UTO7408122F1204159<<<<<<<6";

#[test]
fn td2_specimen_byte_for_byte() {
    let fields = Td2Fields {
        document_code: "I".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "D23145890".to_string(),
        surname: "ERIKSSON".to_string(),
        given_names: "ANNA MARIA".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: "740812".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "120415".to_string(),
        optional_data: None,
    };

    let expected = format!("{TD2_L1}\n{TD2_L2}");
    assert_eq!(format_td2(&fields), expected);
}

#[test]
fn td2_round_trips_as_valid() {
    let fields = Td2Fields {
        document_code: "I".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "D23145890".to_string(),
        surname: "ERIKSSON".to_string(),
        given_names: "ANNA MARIA".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: "740812".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "120415".to_string(),
        optional_data: None,
    };

    let mrz = format_td2(&fields);
    let (l1, l2) = mrz.split_once('\n').unwrap();
    let d = parse_td2(l1, l2).unwrap();
    assert!(d.valid(), "checks: {:?}", d.checks);
}

// Capped so surname + "<<" + given_names never exceeds the 31-char (TD2) /
// 30-char (TD1) name field (14 + 2 + 14 = 30 <= both) — narrower than
// `name_strategy` (which is sized for TD3's wider 39-char field).
fn short_name_strategy() -> impl Strategy<Value = String> {
    "[A-Z]{1,14}"
}

fn optional_data_strategy(max_len: usize) -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        proptest::string::string_regex(&format!("[A-Z0-9]{{1,{max_len}}}"))
            .unwrap()
            .prop_map(Some),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn td2_arbitrary_fields_round_trip(
        document_number in doc_number_strategy(),
        surname in short_name_strategy(),
        given_names in short_name_strategy(),
        date_of_birth in yymmdd_strategy(),
        sex in sex_strategy(),
        date_of_expiry in yymmdd_strategy(),
        optional_data in optional_data_strategy(7),
    ) {
        let fields = Td2Fields {
            document_code: "I".to_string(),
            issuing_country: "UTO".to_string(),
            document_number: document_number.clone(),
            surname: surname.clone(),
            given_names: given_names.clone(),
            nationality: "UTO".to_string(),
            date_of_birth: date_of_birth.clone(),
            sex: sex.clone(),
            date_of_expiry: date_of_expiry.clone(),
            optional_data,
        };

        let mrz = format_td2(&fields);
        let (l1, l2) = mrz.split_once('\n').unwrap();
        prop_assert_eq!(l1.len(), 36);
        prop_assert_eq!(l2.len(), 36);

        let parsed = parse_td2(l1, l2).unwrap();
        prop_assert!(parsed.valid(), "checks: {:?}", parsed.checks);

        prop_assert_eq!(&parsed.document_number, &document_number);
        prop_assert_eq!(&parsed.surname, &surname);
        prop_assert_eq!(&parsed.given_names, &given_names);
        prop_assert_eq!(&parsed.sex, &sex);
    }
}

// Same Utopia/Eriksson identity as the TD3/TD2 specimens (Part 5's own
// examples are images, not text — see `src/lib.rs`'s test module for the full
// provenance note) — same constants as the ones pinned there.
const TD1_L1: &str = "I<UTOD231458907<<<<<<<<<<<<<<<";
const TD1_L2: &str = "7408122F1204159UTO<<<<<<<<<<<6";
const TD1_L3: &str = "ERIKSSON<<ANNA<MARIA<<<<<<<<<<";

#[test]
fn td1_specimen_byte_for_byte() {
    let fields = Td1Fields {
        document_code: "I".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "D23145890".to_string(),
        optional_data_1: None,
        surname: "ERIKSSON".to_string(),
        given_names: "ANNA MARIA".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: "740812".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "120415".to_string(),
        optional_data_2: None,
    };

    let expected = format!("{TD1_L1}\n{TD1_L2}\n{TD1_L3}");
    assert_eq!(format_td1(&fields), expected);
}

#[test]
fn td1_round_trips_as_valid() {
    let fields = Td1Fields {
        document_code: "I".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "D23145890".to_string(),
        optional_data_1: None,
        surname: "ERIKSSON".to_string(),
        given_names: "ANNA MARIA".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: "740812".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "120415".to_string(),
        optional_data_2: None,
    };

    let mrz = format_td1(&fields);
    let mut lines = mrz.split('\n');
    let l1 = lines.next().unwrap();
    let l2 = lines.next().unwrap();
    let l3 = lines.next().unwrap();
    let d = parse_td1(l1, l2, l3).unwrap();
    assert!(d.valid(), "checks: {:?}", d.checks);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn td1_arbitrary_fields_round_trip(
        document_number in doc_number_strategy(),
        optional_data_1 in optional_data_strategy(15),
        surname in short_name_strategy(),
        given_names in short_name_strategy(),
        date_of_birth in yymmdd_strategy(),
        sex in sex_strategy(),
        date_of_expiry in yymmdd_strategy(),
        optional_data_2 in optional_data_strategy(11),
    ) {
        let fields = Td1Fields {
            document_code: "I".to_string(),
            issuing_country: "UTO".to_string(),
            document_number: document_number.clone(),
            optional_data_1,
            surname: surname.clone(),
            given_names: given_names.clone(),
            nationality: "UTO".to_string(),
            date_of_birth: date_of_birth.clone(),
            sex: sex.clone(),
            date_of_expiry: date_of_expiry.clone(),
            optional_data_2,
        };

        let mrz = format_td1(&fields);
        let mut lines = mrz.split('\n');
        let l1 = lines.next().unwrap();
        let l2 = lines.next().unwrap();
        let l3 = lines.next().unwrap();
        prop_assert_eq!(l1.len(), 30);
        prop_assert_eq!(l2.len(), 30);
        prop_assert_eq!(l3.len(), 30);

        let parsed = parse_td1(l1, l2, l3).unwrap();
        prop_assert!(parsed.valid(), "checks: {:?}", parsed.checks);

        prop_assert_eq!(&parsed.document_number, &document_number);
        prop_assert_eq!(&parsed.surname, &surname);
        prop_assert_eq!(&parsed.given_names, &given_names);
        prop_assert_eq!(&parsed.sex, &sex);
    }
}

// Hand-derived MRV-A / MRV-B line-2 vectors — NOT ICAO-published specimens
// (see `src/lib.rs`'s test module for the worked check-digit arithmetic and
// pointers to Part 7's actual, differently-valued Appendix B examples).
const MRV_A_L2: &str = "XK93054875BRA8502212F2703143R5T6U7V8W9<<<<<<";
const MRV_B_L2: &str = "L234567897DEU9201017F2706306QW12ER34";

#[test]
fn mrv_a_specimen_line2_byte_for_byte() {
    let fields = MrvAFields {
        document_code: "V".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "XK9305487".to_string(),
        surname: "ERIKSSON".to_string(),
        given_names: "ANNA MARIA".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "850221".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "270314".to_string(),
        optional_data: Some("R5T6U7V8W9".to_string()),
    };

    let mrz = format_mrv_a(&fields);
    let (_, l2) = mrz.split_once('\n').unwrap();
    assert_eq!(l2, MRV_A_L2);
}

#[test]
fn mrv_b_specimen_line2_byte_for_byte() {
    let fields = MrvBFields {
        document_code: "V".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "L23456789".to_string(),
        surname: "ERIKSSON".to_string(),
        given_names: "ANNA MARIA".to_string(),
        nationality: "DEU".to_string(),
        date_of_birth: "920101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "270630".to_string(),
        optional_data: Some("QW12ER34".to_string()),
    };

    let mrz = format_mrv_b(&fields);
    let (_, l2) = mrz.split_once('\n').unwrap();
    assert_eq!(l2, MRV_B_L2);
}

#[test]
fn mrv_a_round_trips_as_valid() {
    let fields = MrvAFields {
        document_code: "V".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "XK9305487".to_string(),
        surname: "ERIKSSON".to_string(),
        given_names: "ANNA MARIA".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "850221".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "270314".to_string(),
        optional_data: Some("R5T6U7V8W9".to_string()),
    };

    let mrz = format_mrv_a(&fields);
    let (l1, l2) = mrz.split_once('\n').unwrap();
    let d = parse_mrv_a(l1, l2).unwrap();
    assert!(d.valid(), "checks: {:?}", d.checks);
}

#[test]
fn mrv_b_round_trips_as_valid() {
    let fields = MrvBFields {
        document_code: "V".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "L23456789".to_string(),
        surname: "ERIKSSON".to_string(),
        given_names: "ANNA MARIA".to_string(),
        nationality: "DEU".to_string(),
        date_of_birth: "920101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "270630".to_string(),
        optional_data: Some("QW12ER34".to_string()),
    };

    let mrz = format_mrv_b(&fields);
    let (l1, l2) = mrz.split_once('\n').unwrap();
    let d = parse_mrv_b(l1, l2).unwrap();
    assert!(d.valid(), "checks: {:?}", d.checks);
}

// A name strategy narrow enough for MRV-A's 39-wide and MRV-B's 31-wide name
// field (14 + 2 + 14 = 30, comfortably under both) — a wider strategy like
// `name_strategy()` (sized for TD3) can overflow MRV-B's field and truncate,
// breaking round-trip.
fn mrv_name_strategy() -> impl Strategy<Value = String> {
    "[A-Z]{1,14}"
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn mrv_a_arbitrary_fields_round_trip(
        document_number in doc_number_strategy(),
        surname in mrv_name_strategy(),
        given_names in mrv_name_strategy(),
        nationality in "[A-Z]{3}",
        date_of_birth in yymmdd_strategy(),
        sex in sex_strategy(),
        date_of_expiry in yymmdd_strategy(),
        optional_data in optional_data_strategy(16),
    ) {
        let fields = MrvAFields {
            document_code: "V".to_string(),
            issuing_country: "UTO".to_string(),
            document_number: document_number.clone(),
            surname: surname.clone(),
            given_names: given_names.clone(),
            nationality: nationality.clone(),
            date_of_birth,
            sex: sex.clone(),
            date_of_expiry,
            optional_data,
        };

        let mrz = format_mrv_a(&fields);
        let (l1, l2) = mrz.split_once('\n').unwrap();
        prop_assert_eq!(l1.len(), 44);
        prop_assert_eq!(l2.len(), 44);

        let parsed = parse_mrv_a(l1, l2).unwrap();
        prop_assert!(parsed.valid(), "checks: {:?}", parsed.checks);

        prop_assert_eq!(&parsed.document_number, &document_number);
        prop_assert_eq!(&parsed.surname, &surname);
        prop_assert_eq!(&parsed.given_names, &given_names);
        prop_assert_eq!(&parsed.nationality, &nationality);
        prop_assert_eq!(&parsed.sex, &sex);
    }

    #[test]
    fn mrv_b_arbitrary_fields_round_trip(
        document_number in doc_number_strategy(),
        surname in mrv_name_strategy(),
        given_names in mrv_name_strategy(),
        nationality in "[A-Z]{3}",
        date_of_birth in yymmdd_strategy(),
        sex in sex_strategy(),
        date_of_expiry in yymmdd_strategy(),
        optional_data in optional_data_strategy(8),
    ) {
        let fields = MrvBFields {
            document_code: "V".to_string(),
            issuing_country: "UTO".to_string(),
            document_number: document_number.clone(),
            surname: surname.clone(),
            given_names: given_names.clone(),
            nationality: nationality.clone(),
            date_of_birth,
            sex: sex.clone(),
            date_of_expiry,
            optional_data,
        };

        let mrz = format_mrv_b(&fields);
        let (l1, l2) = mrz.split_once('\n').unwrap();
        prop_assert_eq!(l1.len(), 36);
        prop_assert_eq!(l2.len(), 36);

        let parsed = parse_mrv_b(l1, l2).unwrap();
        prop_assert!(parsed.valid(), "checks: {:?}", parsed.checks);

        prop_assert_eq!(&parsed.document_number, &document_number);
        prop_assert_eq!(&parsed.surname, &surname);
        prop_assert_eq!(&parsed.given_names, &given_names);
        prop_assert_eq!(&parsed.nationality, &nationality);
        prop_assert_eq!(&parsed.sex, &sex);
    }
}

#[test]
fn td3_document_number_length_sweep() {
    // TD3's personal_number field is 14 wide: len > 9 && len <= 21 overflows;
    // this sweep (9..=20) never reaches the truncation branch, so every
    // length past 9 must reassemble exactly.
    for len in 9..=20 {
        let document_number = doc_number_of_len(len);
        let fields = Td3Fields {
            document_number: document_number.clone(),
            date_of_birth: "740812".to_string(),
            date_of_expiry: "120415".to_string(),
            ..Td3Fields::default()
        };
        let mrz = format_td3(&fields);
        let (l1, l2) = mrz.split_once('\n').unwrap();
        let d = parse_td3(l1, l2).unwrap();
        assert!(d.valid(), "len {len}: checks {:?}", d.checks);

        if overflows(len, 14) {
            assert_eq!(
                d.document_number_full.as_deref(),
                Some(document_number.as_str()),
                "len {len}"
            );
            assert_eq!(d.full_document_number(), document_number, "len {len}");
        } else {
            assert_eq!(d.document_number_full, None, "len {len}");
            assert_eq!(d.document_number, &document_number[0..9], "len {len}");
        }
    }
}

#[test]
fn td2_document_number_length_sweep() {
    // TD2's optional_data field is only 7 wide: len > 9 && len <= 14
    // overflows; len 15..=20 falls back to truncation — this sweep exercises
    // both branches within a single format, not just the fits-forever case.
    for len in 9..=20 {
        let document_number = doc_number_of_len(len);
        let fields = Td2Fields {
            document_number: document_number.clone(),
            date_of_birth: "740812".to_string(),
            date_of_expiry: "120415".to_string(),
            ..Td2Fields::default()
        };
        let mrz = format_td2(&fields);
        let (l1, l2) = mrz.split_once('\n').unwrap();
        let d = parse_td2(l1, l2).unwrap();
        assert!(d.valid(), "len {len}: checks {:?}", d.checks);

        if overflows(len, 7) {
            assert_eq!(
                d.document_number_full.as_deref(),
                Some(document_number.as_str()),
                "len {len}"
            );
        } else {
            assert_eq!(d.document_number_full, None, "len {len}");
            assert_eq!(d.document_number, &document_number[0..9], "len {len}");
        }
    }
}

#[test]
fn td1_document_number_length_sweep() {
    // TD1's optional_data_1 field is 15 wide: len > 9 && len <= 22 overflows,
    // so (like TD3) this 9..=20 sweep never reaches the truncation branch.
    for len in 9..=20 {
        let document_number = doc_number_of_len(len);
        let fields = Td1Fields {
            document_number: document_number.clone(),
            date_of_birth: "740812".to_string(),
            date_of_expiry: "120415".to_string(),
            ..Td1Fields::default()
        };
        let mrz = format_td1(&fields);
        let mut lines = mrz.lines();
        let l1 = lines.next().unwrap();
        let l2 = lines.next().unwrap();
        let l3 = lines.next().unwrap();
        let d = parse_td1(l1, l2, l3).unwrap();
        assert!(d.valid(), "len {len}: checks {:?}", d.checks);

        if overflows(len, 15) {
            assert_eq!(
                d.document_number_full.as_deref(),
                Some(document_number.as_str()),
                "len {len}"
            );
        } else {
            assert_eq!(d.document_number_full, None, "len {len}");
            assert_eq!(d.document_number, &document_number[0..9], "len {len}");
        }
    }
}

#[test]
fn overflow_coexists_with_nonempty_optional_data_td2_td1() {
    // Overflow writes `remainder + check + filler` at the START of the
    // optional field; any caller-supplied optional data must survive
    // concatenated right after that prefix, not get clobbered by it.
    let td2 = Td2Fields {
        document_number: "D2314589012".to_string(), // 11 chars, remainder 3 fits width 7
        date_of_birth: "740812".to_string(),
        date_of_expiry: "120415".to_string(),
        optional_data: Some("XY".to_string()),
        ..Td2Fields::default()
    };
    let mrz = format_td2(&td2);
    let (l1, l2) = mrz.split_once('\n').unwrap();
    let d = parse_td2(l1, l2).unwrap();
    assert!(d.valid(), "checks: {:?}", d.checks);
    assert_eq!(d.document_number_full.as_deref(), Some("D2314589012"));
    // The overflow prefix (remainder + check + filler) occupies the front of
    // the optional field; the caller-supplied "XY" must survive right after
    // it rather than being overwritten or absorbed into the remainder.
    assert_eq!(d.personal_number.as_deref(), Some("XY"));

    let td1 = Td1Fields {
        document_number: "D231458901234".to_string(), // 13 chars, remainder 5 fits width 15
        date_of_birth: "740812".to_string(),
        date_of_expiry: "120415".to_string(),
        optional_data_1: Some("ZZZ".to_string()),
        ..Td1Fields::default()
    };
    let mrz = format_td1(&td1);
    let mut lines = mrz.lines();
    let l1 = lines.next().unwrap();
    let l2 = lines.next().unwrap();
    let l3 = lines.next().unwrap();
    let d = parse_td1(l1, l2, l3).unwrap();
    assert!(d.valid(), "checks: {:?}", d.checks);
    assert_eq!(d.document_number_full.as_deref(), Some("D231458901234"));
    // optional_data_1's overflow prefix and "ZZZ" join as `personal_number`
    // (optional_data_2 is empty here, so no " " separator survives).
    assert_eq!(d.personal_number.as_deref(), Some("ZZZ"));
}

/// A TD1 long-document-number zone assembled directly from
/// `Doc_9303_Part5_Specs_for_TD1_MROTDs.md`'s position ranges (§4.2.4, Note j)
/// rather than from `format_td1` — this is the vector that proves the parser
/// reads a spec-conformant zone this crate did not write itself, not merely
/// that `format_td1`/`parse_td1` agree with each other.
///
/// Document number `D23145890XYZ` (12 chars, overflows the 9-char field).
/// Per Note j, the nine principal characters (`D23145890`) fill line 1
/// positions 6-14 (0-based `line1[5..14]`), position 15 (`line1[14]`) is a
/// filler instead of a check digit, and the remainder (`XYZ`) plus a check
/// digit *over the whole twelve-character number* plus a terminating filler
/// open the optional-data-1 field (`line1[15..30]`).
///
/// Character values (ICAO 9303 part 3 §4.9): `0-9`→`0-9`, `A-Z`→`10-35` so
/// `D`=13, `X`=33, `Y`=34, `Z`=35; `<`=0. 7-3-1 weights repeat from the start
/// of each check-digit input.
///
/// Long-number check digit over `D23145890XYZ` (12 chars):
///   D·7 + 2·3 + 3·1 + 1·7 + 4·3 + 5·1 + 8·7 + 9·3 + 0·1 + X·7 + Y·3 + Z·1
///   = 13·7 + 2·3 + 3·1 + 1·7 + 4·3 + 5·1 + 8·7 + 9·3 + 0·1 + 33·7 + 34·3 + 35·1
///   = 91 + 6 + 3 + 7 + 12 + 5 + 56 + 27 + 0 + 231 + 102 + 35 = 575 → 575 % 10 = 5
const TD1_LONG_NUMBER_LINE1: &str = "I<UTOD23145890<XYZ5<<<<<<<<<<<";
// dob(740812)+check(2, pt4 TD3 specimen value — see icao_vectors.rs) + sex(F)
// + expiry(120415)+check(9, same specimen) + nationality(UTO) +
// optional_data_2 (11 fillers, empty) + composite.
//
// Composite input = line1[5..30] + line2[0..7] + line2[8..15] + line2[18..29]
// (`parser::parse_td1`'s ranges) =
//   "D23145890<XYZ5<<<<<<<<<<<" (25) + "7408122" (7) + "1204159" (7) +
//   "<<<<<<<<<<<" (11 fillers) = 50 chars.
// Sum of products = 720 → 720 % 10 = 0 (worked the same way as the long-number
// digit above; the trailing 11-filler run contributes 0 regardless of weight).
const TD1_LONG_NUMBER_LINE2: &str = "7408122F1204159UTO<<<<<<<<<<<0";
const TD1_LONG_NUMBER_LINE3: &str = "ERIKSSON<<ANNA<MARIA<<<<<<<<<<";

#[test]
fn td1_hand_built_spec_form_overflow_parses() {
    // Sanity-check the hand-derived check digits against the crate's own
    // primitive before trusting the parse below — this is the cross-check
    // the surrounding arithmetic comments claim, not a substitute for it.
    // Long-number check digit, worked above: 5.
    assert_eq!(check_digit("D23145890XYZ").unwrap(), 5);
    // Composite input, worked above: line1[5..30] + line2[0..7] + line2[8..15]
    // + line2[18..29] = "D23145890<XYZ5<<<<<<<<<<<" + "7408122" + "1204159"
    // + "<<<<<<<<<<<" (11 fillers) → composite digit 0.
    assert_eq!(
        check_digit("D23145890<XYZ5<<<<<<<<<<<74081221204159<<<<<<<<<<<").unwrap(),
        0
    );

    let d = parse_td1(
        TD1_LONG_NUMBER_LINE1,
        TD1_LONG_NUMBER_LINE2,
        TD1_LONG_NUMBER_LINE3,
    )
    .unwrap();
    assert!(d.valid(), "checks: {:?}", d.checks);
    assert_eq!(d.document_number, "D23145890");
    assert_eq!(d.document_number_full.as_deref(), Some("D23145890XYZ"));
    assert_eq!(d.full_document_number(), "D23145890XYZ");
    assert!(!d.document_number_legacy_encoding);
}

#[test]
fn legacy_encoding_zone_parses_and_is_flagged() {
    // Pre-0.6 encoding this crate used to emit for a long document number:
    // first 8 characters + filler in the 9-char number field, filler in the
    // check-digit position, and `remainder (chars 9+) + check digit over the
    // WHOLE number + terminating filler` at the start of the optional field.
    // Hand-assembled here (not via `format_td3`, which now emits the
    // spec-conformant nine-principal-character form) so the parser is proven
    // against a zone this crate itself used to write, not one it still does.
    let full = "L898902C31234"; // 13 chars, overflows the 9-char field
    let head8 = &full[0..8]; // "L898902C" — the pre-0.6 truncation point
    let remainder = &full[8..]; // "31234"
    let full_check = char::from_digit(check_digit(full).unwrap(), 10).unwrap();

    // Number field: 8 real chars + filler = 9; check-digit position is filler.
    let doc_num_field = format!("{head8}<");
    let doc_num_check = '<';

    // Personal-number field: remainder + check + terminating filler, padded
    // to the 14-char width with more filler.
    let mut personal = format!("{remainder}{full_check}<");
    while personal.len() < 14 {
        personal.push('<');
    }
    let personal_check = char::from_digit(check_digit(&personal).unwrap(), 10).unwrap();

    let nationality = "UTO";
    let dob = "740812";
    let dob_check = '2'; // ICAO pt4 TD3 specimen value (icao_vectors.rs)
    let sex = 'F';
    let expiry = "120415";
    let expiry_check = '9'; // ICAO pt4 TD3 specimen value (icao_vectors.rs)

    let line2_body = format!(
        "{doc_num_field}{doc_num_check}{nationality}{dob}{dob_check}{sex}{expiry}{expiry_check}{personal}{personal_check}"
    );
    assert_eq!(line2_body.len(), 43);
    let composite_input = format!(
        "{}{}{}",
        &line2_body[0..10],
        &line2_body[13..20],
        &line2_body[21..43]
    );
    let composite = char::from_digit(check_digit(&composite_input).unwrap(), 10).unwrap();
    let line2 = format!("{line2_body}{composite}");

    let d = parse_td3(TD3_L1, &line2).unwrap();
    assert!(d.valid(), "checks: {:?}", d.checks);
    assert_eq!(d.document_number_full.as_deref(), Some(full));
    assert!(d.document_number_legacy_encoding);
}

#[test]
fn conformant_zone_is_not_flagged_legacy() {
    let fields = Td3Fields {
        document_number: "L898902C31234".to_string(), // 13 chars, overflows 9
        date_of_birth: "740812".to_string(),
        date_of_expiry: "120415".to_string(),
        ..Td3Fields::default()
    };
    let mrz = format_td3(&fields);
    let (l1, l2) = mrz.split_once('\n').unwrap();
    let d = parse_td3(l1, l2).unwrap();
    assert!(d.valid(), "checks: {:?}", d.checks);
    assert_eq!(d.document_number_full.as_deref(), Some("L898902C31234"));
    assert!(!d.document_number_legacy_encoding);
}

#[test]
fn neither_form_verifies_surfaces_full_number_with_failed_check() {
    // Start from a valid spec-form overflow zone, then corrupt the single
    // check-digit character the overflow encoding writes into the optional
    // field. The corrupted zone's document-number field does not end in a
    // filler (`L898902C3` doesn't), so the legacy fallback never even
    // applies — this exercises the "neither form verifies" tail of
    // `read_overflow`, not a legacy/spec collision.
    let fields = Td3Fields {
        document_number: "L898902C31234".to_string(), // 13 chars, overflows 9
        date_of_birth: "740812".to_string(),
        date_of_expiry: "120415".to_string(),
        ..Td3Fields::default()
    };
    let mrz = format_td3(&fields);
    let (l1, l2) = mrz.split_once('\n').unwrap();
    // The overflow check digit sits right after the 4-char remainder ("1234")
    // at the start of the personal-number field (offset 28 in line 2).
    let check_pos = 28 + 4;
    let correct = l2.as_bytes()[check_pos] as char;
    let wrong = char::from_digit((correct.to_digit(10).unwrap() + 1) % 10, 10).unwrap();
    let mut corrupted = l2.to_string().into_bytes();
    corrupted[check_pos] = wrong as u8;
    let corrupted = String::from_utf8(corrupted).unwrap();

    let d = parse_td3(l1, &corrupted).unwrap();
    assert_eq!(d.document_number_full.as_deref(), Some("L898902C31234"));
    assert!(!d.checks.document_number);
    assert!(!d.document_number_legacy_encoding);
}
