//! Regressions for the M6 TD1 root-cause diagnosis (`knowledge/ROADMAP.md`'s
//! M6 execution note): TD1's third MRZ line — the one carrying the name —
//! was routinely lost, not because OCR misreads it, but because
//! `find_and_parse`'s TD1 branch required its three candidate lines to be
//! **strictly consecutive** in the OCR text, where every other format
//! (TD2/TD3/MRV-A/MRV-B) already tolerates a gap. `synthpass-bench
//! --dump-ocr` (run against `--document-type td1 --count 3 --seed 42`)
//! showed why a gap happens in practice: the OCR engine's internal
//! multi-pass retry loop concatenates several attempts into one text blob,
//! and a pass that fails to detect line 3 as its own region leaves the
//! watermark or a duplicate of line 1 sitting where line 3 "should" be,
//! adjacent to a line 1/line 2 pair read in a *different* pass.
//!
//! **The fixtures are emitted, not transcribed** — same discipline
//! `tests/repair.rs` follows: each test builds its own zone with
//! `mrz::format_td1` from placeholder field values, so the suite reproduces
//! the real failure *shape* with no real document's data in the repository.

use mrz::{find_and_parse, format_td1, Format, Td1Fields};

/// A synthetic TD1 zone (three 30-char lines), built and check-digit-verified
/// by the emitter itself — self-verifying the same way `tests/repair.rs`'s
/// fixtures are.
fn td1_lines() -> (String, String, String) {
    let mrz = format_td1(&Td1Fields {
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
    });
    let lines: Vec<&str> = mrz.lines().collect();
    assert_eq!(lines.len(), 3, "format_td1 emits exactly three lines");
    (
        lines[0].to_string(),
        lines[1].to_string(),
        lines[2].to_string(),
    )
}

/// A 30-character, all-letters line shaped like OCR's actual reading of the
/// "SYNTHETIC / SPECIMEN" watermark — e.g. `synthpass-bench --dump-ocr`
/// recorded `"STNHENLSPELJMENSTNNEL"` and `"SYNTHETIC SPECIMEN STHTHETIC"`
/// for the real watermark band. Deliberately does not start with `I`/`A`/`C`
/// (the real watermark text doesn't either), so it must never be picked up
/// as an MRZ line-1 candidate.
const WATERMARK_SHAPED_LINE: &str = "STNHENLSPELJMENSTNNELSPECIMENS";

#[test]
fn watermark_shaped_line_is_not_mistaken_for_mrz_but_the_real_lines_are_still_found() {
    let (l1, l2, l3) = td1_lines();
    assert!(
        mrz::parse_td1(&l1, &l2, &l3)
            .expect("emitted TD1 parses")
            .valid(),
        "fixture must be checksum-valid before use"
    );
    assert_eq!(WATERMARK_SHAPED_LINE.len(), 30);

    let text = format!("{WATERMARK_SHAPED_LINE}\n{l1}\n{l2}\n{l3}");
    let data = find_and_parse(&text).expect("the real triplet is still found after the decoy");

    assert_eq!(data.format, Format::Td1);
    assert_eq!(data.surname, "ESKANDARI");
    assert_eq!(data.given_names, "MAREN");
    assert!(data.valid(), "the recovered reading must be checksum-valid");
}

/// The direct reproduction of the M6 bug: a stray line — shaped like
/// nothing in particular, the way a spurious OCR detection reads — sitting
/// between line 2 and line 3. Before this milestone's fix, TD1's scan
/// required `lines[i]`/`lines[i+1]`/`lines[i+2]` to be exactly consecutive,
/// so this would have fallen through to [`mrz::MrzError::NotFound`] or a
/// checksum-failed fallback.
///
/// The stray is deliberately **short** (well under `variants`'s length
/// tolerance for a 30-char target), not merely "different" — line 3 carries
/// no check digit of its own (TD1's checks live entirely on lines 1-2, per
/// the module doc comment), so a stray line that *is* candidate-shaped would
/// parse as a structurally valid line 3 exactly as readily as the real one,
/// which is the documented check-digit blind spot itself
/// (`knowledge/ROADMAP.md`'s M6 note), not something a parser fix can
/// disambiguate. This test is about the gap surviving at all, not about
/// picking the right line 3 out of two equally check-digit-silent
/// candidates — that would be a different (and by design, unwinnable) test.
#[test]
fn a_short_stray_line_between_line_2_and_line_3_does_not_break_the_read() {
    let (l1, l2, l3) = td1_lines();
    let stray = "PASSPORT"; // 8 chars — below variants()'s tolerance for a 30-char target

    let text = format!("{l1}\n{l2}\n{stray}\n{l3}");
    let data =
        find_and_parse(&text).expect("a stray line between rows 2 and 3 must not hide row 3");

    assert_eq!(data.format, Format::Td1);
    assert_eq!(data.surname, "ESKANDARI");
    assert_eq!(data.given_names, "MAREN");
    assert!(data.valid());
}

/// Same shape, stray line between line 1 and line 2 instead — the gap
/// tolerance has to work on either boundary, not just the one the probe
/// happened to observe. Line 2 *does* carry real check digits (DOB, expiry,
/// composite), so unlike the line-3 case above this stray can be
/// MRZ-shaped and candidate-length without creating any ambiguity: a
/// garbage reading interpreted as line 2 essentially never satisfies three
/// independent check digits by chance, so the scan naturally falls through
/// to the real line 2 before ever returning.
#[test]
fn a_stray_line_between_line_1_and_line_2_does_not_break_the_read() {
    let (l1, l2, l3) = td1_lines();
    let stray = "XJ7QP2M9K4LWZTHV3RCGY8ND5FBS61";

    let text = format!("{l1}\n{stray}\n{l2}\n{l3}");
    let data =
        find_and_parse(&text).expect("a stray line between rows 1 and 2 must not hide the read");

    assert_eq!(data.format, Format::Td1);
    assert_eq!(data.surname, "ESKANDARI");
    assert_eq!(data.given_names, "MAREN");
    assert!(data.valid());
}

/// `synthpass-bench --dump-ocr` also showed TD1 line 1's position-1 filler
/// (`I<` in `document_code`) routinely dropped outright by OCR — read as
/// `IDUTO...` where the truth is `I<UTO...` (using this fixture's `"ID"`
/// document code, position 1 already carries the interesting byte). That
/// shifts `document_number` left by one and breaks TD1's own on-line-1
/// check digit (`repair_td1_line1_unshifted`'s doc comment has the full
/// mechanism). Deletes character index 1 from line 1 to reproduce exactly
/// that damage, the same `delete_at` shape `tests/repair.rs` uses for its
/// punched-hole fixture.
#[test]
fn a_dropped_line_1_filler_position_is_recovered() {
    let (l1, l2, l3) = td1_lines();
    let mut damaged = l1.clone();
    damaged.remove(1);
    assert_eq!(damaged.len(), 29, "dropping one character narrows the line");

    let text = format!("{damaged}\n{l2}\n{l3}");
    let data = find_and_parse(&text).expect("the unshifted candidate must recover the read");

    assert_eq!(data.format, Format::Td1);
    assert_eq!(data.document_number, "E00000000");
    assert!(data.valid());
}

/// The plan's explicit cannibalization guard: `variants`'s padding tolerance
/// (+14) makes a 30-char TD1 line 1 a shape-valid TD2 line-1 candidate once
/// padded, and TD1's document code starts with the same `I`/`A`/`C` bytes
/// TD2 looks for. TD1 must keep winning — it runs first in
/// `find_and_parse_with` specifically so this can't happen.
#[test]
fn a_genuine_td1_never_parses_as_td2() {
    let (l1, l2, l3) = td1_lines();
    let text = format!("{l1}\n{l2}\n{l3}");
    let data = find_and_parse(&text).expect("emitted TD1 parses");
    assert_eq!(
        data.format,
        Format::Td1,
        "a genuine TD1 record must never be reported as TD2"
    );
}
