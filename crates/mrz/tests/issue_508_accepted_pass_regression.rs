//! #508: `synthpass-ocr`'s native retry loop picks one pass's reading, but
//! (before the fix) `OcrPage.text` kept accumulating every pass's candidate
//! lines and `MrzReader` re-ran `mrz::find_and_parse` over all of it,
//! deciding afresh instead of reading what the loop already accepted. The
//! replay behind that decision
//! (`knowledge/benchmarks/retry-handoff-replay-2026-09-27.md`, seed 22) found the mechanism:
//! `single()`, the damaged-pass unanimity gate (#440), refuses the combined
//! text because it holds two disagreeing `accept_damaged` readings -- the
//! general pass's own pair (the true document number, name split
//! `MORAVEC/MAREN`) and a later failed variant's pair (same document number,
//! name split `MORAVECMAREN/`). Handed only the text of the pass the retry
//! loop accepted, `find_and_parse` recovers the single reading the loop
//! already decided on.
//!
//! `S22_CLEAN_FULL` and `S22_ACCEPTED_GENERAL` are `--dump-ocr` output from a
//! synthetic seed-22 TD3 specimen (synthetic generator output, no real
//! person) -- byte-identical to
//! the seed-22 texts that replay fed through `find_and_parse`.
//! `S22_ACCEPTED_GENERAL` is a line-exact prefix of `S22_CLEAN_FULL`: the
//! general pass's own eleven lines, before any retry variant appended its
//! own (disagreeing) candidates.

use mrz::find_and_parse;

/// The full page text Tier 2 sees under the pre-#508 behavior: the general
/// pass's eleven lines, plus two retry variants' MRZ-shaped candidate lines
/// appended after. Must not parse `valid()` -- this is the bug's own
/// reproduction, not the fix.
const S22_CLEAN_FULL: &str = "BRA
MORAVEC
MAREN
600CGRMOU
BRA
2029-01-01
8CHF1F6E6KC73H
1991-10-01
SYNTHETICSPECIMEN STNTHETICSPECIMEN STH
P<BRAMORAVEC<MAREN<<<<<<<<<<<<<<<<<<<<<<<<<
60QCGRMOU5BRA9 1 10018 F29010198 CHF 1F6E 6KC73H78
SYNTHETICSFECIMENSTNTHETICSPECIMENSH
PBRAMORAVECMAREN<<<<<<<<<<<<<<<<<<<<<<<
60QCGRMOU5BRA9110018F29010198CHF1F6E6KC73H78
SNTHETIESFECIMENSNTHETIESFECIMENSTH
PBRAMORAVEC<MAREN<<<<<<
60QCGRMOU5BRA9110018F29010198CHFTF6E6KC73H78
";

/// `S22_CLEAN_FULL`'s line-exact prefix -- the general pass's own eleven
/// lines, exactly what `OcrPage::accepted_mrz_text` (#508) carries for a
/// `general_valid` stop. Must parse `valid()`, with the damaged-capture
/// repair (#440's `accept_damaged`) recovering the checksum-valid document
/// number `6OQCGRMOU` from the raw OCR read `60QCGRMOU` (a lookalike `0`/`O`
/// substitution).
const S22_ACCEPTED_GENERAL: &str = "BRA
MORAVEC
MAREN
600CGRMOU
BRA
2029-01-01
8CHF1F6E6KC73H
1991-10-01
SYNTHETICSPECIMEN STNTHETICSPECIMEN STH
P<BRAMORAVEC<MAREN<<<<<<<<<<<<<<<<<<<<<<<<<
60QCGRMOU5BRA9 1 10018 F29010198 CHF 1F6E 6KC73H78
";

/// The defect's own reproduction: hand `find_and_parse` the full
/// multi-variant concatenation (what `MrzReader` read before #508) and it
/// refuses. Two `accept_damaged` readings disagree only on the name split
/// (`MORAVEC/MAREN` from the general pass's own pair vs `MORAVECMAREN/` from
/// a later failed variant's pair) and `single()` correctly will not choose
/// between them -- so the checksum-valid document number the general pass
/// alone would have recovered is lost to a `checksum_failed` fallback
/// instead.
#[test]
fn full_concatenation_refuses_on_disagreeing_damaged_readings() {
    let data = find_and_parse(S22_CLEAN_FULL)
        .expect("a structurally parseable TD3 shape, just not a checksum-valid one");
    assert!(
        !data.valid(),
        "the full concatenation must NOT validate -- this is the #508 defect \
         this regression pins, not the fix"
    );
}

/// The fix's premise (#508's owner decision): handed only the accepted
/// pass's own text, the same document parses to exactly the reading the
/// retry loop accepted.
#[test]
fn accepted_pass_text_alone_recovers_the_valid_reading() {
    let data =
        find_and_parse(S22_ACCEPTED_GENERAL).expect("the general pass's own text must parse");
    assert!(
        data.valid(),
        "the accepted pass's text alone must validate, unlike the full concatenation"
    );
    assert_eq!(
        data.document_number, "6OQCGRMOU",
        "damaged-capture repair must recover the checksum-valid document number"
    );
    assert!(
        data.damaged_recovery,
        "this reading is only reachable via mrz's damaged-capture search, \
         not the ordinary scan"
    );
}
