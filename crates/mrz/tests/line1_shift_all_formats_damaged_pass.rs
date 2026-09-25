//! #476: generalizes #468/#469's TD3-only fix (`td3_line1_variants`, shared
//! by `class_sweep_pass` and `damaged_pass`, chaining `repair_td3_line1_shifted`
//! ahead of the plain repair) to MRV-A, TD2 and MRV-B via
//! `two_line_line1_variants_shifted`, and gives TD1's own damaged-pass shapes
//! `repair_td1_line1_unshifted` as a second line-1 candidate the way the
//! ordinary TD1 scan already has.
//!
//! Each fixture pairs a dropped line-1 position-1 filler (or, for the guard
//! cases, a genuine issuer-defined second document-code letter) with a
//! line-2 corruption only `damaged_pass`'s `substituted` can repair — a
//! digit swapped for a [`mrz::CONFUSABLES`] lookalike outside the ranges
//! `repair_*_line2` ever touches for the document number, or (TD1) outside
//! what `digitize` maps back — so the ordinary scan never validates and the
//! shift/unshift question is decided inside `damaged_pass` itself.
//!
//! **Fixtures are emitted, not transcribed**, the same discipline
//! `tests/damaged_td3_line1_shift.rs` and `tests/line1_prefix_shift.rs`
//! follow.

mod support;

use mrz::{
    find_and_parse, format_mrv_a, format_mrv_b, format_td1, format_td2, Format, MrvAFields,
    MrvBFields, Td1Fields, Td2Fields,
};

/// Distinct digits, no repeats — see `damaged_td3_line1_shift.rs`'s
/// `confuse_document_number_lead` doc comment for why a repeated digit
/// invites an unrelated residue-class ambiguity these fixtures must not
/// exercise.
const DISTINCT_DIGIT_DOCUMENT_NUMBER: &str = "123456789";

/// Drop line 1's position-1 filler — the exact OCR retry-loop shape
/// `tests/line1_prefix_shift.rs`'s `drop_position_1` reproduces for the
/// ordinary scan.
fn drop_position_1(line1: &str) -> String {
    let mut damaged = line1.to_string();
    damaged.remove(1);
    damaged
}

/// Swap a two-line format's line-2 leading document-number digit for its
/// `CONFUSABLES` lookalike letter `I` — `repair_td2_line2`/`repair_mrv_a_line2`/
/// `repair_mrv_b_line2` never touch the document number's own digits (only
/// its check digit at position 9 and the fields after it), so only
/// `damaged_pass`'s `substituted` sweep, not the ordinary per-line repair,
/// can recover it. Mirrors `damaged_td3_line1_shift.rs`'s
/// `confuse_document_number_lead` exactly; the two-line layout is identical
/// across TD3/MRV-A/TD2/MRV-B for this field.
fn confuse_document_number_lead(line2: &str) -> String {
    let mut chars: Vec<char> = line2.chars().collect();
    assert_eq!(
        chars[0], '1',
        "sanity: fixture's document number must start with a 1, got {line2}"
    );
    chars[0] = 'I';
    chars.into_iter().collect()
}

/// Swap TD1 line 2's leading date-of-birth digit for its `CONFUSABLES`
/// lookalike letter `A` (`('4', "A")`). Unlike the two-line formats' document
/// number, TD1's date-of-birth field *is* inside `repair_td1_line2`'s
/// `digitize` range — but `digitize` only maps `O`/`Q`/`D`/`I`/`L`/`Z`/`S`/`G`/`B`
/// back to a digit, never `A`, so this substitution still survives the
/// ordinary repair untouched and only `damaged_pass`'s `substituted` sweep
/// (which walks the full `CONFUSABLES` table) can recover it.
fn confuse_dob_lead(line2: &str) -> String {
    let mut chars: Vec<char> = line2.chars().collect();
    assert_eq!(
        chars[0], '4',
        "sanity: fixture's date of birth must start with a 4, got {line2}"
    );
    chars[0] = 'A';
    chars.into_iter().collect()
}

/// TD2 seed 37's shape (issue #476): the as-read (plain) issuer `BRN` and the
/// unshifted issuer `GBR` both resolve to real countries and disagree. With
/// no closed document-code table to prefer one the way TD3's
/// `td3_line1_is_genuine_table_code` does, line 1 alone cannot arbitrate --
/// #440's unanimity gate must refuse rather than silently returning whichever
/// reading happens to be admissible first.
#[test]
fn td2_seed37_shape_disagreement_is_refused() {
    let mrz = format_td2(&Td2Fields {
        document_code: "I".to_string(),
        issuing_country: "GBR".to_string(),
        document_number: DISTINCT_DIGIT_DOCUMENT_NUMBER.to_string(),
        surname: "NYSTROM".to_string(),
        given_names: "LEILANI".to_string(),
        nationality: "GBR".to_string(),
        date_of_birth: support::birth("810612"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("340911"),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td2 emits two lines");
    assert!(
        l1.starts_with("I<GBRNYSTROM"),
        "sanity: emitted line 1 is {l1}"
    );

    let damaged_l1 = drop_position_1(l1);
    let damaged_l2 = confuse_document_number_lead(l2);
    let text = format!("{damaged_l1}\n{damaged_l2}");

    let data = find_and_parse(&text)
        .unwrap_or_else(|e| panic!("expected a checksum-failed fallback, got a hard error: {e:?}"));

    assert!(
        !data.valid(),
        "GBR (unshifted) versus BRN (as-read) both resolve and disagree; \
         the read must be refused, not silently accepted as either"
    );
}

/// TD2 seed 22's shape (issue #476): the as-read issuer `RAM` does not
/// resolve to any real country, but the unshifted issuer `BRA` does. Exactly
/// one admissible reading survives, so it is used -- this is the ordinary
/// generalized-shift recovery, exercised here through `damaged_pass` because
/// line 2 also needs `substituted`'s repair.
#[test]
fn td2_seed22_shape_only_unshifted_resolves_is_accepted() {
    let mrz = format_td2(&Td2Fields {
        document_code: "I".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: DISTINCT_DIGIT_DOCUMENT_NUMBER.to_string(),
        surname: "MORAES".to_string(),
        given_names: "PAULA".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("810612"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("340911"),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td2 emits two lines");
    assert!(
        l1.starts_with("I<BRAMORAES"),
        "sanity: emitted line 1 is {l1}"
    );

    let damaged_l1 = drop_position_1(l1);
    let damaged_l2 = confuse_document_number_lead(l2);
    let text = format!("{damaged_l1}\n{damaged_l2}");

    let data = find_and_parse(&text)
        .unwrap_or_else(|e| panic!("expected the corrected reading, got a hard error: {e:?}"));

    assert_eq!(data.format, Format::Td2);
    assert_eq!(
        data.issuing_country, "BRA",
        "RAM does not resolve; the unshifted BRA reading is the only admissible one"
    );
    assert_eq!(data.document_number, DISTINCT_DIGIT_DOCUMENT_NUMBER);
    assert!(data.valid());
}

/// A genuine TD2 issuer-defined two-letter document code (`IP`) must survive
/// `damaged_pass` unchanged when line 2 separately needs a substitution
/// repair -- `two_line_line1_admissible`'s country-resolve-only check keeps
/// the as-read line whenever the unshifted alternative's issuer does not
/// itself resolve, exactly as `unshift_if_country_resolves` already does for
/// the ordinary scan.
#[test]
fn td2_genuine_two_letter_code_survives_damaged_pass() {
    let mrz = format_td2(&Td2Fields {
        document_code: "IP".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: DISTINCT_DIGIT_DOCUMENT_NUMBER.to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("301230"),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td2 emits two lines");
    assert!(l1.starts_with("IPBRA"), "sanity: emitted line 1 is {l1}");

    let damaged_l2 = confuse_document_number_lead(l2);
    let text = format!("{l1}\n{damaged_l2}");

    let data = find_and_parse(&text)
        .expect("a genuine two-letter code plus a line-2 substitution must still recover");
    assert_eq!(data.format, Format::Td2);
    assert_eq!(
        data.document_type, "IP",
        "a genuine two-letter document code must survive the damaged pass unchanged"
    );
    assert_eq!(data.issuing_country, "BRA");
    assert_eq!(data.document_number, DISTINCT_DIGIT_DOCUMENT_NUMBER);
    assert!(data.valid());
}

/// MRV-B recovery case: a dropped position-1 filler whose as-read issuer
/// does not resolve, recovered through `damaged_pass` alongside a line-2
/// substitution -- the same shape `td2_seed22_shape_only_unshifted_resolves_is_accepted`
/// pins for TD2, generalized to MRV-B via `repair_mrv_b_line1_shifted`.
#[test]
fn mrv_b_dropped_filler_recovered_through_damaged_pass() {
    let mrz = format_mrv_b(&MrvBFields {
        document_code: "V".to_string(),
        issuing_country: "GBR".to_string(),
        document_number: DISTINCT_DIGIT_DOCUMENT_NUMBER.to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "GBR".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("301230"),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_mrv_b emits two lines");
    assert!(
        l1.starts_with("V<GBRESKANDARI"),
        "sanity: emitted line 1 is {l1}"
    );

    let damaged_l1 = drop_position_1(l1);
    let damaged_l2 = confuse_document_number_lead(l2);
    let text = format!("{damaged_l1}\n{damaged_l2}");

    let data = find_and_parse(&text)
        .unwrap_or_else(|e| panic!("expected the corrected reading, got a hard error: {e:?}"));

    assert_eq!(data.format, Format::MrvB);
    assert_eq!(data.document_type, "V");
    assert_eq!(data.issuing_country, "GBR");
    assert_eq!(data.document_number, DISTINCT_DIGIT_DOCUMENT_NUMBER);
    assert!(data.valid());
}

/// A genuine MRV-B issuer-defined second document-code letter must survive
/// `damaged_pass` unchanged when line 2 separately needs a substitution
/// repair, mirroring `td2_genuine_two_letter_code_survives_damaged_pass`.
#[test]
fn mrv_b_genuine_second_letter_code_survives_damaged_pass() {
    let mrz = format_mrv_b(&MrvBFields {
        document_code: "VC".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: DISTINCT_DIGIT_DOCUMENT_NUMBER.to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("301230"),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_mrv_b emits two lines");
    assert!(l1.starts_with("VCBRA"), "sanity: emitted line 1 is {l1}");

    let damaged_l2 = confuse_document_number_lead(l2);
    let text = format!("{l1}\n{damaged_l2}");

    let data = find_and_parse(&text)
        .expect("a genuine second-letter code plus a line-2 substitution must still recover");
    assert_eq!(data.format, Format::MrvB);
    assert_eq!(
        data.document_type, "VC",
        "a genuine issuer-defined second letter must survive the damaged pass unchanged"
    );
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid());
}

/// A genuine MRV-A issuer-defined second document-code letter must survive
/// `damaged_pass` unchanged when line 2 separately needs a substitution
/// repair, mirroring the MRV-B/TD2 guard cases.
#[test]
fn mrv_a_genuine_second_letter_code_survives_damaged_pass() {
    let mrz = format_mrv_a(&MrvAFields {
        document_code: "VC".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: DISTINCT_DIGIT_DOCUMENT_NUMBER.to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("301230"),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_mrv_a emits two lines");
    assert!(l1.starts_with("VCBRA"), "sanity: emitted line 1 is {l1}");

    let damaged_l2 = confuse_document_number_lead(l2);
    let text = format!("{l1}\n{damaged_l2}");

    let data = find_and_parse(&text)
        .expect("a genuine second-letter code plus a line-2 substitution must still recover");
    assert_eq!(data.format, Format::MrvA);
    assert_eq!(data.document_type, "VC");
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid());
}

// The TD1 recall case (a dropped line-1 filler recovered by `damaged_pass`
// when line 2 also needs repair) is pinned as a white-box unit test on
// `td1_line1_variants` in `crates/mrz/src/parser.rs`'s own `tests` module,
// not here: TD1 and TD2 share the `I`/`A`/`C` line-1 prefix by design, and
// forcing TD1's *own* ordinary scan to fail (so `damaged_pass` is reached at
// all) also hands `find_and_parse`'s unconditional TD2 scan the same two
// lines. TD2's own `restored`/`substituted` search over that raw text can
// then coincidentally produce a structurally-valid-looking TD2 fallback
// candidate that wins `fallback_rank` before `damaged_pass` ever runs -- a
// pre-existing cross-format ambiguity `crates/mrz/tests/repair.rs`'s TD1
// punched-hole fixtures avoid the same way, by testing the repair primitive
// directly instead of the full multi-format scan.

/// A genuine TD1 issuer-defined two-letter document code (`ID`) must survive
/// `damaged_pass` unchanged when line 2 separately needs a substitution
/// repair. Unlike TD2/MRV-A/MRV-B, no country-resolve admissibility filter
/// gates TD1's extra candidate: `repair_td1_line1_unshifted` applied to an
/// already-genuine line corrupts the document number, so its own check digit
/// -- not a heuristic -- keeps it out of `damaged_pass`'s `hits`.
#[test]
fn td1_genuine_two_letter_code_survives_damaged_pass() {
    let mrz = format_td1(&Td1Fields {
        document_code: "ID".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: DISTINCT_DIGIT_DOCUMENT_NUMBER.to_string(),
        optional_data_1: None,
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth("440101"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("301230"),
        optional_data_2: None,
    });
    let lines: Vec<&str> = mrz.lines().collect();
    assert_eq!(lines.len(), 3, "format_td1 emits exactly three lines");
    let (l1, l2, l3) = (lines[0], lines[1], lines[2]);
    assert!(l1.starts_with("IDUTO"), "sanity: emitted line 1 is {l1}");

    let damaged_l2 = confuse_dob_lead(l2);
    let text = format!("{l1}\n{damaged_l2}\n{l3}");

    let data = find_and_parse(&text)
        .expect("a genuine two-letter code plus a line-2 substitution must still recover");
    assert_eq!(data.format, Format::Td1);
    assert_eq!(
        data.document_type, "ID",
        "a genuine two-letter document code must survive the damaged pass unchanged"
    );
    assert_eq!(data.document_number, DISTINCT_DIGIT_DOCUMENT_NUMBER);
    assert!(data.valid());
}
