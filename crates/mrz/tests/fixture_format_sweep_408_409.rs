//! Public JSON fixture audit for #409. Reports only fixture names and format
//! classes, never document field values.

use mrz::{find_and_parse, parse_td1, parse_td3, Format};
use std::{fs, path::Path};

#[test]
#[ignore = "run explicitly for the #409 fixture-format audit"]
fn compare_every_public_ocr_fixture_with_its_shape_named_parser() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/ocr_fixtures");
    let mut files: Vec<_> = fs::read_dir(directory)
        .expect("public OCR fixtures are available")
        .map(|entry| entry.expect("fixture entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();

    let mut compared = 0;
    let mut disagreements = 0;
    for path in files {
        let raw = fs::read_to_string(&path).expect("read public OCR fixture");
        let fixture: serde_json::Value = serde_json::from_str(&raw).expect("parse fixture JSON");
        let zone = fixture["mrz_line"].as_str().expect("fixture MRZ");
        let lines: Vec<&str> = zone.lines().collect();
        let direct = match lines.as_slice() {
            [a, b, c] if [a, b, c].iter().all(|line| line.len() == 30) => {
                parse_td1(a, b, c).map(|data| data.format)
            }
            [a, b] if [a, b].iter().all(|line| line.len() == 44) => {
                parse_td3(a, b).map(|data| data.format)
            }
            _ => panic!("unexpected public fixture shape"),
        };
        let detected = find_and_parse(zone).map(|data| data.format);
        compared += 1;
        if direct.as_ref().ok() != detected.as_ref().ok() {
            disagreements += 1;
            let name = path
                .file_name()
                .expect("fixture file name")
                .to_string_lossy();
            let format_name = |format: &Result<Format, mrz::MrzError>| match format {
                Ok(Format::Td1) => "TD1",
                Ok(Format::Td2) => "TD2",
                Ok(Format::Td3) => "TD3",
                Ok(Format::MrvA) => "MRV-A",
                Ok(Format::MrvB) => "MRV-B",
                Ok(_) => "other",
                Err(_) => "error",
            };
            println!(
                "{name}: shape={} scanner={}",
                format_name(&direct),
                format_name(&detected)
            );
        }
    }
    println!("compared={compared} disagreements={disagreements}");
    assert_eq!(compared, 65, "fixture audit denominator changed");
}
