//! Synthetic counterexamples from Astra's ADR-0021 review, pinned against
//! today's parser. These are observations of legacy behavior, not assertions
//! that a checksum-consistent result identifies the printed cells.

use mrz::{find_and_parse, parse_mrv_b, parse_td3, Format};

const TD3_UPPER: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
const UTOPIA_LOWER: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";

#[test]
fn two_optional_alignments_satisfy_local_and_composite_digits() {
    let a = "1234567897UTO7408122F1204159A5<<<<<<<<<<<<56";
    let b = "1234567897UTO7408122F1204159A<5<<<<<<<<<<<56";
    assert_eq!(a.len(), 44);
    assert_eq!(b.len(), 44);
    let first = parse_td3(TD3_UPPER, a).expect("synthetic TD3 A parses");
    let second = parse_td3(TD3_UPPER, b).expect("synthetic TD3 B parses");
    assert_eq!(first.checks.document_number, Some(true));
    assert_eq!(second.checks.document_number, Some(true));
    assert_eq!(first.checks.personal_number, Some(true));
    assert_eq!(second.checks.personal_number, Some(true));
    assert_eq!(first.checks.composite, Some(true));
    assert_eq!(second.checks.composite, Some(true));
    assert!(first.valid() && second.valid());
    assert_ne!(first.optional_data_1, second.optional_data_1);
    // A future aligner allowing two edits must surface both consistent
    // readings and refuse to infer which optional field was printed.
}

#[test]
fn prefix_insertion_leaves_a_checksum_consistent_unanchored_issuer() {
    let ocr_upper = "P<COHLERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
    assert_eq!(ocr_upper.len(), 45);
    let read = find_and_parse(&format!("{ocr_upper}\n{UTOPIA_LOWER}"))
        .expect("today's scanner accepts the synthetic OCR text");
    assert_eq!(read.format, Format::Td3);
    assert!(read.valid());
    assert_eq!(read.issuing_country, "COH");
    assert_eq!(read.surname, "LERIKSSON");
    // Known limitation: COH is outside the two structurally admissible
    // deletion readings (COL and CHL). A future aligner must refuse or
    // report the unresolved issuer instead of calling COH established.
}

#[test]
fn omitted_truth_can_make_a_wrong_filler_alignment_look_unique() {
    let upper = "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<";
    let printed = "4FWF<E9E21UTO7408122F1204159<<<<<<<<";
    let ocr = "4FWFE9G21UTO7408122F1204159<<<<<<<<";
    let wrong_completion = "4FWFE9<G21UTO7408122F1204159<<<<<<<<";
    assert_eq!(
        (printed.len(), ocr.len(), wrong_completion.len()),
        (36, 35, 36)
    );
    let read =
        find_and_parse(&format!("{upper}\n{ocr}")).expect("today's scanner returns a candidate");
    assert_eq!(read.format, Format::MrvB);
    assert_eq!(
        read.mrz_lines.lines().nth(1),
        Some("4FWFE9G21UTO7408122F1204159<<<<<<<<<")
    );
    assert!(
        !read.valid(),
        "today's scanner pads the tail but does not find the filler insertion"
    );
    assert!(parse_mrv_b(upper, printed).unwrap().valid());
    assert!(parse_mrv_b(upper, wrong_completion).unwrap().valid());
    // The printed truth needs both an inserted filler and E/G correction.
    // A filler-only future search must not describe the surviving wrong
    // alignment as proof of identity with the printed line.
}
