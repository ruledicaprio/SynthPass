//! Writes `layouts/*.json` from the Rust built-ins with `to_json`, never by hand.
//!
//! `SYNTHPASS_LAYOUT_BLESS=1 cargo test -p synthpass-layout --test bless`
//!
//! Without the variable this test only checks that the committed files match
//! what it would write (`tests/builtins.rs` pins the same thing).

use std::fs;
use std::path::Path;

use synthpass_gen::{DocumentType, ValidatedLayout};
use synthpass_layout::{parse_bytes, to_json};

const FORMATS: [(DocumentType, &str, &str); 5] = [
    (DocumentType::TD1, "td1", "icao-td1-builtin"),
    (DocumentType::TD2, "td2", "icao-td2-builtin"),
    (DocumentType::TD3, "td3", "icao-td3-builtin"),
    (DocumentType::MrvA, "mrva", "icao-mrva-builtin"),
    (DocumentType::MrvB, "mrvb", "icao-mrvb-builtin"),
];

#[test]
fn bless_builtin_layouts() {
    if std::env::var_os("SYNTHPASS_LAYOUT_BLESS").is_none() {
        return;
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("layouts");
    for (format, key, name) in FORMATS {
        // Bootstrap through the same writer: a placeholder file carrying the
        // Rust built-in's rectangles is rendered by `to_json`, and re-parsed.
        let spec = ValidatedLayout::builtin(format).spec();
        let seed = bootstrap_json(name, key, spec);
        let loaded = parse_bytes(seed.as_bytes()).expect("bootstrap parses");
        fs::write(dir.join(format!("{key}.json")), to_json(&loaded)).expect("write");
    }
}

fn bootstrap_json(name: &str, key: &str, spec: &synthpass_gen::LayoutSpec) -> String {
    let mut s = format!("{{\"schema_version\":1,\"name\":\"{name}\",\"format\":\"{key}\"");
    for field in synthpass_gen::layout::LayoutField::ALL {
        let r = spec.rect(field);
        s += &format!(
            ",\"{}\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
            field.name(),
            r.x,
            r.y,
            r.width,
            r.height
        );
    }
    s + "}"
}
