//! ADR-0022 Decision 6: the five built-ins are data, and the data equals the
//! Rust definitions and renders the same pixels.

use std::fs;
use std::path::Path;

use synthpass_gen::{
    generate_from_seed_with_layout, DocumentType, GeneratorConfig, ValidatedLayout,
};
use synthpass_layout::{builtin, to_json};

const FORMATS: [(DocumentType, &str); 5] = [
    (DocumentType::TD1, "td1"),
    (DocumentType::TD2, "td2"),
    (DocumentType::TD3, "td3"),
    (DocumentType::MrvA, "mrva"),
    (DocumentType::MrvB, "mrvb"),
];

#[test]
fn all_five_builtins_load_and_equal_the_rust_definitions() {
    for (format, key) in FORMATS {
        assert_eq!(
            builtin(format).layout(),
            ValidatedLayout::builtin(format),
            "{key}"
        );
        assert_eq!(builtin(format).layout().format(), format, "{key}");
        assert_eq!(builtin(format).name(), format!("icao-{key}-builtin"));
        assert_eq!(builtin(format).description(), None);
    }
}

#[test]
fn renders_through_the_json_builtins_equal_the_rust_builtins() {
    for (format, key) in FORMATS {
        for seed in [0u64, 1, 42, 1000] {
            let cfg = GeneratorConfig::with_document_type(seed, format);
            let (json_image, json_labels, _) =
                generate_from_seed_with_layout(&cfg, builtin(format).layout()).expect("json path");
            let (rust_image, rust_labels, _) =
                generate_from_seed_with_layout(&cfg, ValidatedLayout::builtin(format))
                    .expect("rust path");
            assert_eq!(
                json_image.to_rgb8().as_raw(),
                rust_image.to_rgb8().as_raw(),
                "{key} seed {seed}: pixels"
            );
            assert_eq!(json_labels, rust_labels, "{key} seed {seed}: labels");
        }
    }
}

#[test]
fn committed_layout_files_are_the_emitters_output_byte_for_byte() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("layouts");
    for (format, key) in FORMATS {
        let path = dir.join(format!("{key}.json"));
        let committed = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        assert_eq!(
            committed,
            to_json(builtin(format)),
            "{key}.json differs from to_json; re-bless with SYNTHPASS_LAYOUT_BLESS=1"
        );
    }
}
