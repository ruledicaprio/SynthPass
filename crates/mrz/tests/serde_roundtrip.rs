//! `serde` symmetry: a parsed `MrzData` survives a JSON round-trip unchanged.
//!
//! Compiled only when the `serde` feature is on (which pulls in the
//! `Serialize`/`Deserialize` derives this test exercises).

#![cfg(feature = "serde")]

use mrz::{parse_td2, parse_td3, MrzData};

// Utopia/Eriksson line 2 is printed in Part 3 §3.2 (PDF p11).
// The P< line 1 is from a pre-amendment edition.
const TD3_L1: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
const TD3_L2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";

#[test]
fn mrzdata_json_round_trip_is_identity() {
    let original = parse_td3(TD3_L1, TD3_L2).unwrap();

    let json = serde_json::to_string(&original).expect("serialize");
    let restored: MrzData = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(original, restored);
    assert!(restored.valid(), "checks should survive the round-trip");
}

#[test]
fn check_states_serialize_true_false_and_absent_as_null() {
    let td2 = parse_td2(
        "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
        "D231458907UTO7408122F1204159<<<<<<<6",
    )
    .unwrap();
    let td2_checks = serde_json::to_value(&td2.checks).expect("serialize TD2 checks");

    // Assert the key is *present* before asserting its value. `Value`'s `Index`
    // impl returns `Value::Null` for a key that is absent from the object, so
    // `checks["personal_number"] == Null` alone passes whether the field
    // serializes as null or is skipped entirely — and skipping it is the one
    // regression this test exists to catch. ADR-0017 calls the explicit null a
    // breaking wire change; `synthpass-core`'s mirror pins it the same way.
    let td2_object = td2_checks
        .as_object()
        .expect("checks serialize as a JSON object");
    assert_eq!(
        td2_object.len(),
        5,
        "no check-state key may be omitted: {td2_checks}"
    );

    assert_eq!(td2_checks["document_number"], true);
    assert_eq!(td2_checks["personal_number"], serde_json::Value::Null);

    let tampered = parse_td3(TD3_L1, "L898902C36UTO7508122F1204159ZE184226B<<<<<10").unwrap();
    let tampered_checks = serde_json::to_value(&tampered.checks).expect("serialize TD3 checks");
    assert_eq!(tampered_checks["date_of_birth"], false);
}

#[test]
fn typed_fields_serialise_as_their_text_form() {
    let doc = parse_td3(TD3_L1, TD3_L2).unwrap();
    let json = serde_json::to_value(&doc).expect("serialize");
    assert_eq!(json["date_of_birth"], "1974-08-12");
    assert_eq!(json["date_of_expiry"], "2012-04-15");
    assert_eq!(json["sex"], "F");
    assert!(json
        .as_object()
        .expect("an object")
        .get("date_of_birth_completeness")
        .is_none());

    let zone = mrz::format_td3(&mrz::Td3Fields {
        sex: mrz::Sex::Unspecified,
        // Non-empty and non-`<`-led: an empty document number formats to an
        // all-filler field, which the parsers now refuse outright (#536),
        // irrelevant to what this test checks (the sex cell's JSON form).
        document_number: "K12345670".to_string(),
        ..Default::default()
    });
    let (l1, l2) = zone.split_once('\n').unwrap();
    let unspecified = serde_json::to_value(parse_td3(l1, l2).unwrap()).expect("serialize");
    assert_eq!(unspecified["sex"], "<");
}

#[test]
fn old_parseoptions_json_defaults_class_sweep_off() {
    let options: mrz::ParseOptions = serde_json::from_str(r#"{"pivot_yy":30}"#).expect("0.7 shape");
    assert_eq!(options.pivot_yy, 30);
    assert!(!options.class_sweep);
}

#[test]
fn old_parseoptions_json_defaults_date_digits_off() {
    let options: mrz::ParseOptions =
        serde_json::from_str(r#"{"pivot_yy":30,"class_sweep":true}"#).expect("0.9 shape");
    assert!(options.class_sweep);
    assert!(!options.date_digits);
}

/// #579: the flag `valid()` needs is recorded on the value, and stays out of the
/// JSON unless the option was on, so a default read serializes exactly as it did
/// before the flag existed.
#[test]
fn date_digits_required_is_absent_by_default_and_round_trips_when_on() {
    let default = parse_td3(TD3_L1, TD3_L2).unwrap();
    let json = serde_json::to_value(&default).expect("serialize");
    assert!(
        json.as_object()
            .expect("an object")
            .get("date_digits_required")
            .is_none(),
        "a default read must not gain a key: {json}"
    );

    let opts = mrz::ParseOptions::default().with_date_digits(true);
    let on = mrz::parse_td3_with(TD3_L1, TD3_L2, &opts).unwrap();
    let json_on = serde_json::to_value(&on).expect("serialize");
    assert_eq!(json_on["date_digits_required"], true);
    let restored: MrzData = serde_json::from_value(json_on).expect("deserialize");
    assert_eq!(restored, on);
}

/// The verdict of a read made under the option survives a JSON round trip.
#[test]
fn a_letters_date_read_under_the_option_stays_not_valid_through_json() {
    let letters = mrz::MrzDate::from_field(
        mrz::RawDateField::try_from("ABCDEF").expect("six MRZ characters"),
        mrz::DateRole::Birth,
        mrz::CURRENT_YY,
    );
    let zone = mrz::format_td3(&mrz::Td3Fields {
        document_number: "K12345670".to_string(),
        date_of_birth: letters,
        ..Default::default()
    });
    let (l1, l2) = zone.split_once('\n').unwrap();
    let opts = mrz::ParseOptions::default().with_date_digits(true);
    let read = mrz::parse_td3_with(l1, l2, &opts).unwrap();
    assert!(!read.valid());

    let restored: MrzData =
        serde_json::from_str(&serde_json::to_string(&read).unwrap()).expect("deserialize");
    assert!(!restored.valid());
    assert_eq!(restored, read);
}
