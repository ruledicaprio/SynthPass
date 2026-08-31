//! Regressions for `MrzError::IncompleteSequence` (`crates/mrz/src/parser.rs`,
//! `find_and_parse_with`) — `knowledge/MRZ_SEQUENCE_COMPLETENESS.md` chunk 4.
//!
//! A line-1 whose document-code prefix is recognized, with no companion line
//! anywhere in the text, must be distinguished from finding nothing
//! MRZ-shaped at all (`MrzError::NotFound`) — the literal "sequence
//! completeness" gap the chunk is named for. One case per format; fixtures
//! are emitted via the real formatters (`format_td3` etc.), same discipline
//! `line1_right_shift.rs`/`td1_line_gap.rs` follow, then only line 1 is kept.
//!
//! Fields are realistic (matching the identity `line1_right_shift.rs` and
//! `td1_line_gap.rs` already use), not `Default::default()`'s all-filler
//! values: an all-filler TD1 line degenerates into content generous enough
//! for `variants()`'s padding tolerance to also parse as a checksum-valid
//! TD2 read, which is a real, separate, already-guarded-against crate
//! behavior (`a_genuine_td1_never_parses_as_td2` in `td1_line_gap.rs`) and
//! not what these tests exist to exercise.

use mrz::{
    find_and_parse, format_mrv_a, format_mrv_b, format_td1, format_td2, format_td3, Format,
    MrvAFields, MrvBFields, MrzError, Td1Fields, Td2Fields, Td3Fields,
};

fn assert_incomplete(text: &str, expected_format: Format, expected_lines_expected: u8) {
    match find_and_parse(text) {
        Err(MrzError::IncompleteSequence {
            format,
            lines_found,
            lines_expected,
        }) => {
            assert_eq!(format, expected_format);
            assert_eq!(lines_found, 1);
            assert_eq!(lines_expected, expected_lines_expected);
        }
        other => panic!(
            "expected IncompleteSequence for {expected_format:?}, got {other:?} \
             (text: {text:?})"
        ),
    }
}

#[test]
fn td3_line_1_alone_is_incomplete_not_not_found() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "E00000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        personal_number: None,
    });
    let l1 = mrz
        .lines()
        .next()
        .expect("format_td3 emits at least one line");
    assert_incomplete(l1, Format::Td3, 2);
}

#[test]
fn mrv_a_line_1_alone_is_incomplete_not_not_found() {
    let mrz = format_mrv_a(&MrvAFields {
        document_code: "V".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "E00000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        optional_data: None,
    });
    let l1 = mrz
        .lines()
        .next()
        .expect("format_mrv_a emits at least one line");
    assert_incomplete(l1, Format::MrvA, 2);
}

#[test]
fn mrv_b_line_1_alone_is_incomplete_not_not_found() {
    let mrz = format_mrv_b(&MrvBFields {
        document_code: "V".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "E00000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        optional_data: None,
    });
    let l1 = mrz
        .lines()
        .next()
        .expect("format_mrv_b emits at least one line");
    assert_incomplete(l1, Format::MrvB, 2);
}

/// TD2's split-line loop, unlike every other format's, iterates
/// `0..lines.len().saturating_sub(1)` (no lookahead-window gap tolerance —
/// it needs `lines[i+1]` to be the very next physical line, a pre-existing
/// asymmetry this chunk doesn't touch). That means a *single*-line input
/// never reaches the loop body at all, so this fixture pairs line 1 with an
/// unrelated non-MRZ companion line to exercise the loop the same way real
/// OCR noise would, rather than testing something the scan structurally
/// cannot see.
#[test]
fn td2_line_1_with_an_unrelated_companion_is_incomplete_not_not_found() {
    let mrz = format_td2(&Td2Fields {
        document_code: "IP".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "E00000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        optional_data: None,
    });
    let l1 = mrz
        .lines()
        .next()
        .expect("format_td2 emits at least one line");
    let text = format!("{l1}\nfooter text, not an MRZ line at all");
    assert_incomplete(&text, Format::Td2, 2);
}

fn td1_fields() -> Td1Fields {
    Td1Fields {
        document_code: "ID".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "E00000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        optional_data_1: None,
        optional_data_2: None,
    }
}

/// Line 1 alone — TD1 needs three lines, `lines_expected` must say so.
///
/// (A "line 1 + line 2, no line 3" case was tried and dropped: with this
/// fixture's line 1/2 alone, `variants()`'s generous padding tolerance lets
/// TD2's scan — which runs after TD1's — manufacture an unrelated
/// checksum-invalid *partial* reading from the same 60 characters, and a
/// partial reading legitimately outranks `IncompleteSequence` in
/// `find_and_parse`'s own priority order (see the module doc). That is
/// correct behavior of the existing fallback mechanism, not a bug, but it
/// makes "1 of 3" the reliable case to pin here rather than "2 of 3.")
#[test]
fn td1_line_1_alone_is_incomplete_not_not_found() {
    let mrz = format_td1(&td1_fields());
    let l1 = mrz
        .lines()
        .next()
        .expect("format_td1 emits at least one line");
    assert_incomplete(l1, Format::Td1, 3);
}

/// A genuinely empty document — nothing MRZ-shaped anywhere — must still
/// report the original `NotFound`, not a false `IncompleteSequence`.
#[test]
fn genuinely_empty_text_is_still_not_found() {
    match find_and_parse("just some ordinary prose, no MRZ in sight") {
        Err(MrzError::NotFound) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}
