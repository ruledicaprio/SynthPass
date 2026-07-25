//! The serialized shape of [`ExtractionV2`] is a **published contract**, and
//! until now nothing checked it.
//!
//! `crates/synthpass-core` had no `tests/` directory at all, so any change to a
//! field name, a `skip_serializing_if`, or a `#[serde(rename)]` would ship
//! silently and break every consumer parsing the JSON. That is the single
//! highest-severity risk in the provider-model work, which adds keys to this
//! type.
//!
//! These tests pin the exact top-level key set for each production path. They
//! are *supposed* to fail when the schema changes — a failure means "you
//! changed the wire format; confirm that was deliberate and bump
//! `schema_version` if it was breaking", not "fix the test".

use synthpass_core::v2::{
    CheckDigits, CoreField, EscalationKind, ExtractionFields, ExtractionTrace, ExtractionV2,
    FieldConfidence, MrzBlock, MrzFormat, PromptRef, Provenance, SCHEMA_VERSION_V2,
};

/// Top-level keys of a value, sorted.
fn keys(v: &serde_json::Value) -> Vec<String> {
    let mut k: Vec<String> = v
        .as_object()
        .expect("ExtractionV2 serializes to an object")
        .keys()
        .cloned()
        .collect();
    k.sort();
    k
}

fn to_json(e: &ExtractionV2) -> serde_json::Value {
    serde_json::to_value(e).expect("ExtractionV2 serializes")
}

// `ExtractionV2` is `ZeroizeOnDrop`, so `..Default::default()` is a partial
// move out of a `Drop` type and will not compile — the same constraint that
// makes the v1 lift clone rather than move. Build by mutation instead.

/// A Tier-1 record: checksum-valid MRZ, no LLM involved.
fn tier1() -> ExtractionV2 {
    let mut e = ExtractionV2::default();
    e.fields = ExtractionFields {
        document_number: Some("L898902C3".into()),
        surname: Some("ERIKSSON".into()),
        ..Default::default()
    };
    e.confidence = FieldConfidence::mrz_checksum_scope();
    e.provenance = Provenance::MrzChecksum;
    e.mrz = Some(MrzBlock {
        lines: "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<".into(),
        format: MrzFormat::Td3,
        checks: CheckDigits {
            document_number: true,
            date_of_birth: true,
            date_of_expiry: true,
            personal_number: true,
            composite: true,
        },
    });
    e.extraction_method = "mrz-deterministic".into();
    e
}

/// A Tier-2 record: no valid MRZ, the LLM produced the fields.
fn tier2() -> ExtractionV2 {
    let mut e = ExtractionV2::default();
    e.confidence = FieldConfidence::llm_heuristic();
    e.provenance = Provenance::Llm {
        model: "qwen2.5-1.5b-instruct-q4_k_m.gguf".into(),
    };
    e.extraction_method = "llm".into();
    e
}

/// Keys present on **every** record regardless of path. Adding one here is a
/// breaking change for any consumer with a strict schema.
const ALWAYS: &[&str] = &[
    "confidence",
    "document",
    "extraction_method",
    "fields",
    "provenance",
    "schema_version",
];

#[test]
fn tier1_top_level_keys_are_pinned() {
    let mut expected: Vec<String> = ALWAYS.iter().map(|s| s.to_string()).collect();
    // Present only because this record has an MRZ.
    expected.push("mrz".into());
    expected.sort();

    assert_eq!(
        keys(&to_json(&tier1())),
        expected,
        "the Tier-1 JSON key set changed. This is the published wire contract — \
         if the change is deliberate, update this test and consider whether \
         consumers with a mirror type break."
    );
}

#[test]
fn tier2_top_level_keys_are_pinned() {
    let mut expected: Vec<String> = ALWAYS.iter().map(|s| s.to_string()).collect();
    expected.sort();

    assert_eq!(
        keys(&to_json(&tier2())),
        expected,
        "the Tier-2 JSON key set changed. This is the published wire contract."
    );
}

/// The provider model adds `trace`, and it must stay invisible until something
/// actually populates it. A key that appears on every record — even as `null` —
/// is a schema change; one that appears only when meaningful is not.
#[test]
fn trace_is_absent_unless_populated() {
    for (name, record) in [("tier1", tier1()), ("tier2", tier2())] {
        assert!(
            !keys(&to_json(&record)).contains(&"trace".to_string()),
            "{name}: `trace` must not serialize while None — an unconditional \
             key is a schema change for every existing consumer"
        );
    }
}

#[test]
fn trace_appears_only_with_content_and_carries_no_score() {
    let mut e = tier2();
    e.trace = Some(ExtractionTrace {
        providers: vec!["mrz".into(), "qwen-text".into()],
        escalation: Some(EscalationKind::MrzChecksumFailed),
        prompt: Some(PromptRef {
            id: "qwen2.5-fields".into(),
            version: 1,
            digest: "deadbeef".into(),
        }),
    });

    let json = to_json(&e);
    assert!(keys(&json).contains(&"trace".to_string()));

    let trace = &json["trace"];
    assert_eq!(trace["escalation"], "mrz_checksum_failed");
    assert_eq!(trace["providers"][0], "mrz");
    assert_eq!(trace["prompt"]["version"], 1);

    // The load-bearing assertion: the routing score is not here, under any
    // spelling. A spend decision may rest on an uncalibrated number; a value
    // that reaches a consumer alongside `confidence` may not.
    // See knowledge/project_principles.md §2.
    let serialized = serde_json::to_string(trace).expect("trace serializes");
    for forbidden in ["score", "routing", "weight", "threshold"] {
        assert!(
            !serialized.contains(forbidden),
            "`{forbidden}` leaked into the serialized trace: {serialized}"
        );
    }
}

/// An empty trace is indistinguishable from no trace, so callers should not
/// attach one. `is_empty` is what they ask.
#[test]
fn empty_trace_serializes_to_nothing() {
    let empty = ExtractionTrace::default();
    assert!(empty.is_empty());
    assert_eq!(
        serde_json::to_string(&empty).expect("serializes"),
        "{}",
        "every field of an empty trace must be skipped"
    );

    let populated = ExtractionTrace {
        providers: vec!["mrz".into()],
        ..Default::default()
    };
    assert!(!populated.is_empty());
}

/// v2 records must keep declaring their generation, or version-dispatching
/// consumers have nothing to dispatch on.
#[test]
fn schema_version_is_two_and_always_present() {
    for record in [tier1(), tier2()] {
        let json = to_json(&record);
        assert_eq!(json["schema_version"], SCHEMA_VERSION_V2);
        assert_eq!(json["schema_version"], 2);
    }
}

/// Adding a `Provenance` variant is safe in Rust (the enum is
/// `#[non_exhaustive]`) but **not** on the wire: a consumer with a mirror enum
/// fails on an unknown `"kind"`. Pinning the tags keeps that a deliberate act.
#[test]
fn provenance_wire_tags_are_pinned() {
    let cases = [
        (Provenance::MrzChecksum, "mrz_checksum"),
        (
            Provenance::Llm {
                model: "m.gguf".into(),
            },
            "llm",
        ),
        (Provenance::WasmClient, "wasm_client"),
    ];
    for (provenance, tag) in cases {
        let json = serde_json::to_value(&provenance).expect("serializes");
        assert_eq!(
            json["kind"], tag,
            "Provenance wire tag changed — this breaks consumers matching on `kind`"
        );
    }
}

/// The ten ICAO field names must match the JSON keys of `ExtractionFields`
/// exactly. Two lists that are supposed to agree, and nothing forcing them to,
/// is how they drift.
#[test]
fn core_field_names_match_the_serialized_fields() {
    let json = serde_json::to_value(ExtractionFields::default()).expect("serializes");
    let object = json.as_object().expect("object");

    for field in CoreField::ALL {
        assert!(
            object.contains_key(field.as_str()),
            "CoreField::{field:?} serializes as `{}`, which is not a key of \
             ExtractionFields",
            field.as_str()
        );
    }

    // `serde(rename_all)` is not applied to this enum by accident: the wire
    // name and the field name must be the same string.
    for field in CoreField::ALL {
        let as_json = serde_json::to_value(field).expect("serializes");
        assert_eq!(as_json, serde_json::Value::String(field.as_str().into()));
    }
}

#[test]
fn missing_reports_empty_and_whitespace_only_fields() {
    let all_empty = ExtractionFields::default();
    assert_eq!(
        all_empty.missing().len(),
        CoreField::ALL.len(),
        "a default record has nothing in it"
    );

    let fields = ExtractionFields {
        surname: Some("ERIKSSON".into()),
        // Present but useless — an empty string is not a value a consumer can
        // act on, so it must count as missing.
        given_names: Some("   ".into()),
        document_number: Some(String::new()),
        ..Default::default()
    };
    let missing = fields.missing();
    assert!(!missing.contains(&CoreField::Surname));
    assert!(missing.contains(&CoreField::GivenNames));
    assert!(missing.contains(&CoreField::DocumentNumber));
    assert_eq!(fields.get(CoreField::Surname), Some("ERIKSSON"));
    assert_eq!(fields.get(CoreField::GivenNames), None);
}

/// A record round-trips through JSON unchanged, including the new trace.
#[test]
fn extraction_v2_round_trips() {
    let mut original = tier1();
    original.trace = Some(ExtractionTrace {
        providers: vec!["mrz".into()],
        escalation: None,
        prompt: None,
    });

    let json = serde_json::to_string(&original).expect("serializes");
    let back: ExtractionV2 = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(back, original);
}

/// A record written before `trace` existed must still deserialize.
#[test]
fn records_without_trace_still_deserialize() {
    let legacy = serde_json::json!({
        "schema_version": 2,
        "document": {"kind": "passport", "mrz_format": "td3"},
        "fields": {
            "document_type": "P", "issuing_country": "UTO", "document_number": "L898902C3",
            "surname": "ERIKSSON", "given_names": "ANNA MARIA", "nationality": "UTO",
            "date_of_birth": "1974-08-12", "sex": "F", "date_of_expiry": "2012-04-15",
            "personal_number": "ZE184226B"
        },
        "confidence": {},
        "provenance": {"kind": "mrz_checksum"},
        "extraction_method": "mrz-deterministic"
    });

    let parsed: ExtractionV2 =
        serde_json::from_value(legacy).expect("a pre-trace record must still parse");
    assert_eq!(parsed.trace, None);
    assert_eq!(parsed.schema_version, SCHEMA_VERSION_V2);
}
