//! An intact three-row TD1 zone keeps its format even when its printed check
//! digits fail. The fixture is a public specimen, not a private document.
//! The current TD2 fitter inserts six fillers before the middle row's final
//! digit, accidentally verifying its TD2 composite check (1/4 versus TD1's
//! 0/4); fallback ranking then prefers that reflow.

use mrz::{find_and_parse, format_td2, parse_td1, Format, Td2Fields};

#[test]
fn turkiye_2020_intact_td1_is_not_reflowed_to_td2() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../samples/ocr_fixtures/Turkiye_ID_Specimen_2020_back_mrz.json"
    ))
    .expect("the tracked OCR fixture is JSON");
    assert_eq!(fixture["mrz_checksums_valid"], false);
    let zone = fixture["mrz_line"].as_str().expect("fixture has an MRZ");
    let lines: Vec<&str> = zone.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines.iter().all(|line| line.len() == 30));

    let direct = parse_td1(lines[0], lines[1], lines[2]).expect("TD1 layout parses");
    assert_eq!(direct.format, Format::Td1);
    assert!(!direct.valid(), "the printed zone fails its own checks");
    assert_eq!(direct.checks.verified(), 0);
    assert_eq!(direct.checks.applicable(), 4);

    let detected = find_and_parse(zone).expect("the intact zone remains parseable");
    assert_eq!(detected.format, Format::Td1);
    assert!(!detected.valid(), "failed checks must remain visible");
    assert_eq!(detected.document_number, direct.document_number);
}

#[test]
fn intact_td1_does_not_hide_a_separate_valid_td2() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../samples/ocr_fixtures/Turkiye_ID_Specimen_2020_back_mrz.json"
    ))
    .expect("the tracked OCR fixture is JSON");
    let td1 = fixture["mrz_line"].as_str().expect("fixture has an MRZ");
    let td2 = format_td2(&Td2Fields::default());
    let detected = find_and_parse(&format!("{td1}\n{td2}"))
        .expect("the separate validating zone remains eligible");
    assert_eq!(detected.format, Format::Td2);
    assert!(detected.valid());
}
