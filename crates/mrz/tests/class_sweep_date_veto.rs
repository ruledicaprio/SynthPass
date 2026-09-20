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
//! variable with no instrumentation:
//!
//! - **Case A** -- placeholder dates (`000000` / `000000`).
//! - **Case B** -- the same zone with genuine calendar dates.
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

use mrz::{
    check_digit, find_and_parse_with, format_td1, parse_td1_with, solve_class_sweep, Date,
    FieldKind, ParseOptions, Resolution, Td1Fields,
};

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
        date_of_birth: date_of_birth.to_string(),
        sex: "M".to_string(),
        date_of_expiry: date_of_expiry.to_string(),
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

/// The mechanism, not just the outcome: reconstruct exactly what
/// `class_sweep_pass` computes for Case A's corrupted document number, and
/// show the result is checksum-valid on its own terms while failing the date
/// constraint -- the two facts `accept_damaged` combines to refuse it.
///
/// This does not call the private `accept_damaged`/`class_sweep_pass`
/// functions; it uses the same public primitive they are built from
/// (`solve_class_sweep`) to repair the field, splices the repair back in by
/// hand, and parses the result directly with [`parse_td1_with`] -- a plain,
/// non-repairing parse, so nothing here can be mistaken for the search
/// pass succeeding on its own.
#[test]
fn the_swept_reading_is_checksum_valid_but_fails_the_date_constraint() {
    let zone = td1_zone("000000", "000000");
    let broken = sweep_document_number_to_letters(&zone);
    let lines: Vec<&str> = broken.lines().collect();
    let (l1, l2, l3) = (lines[0], lines[1], lines[2]);

    let cells: Vec<char> = l1.chars().collect();
    let field: String = cells[5..14].iter().collect();
    let field_check = cells[14];

    let Resolution::Unique(fixed_field) =
        solve_class_sweep(&field, field_check, FieldKind::DocumentNumber)
    else {
        panic!(
            "expected a unique resolution for a uniform run of one confusable \
             class -- this is `class_sweep_wiring.rs`'s own precondition"
        );
    };
    assert_eq!(
        fixed_field, DOCUMENT_NUMBER,
        "the sweep must recover the printed digits, not merely some reading"
    );

    let fixed_check_digit =
        check_digit(&fixed_field).expect("nine digits always yield a check digit");
    let fixed_check_char =
        char::from_digit(fixed_check_digit, 10).expect("a base-10 digit always converts");

    let mut fixed_cells = cells;
    for (i, c) in fixed_field.chars().enumerate() {
        fixed_cells[5 + i] = c;
    }
    fixed_cells[14] = fixed_check_char;
    let fixed_l1: String = fixed_cells.into_iter().collect();

    let data = parse_td1_with(&fixed_l1, l2, l3, &ParseOptions::default())
        .expect("the sweep-repaired zone must parse");

    assert!(
        data.valid(),
        "the swept reading is a proof, not a guess: every check digit -- \
         including the composite -- validates"
    );
    assert!(
        !data.validity(Date::new(2000, 1, 1)).dates_well_formed,
        "yet the printed dates are the placeholder `000000`, so it is this \
         flag -- not a check-digit failure -- that `accept_damaged` is acting on"
    );
}
