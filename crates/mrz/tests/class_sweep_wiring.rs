//! End-to-end proof that the uniform class sweep is reachable from
//! `find_and_parse_with`, and that it stays inert when its arm is off.
//!
//! The unit tests in `repair.rs` prove `solve_class_sweep` resolves a field.
//! They cannot prove the parser ever *calls* it: the sweep lives behind the
//! damaged pass, which runs only after an ordinary read has already failed to
//! validate, and only for fields that carry their own check digit. This file
//! exercises that path from raw text, which is the only way to know the wiring
//! is real rather than merely compiled.
//!
//! The shape under test is the one measured on a real TD1 card back: a
//! document number printed as a run of one character and read as a run of its
//! lookalike, **check-digit cell included**. See
//! `knowledge/benchmarks/twelve-scored-misses-2026-09-19.md`.

use mrz::{find_and_parse_with, format_td1, ParseOptions, Td1Fields};

/// A checksum-valid TD1 zone whose document number is nine identical digits,
/// built by this crate's own emitter so the fixture cannot drift from the
/// format it claims to be.
fn valid_zone() -> String {
    format_td1(&Td1Fields {
        document_code: "IO".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "000000000".to_string(),
        optional_data_1: None,
        surname: "ERIKSSON".to_string(),
        given_names: "ANNA MARIA".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: "740812".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "120415".to_string(),
        optional_data_2: None,
    })
}

/// Misread every cell of the document number **and its check digit** as the
/// confusable letter — one class decision, ten wrong cells.
fn swept_to_letters(zone: &str) -> String {
    let mut lines: Vec<String> = zone.lines().map(str::to_string).collect();
    let l1: Vec<char> = lines[0].chars().collect();
    let corrupted: String = l1
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            if (5..=14).contains(&i) && c == '0' {
                'O'
            } else {
                c
            }
        })
        .collect();
    lines[0] = corrupted;
    lines.join("\n")
}

#[test]
fn the_fixture_is_valid_before_corruption_and_invalid_after() {
    let zone = valid_zone();
    let parsed =
        find_and_parse_with(&zone, &ParseOptions::default()).expect("emitter output must parse");
    assert!(parsed.valid(), "sanity: the emitted zone validates");

    let broken = swept_to_letters(&zone);
    assert_ne!(broken, zone, "sanity: corruption changed the zone");
    assert_eq!(
        broken.lines().next().unwrap().matches('O').count(),
        12,
        "sanity: ten swept cells plus two legitimate letters already on the line (document code IO, issuing country UTO)"
    );
}

#[test]
fn the_sweep_is_inert_when_its_arm_is_off() {
    let broken = swept_to_letters(&valid_zone());
    let off = ParseOptions::default();
    assert!(!off.class_sweep, "the arm is off by default");

    // Either it fails to parse or it parses without validating — what it must
    // not do is silently recover, because that would mean the repair is on a
    // path nobody asked for.
    let recovered = find_and_parse_with(&broken, &off)
        .ok()
        .is_some_and(|d| d.valid());
    assert!(!recovered, "arm off must not recover the swept zone");
}

#[test]
fn the_sweep_recovers_the_document_number_when_its_arm_is_on() {
    let broken = swept_to_letters(&valid_zone());
    let on = ParseOptions::default().with_class_sweep(true);

    let parsed =
        find_and_parse_with(&broken, &on).expect("the swept zone must parse with the arm on");
    assert!(
        parsed.valid(),
        "every check digit must validate after the sweep"
    );
    assert_eq!(
        parsed.document_number, "000000000",
        "the document number must come back as the printed digits"
    );
}

/// The safety property, stated as a test: the sweep must not rewrite a
/// correct occurrence of the same character **outside** the field it repairs.
///
/// This fixture's document code is `IO` — a legitimate letter in the class
/// being swept, sitting at line 1 column 1, outside the document number. The
/// document code carries no check digit, so a line-scoped sweep would rewrite
/// it to `I0` and still produce a zone whose every check digit validates:
/// wrong, and undetectably so. A field-scoped sweep cannot reach it.
#[test]
fn the_sweep_does_not_touch_a_legitimate_letter_outside_the_field() {
    let broken = swept_to_letters(&valid_zone());
    let on = ParseOptions::default().with_class_sweep(true);

    let parsed = find_and_parse_with(&broken, &on).expect("must parse");
    assert_eq!(
        parsed.document_type, "IO",
        "the document code's own letter must survive the sweep untouched"
    );
}
