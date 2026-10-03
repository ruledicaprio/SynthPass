//! #579 (PR 2): `ParseOptions::refuse_repeated_line`, an opt-in refusal of a
//! checksum-valid zone in which one line repeats another (check C3).
//!
//! The rank in `find_and_parse_with` already puts such a zone below an
//! unflagged alternative, but it returns the zone when nothing else validates.
//! With the switch on, the zone is dropped instead, and the scan falls through
//! to what is left: an unflagged valid zone, then another flagged one, then the
//! checksum-failed reading, then `MrzError::RepeatedLine`.
//!
//! The switch is off by default and the default parse must not move, so the
//! first test pins that. Every zone here is constructed from the crate's
//! emitters with invented data, except two public specimens' OCR text:
//! `KOSOVO_2023`, which `rank_wrong_line_zones.rs` already pins, and
//! `CROATIA_2002`.

mod support;

use mrz::{
    find_and_parse, find_and_parse_with, format_mrv_a, format_mrv_b, format_td1, format_td2,
    format_td3, parse_td1, parse_td3, Format, MrvAFields, MrvBFields, MrzData, MrzError,
    ParseOptions, Sex, Td1Fields, Td2Fields, Td3Fields,
};

fn refusing() -> ParseOptions {
    ParseOptions::default().with_refuse_repeated_line(true)
}

// --- Constructed zones ------------------------------------------------------

/// A valid TD3 zone, as two lines. `issuer` also picks the nationality.
fn td3(issuer: &str, document_number: &str) -> (String, String) {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: issuer.to_string(),
        document_number: document_number.to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: issuer.to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    (l1.to_string(), l2.to_string())
}

/// A zone whose line 1 is its line 2 read a second time behind line 1's own
/// document code and issuer: the wrong physical line, and it still validates
/// because line 1 enters no check digit of a two-line format.
fn repeat_of(line1: &str, line2: &str) -> String {
    format!("{}{}", &line1[..5], &line2[5..])
}

/// The two-line fixtures, one per format, as `(format, good line 1, line 2)`.
fn two_line_zones() -> Vec<(Format, String, String)> {
    let birth = support::birth("800101");
    let expiry = support::expiry("301230");
    let mut zones = Vec::new();
    let (l1, l2) = td3("UTO", "AB123457");
    zones.push((Format::Td3, l1, l2));
    let td2 = format_td2(&Td2Fields {
        issuing_country: "UTO".to_string(),
        document_number: "AB123457".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: birth,
        sex: Sex::Female,
        date_of_expiry: expiry,
        ..Td2Fields::default()
    });
    let mrv_a = format_mrv_a(&MrvAFields {
        issuing_country: "UTO".to_string(),
        document_number: "AB123457".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: birth,
        sex: Sex::Female,
        date_of_expiry: expiry,
        ..MrvAFields::default()
    });
    let mrv_b = format_mrv_b(&MrvBFields {
        issuing_country: "UTO".to_string(),
        document_number: "AB123457".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: birth,
        sex: Sex::Female,
        date_of_expiry: expiry,
        ..MrvBFields::default()
    });
    for (format, zone) in [
        (Format::Td2, td2),
        (Format::MrvA, mrv_a),
        (Format::MrvB, mrv_b),
    ] {
        let (l1, l2) = zone.split_once('\n').expect("a two-line format");
        zones.push((format, l1.to_string(), l2.to_string()));
    }
    zones
}

/// A valid TD1 zone as three lines.
fn td1_lines() -> [String; 3] {
    let zone = format_td1(&Td1Fields {
        issuing_country: "UTO".to_string(),
        document_number: "AB1234567".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("301230"),
        ..Td1Fields::default()
    });
    let mut lines = zone.lines().map(str::to_string);
    [
        lines.next().expect("TD1 line 1"),
        lines.next().expect("TD1 line 2"),
        lines.next().expect("TD1 line 3"),
    ]
}

/// `line 2` with the digit in cell 2 of its document number read as a
/// lookalike letter, which only the damaged-capture pass repairs.
fn misread(l2: &str) -> String {
    let mut cells: Vec<char> = l2.chars().collect();
    cells[2] = match cells[2] {
        '1' => 'I',
        '6' => 'G',
        other => panic!("fixture cell 2 is {other}, not a digit with a lookalike"),
    };
    cells.into_iter().collect()
}

/// `line 2` with one digit of the date of expiry (cells 21..27) altered, so the
/// expiry and composite check digits fail and the zone parses but is invalid.
fn tampered(l2: &str) -> String {
    let mut cells: Vec<char> = l2.chars().collect();
    cells[22] = if cells[22] == '9' { '8' } else { '9' };
    cells.into_iter().collect()
}

/// The repeated TD3 zone and the true one it stands in for, with the same
/// line 2.
fn repeated_td3() -> (String, String, String) {
    let (good, line2) = td3("UTO", "AB123457");
    let repeated = repeat_of(&good, &line2);
    (repeated, good, line2)
}

fn assert_refused(
    result: Result<MrzData, MrzError>,
    format: Format,
    first_line: usize,
    second_line: usize,
) {
    assert_eq!(
        result,
        Err(MrzError::RepeatedLine {
            format,
            first_line,
            second_line
        })
    );
}

// --- 1. Off by default ------------------------------------------------------

#[test]
fn default_options_never_refuse() {
    assert!(!ParseOptions::default().refuse_repeated_line);
    assert_eq!(
        ParseOptions::default().with_refuse_repeated_line(false),
        ParseOptions::default()
    );

    let (repeated, good, line2) = repeated_td3();
    let (dl1, dl2) = td3("UTO", "AB123457");
    let td1 = td1_lines();
    let td1_repeat = format!("{}\n{}\n{}", td1[0], td1[1], td1[0]);
    let texts = [
        format!("{repeated}\n{line2}"),
        format!("{repeated}\n{line2}\n\n{good}\n{line2}"),
        format!("{repeated}\n{}", tampered(&line2)),
        format!("{repeated}\n{}", misread(&line2)),
        format!("{dl1}\n{dl2}"),
        td1_repeat,
        "no zone here".to_string(),
    ];
    for text in &texts {
        assert_eq!(
            find_and_parse_with(text, &ParseOptions::default()),
            find_and_parse(text),
            "{text}"
        );
    }
    // With the switch off, a repeated zone is still returned, as before.
    let data = find_and_parse(&format!("{repeated}\n{line2}")).expect("parses");
    assert!(data.valid());
    assert_eq!(data.mrz_lines, format!("{repeated}\n{line2}"));
}

// --- 2..7: the mechanics ----------------------------------------------------

#[test]
fn a_repeated_zone_alone_is_refused() {
    let (repeated, _, line2) = repeated_td3();
    let text = format!("{repeated}\n{line2}");
    // It validates on its own first: the refusal is the switch, not the parse.
    let data = find_and_parse_with(&text, &ParseOptions::default()).expect("parses");
    assert!(data.valid());
    assert_eq!(data.format, Format::Td3);
    assert_refused(find_and_parse_with(&text, &refusing()), Format::Td3, 0, 1);
}

#[test]
fn a_refusal_yields_to_the_checksum_failed_read() {
    let (repeated, good, line2) = repeated_td3();
    let text = format!("{repeated}\n{line2}\n\n\n\n{good}\n{}", tampered(&line2));
    // Off: the repeated zone validates and is returned.
    let off = find_and_parse_with(&text, &ParseOptions::default()).expect("parses");
    assert!(off.valid());
    assert_eq!(off.mrz_lines, format!("{repeated}\n{line2}"));
    // On: it is dropped, and the checksum-failed true zone is what is left.
    let on = find_and_parse_with(&text, &refusing()).expect("a checksum-failed read is returned");
    assert!(!on.valid());
    assert_eq!(on.mrz_lines, format!("{good}\n{}", tampered(&line2)));
}

#[test]
fn an_unflagged_alternative_wins_either_way() {
    let (repeated, good, line2) = repeated_td3();
    let text = format!("{repeated}\n{line2}\n\n\n\n{good}\n{line2}");
    let off = find_and_parse_with(&text, &ParseOptions::default()).expect("parses");
    let on = find_and_parse_with(&text, &refusing()).expect("parses");
    assert_eq!(on, off);
    assert_eq!(on.mrz_lines, format!("{good}\n{line2}"));
    assert!(on.valid());
}

#[test]
fn refusal_drops_only_the_repeat() {
    let (repeated, _, line2) = repeated_td3();
    let (q1, q2) = td3("QQQ", "CD987654");
    let text = format!("{repeated}\n{line2}\n\n\n\n{q1}\n{q2}");
    // Off: both are flagged and the first is kept.
    let off = find_and_parse_with(&text, &ParseOptions::default()).expect("parses");
    assert_eq!(off.mrz_lines, format!("{repeated}\n{line2}"));
    // On: the repeat is dropped, and the flagged non-repeat is still returned.
    let on = find_and_parse_with(&text, &refusing()).expect("parses");
    assert_eq!(on.mrz_lines, format!("{q1}\n{q2}"));
    assert_eq!(on.issuing_country, "QQQ");
    assert!(on.valid());
}

/// A repeat whose issuer does not resolve either: the issuer check fires first,
/// but the refusal does not depend on which check reports.
#[test]
fn the_repeat_is_refused_whichever_check_fires_first() {
    let (q1, q2) = td3("QQQ", "CD987654");
    let repeated = repeat_of(&q1, &q2);
    let text = format!("{repeated}\n{q2}");
    let off = find_and_parse_with(&text, &ParseOptions::default()).expect("parses");
    assert!(off.valid());
    assert_eq!(off.issuing_country, "QQQ");
    assert_refused(find_and_parse_with(&text, &refusing()), Format::Td3, 0, 1);
}

/// The damaged-capture pass's recovery is refused too. Here the ordinary scan
/// read the same two lines checksum-failed, so that reading is what is left.
#[test]
fn a_repeated_damaged_recovery_is_refused() {
    let (repeated, _, line2) = repeated_td3();
    let text = format!("{repeated}\n{}", misread(&line2));
    let off = find_and_parse_with(&text, &ParseOptions::default()).expect("parses");
    assert!(off.valid());
    assert!(off.damaged_recovery);
    assert_eq!(off.mrz_lines, format!("{repeated}\n{line2}"));

    let on = find_and_parse_with(&text, &refusing()).expect("a checksum-failed read is returned");
    assert!(!on.valid());
    assert!(!on.damaged_recovery);
}

/// A refused ordinary zone still opens the damaged pass: an unflagged recovery
/// later in the text beats it, as it beats a flagged zone with the switch off.
#[test]
fn a_refused_zone_still_lets_the_damaged_pass_run() {
    let (repeated, _, line2) = repeated_td3();
    let (u1, u2) = td3("UTO", "GH135791");
    let text = format!("{repeated}\nnoise\n{line2}\n\n\n\n{u1}\n{}", misread(&u2));
    let on = find_and_parse_with(&text, &refusing()).expect("parses");
    assert!(on.valid());
    assert!(on.damaged_recovery);
    assert_eq!(on.mrz_lines, format!("{u1}\n{u2}"));
}

/// The same, with the later zone's line 2 one glyph narrow (its cell 14 was
/// dropped), so only the damaged pass can build the zone. The refused zone
/// opens that pass too; in practice a narrow line also leaves a checksum-failed
/// reading that opens it, so this pins the outcome, not the gate clause alone.
#[test]
fn a_refusal_alone_opens_the_damaged_pass() {
    let (repeated, _, line2) = repeated_td3();
    let (u1, u2) = td3("UTO", "GH135791");
    let mut narrow = u2.clone();
    narrow.remove(14);
    let text = format!("{repeated}\nnoise\n{line2}\n\n\n\n{u1}\n{narrow}");
    let off = find_and_parse_with(&text, &ParseOptions::default()).expect("parses");
    assert_eq!(
        off.mrz_lines,
        format!("{u1}\n{u2}"),
        "the recovery beats the repeat"
    );
    assert!(off.damaged_recovery);
    let on = find_and_parse_with(&text, &refusing()).expect("parses");
    assert_eq!(on, off);
}

// --- 8. Every format --------------------------------------------------------

#[test]
fn a_repeated_line_is_refused_on_every_two_line_format() {
    for (format, good, line2) in two_line_zones() {
        let repeated = repeat_of(&good, &line2);
        let text = format!("{repeated}\n{line2}");
        let off = find_and_parse_with(&text, &ParseOptions::default()).expect("parses");
        assert!(off.valid(), "{format:?} validates before refusal");
        assert_eq!(off.format, format);
        assert_refused(find_and_parse_with(&text, &refusing()), format, 0, 1);

        // The true zone is untouched by the switch.
        let honest = format!("{good}\n{line2}");
        let read = find_and_parse_with(&honest, &refusing()).expect("parses");
        assert_eq!(read, find_and_parse(&honest).expect("parses"), "{format:?}");
        assert!(read.valid(), "{format:?}");
    }
}

/// Belgium-shaped: TD1's line 3 (the name line) holds line 1 a second time.
#[test]
fn a_td1_line_3_repeating_line_1_is_refused() {
    let lines = td1_lines();
    let honest = lines.join("\n");
    assert!(parse_td1(&lines[0], &lines[1], &lines[2])
        .expect("parses")
        .valid());
    assert_eq!(
        find_and_parse_with(&honest, &refusing()).expect("parses"),
        find_and_parse(&honest).expect("parses")
    );

    let repeated = format!("{}\n{}\n{}", lines[0], lines[1], lines[0]);
    let off = find_and_parse_with(&repeated, &ParseOptions::default()).expect("parses");
    assert_eq!(off.format, Format::Td1);
    assert!(off.valid());
    assert_refused(
        find_and_parse_with(&repeated, &refusing()),
        Format::Td1,
        0,
        2,
    );
}

/// A correctly read zone holds different fields on every line, so the switch
/// never touches it.
#[test]
fn different_lines_are_never_refused() {
    let (good, line2) = td3("UTO", "AB123457");
    let text = format!("{good}\n{line2}");
    let data = find_and_parse_with(&text, &refusing()).expect("parses");
    assert!(data.valid());
    assert_eq!(data.mrz_lines, text);
}

// --- 9. Real specimens ------------------------------------------------------

/// Kosovo passport 2023 (TD3), the OCR text the real-specimen bench dumped, as
/// `rank_wrong_line_zones.rs` pins it. The first pass repeats line 2; the rank
/// already returns the true line 1 from a later pass, so the switch changes
/// nothing.
const KOSOVO_2023: &str = r##"I
REPUBLIC OF KOSOVO
NUMRL PASAPORT?S6PO ?IACO?
PASSPORT NO PO0000000
NO
V
ME
GJAT?SIA /BICHHA NGUYRA E SYVE/BOJROUNY
CIE HEIGHTEYE COLOUR
NATIONALITY2.176 Cm BLUE
KOSOVAR
DATELINDJA/RATYM ROT C RATE OF BIRTH W MEL PERRCNAL  IVN EPOI PERBONAI NO
30.08.200151001234567
VENDLINDA /NECTO POBEHA/PLACE OF BIRTH
FPRIZREN
DATA E LESHIMIT FATYM MSDABAHSA
DATE OP ISSUE
31.07.2023
DATA& SKADIMITATYMMCTEKA
DATE OF EXPIRY 
30.07.2033
P<RKSBERISHA<<VLORA<<<<E<<<<<<<<<<<<<<<<<
PO00000005RKS0108308F33073081001234567<<<<14
RR
REPUBLIKA .E KOSOV?S  PE??????KA KOC??O
KODL
KOL
TYPE P CODE RKS
MBENR/TIPE3MME SURNAME
BERISHA
EMRL/?NE/ GIVEN NAME.
VLORA
SHTETESIA/ DPKABMAHCTBO
DNVO
LLO/BPCTA
GJINA ION/SEX
PASAPORTE
?IACOW
PASSPORT
CAN
123456
NENSHKRL OTTMC SIGNATURE
Veora Berisha
LESHMAR NGA/?3/ATO
ISSED AY
MPB / MUP / MIA
NENSHKRIMIDOTTCSIGNATURE
DATAESKADIMITINATYMMCTEKA
P<RKSBERISHA<VLORA<K<<<KKKK
PO000000O5RKS0108308F33073081001234567<<<<14
NENSTKRBNIOIMCSIGNATURE
P<RKSBERISHA<<VLORA<<<<<<<<<<<<<<<<<<<<<<<<<<
P000000005RKS0108308F33073081001234567<<<<14
DATAESKADMT1AATYMKCTEKA"##;

#[test]
fn kosovo_2023_is_unchanged_under_refusal() {
    let off = find_and_parse_with(KOSOVO_2023, &ParseOptions::default()).expect("parses");
    let on = find_and_parse_with(KOSOVO_2023, &refusing()).expect("parses");
    assert_eq!(on, off);
    assert_eq!(off, find_and_parse(KOSOVO_2023).expect("parses"));
    assert!(on.valid());
    assert_eq!(
        on.mrz_lines,
        "P<RKSBERISHA<<VLORA<<<<<<<<<<<<<<<<<<<<<<<<<\n\
         P000000005RKS0108308F33073081001234567<<<<14"
    );
}

/// Croatia ID card 2002, the OCR text the real-specimen bench dumped (#579; pinned
/// on review of #633). By default the scan returns a checksum-valid TD2 zone in
/// which one line repeats another. With the switch on, that zone is refused and
/// the checksum-failed TD1 reading is what is left.
const CROATIA_2002: &str = r##"Banide
"SAVSKA CESTA 31
ZAGREB
Tadaia
PU/ZAGREB??KA
Datu izavania/Date 
12.12.2002
Tads
ond arkiress
HR
jeRin
2002
IOHRVO00000OOOO<<<<<<<<<<<<<<
7701018F0212126HRV<<<<<<<<<<<0
SPECIMEN~<SPECIMEN<<<<<<<<<<<
IOHRVOOOOOOOOOO<K<<<<<<<<<<K
TOHRVO0O0000000<<<<<<<<<<<<<<<
7701018F0212126HRV<<<<<<3
SPECTMENS<SPECIMEN<<<<<<<<<<
TOHRVOOOOOOOOOO<<<K<<<<<<<<<<<
7701018FO212126HRV<S<SS<S<
SPECIMEN<SPECIMEN<<<<<<<
7701018F0212126HRV<<<
SPECIMEN<SPECIMEN<<<<<<
IOHRVOOODOOOOOO<<<<<<<<<<<<<<<
7701018F0212126HRV<<<<<<<<<<<0
SPECTMEN<<SPECIMEN<<<<<<<<<<33
7701018F0212126HRV<<<
SPECIMEN<SPECIMEN<<<<<<
IOHRVO0000000O0<<<<<<<<<<<
7701018F0212126HRV<<<<<<<
SPECIMEN<SPECIMEN<<<
7701018FO212126HRV<<<<<<<<
7701018F0212126HRV3<<233<<<<0
IOHRVOOOO0OOOOO<<K<<<<<<<<<<<<
SPECIMEN<SPECIMEN<<<
TOHRVOOO000OOOO<<<<<<<<<<<<<<<
7701018F0212126HRV3<<3<<<<<<<<0
SPECIMENS<SPECIMENE<<<3333333<"##;

#[test]
fn croatia_2002_falls_back_to_its_checksum_failed_reading_under_refusal() {
    let off = find_and_parse_with(CROATIA_2002, &ParseOptions::default()).expect("parses");
    assert_eq!(off, find_and_parse(CROATIA_2002).expect("parses"));
    assert_eq!(off.format, Format::Td2);
    assert!(off.valid());

    let on = find_and_parse_with(CROATIA_2002, &refusing()).expect("parses");
    assert_eq!(on.format, Format::Td1);
    assert!(!on.valid(), "checks: {:?}", on.checks);
}

// --- Display ----------------------------------------------------------------

#[test]
fn the_refusal_renders_a_readable_message() {
    let error = MrzError::RepeatedLine {
        format: Format::Td3,
        first_line: 0,
        second_line: 1,
    };
    assert_eq!(error.to_string(), "Td3 zone: line 1 repeats line 0");
    // Sanity: the fixture the message is about parses.
    let (l1, l2) = td3("UTO", "AB123457");
    assert!(parse_td3(&l1, &l2).expect("parses").valid());
}
