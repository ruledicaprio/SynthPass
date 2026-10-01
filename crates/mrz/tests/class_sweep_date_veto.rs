//! Differential proof for a specific reading of `accept_damaged`
//! (`crates/mrz/src/parser.rs`): the class sweep can resolve a damaged
//! document number to a checksum-valid reading and still lose, because
//! `accept_damaged` additionally requires
//! `data.validity(..).dates_well_formed`, and a SPECIMEN-style card whose
//! printed date of birth and expiry are both the placeholder `000000` can
//! never satisfy that -- `000000` is not a real calendar date no matter how
//! correctly it is read.
//!
//! Two fixtures, identical in every respect except the dates, isolate the
//! variable with no instrumentation. The composite check digit differs too,
//! necessarily -- it covers the date fields -- so it is a consequence of the
//! change rather than a second variable.
//!
//! - **Case A** -- placeholder dates (`000000` / `000000`).
//! - **Case B** -- the same zone with genuine calendar dates.
//!
//! **What carries the proof is the pair, and only under mutation.** Case A
//! asserts a negative, which passes for any reason at all; Case B is what
//! rules the alternatives out. Deleting the date clause from `accept_damaged`
//! turns Case A into a recovery, and neutralising `class_sweep_pass` turns
//! Case B into a failure -- so each test is sensitive to exactly the thing it
//! names. Neither fact is an assertion in this file, and no test below should
//! be read as establishing it on its own.
//!
//! In both, the same nine-digit document number is corrupted identically: the
//! whole field plus its check-digit cell rewritten from `0` to the confusable
//! `O` -- one OCR class decision, ten wrong cells, the shape
//! `class_sweep_wiring.rs` already proves the sweep recovers. If the veto
//! reading is right, Case B recovers under the sweep and Case A does not,
//! even though the sweep resolves the document number identically in both.
//!
//! **Fixtures are emitted, not transcribed.** Every zone here comes from
//! `mrz::format_td1`, which computes real check digits from placeholder field
//! values, then is corrupted programmatically. No real specimen's zone text,
//! character value, or read name appears in this file.

mod support;

use mrz::{find_and_parse_with, format_td1, parse_td1_with, Date, ParseOptions, Td1Fields};

/// Nine identical digits -- the shape that leaves exactly one residue class
/// for the sweep to resolve to, and the one measured on the motivating card.
const DOCUMENT_NUMBER: &str = "000000000";

/// A checksum-valid TD1 zone with the given dates and a nine-zero document
/// number, built by this crate's own emitter so the fixture cannot drift from
/// the format it claims to be.
fn td1_zone(date_of_birth: &str, date_of_expiry: &str) -> String {
    format_td1(&Td1Fields {
        document_code: "I".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: DOCUMENT_NUMBER.to_string(),
        optional_data_1: None,
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth(date_of_birth),
        sex: mrz::Sex::Male,
        date_of_expiry: support::expiry(date_of_expiry),
        optional_data_2: None,
    })
}

/// Misread every cell of line 1's document number **and its check digit** as
/// the confusable letter -- one class decision, ten wrong cells. Mirrors the
/// real defect this sweep exists for: a run of one glyph read as its
/// lookalike, check-digit cell included.
fn sweep_document_number_to_letters(zone: &str) -> String {
    let mut lines: Vec<String> = zone.lines().map(str::to_string).collect();
    let chars: Vec<char> = lines[0].chars().collect();
    let corrupted: String = chars
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
fn case_b_real_dates_recover_under_sweep_on_but_not_off() {
    let zone = td1_zone("900101", "300101");
    let sanity =
        find_and_parse_with(&zone, &ParseOptions::default()).expect("emitter output must parse");
    assert!(sanity.valid(), "sanity: the unbroken zone validates");

    let broken = sweep_document_number_to_letters(&zone);
    assert_ne!(broken, zone, "sanity: corruption changed the zone");

    let off = ParseOptions::default();
    assert!(!off.class_sweep, "sanity: the arm is off by default");
    let recovered_off = find_and_parse_with(&broken, &off)
        .ok()
        .is_some_and(|d| d.valid());
    assert!(!recovered_off, "sweep off must not recover the swept zone");

    let on = ParseOptions::default().with_class_sweep(true);
    let parsed =
        find_and_parse_with(&broken, &on).expect("real dates: the sweep must recover this reading");
    assert!(
        parsed.valid(),
        "real dates: every check digit must validate after the sweep"
    );
    assert_eq!(parsed.document_number, DOCUMENT_NUMBER);
}

#[test]
fn case_a_placeholder_dates_do_not_recover_even_with_sweep_on() {
    let zone = td1_zone("000000", "000000");
    let sanity =
        find_and_parse_with(&zone, &ParseOptions::default()).expect("emitter output must parse");
    assert!(
        sanity.valid(),
        "sanity: the unbroken zone validates -- `valid()` is check digits only, \
         placeholder dates do not fail it"
    );

    let broken = sweep_document_number_to_letters(&zone);
    assert_ne!(broken, zone, "sanity: corruption changed the zone");

    let off = ParseOptions::default();
    // Without this, a flipped default would silently turn this control arm
    // into a second treatment arm and the assertion below would still pass.
    assert!(!off.class_sweep, "sanity: the arm is off by default");
    let recovered_off = find_and_parse_with(&broken, &off)
        .ok()
        .is_some_and(|d| d.valid());
    assert!(!recovered_off, "sweep off must not recover the swept zone");

    let on = ParseOptions::default().with_class_sweep(true);
    let recovered_on = find_and_parse_with(&broken, &on)
        .ok()
        .is_some_and(|d| d.valid());
    assert!(
        !recovered_on,
        "placeholder dates: this reading must still be refused with the sweep on -- \
         if this fails, the date veto in `accept_damaged` is not what is blocking \
         the real defect"
    );
}

/// The raw material of the veto, asserted on `validity()` itself: a zone whose
/// printed dates are the placeholder `000000` is not calendar-well-formed,
/// and the same zone with real dates is. `accept_damaged` combines this with
/// checksum validity, so the veto in Case A above can only work if this holds.
///
/// Unit-level on purpose. Whether the sweep recovers a reading is Case A and
/// Case B's business; this test says nothing about the sweep, and re-asserts
/// nothing that holds by fixture construction (no `valid()` on emitted lines).
#[test]
fn a_placeholder_date_is_not_well_formed_and_a_real_date_is() {
    let reference = Date::new(2000, 1, 1);
    let parse = |dob: &str, expiry: &str| {
        let zone = td1_zone(dob, expiry);
        let lines: Vec<&str> = zone.lines().collect();
        parse_td1_with(lines[0], lines[1], lines[2], &ParseOptions::default())
            .expect("an emitted zone parses")
    };

    let placeholder = parse("000000", "000000");
    assert!(
        !placeholder.validity(reference).dates_well_formed,
        "000000 is not a calendar date"
    );

    let real = parse("900101", "300101");
    assert!(
        real.validity(reference).dates_well_formed,
        "1990-01-01 and 2030-01-01 are calendar dates"
    );
}
