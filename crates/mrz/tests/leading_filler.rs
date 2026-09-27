//! #536: a document-number field whose first cell is the filler `<` is
//! refused outright by the five `parse_*` functions and by `find_and_parse`,
//! as a structural error (`MrzError::LeadingFiller`) rather than a `Checks`
//! failure — because the ICAO 7-3-1 check-digit arithmetic cannot tell `<`
//! apart from several ordinary letters and digits that share its residue
//! class (`mrz::Blindspot`). Doc 9303 Part 3 (PDF p.28) and Part 4 (PDF
//! p.25, TD3) enter data from the left-hand position of each field, so a
//! leading filler represents no printed character at all — unlike an
//! *interior* filler (Part 4 §4.2.2.2), which stays legal.
//!
//! **The fixtures are emitted, not transcribed**, the same discipline
//! `tests/repair.rs` and `tests/td1_line_gap.rs` follow: every document
//! number here is fabricated (`K12345678`, `B<9873001`, and friends) —
//! never the Cyprus specimen number the real-specimen gate actually hit
//! (#535), and never any other real specimen's text.

mod support;

use mrz::{
    check_digit, find_and_parse, format_mrv_a, format_mrv_b, format_td1, format_td2, format_td3,
    parse_mrv_a, parse_mrv_b, parse_td1, parse_td2, parse_td3, verify, Field, Format, MrvAFields,
    MrvBFields, MrzError, Sex, Td1Fields, Td2Fields, Td3Fields,
};

/// A fabricated TD3 zone whose document number is `K12345678`, plus the same
/// line 2 with its first cell swapped `K` -> `<` — the misread #536 traces
/// to (`K00000220` read as `<00000220` on the real specimen; this is a
/// different, fabricated number of the same shape).
fn td3_k_and_filler_led() -> (String, String, String) {
    let fields = Td3Fields {
        issuing_country: "UTO".to_string(),
        document_number: "K12345678".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth("740812"),
        sex: Sex::Unspecified,
        date_of_expiry: support::expiry("300101"),
        personal_number: None,
        ..Default::default()
    };
    let zone = format_td3(&fields);
    let (l1, l2) = zone.split_once('\n').expect("two TD3 lines");
    let filler_led = format!("<{}", &l2[1..]);
    (l1.to_string(), l2.to_string(), filler_led)
}

/// The blind spot the #536 bug exploits: `K` (value 20) and `<` (value 0)
/// share residue 0 mod 10 at *every* 7-3-1 weight, so a document-number check
/// digit computed over a field beginning with either character is identical
/// — the arithmetic alone cannot refuse the misread.
#[test]
fn blindspot_k_and_filler_share_a_residue_the_checksum_cannot_separate() {
    let with_k = "K12345678";
    let with_filler = "<12345678";
    assert_eq!(
        check_digit(with_k).unwrap(),
        check_digit(with_filler).unwrap(),
        "K and < must be checksum-indistinguishable in this field"
    );
}

/// The same blind spot survives into the composite: replacing the document
/// number's leading `K` with `<` leaves the printed composite digit
/// verifying too, which is exactly why the refusal has to be structural
/// rather than left to `Checks`.
#[test]
fn blindspot_survives_into_the_composite_check() {
    let (_l1, l2, filler_led) = td3_k_and_filler_led();
    // Composite: line 2 positions 1-10, 14-20, 22-43 (doc number + check,
    // DOB + check, expiry + check + personal number + check) — same slices
    // `parse_td3_with` itself checks against.
    let composite_input =
        |line2: &str| format!("{}{}{}", &line2[0..10], &line2[13..20], &line2[21..43]);
    assert!(verify(&composite_input(&l2), l2.as_bytes()[43] as char));
    assert!(
        verify(
            &composite_input(&filler_led),
            filler_led.as_bytes()[43] as char
        ),
        "the composite math alone still agrees; parse_td3 refuses this for a different reason"
    );
}

#[test]
fn parse_td3_refuses_a_leading_filler_document_number() {
    let (l1, _l2, filler_led) = td3_k_and_filler_led();
    assert_eq!(
        parse_td3(&l1, &filler_led),
        Err(MrzError::LeadingFiller {
            field: Field::DocumentNumber,
            line: 1,
            position: 0,
        })
    );
}

#[test]
fn find_and_parse_reports_leading_filler_not_incomplete_sequence() {
    let (l1, _l2, filler_led) = td3_k_and_filler_led();
    let text = format!("{l1}\n{filler_led}");
    assert_eq!(
        find_and_parse(&text),
        Err(MrzError::LeadingFiller {
            field: Field::DocumentNumber,
            line: 1,
            position: 0,
        }),
        "a refused reading is more informative than a bare IncompleteSequence/NotFound"
    );
}

/// A refused zone followed by a separate, fully validating one: the
/// validating reading still wins — `find_and_parse` keeps only the *first*
/// refusal in `refused`, and precedence puts a validating reading ahead of
/// it.
#[test]
fn find_and_parse_skips_a_refused_reading_and_returns_a_later_valid_one() {
    let (l1, l2, filler_led) = td3_k_and_filler_led();
    let text = format!("{l1}\n{filler_led}\n{l1}\n{l2}");
    let doc =
        find_and_parse(&text).expect("the valid K-led reading further down must still be found");
    assert_eq!(doc.document_number, "K12345678");
    assert!(doc.valid());
}

/// TD1's document number sits on line 0 at column 5 (ICAO positions 6-14),
/// unlike the two-line formats' line 1 column 0 — verify the position the
/// error reports matches.
#[test]
fn td1_refuses_a_leading_filler_document_number_at_line_0_column_5() {
    let mrz = format_td1(&Td1Fields {
        issuing_country: "UTO".to_string(),
        document_number: "K1234567".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Male,
        date_of_expiry: support::expiry("301230"),
        optional_data_1: None,
        optional_data_2: None,
        ..Default::default()
    });
    let lines: Vec<&str> = mrz.lines().collect();
    let (l1, l2, l3) = (lines[0], lines[1], lines[2]);

    // Confirm the fixture is genuinely valid before mutating it.
    assert!(parse_td1(l1, l2, l3).unwrap().valid());

    let mut bytes = l1.as_bytes().to_vec();
    bytes[5] = b'<'; // document number's first cell (line1[5..14])
    let filler_led = String::from_utf8(bytes).unwrap();
    assert_eq!(
        parse_td1(&filler_led, l2, l3),
        Err(MrzError::LeadingFiller {
            field: Field::DocumentNumber,
            line: 0,
            position: 5,
        })
    );
}

#[test]
fn td2_mrv_a_mrv_b_refuse_a_leading_filler_document_number_at_line_1_column_0() {
    // TD2
    let td2 = format_td2(&Td2Fields {
        document_code: "I".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "K1234567".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth("740812"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("120415"),
        optional_data: None,
    });
    let (l1, l2) = td2.split_once('\n').expect("two TD2 lines");
    assert!(parse_td2(l1, l2).unwrap().valid());
    let filler_led = format!("<{}", &l2[1..]);
    assert_eq!(
        parse_td2(l1, &filler_led),
        Err(MrzError::LeadingFiller {
            field: Field::DocumentNumber,
            line: 1,
            position: 0,
        })
    );

    // MRV-A
    let mrv_a = format_mrv_a(&MrvAFields {
        issuing_country: "UTO".to_string(),
        document_number: "K1234567".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth("690806"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("940623"),
        optional_data: None,
        ..Default::default()
    });
    let (l1, l2) = mrv_a.split_once('\n').expect("two MRV-A lines");
    assert!(parse_mrv_a(l1, l2).unwrap().valid());
    let filler_led = format!("<{}", &l2[1..]);
    assert_eq!(
        parse_mrv_a(l1, &filler_led),
        Err(MrzError::LeadingFiller {
            field: Field::DocumentNumber,
            line: 1,
            position: 0,
        })
    );

    // MRV-B
    let mrv_b = format_mrv_b(&MrvBFields {
        issuing_country: "UTO".to_string(),
        document_number: "K1234567".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth("690806"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("940623"),
        optional_data: None,
        ..Default::default()
    });
    let (l1, l2) = mrv_b.split_once('\n').expect("two MRV-B lines");
    assert!(parse_mrv_b(l1, l2).unwrap().valid());
    let filler_led = format!("<{}", &l2[1..]);
    assert_eq!(
        parse_mrv_b(l1, &filler_led),
        Err(MrzError::LeadingFiller {
            field: Field::DocumentNumber,
            line: 1,
            position: 0,
        })
    );
}

/// `0`, `A`, `K` and `U` all share residue 0 with `<` under every 7-3-1
/// weight (`mrz::collisions('K') == ['0', 'A', 'U', '<']`) — but the rule is
/// "starts with the filler", not "starts with anything congruent to it".
/// Only `<` refuses; the other three are ordinary printed characters and
/// stay accepted.
#[test]
fn a_document_number_led_by_a_blindspot_collision_other_than_filler_stays_accepted() {
    for lead in ['0', 'A', 'U'] {
        let fields = Td3Fields {
            issuing_country: "UTO".to_string(),
            document_number: format!("{lead}12345678"),
            surname: "SPECIMEN".to_string(),
            given_names: "TEST".to_string(),
            nationality: "UTO".to_string(),
            date_of_birth: support::birth("740812"),
            sex: Sex::Unspecified,
            date_of_expiry: support::expiry("300101"),
            personal_number: None,
            ..Default::default()
        };
        let zone = format_td3(&fields);
        let (l1, l2) = zone.split_once('\n').expect("two TD3 lines");
        let doc = parse_td3(l1, l2)
            .unwrap_or_else(|e| panic!("a document number led by {lead:?} must parse: {e:?}"));
        assert!(doc.valid(), "{lead}-led number must still validate");
    }
}

/// The long-document-number overflow encoding stays accepted: every
/// principal character is printed (never a leading filler), and the filler
/// this rule cares about sits in the check-digit cell instead, which
/// `ensure_document_number_leads` never inspects.
#[test]
fn td1_overflow_document_number_stays_accepted() {
    let fields = Td1Fields {
        issuing_country: "UTO".to_string(),
        document_number: "D231458901234".to_string(), // 13 chars, overflows the 9-char field
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth("740812"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("120415"),
        optional_data_1: None,
        optional_data_2: None,
        ..Default::default()
    };
    let mrz = format_td1(&fields);
    let mut lines = mrz.lines();
    let (l1, l2, l3) = (
        lines.next().unwrap(),
        lines.next().unwrap(),
        lines.next().unwrap(),
    );
    let d = parse_td1(l1, l2, l3).expect("overflow form must not be refused");
    assert!(d.valid(), "checks: {:?}", d.checks);
    assert_eq!(d.document_number_full.as_deref(), Some("D231458901234"));
}

/// The pre-0.6 legacy overflow encoding (this crate's own truncated
/// 8-character form) also stays accepted — hand-assembled the way
/// `tests/roundtrip.rs::legacy_encoding_zone_parses_and_is_flagged` builds
/// its TD3 equivalent, since `format_td1` itself only ever emits the
/// spec-conformant form.
#[test]
fn td1_legacy_overflow_document_number_stays_accepted() {
    let full = "D231458901234"; // 13 chars, overflows the 9-char field
    let head8 = &full[0..8]; // pre-0.6 truncation point
    let remainder = &full[8..];
    let full_check = char::from_digit(check_digit(full).unwrap(), 10).unwrap();

    let doc_num_field = format!("{head8}<"); // 8 real chars + filler = 9
    let doc_num_check = '<';

    let mut optional1 = format!("{remainder}{full_check}<");
    while optional1.len() < 15 {
        optional1.push('<');
    }

    let issuer = "UTO";
    let line1 = format!("I<{issuer}{doc_num_field}{doc_num_check}{optional1}");
    assert_eq!(line1.chars().count(), 30);

    let dob = "740812";
    let dob_check = char::from_digit(check_digit(dob).unwrap(), 10).unwrap();
    let sex = 'F';
    let expiry = "120415";
    let expiry_check = char::from_digit(check_digit(expiry).unwrap(), 10).unwrap();
    let nationality = "UTO";
    let optional2 = "<".repeat(11);
    let composite_input = format!("{}{}{}{}", &line1[5..30], dob, dob_check, expiry);
    // Composite: line1[5..30] + line2[0..7] (dob+check) + line2[8..15]
    // (expiry+check) + line2[18..29] (optional2, 11 chars) — same slices
    // `parse_td1_with` checks against. Nationality (line2[15..18]) sits
    // *between* those last two spans and is deliberately excluded.
    let composite_input = format!("{composite_input}{expiry_check}{optional2}");
    let composite = char::from_digit(check_digit(&composite_input).unwrap(), 10).unwrap();
    let line2 =
        format!("{dob}{dob_check}{sex}{expiry}{expiry_check}{nationality}{optional2}{composite}");
    assert_eq!(line2.chars().count(), 30);

    let line3 = format!("{:<30}", "SPECIMEN<<TEST").replace(' ', "<");

    let d = parse_td1(&line1, &line2, &line3).expect("legacy overflow form must not be refused");
    assert!(d.valid(), "checks: {:?}", d.checks);
    assert_eq!(d.document_number_full.as_deref(), Some(full));
    assert!(d.document_number_legacy_encoding);
}

/// An interior filler (Part 4 §4.2.2.2) stays legal: `B<98730` has its
/// filler at the *second* cell, never the first.
#[test]
fn an_interior_filler_document_number_stays_accepted() {
    let fields = Td3Fields {
        issuing_country: "UTO".to_string(),
        document_number: "B<98730".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth("740812"),
        sex: Sex::Unspecified,
        date_of_expiry: support::expiry("300101"),
        personal_number: None,
        ..Default::default()
    };
    let zone = format_td3(&fields);
    let (l1, l2) = zone.split_once('\n').expect("two TD3 lines");
    let d = parse_td3(l1, l2).expect("an interior filler must not be refused");
    assert!(d.valid(), "checks: {:?}", d.checks);
}

/// The ICAO MRV-A specimen's own document number, `L898902C<3...` — a
/// trailing filler in the ninth cell (before the check digit), never a
/// leading one.
#[test]
fn mrv_trailing_filler_document_number_stays_accepted() {
    let d = parse_mrv_a(
        "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
        "L898902C<3UTO6908061F9406236ZE184226B<<<<<<<",
    )
    .expect("the ICAO specimen must not be refused");
    assert!(d.valid());
    assert_eq!(d.document_number, "L898902C");
}

/// An all-filler document number is refused the same as any other leading
/// filler — the owner's rule is "starts with `<`", which an all-filler field
/// satisfies trivially, not a special case.
#[test]
fn an_all_filler_document_number_is_refused() {
    let fields = Td3Fields {
        issuing_country: "UTO".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth("740812"),
        sex: Sex::Unspecified,
        date_of_expiry: support::expiry("300101"),
        personal_number: None,
        ..Default::default()
    };
    let zone = format_td3(&fields);
    let (l1, l2) = zone.split_once('\n').expect("two TD3 lines");
    assert_eq!(
        parse_td3(l1, l2),
        Err(MrzError::LeadingFiller {
            field: Field::DocumentNumber,
            line: 1,
            position: 0,
        })
    );
}

/// Knock-on (b): `intact_td1_starts` must treat a TD1 whose document number
/// is refused for a leading filler as an intact TD1 layout, the same as one
/// whose check digits merely fail (#409) — otherwise the TD2 loop pads its
/// first two rows into a checksum-valid-looking TD2 reading, reopening #409
/// for this specific new refusal.
#[test]
fn td1_with_leading_filler_document_number_never_reflows_to_td2() {
    let mrz = format_td1(&Td1Fields {
        issuing_country: "UTO".to_string(),
        document_number: "<1234567".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Male,
        date_of_expiry: support::expiry("301230"),
        optional_data_1: None,
        optional_data_2: None,
        ..Default::default()
    });
    let lines: Vec<&str> = mrz.lines().collect();
    let (l1, l2, l3) = (lines[0], lines[1], lines[2]);

    assert_eq!(
        parse_td1(l1, l2, l3),
        Err(MrzError::LeadingFiller {
            field: Field::DocumentNumber,
            line: 0,
            position: 5,
        })
    );

    let text = format!("{l1}\n{l2}\n{l3}");
    // Refused or incomplete is fine; a false TD2 reflow is the only forbidden outcome.
    if let Ok(doc) = find_and_parse(&text) {
        assert_ne!(
            doc.format,
            Format::Td2,
            "a refused TD1 must never be reported as a padded TD2 reading"
        );
    }
}

// Knock-on (a), the damaged-pass gate: before this fix, every candidate
// that hit `LeadingFiller` left `fallback` at `None` (only `Ok` readings
// reach `consider`), so `if fallback.is_some()` alone would skip
// `damaged_pass` entirely and fall through to `shape_seen`'s
// `IncompleteSequence` — reported above at
// `find_and_parse_reports_leading_filler_not_incomplete_sequence`, which is
// the direct, observable proof that `refused.is_some()` now keeps that gate
// open. This module deliberately does not also attempt a *positive*
// damaged-pass recovery on a refused reading: for every two-line format the
// document number sits at column 0, and `restored()` (`parser.rs`) excludes
// a dropped character there by construction, so a genuine leading-filler
// misread is never damaged-pass-recoverable there by design -- and TD1,
// whose document number starts at column 5 and would otherwise qualify,
// loses `intact_td1_starts`'s protection the moment any of its three rows
// arrives narrower than 30 cells, which hands the same lines to the
// TD2-cannibalization guard (#409) and sets `fallback` from that unrelated
// misreading before `damaged_pass` is ever reached. That is a real, static
// gap in `intact_td1_starts` (unrelated to #536: it does not recognize a
// merely-narrowed TD1 row as intact, only an already-full-width one), not
// something a differently-chosen fixture can route around -- confirmed by
// tracing a concrete case through `find_and_parse`, which returned the
// wrong `Format::Td2` fallback every time, independent of field values.
