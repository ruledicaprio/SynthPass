//! `knowledge/LAYOUTS.md` states per-format numbers. They are taken from
//! `frame_for`, and this test fails when the page and the code disagree.

use std::fs;
use std::path::Path;

use synthpass_gen::layout::{frame_for, LayoutField, LayoutSpec};
use synthpass_gen::DocumentType;

const FORMATS: [(DocumentType, &str); 5] = [
    (DocumentType::TD1, "td1"),
    (DocumentType::TD2, "td2"),
    (DocumentType::TD3, "td3"),
    (DocumentType::MrvA, "mrva"),
    (DocumentType::MrvB, "mrvb"),
];

fn page() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../knowledge/LAYOUTS.md");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"))
}

#[test]
fn the_per_format_table_equals_frame_for() {
    let page = page();
    for (format, key) in FORMATS {
        let f = frame_for(format);
        let p = f.permitted;
        let row = format!(
            "| {key} | {} × {} | {} | {}..{} | {}..{} | ({}, {}) {} × {} |",
            f.width,
            f.height,
            f.frame_thickness,
            p.x,
            p.x + p.width,
            p.y,
            p.y + p.height,
            f.watermark.x,
            f.watermark.y,
            f.watermark.width,
            f.watermark.height,
        );
        assert!(
            page.lines().any(|line| line == row),
            "LAYOUTS.md has no row `{row}`; the table and frame_for disagree"
        );
    }
}

#[test]
fn the_shortest_row_is_the_documented_28() {
    let shortest = FORMATS
        .iter()
        .flat_map(|&(format, _)| {
            let spec = LayoutSpec::builtin(format);
            LayoutField::ALL
                .into_iter()
                .filter(|f| f.is_visual_zone())
                .map(move |f| spec.rect(f).height)
        })
        .min()
        .expect("fields");
    assert!(
        page().contains(&format!("is **{shortest} px** (TD1)")),
        "LAYOUTS.md does not state the shortest built-in row, {shortest} px"
    );
}

#[test]
fn the_documented_keys_are_the_layout_field_names() {
    let page = page();
    for field in LayoutField::ALL {
        assert!(
            page.contains(&format!("| `{}` | rectangle |", field.name())),
            "LAYOUTS.md has no key row for `{}`",
            field.name()
        );
    }
}
