//! `find_and_parse_with`'s format dispatch keys off line 1 position 0 -- the
//! document-code cell -- and no ICAO 9303 check digit anywhere in the zone
//! covers that byte. Mutating only that one character of the ICAO Part 4
//! published specimen (UTOPIA / ANNA MARIA ERIKSSON), and nothing else, drives
//! three distinct outcomes depending on which format's line-1 prefix the
//! mutated byte happens to match:
//!
//!   - `P` (truth): reads as TD3, checksum-valid.
//!   - `A`/`I`/`C`: shape-matches TD1's document-code family, but the
//!     checksums don't hold -- wrong, but visibly so.
//!   - `V`: reads as a fully checksum-valid MRV-A. MRV-A shares TD3's 2x44
//!     geometry and its document-number/date-of-birth/expiry check-digit
//!     positions, but prints **neither a personal-number nor a composite
//!     check digit** -- so the two checks that would have caught a TD3
//!     misread simply don't exist for MRV-A. A confident wrong answer with
//!     every check green, not a detectable failure.
//!   - `R`/`F`/`D`: match no format's line-1 prefix at all, so the scanner
//!     reports nothing -- the safe failure mode the bug does *not* produce.
//!
//! Measured over the 64 ground-truth fixtures elsewhere in this workspace
//! (not reproduced here -- see the mrz-format-gate work item): of 55 genuine
//! TD3 zones, 41 become a fully valid MRV-A from this single-cell mutation,
//! and in 3 of those the true TD3 read itself fails its own check digits
//! while the MRV-A misread reports valid. There is one real instance of this
//! in the sample corpus.
//!
//! **Fixing this needs a ranking rule that hasn't been decided yet.**
//! ADR-0017 ("What this ADR deliberately does not decide") leaves open
//! whether `Checks::score()` is re-derived from its new three-state check
//! representation or simply stops counting absent digits -- either fixes the
//! MRV ranking bias `find_and_parse_with` currently has, and the choice is
//! explicitly left for review. This module does not choose; it measures.
//!
//! ICAO 9303's own published specimen only, never a real document.

use mrz::{find_and_parse, Format, MrzData, MrzError};

/// ICAO 9303 Part 4 §4.2.2's published TD3 specimen (UTOPIA / ANNA MARIA
/// ERIKSSON), unmodified except for the one byte each test below replaces.
const TD3_L1: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
const TD3_L2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";

/// Replace line 1's byte 0 -- the document-code cell -- and merge both lines
/// into one 88-char blob, the shape `find_and_parse`'s single-token fast path
/// takes when OCR (or Markdown flattening) concatenates a zone onto one
/// physical line.
fn parse_with_document_code(code: char) -> Result<MrzData, MrzError> {
    let mut merged = format!("{TD3_L1}{TD3_L2}");
    merged.replace_range(0..1, &code.to_string());
    find_and_parse(&merged)
}

/// Documents the current, measured behaviour of the document-code gate: three
/// qualitatively different outcomes from mutating one byte no check digit
/// covers. This is the test that must start failing the moment a fix changes
/// any of these outcomes -- in particular, the day `parse_with_document_code('V')`
/// stops reporting a valid MRV-A, this assertion catches it before the guard
/// test below (`a_genuine_td3_never_parses_as_mrva`) does, because a fix might
/// change the *other* branches' outcomes too and this is the one place that
/// would notice.
#[test]
fn document_code_gate_produces_three_measured_outcomes() {
    // Truth: the unmodified specimen is a checksum-valid TD3.
    let truth = parse_with_document_code('P').expect("the unmodified specimen parses");
    assert_eq!(truth.format, Format::Td3);
    assert!(truth.valid(), "the genuine specimen must be checksum-valid");

    // TD1's document-code family: the mutated line now shape-matches TD1's
    // line-1 prefix, but none of TD1's own check digits hold against data
    // that was laid out for TD3 -- wrong, but visibly wrong.
    for code in ['A', 'I', 'C'] {
        let data = parse_with_document_code(code)
            .unwrap_or_else(|e| panic!("document code {code:?} was expected to parse: {e}"));
        assert_eq!(data.format, Format::Td1, "document code {code:?}");
        assert!(
            !data.valid(),
            "document code {code:?}: a TD3 zone misread as TD1 must not check out as valid"
        );
    }

    // The dangerous case, and this module's whole reason to exist: 'V' reads
    // as a fully valid MRV-A because MRV-A's checked fields (document number,
    // date of birth, date of expiry) are exactly TD3's, and the two fields
    // that would have differed -- personal number, composite -- are simply
    // absent from MRV-A, not failed.
    let mrva = parse_with_document_code('V').expect("document code 'V' parses");
    assert_eq!(mrva.format, Format::MrvA);
    assert!(
        mrva.valid(),
        "MRV-A's own check digits are untouched by a mutation that only changes byte 0"
    );

    // Codes matching no format's line-1 prefix: the safe failure mode this
    // defect does not produce. Kept in the same test so a future change that
    // makes one of these start matching something is visible here too.
    for code in ['R', 'F', 'D'] {
        assert!(
            parse_with_document_code(code).is_err(),
            "document code {code:?}: expected no format to match"
        );
    }
}

/// The cannibalization guard this defect is missing: a TD3 zone whose only
/// damage is a misread document-code byte must never be reported as a valid
/// MRV-A, because the two check digits that would have exposed the misread
/// (personal number, composite) don't exist on that format.
///
/// **This assertion fails today, and that failure is the defect, not a
/// broken test.** `find_and_parse_with` tries MRV-A before TD1/TD2 in its
/// scan order (see ADR-0017, "The same bias one level down"), and nothing
/// currently penalizes a reading for proving fewer check digits than the
/// format it started as would have. Un-ignoring this test and watching it
/// pass is the acceptance criterion for that fix -- which needs the ranking
/// decision ADR-0017 leaves open (whether `Checks::score()` is re-derived or
/// simply stops counting absent digits), not a change here.
#[test]
#[ignore = "known defect: see this test's doc comment and ADR-0017 -- un-ignoring it is the fix's acceptance criterion"]
fn a_genuine_td3_never_parses_as_mrva() {
    let mrva = parse_with_document_code('V').expect("document code 'V' parses");
    assert_ne!(
        mrva.format,
        Format::MrvA,
        "a TD3 zone with one misread document-code cell must not report as a valid MRV-A"
    );
}
