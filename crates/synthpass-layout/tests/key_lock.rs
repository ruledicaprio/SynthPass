//! The accepted keys, at both levels, are pinned. The lists are read from
//! serde's own "expected one of" text for an unknown key, so the test sees the
//! structs themselves, not only the emitter.

use synthpass_layout::parse_bytes;

const LOCK: &str = "schema change: amend ADR-0022";

/// The backticked names after "expected one of" in the unknown-key error
/// for `json`, sorted.
fn expected_keys(json: &str) -> Vec<String> {
    let message = parse_bytes(json.as_bytes())
        .expect_err("an unknown key is refused")
        .to_string();
    let (_, tail) = message
        .split_once("expected one of ")
        .unwrap_or_else(|| panic!("no key list in: {message}"));
    // serde ends the list at ` at line`; our `near` suffix follows after that.
    let list = tail.split(" at line").next().unwrap_or(tail);
    let mut keys: Vec<String> = list
        .split(", ")
        .map(|k| k.trim().trim_matches('`').to_string())
        .collect();
    keys.sort();
    keys
}

#[test]
fn top_level_keys_are_locked() {
    assert_eq!(
        expected_keys(r#"{"zzz": 1}"#),
        [
            "date_of_birth",
            "date_of_expiry",
            "description",
            "document_number",
            "document_type",
            "format",
            "given_names",
            "issuing_country",
            "name",
            "nationality",
            "personal_number",
            "portrait",
            "schema_version",
            "sex",
            "surname",
        ],
        "{LOCK}"
    );
}

#[test]
fn rectangle_keys_are_locked() {
    assert_eq!(
        expected_keys(r#"{"portrait": {"zzz": 1}}"#),
        ["height", "width", "x", "y"],
        "{LOCK}"
    );
}

#[test]
fn rectangle_keys_equal_the_layout_field_names() {
    let mut fields: Vec<&str> = synthpass_gen::layout::LayoutField::ALL
        .iter()
        .map(|f| f.name())
        .collect();
    fields.extend(["schema_version", "name", "description", "format"]);
    fields.sort_unstable();
    assert_eq!(expected_keys(r#"{"zzz": 1}"#), fields);
}
