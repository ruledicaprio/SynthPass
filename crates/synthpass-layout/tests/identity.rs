//! ADR-0022 Decision 7: the identity is SHA-256 over `canonical_bytes()`.

use synthpass_gen::DocumentType;
use synthpass_layout::{builtin, identity, identity_hex, parse_bytes, to_json};

fn td3() -> String {
    to_json(builtin(DocumentType::TD3))
}

fn id_of(json: &str) -> String {
    identity_hex(parse_bytes(json.as_bytes()).expect("loads").layout())
}

#[test]
fn identity_ignores_whitespace_key_order_name_and_description() {
    let base = id_of(&td3());

    // Whitespace: the whole file on one line, then padded.
    let compact: String = td3().split_whitespace().collect::<Vec<_>>().join("");
    assert_eq!(id_of(&compact), base, "compact");
    assert_eq!(id_of(&format!("\n\n  {}\t\n", td3())), base, "padded");

    // Key order: the rectangle lines reversed.
    let text = td3();
    let lines: Vec<&str> = text.lines().collect();
    let (head, rest) = lines.split_at(4); // `{`, schema_version, name, format
    let (rects, _close) = rest.split_at(rest.len() - 1);
    let mut rects: Vec<String> = rects
        .iter()
        .map(|l| l.trim_end_matches(',').to_string())
        .collect();
    rects.reverse();
    let reordered = format!("{}\n{}\n}}\n", head.join("\n"), rects.join(",\n"));
    assert_eq!(id_of(&reordered), base, "key order");

    // Name and description.
    let renamed = td3().replace("icao-td3-builtin", "something.else/entirely");
    assert_eq!(id_of(&renamed), base, "name");
    let described = td3().replace(
        "\"format\"",
        "\"description\": \"any words at all\",\n  \"format\"",
    );
    assert_eq!(id_of(&described), base, "description");
}

#[test]
fn one_coordinate_changes_the_identity() {
    let base = id_of(&td3());
    // surname x: 60 -> 61 (still inside the permitted area, still fits).
    let moved = td3().replace("\"surname\": {\"x\": 60,", "\"surname\": {\"x\": 61,");
    assert_ne!(moved, td3());
    assert_ne!(id_of(&moved), base);
}

#[test]
fn td3_builtin_identity_is_pinned() {
    // SHA-256 of the 197-byte canonical encoding of the TD3 built-in. A change
    // here is a change of identity for every sidecar that cites it.
    assert_eq!(
        identity_hex(builtin(DocumentType::TD3).layout()),
        "3ee385f55691249dcde2b4e2243f3501f8a0ad012e57eefb7f94cd2947abbc06"
    );
}

#[test]
fn hex_is_the_lowercase_form_of_the_bytes() {
    let layout = builtin(DocumentType::TD1).layout();
    let hex = identity_hex(layout);
    assert_eq!(hex.len(), 64);
    assert!(hex
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    let bytes = identity(layout);
    assert_eq!(&hex[..2], format!("{:02x}", bytes[0]));
}
