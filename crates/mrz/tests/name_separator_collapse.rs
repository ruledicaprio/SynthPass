//! Regressions for `clean_name`'s single-`<` fallback (`crates/mrz/src/parser.rs`).
//!
//! `synthpass-bench --dump-ocr --document-type mrva --count 5 --seed 42`
//! (`knowledge/ROADMAP.md`'s M6 MRVA/MRVB measurement note) showed OCR
//! dropping one of the two filler characters in the `<<` primary/secondary
//! name separator outright — `SURNAME<<GIVEN` read as `SURNAME<GIVEN`, with
//! no `KK` misread anywhere for [`mrz::checksum`]'s existing
//! `fix_name_separator` to catch (that repair targets a *different* failure
//! mode: `<` misread as `K`, not `<` dropped). Before the fix, `clean_name`
//! required a literal `<<` to split at all, so a collapsed separator sent
//! `given_names` to `""` unconditionally — first measured on MRV-A/MRV-B
//! (`given_names` CER 63.5%/76.7%) but present on every format that calls
//! `clean_name`, since none of them check-digit-cover line 1's name field.
//!
//! **Fixtures are emitted, not transcribed**, same discipline
//! `tests/td1_line_gap.rs` follows: each test builds its own zone with
//! `mrz::format_*` from placeholder field values, then damages exactly the
//! separator to reproduce the measured failure shape — no real document's
//! data in the repository.

use mrz::{parse_mrv_a, parse_mrv_b, parse_td3, MrvAFields, MrvBFields, Td3Fields};

/// Replace a fixture's `<<` primary/secondary separator with a single `<`,
/// then pad the line back to its original width with a trailing filler —
/// reproducing "OCR dropped one filler character" without touching anything
/// else, so the test isolates `clean_name`'s fallback from the unrelated
/// width-recovery machinery `find_and_parse`'s `variants()` would otherwise
/// also exercise.
fn collapse_separator(line1: &str) -> String {
    let sep_pos = line1
        .find("<<")
        .expect("fixture line 1 must carry the primary/secondary separator");
    let mut damaged = line1.to_string();
    damaged.replace_range(sep_pos..sep_pos + 2, "<");
    damaged.push('<');
    assert_eq!(
        damaged.len(),
        line1.len(),
        "collapsing one filler and padding one back must preserve line width"
    );
    damaged
}

#[test]
fn mrv_a_recovers_the_name_split_after_a_collapsed_separator() {
    let mrz = mrz::format_mrv_a(&MrvAFields {
        document_code: "V".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E00000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_mrv_a emits two lines");

    let baseline = parse_mrv_a(l1, l2).expect("emitted MRV-A parses");
    assert_eq!(baseline.surname, "ESKANDARI");
    assert_eq!(baseline.given_names, "MAREN");

    let damaged = collapse_separator(l1);
    let data = parse_mrv_a(&damaged, l2).expect("a collapsed separator must still parse");
    assert_eq!(data.surname, "ESKANDARI");
    assert_eq!(data.given_names, "MAREN");
    assert!(
        data.valid(),
        "line 2's real check digits are untouched by the line-1 damage"
    );
}

#[test]
fn mrv_b_recovers_the_name_split_after_a_collapsed_separator() {
    let mrz = mrz::format_mrv_b(&MrvBFields {
        document_code: "V".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E0000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_mrv_b emits two lines");

    let baseline = parse_mrv_b(l1, l2).expect("emitted MRV-B parses");
    assert_eq!(baseline.surname, "ESKANDARI");
    assert_eq!(baseline.given_names, "MAREN");

    let damaged = collapse_separator(l1);
    let data = parse_mrv_b(&damaged, l2).expect("a collapsed separator must still parse");
    assert_eq!(data.surname, "ESKANDARI");
    assert_eq!(data.given_names, "MAREN");
    assert!(data.valid());
}

/// TD3 measured the identical pattern the same day (`given_names` CER
/// 63.9%, same-commit baseline in the ROADMAP note) — this is not an
/// MRV-specific defect, `clean_name` is shared by every format.
#[test]
fn td3_recovers_the_name_split_after_a_collapsed_separator() {
    let mrz = mrz::format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E000000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");

    let damaged = collapse_separator(l1);
    let data = parse_td3(&damaged, l2).expect("a collapsed separator must still parse");
    assert_eq!(data.surname, "ESKANDARI");
    assert_eq!(data.given_names, "MAREN");
    assert!(data.valid());
}

/// The explicit known limit, pinned rather than left implicit: when OCR
/// drops **both** filler characters (not just one — observed directly in
/// the same `--dump-ocr` run, e.g. `ESKANDARI<<MAREN` read as
/// `ESKANDARIMAREN`), there is no `<` left anywhere to split on. This is
/// unrecoverable by construction, not a gap in the fallback above — pinned
/// so a future change to `clean_name` doesn't accidentally start guessing a
/// split boundary with zero evidence for it.
#[test]
fn a_fully_collapsed_separator_is_not_recoverable_and_must_not_be_silently_guessed() {
    let mrz = mrz::format_mrv_a(&MrvAFields {
        document_code: "V".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E00000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_mrv_a emits two lines");

    let sep_pos = l1.find("<<").unwrap();
    let mut damaged = l1.to_string();
    damaged.replace_range(sep_pos..sep_pos + 2, "");
    damaged.push_str("<<");
    assert_eq!(damaged.len(), l1.len());

    let data = parse_mrv_a(&damaged, l2).expect("still parses — line 2's checks are untouched");
    assert_eq!(
        data.surname, "ESKANDARIMAREN",
        "both names collapse into surname with no separator evidence left"
    );
    assert_eq!(data.given_names, "");
    assert!(
        data.valid(),
        "line 1 carries no check digit in any format, so this reads as a Tier-1 hit despite \
         the fused name — the known, already-documented line-1 blind spot"
    );
}
