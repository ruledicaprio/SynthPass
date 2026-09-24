//! `serde` symmetry: a parsed `MrzData` survives a JSON round-trip unchanged.
//!
//! Compiled only when the `serde` feature is on (which pulls in the
//! `Serialize`/`Deserialize` derives this test exercises).

#![cfg(feature = "serde")]

use mrz::{parse_td2, parse_td3, MrzData};

// ICAO 9303 specimen identity (Utopia / Anna Maria Eriksson) — see
// `src/lib.rs`'s test module for the full provenance note (Part 4's own copy
// is a figure, not extracted text; corroborated via Part 6's literal TD2
// specimen).
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
        sex: "X".into(),
        ..Default::default()
    });
    let (l1, l2) = zone.split_once('\n').unwrap();
    let unspecified = serde_json::to_value(parse_td3(l1, l2).unwrap()).expect("serialize");
    assert_eq!(unspecified["sex"], "<");
}
