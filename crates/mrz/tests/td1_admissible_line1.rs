//! #574 (PR 6): TD1 line 1 prefers the candidate whose issuing state resolves.
//!
//! TD1's line 1 carries a check digit, so `td1_line1_variants` (and the
//! per-line and merged-line TD1 scans) used to rely on it alone to pick
//! between the as-read line and `repair_td1_line1_unshifted`'s reading with a
//! dropped position-1 filler put back. That digit does not cover the document
//! code or the issuing state, and a *shifted* reading can validate on its own:
//!
//! - With empty optional data, the shifted document number's check cell is the
//!   first optional-data cell, `<`, which has the value 0. `truth[1..9] + c`
//!   (`c` being the true check digit) therefore passes about one time in ten.
//! - When it does, the composite's line-1 contribution differs from the true
//!   one by `8 * c`, so it also passes whenever `c` is 0 or 5.
//!
//! The shifted reading reads back as code `IU`/issuer `SAK`-style: wrong code,
//! wrong issuer, wrong document number, every check digit valid. Both readings
//! then validate, and the as-read one came first. Line 1's admissibility
//! (width 30, leading `I`/`A`/`C`, issuer resolving in `country_name`) now
//! narrows the candidates whenever at least one is admissible, and leaves them
//! untouched when none is.
//!
//! The same rule reaches the two damaged-capture searches that build TD1
//! line 1 candidates from a search of their own, `substituted` (one misread
//! cell) and `class_swept` (one glyph class read uniformly across the
//! document number): each also offers the unshifted reading now, through
//! `td1_line1_searched`. `restored` is left alone; see that helper's doc.
//!
//! The three pinned seeds are `synthpass-bench --document-type td1 --profile
//! clean --count 1 --seed N --dump-ocr` output from a **release** build, the
//! text the bench actually parsed. The OCR retry loop is wall-clock budgeted,
//! so a slower build appends fewer passes and reads a shorter text; these are
//! the complete ones. The constructed zones are emitted by `format_td1`, so
//! their check digits are the crate's own.

mod support;

use mrz::{
    country_name, find_and_parse, find_and_parse_with, format_td1, parse_td1, Format, MrzData,
    ParseOptions, Sex, Td1Fields,
};

/// Seed 52, complete (24 lines). Its first pass reads line 1 as `I <BGR...`
/// (the space is dropped by normalisation, so the filler is intact); later
/// passes read it intact and, in the last one, with the filler dropped:
/// `IBGROWZ31DC275<<<...`, followed by a line 2 and a line 3 that read
/// cleanly. That shifted line validates (its check cell is `<`).
const SEED_52: &str = "\
BGR\n?OBA?E?KO\nJOPI?\nowz31DC27\nBGR\n2034-06-16\nRP6DN8HONKY\n\
SYHTHETIC SPECIMEN NTHETIC\nI <BGROWZ31DC275<<<<<<<<<<<<<\n\
5909164M3406164BGRRP6DN8HONKY6\nKOVALENKO<<IURII<<<<<<<<<<<<<<\n1959-09-16\n\
M\nSYNTHETICSPECIMENSTNTHETIC\nI<BGROWZ31DC275<<<<<<<<<<<<<<\n\
5909164M3406164BGRRP6DN8HONKY6\nKOVALENKO<IURII<<<<<<<<<<\n\
SYHTHETISPECIMENTNTHETTC\nI<BGROWZ31DC275<<<<<<<<<<<<<\n\
5909164M3406164BGRRP6DN8HONKY6\nSYHTHETICSFECIMENSYNTHETIC\n\
IBGROWZ31DC275<<<<<<<<<<<<<\n5909164M3406164BGRRP6DN8HONKY6\n\
KOVALENKO<<IURII<<<<<<<<<<<<<\n";

/// Seed 57, complete (12 lines). `truth` is `I<USAK2HPBKQ2L0<<<...`; OCR
/// dropped the filler after `I` and read `L` as `1` (the same residue class,
/// so the check digit cannot tell), and the shifted reading validates.
const SEED_57: &str = "\
USA\nADEYEMI\nYUKI\nK2HPBKQ2L\nUSA\n2033-11-22\nT8C7EXTO?53\n\
S'TNTHETI SPECIMEN S\"NTHETIC\nIUSAK2HPBKQ210<<<<<<<<<<<<<<\n\
6507110F3311226USAT8C7EXTQH531\nADEYEM<YUKI<<<<<<<<<<<<<<<<\n1965-07-11 \n";

/// Seed 75, complete (28 lines). Its last pass carries line 1 as
/// `IUTOAMGZTQCG1Z<<<...` with the filler dropped, followed by a clean line 2
/// and line 3. That shifted reading validates too.
const SEED_75: &str = "\
UTO\nDUBOISARD\nELENA\nAMG7TOCG1\nUTO 2004-03-27 \n2031-09-07\n\
X9OY40XOANB \nS'TNTHETISPECIMEN S\"NTHETIC\nIUTOAMGZTQCGIZ<<<<<<<<<<<<<\n\
0403276F3109074UTOX90Y40XOANB1\nDUBOISARD<<ELENA<<<<<<<<<<<<<<\n\
STNTHETIESPECIMENSTNTHETIC\nIUTOAMG7TQCG1T<<<<<<<<<<<<<K<\n\
0403276F3109074UTOX90Y40XOANB1\nDUBOISARDKELENAC<CCK<<K\n\
SHTHETIESPECIMENSTNTHETIE\n04032763109074UTOX90Y40XOANB1\n\
SYHTHETICEFTNENSTNTHETIC\nIUTOAMG7TQCG1Z<<<<<<<<<<<<<<\n\
0403276F3109074UTOX90Y40XOANB1\nDUBOISARDKELENA<<<<<<\n\
0403276F3109074UTOX90Y40XOANB1\nDUBOISARD<ELENA<<<KY<\n\
0403276F3109074UT0X90Y40XOANB1\nSYNTHETIESPECIMENSTNTHETIC\n\
IUTOAMGZTQCG1Z<<<<<<<<<<<<<<\n0403276F3109074UTOX90Y40XOANB1\n\
DUBOISARDELENA<<<<<<\n";
/// The first eleven lines of [`SEED_75`] only: the first pass, in which line 1
/// exists only shifted and the other candidates do not validate. This is what
/// a build that runs fewer OCR passes would hand the parser.
fn seed_75_first_pass() -> String {
    SEED_75.lines().take(11).collect::<Vec<_>>().join("\n")
}

/// Seed 52, complete. Before the fix the ordinary per-line scan reached the
/// shifted line `IBGROWZ31DC275<<<...` (from the last pass) and returned it as
/// a validating read: code `IB`, issuer `GRO`, document number `WZ31DC275`.
/// The unshifted candidate is admissible (`BGR`), so it is the only one
/// considered now. It does not validate: the document number carries an `O`
/// where the truth has a `0`, which no check digit can tell. A checksum-failed
/// read with the right code and issuer replaces a validating wrong one.
#[test]
fn seed_52_no_longer_accepts_the_shifted_line_1() {
    let data = find_and_parse(SEED_52).expect("seed 52 parses as a checksum-failed TD1");
    assert_eq!(data.format, Format::Td1);
    assert_eq!(
        data.mrz_lines,
        "I<BGROWZ31DC275<<<<<<<<<<<<<<<\n\
         5909164M3406164BGRRP6DN8HONKY6\n\
         KOVALENKO<<IURII<<<<<<<<<<<<<<"
    );
    assert_eq!(data.document_type, "I");
    assert_eq!(data.issuing_country, "BGR");
    assert_eq!(data.document_number, "OWZ31DC27");
    assert!(!data.valid());
    assert!(!data.damaged_recovery);
}

/// Seed 57, complete: the ordinary per-line scan returned the shifted reading
/// (`IU` / `SAK`), which validates. The unshifted one is admissible (`USA`),
/// so it wins. The document number is still `K2HPBKQ21` where the truth is
/// `K2HPBKQ2L`: `L` and `1` are one residue class, which no check digit can
/// separate. That misread is not this fix's to repair.
#[test]
fn seed_57_no_longer_accepts_the_shifted_line_1() {
    let data = find_and_parse(SEED_57).expect("seed 57 parses");
    assert_eq!(data.format, Format::Td1);
    assert_eq!(
        data.mrz_lines,
        "I<USAK2HPBKQ210<<<<<<<<<<<<<<<\n\
         6507110F3311226USAT8C7EXTQH531\n\
         ADEYEM<YUKI<<<<<<<<<<<<<<<<<<<"
    );
    assert_eq!(data.document_type, "I");
    assert_eq!(data.issuing_country, "USA");
    assert_eq!(data.document_number, "K2HPBKQ21");
    assert!(data.valid());
    assert!(!data.damaged_recovery);
}

/// Seed 75, complete: the ordinary per-line scan returned the shifted line
/// `IUTOAMGZTQCG1Z<<<...` (issuer `TOA`, document number `MGZTQCG1Z`) as a
/// validating read. Now the unshifted `UTO` candidate is the only one
/// considered: the document number is `AMG7TQCG1`, the printed truth, and the
/// read stays checksum-failed because its check cell reads `T` for `7`.
///
/// Line 3 of this read is the noisy second-pass one
/// (`DUBOISARDKELENAC<CCK<<<<<<<<<K`). The fallback rank prefers it over the
/// clean last-pass line 3, exactly as it ranks any two checksum-failed reads.
/// Names are not this fix's concern; they are pinned so a later change shows.
#[test]
fn seed_75_no_longer_accepts_the_shifted_line_1() {
    let data = find_and_parse(SEED_75).expect("seed 75 parses as checksum-failed");
    assert_eq!(
        data.mrz_lines,
        "I<UTOAMG7TQCG1T<<<<<<<<<<<<<<<\n\
         0403276F3109074UTOX90Y40XOANB1\n\
         DUBOISARDKELENAC<CCK<<<<<<<<<K"
    );
    assert_eq!(data.document_type, "I");
    assert_eq!(data.issuing_country, "UTO");
    assert_eq!(data.document_number, "AMG7TQCG1");
    assert!(!data.valid());
    assert!(!data.damaged_recovery);
}

/// Seed 75, first pass only: line 1 exists only shifted, and it does not
/// validate. Before the fix the shifted candidate's issuer (`TOA`) was accepted
/// as the best checksum-failed read; now the unshifted `UTO` candidate is the
/// only one considered. Kept beside the complete text because it is a
/// different path (the filter's effect on a checksum-failed fallback, with no
/// validating reading involved) and a build with fewer OCR passes reads it.
#[test]
fn seed_75_first_pass_no_longer_reads_a_shifted_issuer() {
    let data = find_and_parse(&seed_75_first_pass()).expect("seed 75 parses as checksum-failed");
    assert_eq!(
        data.mrz_lines,
        "I<UTOAMGZTQCGIZ<<<<<<<<<<<<<<<\n\
         0403276F3109074UTOX90Y40XOANB1\n\
         DUBOISARD<<ELENA<<<<<<<<<<<<<<"
    );
    assert_eq!(data.document_type, "I");
    assert_eq!(data.issuing_country, "UTO");
    assert_ne!(data.issuing_country, "TOA");
    assert!(!data.valid());
}

// --- The composite blind spot, as a constructed TD1 zone --------------------

/// A TD1 zone plus the line 1 OCR would read if it dropped the filler at
/// position 1 and padded the tail (`shifted`), both checked valid on their own.
struct Zone {
    l1: String,
    l2: String,
    l3: String,
    shifted: String,
}

impl Zone {
    fn text(&self) -> String {
        format!("{}\n{}\n{}", self.l1, self.l2, self.l3)
    }

    /// The same zone with line 1 replaced by `l1`, one line per row.
    fn with_line_1(&self, l1: &str) -> String {
        format!("{l1}\n{}\n{}", self.l2, self.l3)
    }
}

fn zone(document_code: &str, issuer: &str, document_number: &str, dob: &str) -> Zone {
    let mrz = format_td1(&Td1Fields {
        document_code: document_code.to_string(),
        issuing_country: issuer.to_string(),
        document_number: document_number.to_string(),
        optional_data_1: None,
        surname: "MARTIN".to_string(),
        given_names: "CLAIRE".to_string(),
        nationality: issuer.to_string(),
        date_of_birth: support::birth(dob),
        sex: Sex::Female,
        date_of_expiry: support::expiry("330615"),
        optional_data_2: None,
    });
    let lines: Vec<&str> = mrz.lines().collect();
    assert_eq!(lines.len(), 3);
    let mut shifted = lines[0].to_string();
    shifted.remove(1);
    shifted.push('<');
    Zone {
        l1: lines[0].to_string(),
        l2: lines[1].to_string(),
        l3: lines[2].to_string(),
        shifted,
    }
}

/// A zone whose shifted line 1 *also* validates: the true check digit is 0 or
/// 5 and the shifted document number happens to pass against `<`. Asserts the
/// whole premise, so a fixture that stops being a blind spot fails loudly
/// instead of quietly testing nothing.
fn blind_spot_zone(document_number: &str, check_digit: char, dob: &str) -> Zone {
    let z = zone("I", "FRA", document_number, dob);
    assert_eq!(
        z.l1.as_bytes()[14] as char,
        check_digit,
        "sanity: true check digit"
    );
    assert!(matches!(check_digit, '0' | '5'));
    assert_eq!(z.shifted.len(), 30);

    let truth = parse_td1(&z.l1, &z.l2, &z.l3).expect("the emitted zone parses");
    assert!(truth.valid(), "the unshifted reading validates on its own");
    assert_eq!(truth.issuing_country, "FRA");
    assert_eq!(truth.document_number, document_number);

    let shifted = parse_td1(&z.shifted, &z.l2, &z.l3).expect("the shifted reading parses");
    assert!(
        shifted.valid(),
        "the shifted reading validates on its own: {}",
        shifted.mrz_lines
    );
    assert_ne!(shifted.document_number, document_number);
    assert!(
        country_name(&shifted.issuing_country).is_none(),
        "the shifted issuer {:?} must not resolve",
        shifted.issuing_country
    );
    z
}

/// `(document number, true check digit)`: one of each blind-spot digit.
const BLIND_SPOT_NUMBERS: [(&str, char); 2] = [("AHGEK9H88", '5'), ("G2CPZV951", '0')];

fn assert_is_the_truth(data: &MrzData, z: &Zone) {
    assert_eq!(data.mrz_lines, z.text());
    assert_eq!(data.issuing_country, "FRA");
    assert_eq!(data.document_type, "I");
    assert!(data.valid());
}

/// The ordinary per-line scan: line 1 arrives one cell short.
#[test]
fn ordinary_scan_prefers_the_admissible_unshifted_line() {
    for (number, check) in BLIND_SPOT_NUMBERS {
        let z = blind_spot_zone(number, check, "070707");
        let dropped: String = z.shifted[..29].to_string();
        let data = find_and_parse(&z.with_line_1(&dropped)).expect("must parse");
        assert_is_the_truth(&data, &z);
        assert!(!data.damaged_recovery);
    }
}

/// The merged-line scan: all three rows on one physical line.
#[test]
fn merged_line_scan_prefers_the_admissible_unshifted_line() {
    for (number, check) in BLIND_SPOT_NUMBERS {
        let z = blind_spot_zone(number, check, "070707");
        let merged = format!("{}{}{}", z.shifted, z.l2, z.l3);
        assert_eq!(merged.len(), 90);
        let data = find_and_parse(&merged).expect("must parse");
        assert_is_the_truth(&data, &z);
        assert!(!data.damaged_recovery);
    }
}

/// `class_sweep_pass`: line 2's date of birth `170707` (check digit 4) reads
/// back as `1T0T0T`, the run of `7`s read as their lookalike `T` (which
/// `digitize` does not map). Nothing validates ordinarily; the sweep recovers
/// line 2 (`T` as `1` would not pass the check digit), and both line-1
/// readings then validate against it.
#[test]
fn class_sweep_pass_prefers_the_admissible_unshifted_line() {
    for (number, check) in BLIND_SPOT_NUMBERS {
        let z = blind_spot_zone(number, check, "170707");
        assert_eq!(
            &z.l2[..7],
            "1707074",
            "sanity: the fixture's date of birth cells"
        );
        let l2 = format!("1T0T0T4{}", &z.l2[7..]);
        let text = format!("{}\n{l2}\n{}", z.shifted, z.l3);

        let off = find_and_parse_with(&text, &ParseOptions::default()).expect("must parse");
        assert!(!off.valid(), "sanity: nothing validates with the sweep off");

        let on = ParseOptions::default().with_class_sweep(true);
        let data = find_and_parse_with(&text, &on).expect("the sweep recovers this reading");
        assert_is_the_truth(&data, &z);
        assert!(
            data.damaged_recovery,
            "recovered by the damaged-capture passes"
        );
    }
}

/// `damaged_pass` proper (sweep off): line 2's leading date-of-birth digit `4`
/// is read as its `CONFUSABLES` lookalike `A`, which only the substitution
/// search recovers. Again both line-1 readings validate against the recovered
/// line 2.
#[test]
fn damaged_pass_prefers_the_admissible_unshifted_line() {
    for (number, check) in BLIND_SPOT_NUMBERS {
        let z = blind_spot_zone(number, check, "440101");
        assert!(z.l2.starts_with('4'), "sanity: {}", z.l2);
        let l2 = format!("A{}", &z.l2[1..]);
        let text = format!("{}\n{l2}\n{}", z.shifted, z.l3);

        let data = find_and_parse(&text).expect("the damaged pass recovers this reading");
        assert_is_the_truth(&data, &z);
        assert!(
            data.damaged_recovery,
            "recovered by the damaged-capture pass"
        );
    }
}

/// `damaged_pass`'s `substituted` shape (sweep off): line 1 lost its filler
/// (30 cells, padded) *and* carries one more misread cell, a `0` read as `O` in
/// the document number, which neither the ordinary repairs nor the unshifted
/// reading survive. The single-cell substitution search recovers the `0`, and
/// the shifted line, sitting in the composite blind spot, validated with it and
/// was the only hit and was recorded as the answer (issuer `RAW`, `RAE`, ...);
/// the unshifted reading was never offered to that search. The fixtures are
/// chosen so that the unshifted reading is also the *only* one that validates:
/// the search can find a second, coincidental one (an `A` read as `4` in a
/// cell the check digit cannot police), and two hits are refused as ambiguous,
/// which is the honest outcome and not what this test pins.
/// `(document number, true check digit)`.
#[test]
fn substituted_search_prefers_the_admissible_unshifted_line() {
    for (number, check) in [("WYR0TNRBZ", '0'), ("EK5JDJZ0G", '5')] {
        let z = blind_spot_zone(number, check, "070707");
        let misread = z.shifted.replacen('0', "O", 1);
        assert_eq!(misread.matches('O').count(), 1, "sanity: {misread}");
        let text = z.with_line_1(&misread);

        let data = find_and_parse(&text).expect("the substitution search recovers this reading");
        assert_is_the_truth(&data, &z);
        assert!(
            data.damaged_recovery,
            "recovered by the damaged-capture pass"
        );
    }
}

/// `class_sweep_pass` (sweep on): the same shifted line 1, with every `0` of
/// the document number (and of its check cell, when that is `0`) read as `O`,
/// one glyph class read uniformly, which is the case the sweep exists for.
/// The shifted alignment validated after the sweep and was returned (issuer
/// `RAM`); the sweep now also runs on the unshifted alignment, whose reading
/// is admissible. `(document number, true check digit)`.
#[test]
fn class_sweep_line_1_prefers_the_admissible_unshifted_line() {
    for (number, check) in [("MNVYA0KG0", '0'), ("K0Z90T008", '5')] {
        let z = blind_spot_zone(number, check, "070707");
        let misread = z.shifted.replace('0', "O");
        assert!(
            misread.matches('O').count() >= 2,
            "sanity: a run of one class, {misread}"
        );
        let text = z.with_line_1(&misread);

        let off = find_and_parse_with(&text, &ParseOptions::default()).expect("must parse");
        assert!(!off.valid(), "sanity: nothing validates with the sweep off");

        let on = ParseOptions::default().with_class_sweep(true);
        let data = find_and_parse_with(&text, &on).expect("the sweep recovers this reading");
        assert_is_the_truth(&data, &z);
        assert!(
            data.damaged_recovery,
            "recovered by the damaged-capture passes"
        );
    }
}

/// The extended sites keep today's candidates when no candidate's issuer
/// resolves. Issuer `QQQ`, a `0` read as `O`, filler intact: the substitution
/// search recovers the document number exactly as before, and the sweep does
/// too with two `O`s.
#[test]
fn searched_sites_never_refuse_an_unknown_issuer() {
    assert!(country_name("QQQ").is_none(), "sanity: QQQ is not a state");
    let z = zone("I", "QQQ", "RA0BDRNV8", "070707");
    assert!(z.l1.starts_with("I<QQQ"), "sanity: {}", z.l1);

    let one_o = z.l1.replacen('0', "O", 1);
    let data = find_and_parse(&z.with_line_1(&one_o)).expect("the substitution search runs");
    assert_eq!(data.mrz_lines, z.text());
    assert_eq!(data.issuing_country, "QQQ");
    assert!(data.valid());
    assert!(data.damaged_recovery);

    let z = zone("I", "QQQ", "401U0Z54N", "070707");
    let two_o = z.l1.replace('0', "O");
    assert_eq!(two_o.matches('O').count(), 2, "sanity: {two_o}");
    let on = ParseOptions::default().with_class_sweep(true);
    let data = find_and_parse_with(&z.with_line_1(&two_o), &on).expect("the sweep runs");
    assert_eq!(data.mrz_lines, z.text());
    assert_eq!(data.issuing_country, "QQQ");
    assert!(data.valid());
    assert!(data.damaged_recovery);
}

// --- #580: the unshifted search runs even when the shifted one finds nothing --

/// A zone plus the text OCR would hand the parser when line 1 lost its
/// position-1 filler (30 cells, padded) *and* every `0` of the document number
/// was read as `O`, one glyph class read uniformly. The number is chosen so
/// that the shifted reading cannot validate on its own: no blind spot, so the
/// class sweep of the shifted alignment finds nothing and only the unshifted
/// alignment can recover the line.
fn shifted_class_misread(issuer: &str, number: &str) -> (Zone, String) {
    let z = zone("I", issuer, number, "070707");
    assert!(
        parse_td1(&z.shifted, &z.l2, &z.l3).is_ok_and(|d| !d.valid()),
        "sanity: the shifted reading must not validate: {}",
        z.shifted
    );
    let misread = z.shifted.replace('0', "O");
    assert!(
        misread.matches('O').count() >= 2,
        "sanity: a run of one class, {misread}"
    );
    let text = z.with_line_1(&misread);
    (z, text)
}

/// `class_sweep_pass`: the shifted search finds nothing, and the unshifted
/// reading, swept, is the only way to the truth. Before #580's fix the
/// early return in `td1_line1_searched` skipped the unshifted search whenever
/// the shifted one was empty, so the sweep offered nothing usable and the read
/// stayed a checksum-failed one from the ordinary scan.
#[test]
fn class_sweep_offers_the_unshifted_line_when_the_shifted_search_is_empty() {
    for number in ["401U0Z54N", "TK0NM0P4C"] {
        let (z, text) = shifted_class_misread("FRA", number);

        let off = find_and_parse_with(&text, &ParseOptions::default()).expect("must parse");
        assert!(!off.valid(), "sanity: nothing validates with the sweep off");

        let on = ParseOptions::default().with_class_sweep(true);
        let data = find_and_parse_with(&text, &on).expect("the sweep recovers this reading");
        assert_is_the_truth(&data, &z);
        assert!(
            data.damaged_recovery,
            "recovered by the damaged-capture passes"
        );
    }
}

/// The guard on the same shape: when no combined candidate is admissible (the
/// issuer does not resolve), the searched site returns exactly the shifted
/// search's list, which here is empty. The read must be the one the sweep-off
/// parse gives, so an unknown issuer gains no candidate from the unshifted
/// search.
#[test]
fn class_sweep_gains_no_candidate_for_an_unknown_issuer() {
    assert!(country_name("QQQ").is_none(), "sanity: QQQ is not a state");
    for number in ["401U0Z54N", "TK0NM0P4C"] {
        let (_, text) = shifted_class_misread("QQQ", number);

        let summary = |data: Result<MrzData, mrz::MrzError>| {
            data.ok()
                .map(|d| (d.mrz_lines.clone(), d.valid(), d.damaged_recovery))
        };
        let off = summary(find_and_parse_with(&text, &ParseOptions::default()));
        let on = ParseOptions::default().with_class_sweep(true);
        let with_sweep = summary(find_and_parse_with(&text, &on));
        assert_eq!(with_sweep, off, "the sweep must add nothing here");
        assert!(
            !off.as_ref()
                .is_some_and(|(_, valid, recovered)| *valid || *recovered),
            "sanity: the ordinary scan does not recover this line: {off:?}"
        );
    }
}

// --- Forms the filter must leave alone --------------------------------------

/// A genuine two-letter TD1 document code with a resolving issuer.
#[test]
fn a_genuine_two_letter_code_is_unchanged() {
    let z = zone("ID", "FRA", "AHGEK9H88", "070707");
    assert!(z.l1.starts_with("IDFRA"), "sanity: {}", z.l1);
    let data = find_and_parse(&z.text()).expect("must parse");
    assert_eq!(data.mrz_lines, z.text());
    assert_eq!(data.document_type, "ID");
    assert_eq!(data.issuing_country, "FRA");
    assert!(data.valid());
}

/// A correct `I<` read, one cell at a time and merged.
#[test]
fn a_correct_i_filler_read_is_unchanged() {
    let z = zone("I", "FRA", "AHGEK9H88", "070707");
    assert!(z.l1.starts_with("I<FRA"), "sanity: {}", z.l1);
    for text in [z.text(), format!("{}{}{}", z.l1, z.l2, z.l3)] {
        let data = find_and_parse(&text).expect("must parse");
        assert_eq!(data.mrz_lines, z.text());
        assert_eq!(data.document_type, "I");
        assert_eq!(data.issuing_country, "FRA");
        assert!(data.valid());
    }
}

/// An issuer that is not in the registry: no candidate is admissible, so every
/// candidate is kept, exactly as before — the document is read, not refused.
/// Covers an intact line 1, and one whose dropped filler the check digit alone
/// recovers.
#[test]
fn an_unknown_issuer_is_never_refused() {
    assert!(country_name("QQQ").is_none(), "sanity: QQQ is not a state");
    let z = zone("I", "QQQ", "X12345678", "070707");
    assert!(z.l1.starts_with("I<QQQ"), "sanity: {}", z.l1);

    let data = find_and_parse(&z.text()).expect("an intact unknown issuer parses");
    assert_eq!(data.mrz_lines, z.text());
    assert_eq!(data.issuing_country, "QQQ");
    assert!(data.valid());

    // The shifted reading of this number fails its own check digit, so the
    // unshifted one is the only valid candidate whether or not it is filtered.
    let dropped = &z.shifted[..29];
    assert!(
        parse_td1(&z.shifted, &z.l2, &z.l3).is_ok_and(|d| !d.valid()),
        "sanity: the shifted reading must not validate"
    );
    let data = find_and_parse(&z.with_line_1(dropped)).expect("a dropped filler still parses");
    assert_eq!(data.mrz_lines, z.text());
    assert_eq!(data.issuing_country, "QQQ");
    assert!(data.valid());
}
