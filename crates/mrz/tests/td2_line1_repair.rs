//! Regression for TD2's line-1 dropped-filler repair
//! (`crates/mrz/src/parser.rs::repair_td2_line1_shifted`) —
//! `knowledge/MRZ_SEQUENCE_COMPLETENESS.md`'s Chunk 6.
//!
//! TD2 shares TD1's genuine two-real-letter document-code family, but unlike
//! TD1 has no check digit anywhere on line 1 to arbitrate an unshifted
//! reading against the as-read one. The fix reuses the issuing-country gate
//! `repair_td3_line1_shifted`/`repair_mrv_a_line1_shifted` already ship
//! (`country_name` on the *unshifted* reading's positions 2..5) rather than
//! a document-code dictionary — see `repair_td2_line1_shifted`'s doc comment
//! for why no such dictionary actually exists to build one from (ICAO 9303
//! Part 5/6 Note k makes the second document-code character issuer-
//! discretionary, not a closed set).
//!
//! The "a genuine two-letter document code must survive untouched" guard
//! already exists — `crates/mrz/tests/line1_prefix_shift.rs`'s
//! `td2_with_a_genuine_two_letter_document_code_is_unaffected` — and keeps
//! passing unchanged by this fix (verified, not merely assumed: the gate
//! only fires when the *unshifted* reading's country slot resolves, and a
//! genuine two-letter code's unshifted country slot is garbage — see that
//! function's doc comment for the worked trace). This file adds the missing
//! other half: the drop itself must actually recover.
//!
//! **Fixtures are emitted, not transcribed**, same discipline
//! `tests/td1_line_gap.rs` and `tests/line1_prefix_shift.rs` follow.

use mrz::{find_and_parse, format_td2, Format, Td2Fields};

fn td2_lines() -> (String, String) {
    let mrz = format_td2(&Td2Fields {
        document_code: "I".to_string(),
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
    let (l1, l2) = mrz.split_once('\n').expect("format_td2 emits two lines");
    (l1.to_string(), l2.to_string())
}

/// The direct reproduction of the OCR failure `repair_td2_line1_shifted`
/// fixes: line 1's position-1 filler (`I<` in `document_code`) dropped
/// outright, reading `IUTO...` where the truth is `I<UTO...`, shifting
/// `issuing_country` and everything after it one position left — the same
/// mechanism `tests/line1_prefix_shift.rs`'s TD3/MRV-A/MRV-B tests and
/// `tests/td1_line_gap.rs`'s `a_dropped_line_1_filler_position_is_recovered`
/// already pin for their formats.
#[test]
fn a_dropped_line_1_filler_position_is_recovered() {
    let (l1, l2) = td2_lines();
    let mut damaged = l1.clone();
    damaged.remove(1);
    assert_eq!(damaged.len(), 35, "dropping one character narrows the line");

    let text = format!("{damaged}\n{l2}");
    let data = find_and_parse(&text).expect("the unshifted candidate must recover the read");

    assert_eq!(data.format, Format::Td2);
    assert_eq!(data.document_type, "I");
    assert_eq!(data.issuing_country, "UTO");
    assert!(data.valid(), "line 2's real check digits are untouched");
}

/// The guard case: an undamaged zone (position 1 genuinely already `<`)
/// must round-trip unchanged — `repair_td2_line1_shifted` must never fire
/// when there's nothing to unshift, mirroring
/// `tests/line1_prefix_shift.rs`'s `an_undamaged_td3_zone_is_not_spuriously_unshifted`.
#[test]
fn an_undamaged_td2_zone_is_not_spuriously_unshifted() {
    let (l1, l2) = td2_lines();
    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("emitted TD2 parses");
    assert_eq!(data.format, Format::Td2);
    assert_eq!(data.document_type, "I");
    assert_eq!(data.issuing_country, "UTO");
    assert!(data.valid());
}
