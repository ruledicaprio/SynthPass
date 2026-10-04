//! ADR-0022 Decision 4: every rule fails closed, and the error names the key
//! or field and the rule.

use synthpass_gen::DocumentType;
use synthpass_layout::{builtin, parse_bytes, to_json, LayoutFileError, MAX_BYTES};

fn td3() -> String {
    to_json(builtin(DocumentType::TD3))
}

/// The error text for `json`; panics when it loads.
fn refused(json: &str) -> String {
    parse_bytes(json.as_bytes())
        .expect_err("the file must be refused")
        .to_string()
}

fn assert_names(message: &str, parts: &[&str]) {
    for part in parts {
        assert!(message.contains(part), "`{part}` missing from: {message}");
    }
}

/// The TD3 file with `from` replaced by `to`; fails when `from` is absent.
fn edit(from: &str, to: &str) -> String {
    let text = td3();
    assert!(text.contains(from), "`{from}` not in the TD3 file");
    text.replacen(from, to, 1)
}

const FORBIDDEN: [&str; 11] = [
    "watermark",
    "mrz",
    "emblem",
    "background",
    "image",
    "font",
    "color",
    "colour",
    "caption",
    "title",
    "issuing_country",
];

#[test]
fn an_unknown_key_is_refused_at_both_levels() {
    assert_names(
        &refused(&edit("\"format\"", "\"flavour\": 1,\n  \"format\"")),
        &["unknown field `flavour`", "expected one of"],
    );
    assert_names(
        &refused(&edit("\"surname\": {", "\"surname\": {\"flavour\": 1, ")),
        &["unknown field `flavour`", "expected one of `x`"],
    );
}

#[test]
fn every_forbidden_key_is_refused_at_both_levels() {
    for key in FORBIDDEN {
        let needle = format!("unknown field `{key}`");
        // Inside a rectangle, every forbidden key is simply unknown.
        let inside = refused(&edit(
            "\"surname\": {",
            &format!("\"surname\": {{\"{key}\": 1, "),
        ));
        assert_names(&inside, &[&needle]);

        // At the top level `issuing_country` is a real rectangle key, so a
        // second one is a duplicate; every other forbidden key is unknown.
        let top = refused(&edit("\n}\n", &format!(",\n  \"{key}\": \"x\"\n}}\n")));
        if key == "issuing_country" {
            assert_names(&top, &["duplicate field `issuing_country`"]);
        } else {
            assert_names(&top, &[&needle]);
        }
    }
}

#[test]
fn a_string_where_a_rectangle_belongs_is_refused_with_its_key() {
    // The natural attempt at giving `issuing_country` a value.
    let json = edit(
        "\"issuing_country\": {\"x\": 220, \"y\": 60, \"width\": 120, \"height\": 34}",
        "\"issuing_country\": \"UTO\"",
    );
    assert_names(
        &refused(&json),
        &["invalid type", "issuing_country", "line"],
    );
}

#[test]
fn a_duplicate_key_is_refused_at_both_levels() {
    assert_names(
        &refused(&edit("\"name\"", "\"name\": \"a\",\n  \"name\"")),
        &["duplicate field `name`"],
    );
    assert_names(
        &refused(&edit(
            "\"surname\": {\"x\": 60,",
            "\"surname\": {\"x\": 60, \"x\": 61,",
        )),
        &["duplicate field `x`"],
    );
}

#[test]
fn schema_version_missing_zero_two_or_a_string_is_refused() {
    let line = "\"schema_version\": 1,\n  ";
    assert_names(
        &refused(&edit(line, "")),
        &["missing field `schema_version`"],
    );
    for bad in ["0", "2"] {
        assert_names(
            &refused(&edit(
                "\"schema_version\": 1",
                &format!("\"schema_version\": {bad}"),
            )),
            &["schema_version", bad, "not supported"],
        );
    }
    assert_names(
        &refused(&edit("\"schema_version\": 1", "\"schema_version\": \"1\"")),
        &["invalid type", "schema_version"],
    );
}

#[test]
fn a_missing_rectangle_or_coordinate_is_refused() {
    let surname = "  \"surname\": {\"x\": 60, \"y\": 120, \"width\": 700, \"height\": 34},\n";
    assert_names(&refused(&edit(surname, "")), &["missing field `surname`"]);
    assert_names(
        &refused(&edit("\"width\": 700, \"height\": 34}", "\"height\": 34}")),
        &["missing field `width`"],
    );
}

#[test]
fn numbers_that_are_not_a_u32_are_refused_with_their_key() {
    for (value, rule) in [
        ("-1", "invalid value: integer `-1`, expected u32"),
        ("1.5", "invalid type: floating point `1.5`, expected u32"),
        ("1.0", "invalid type: floating point `1.0`, expected u32"),
        (
            "4294967296",
            "invalid value: integer `4294967296`, expected u32",
        ),
    ] {
        let json = edit(
            "\"surname\": {\"x\": 60,",
            &format!("\"surname\": {{\"x\": {value},"),
        );
        // `near` carries the key the bad value sat under.
        assert_names(&refused(&json), &[rule, "line", "\"surname\": {\"x\""]);
    }
}

#[test]
fn input_over_64_kib_is_refused_before_parsing_and_exactly_64_kib_loads() {
    // Not JSON at all: only the size check can have refused it.
    let oversized = vec![b'x'; MAX_BYTES + 1];
    let err = parse_bytes(&oversized).expect_err("oversized");
    assert!(matches!(err, LayoutFileError::TooLarge));
    assert_names(&err.to_string(), &["input", "over 65536 bytes"]);

    let mut exact = td3().into_bytes();
    exact.resize(MAX_BYTES, b' ');
    assert_eq!(exact.len(), 65_536);
    parse_bytes(&exact).expect("exactly 64 KiB of valid JSON loads");

    exact.push(b' ');
    assert!(matches!(
        parse_bytes(&exact),
        Err(LayoutFileError::TooLarge)
    ));
}

#[test]
fn invalid_utf8_is_refused() {
    let mut bytes = td3().into_bytes();
    let at = bytes
        .windows(8)
        .position(|w| w == b"icao-td3")
        .expect("name");
    bytes[at] = 0xff;
    let err = parse_bytes(&bytes).expect_err("not UTF-8");
    assert!(matches!(err, LayoutFileError::NotUtf8 { valid_up_to } if valid_up_to == at));
    assert_names(&err.to_string(), &["input", "UTF-8"]);
}

#[test]
fn a_bad_name_is_refused() {
    let with_name = |name: &str| edit("\"icao-td3-builtin\"", &format!("\"{name}\""));
    assert_names(&refused(&with_name("")), &["name", "1 to 64", "found 0"]);
    assert_names(
        &refused(&with_name(&"a".repeat(65))),
        &["name", "1 to 64", "found 65"],
    );
    parse_bytes(with_name(&"a".repeat(64)).as_bytes()).expect("64 characters load");
    assert_names(
        &refused(&with_name("icaoTd3")),
        &["name", "'T'", "index 4", "[a-z0-9._/-]"],
    );
    assert_names(
        &refused(&with_name("icao td3")),
        &["name", "' '", "index 4"],
    );
    parse_bytes(with_name("a.b_c/d-e9").as_bytes()).expect("the whole alphabet loads");
}

#[test]
fn an_overlong_description_is_refused() {
    let with = |d: &str| {
        edit(
            "\"format\"",
            &format!("\"description\": \"{d}\",\n  \"format\""),
        )
    };
    parse_bytes(with(&"d".repeat(1024)).as_bytes()).expect("1,024 bytes load");
    assert_names(
        &refused(&with(&"d".repeat(1025))),
        &["description", "1025 bytes", "1024-byte limit"],
    );
    // The limit counts bytes, not characters: 513 two-byte characters.
    assert_names(
        &refused(&with(&"é".repeat(513))),
        &["description", "1026 bytes"],
    );
    // `null` is not "absent".
    assert_names(
        &refused(&edit("\"format\"", "\"description\": null,\n  \"format\"")),
        &["invalid type", "null"],
    );
}

#[test]
fn an_unknown_format_is_refused() {
    assert_names(
        &refused(&edit("\"format\": \"td3\"", "\"format\": \"td4\"")),
        &[
            "unknown variant `td4`",
            "td1",
            "mrvb",
            "\"format\": \"td4\"",
        ],
    );
    assert_names(
        &refused(&edit("\"format\": \"td3\"", "\"format\": \"TD3\"")),
        &["unknown variant `TD3`", "\"format\": \"TD3\""],
    );
}

#[test]
fn a_layout_error_passes_through_naming_field_and_rule() {
    // given_names on top of surname.
    let err = parse_bytes(
        edit(
            "\"given_names\": {\"x\": 60, \"y\": 170,",
            "\"given_names\": {\"x\": 60, \"y\": 120,",
        )
        .as_bytes(),
    )
    .expect_err("overlap");
    assert!(matches!(err, LayoutFileError::Layout(_)));
    assert_names(
        &err.to_string(),
        &["surname: overlaps given_names", "rule overlap"],
    );
}

#[test]
fn syntax_errors_keep_line_and_column() {
    assert_names(
        &refused("{\n  \"schema_version\": 1,\n  oops"),
        &["line 3", "column"],
    );
}

#[test]
fn a_missing_file_names_its_path() {
    let path = std::path::Path::new("/nonexistent/dir/layout.json");
    let err = synthpass_layout::load_path(path).expect_err("missing");
    assert_names(&err.to_string(), &["/nonexistent/dir/layout.json"]);
}

/// A layout file is one JSON object, and so is every rectangle: serde's
/// sequence form and its externally tagged enum form are not part of the format.
#[test]
fn a_rectangle_given_as_an_array_is_refused() {
    let json = edit(
        "\"surname\": {\"x\": 60, \"y\": 120, \"width\": 700, \"height\": 34}",
        "\"surname\": [60, 120, 700, 34]",
    );
    assert_names(
        &refused(&json),
        &[
            "invalid type",
            "sequence",
            "a JSON object",
            "\"surname\": [",
        ],
    );
}

#[test]
fn a_file_given_as_an_array_is_refused() {
    let json = "[1, \"icao-td3-builtin\", null, \"td3\", [860,100,260,340], [60,60,120,34], \
                [220,60,120,34], [60,120,700,34], [60,170,700,34], [60,220,300,34], \
                [60,270,200,34], [280,270,240,34], [540,270,80,34], [60,320,240,34], \
                [60,370,400,34]]";
    assert_names(
        &refused(json),
        &["invalid type", "sequence", "a JSON object"],
    );
}

#[test]
fn a_format_given_as_an_object_is_refused() {
    let json = edit("\"format\": \"td3\"", "\"format\": {\"td3\": null}");
    assert_names(
        &refused(&json),
        &["invalid type", "map", "a string", "\"format\": {"],
    );
}

/// A scratch file under the OS temp dir, removed on drop.
struct TempFile(std::path::PathBuf);

impl TempFile {
    fn new(tag: &str, bytes: &[u8]) -> Self {
        let path = std::env::temp_dir().join(format!(
            "synthpass-layout-{}-{tag}.json",
            std::process::id()
        ));
        std::fs::write(&path, bytes).expect("write the scratch file");
        TempFile(path)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn load_path_stops_reading_one_byte_past_the_limit() {
    // Valid JSON padded with whitespace: if `take` allowed only MAX_BYTES the
    // oversized file would be truncated to a valid 64 KiB file and load.
    let mut bytes = td3().into_bytes();
    bytes.resize(MAX_BYTES, b' ');
    let exact = TempFile::new("exact", &bytes);
    synthpass_layout::load_path(&exact.0).expect("exactly 64 KiB loads");

    bytes.push(b' ');
    let over = TempFile::new("over", &bytes);
    let err = synthpass_layout::load_path(&over.0).expect_err("one byte over");
    assert!(matches!(err, LayoutFileError::TooLarge), "{err}");

    // A far larger file is refused the same way (and read only to the limit).
    bytes.resize(4 * MAX_BYTES, b' ');
    let huge = TempFile::new("huge", &bytes);
    let err = synthpass_layout::load_path(&huge.0).expect_err("4x over");
    assert!(matches!(err, LayoutFileError::TooLarge), "{err}");
}
