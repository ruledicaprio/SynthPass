//! #574 (PR 7): a checksum-valid zone that shows a symptom of holding a wrong
//! physical line is ranked below an unflagged valid zone.
//!
//! A format's line 1 enters few or none of its check digits, so a second reading
//! of line 2, a visual-zone header or a name line can stand in for it and the
//! zone still validates. `find_and_parse` now keeps looking past such a zone
//! (three private checks: the issuer does not resolve, two lines repeat each
//! other, a digit sits in a two-line format's name field) and returns it only
//! when nothing unflagged validates. The rank never refuses: a flagged zone is
//! still a valid parse.
//!
//! The constructed cases pin the rank semantics; the three real texts pin the
//! measured behaviour on public specimens. Each real text is the OCR output the
//! real-specimen bench dumped for that specimen.

mod support;

use mrz::{find_and_parse, format_td3, parse_td3, Format, Sex, Td3Fields};

// --- Constructed zones ------------------------------------------------------

/// A valid TD3 zone. `issuing_country` is the only thing the callers vary to
/// make a zone flagged: an issuer the registry does not hold.
fn td3(issuing_country: &str, document_number: &str) -> (String, String) {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: issuing_country.to_string(),
        document_number: document_number.to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: issuing_country.to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    (l1.to_string(), l2.to_string())
}

/// A zone whose issuer is in the registry: no check flags it.
fn unflagged() -> (String, String) {
    td3("UTO", "AB123457")
}

/// A zone whose issuer is not in the registry: valid, and flagged.
fn flagged() -> (String, String) {
    td3("QQQ", "CD987654")
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

/// The flagged zone parses as a valid TD3 read on its own, so the tests below
/// are about ranking and not about whether it parses.
#[test]
fn the_flagged_fixture_is_a_valid_read_on_its_own() {
    let (l1, l2) = flagged();
    let data = parse_td3(&l1, &l2).expect("parses");
    assert!(data.valid());
    assert_eq!(data.issuing_country, "QQQ");
    let (l1, l2) = unflagged();
    assert!(parse_td3(&l1, &l2).expect("parses").valid());
}

/// (a) A flagged valid zone comes first, an unflagged valid zone after it: the
/// unflagged one is returned.
#[test]
fn an_unflagged_zone_after_a_flagged_one_is_returned() {
    let (f1, f2) = flagged();
    let (u1, u2) = unflagged();
    let data = find_and_parse(&format!("{f1}\n{f2}\n\n{u1}\n{u2}")).expect("parses");
    assert_eq!(data.mrz_lines, format!("{u1}\n{u2}"));
    assert_eq!(data.issuing_country, "UTO");
    assert!(data.valid());
    assert!(!data.damaged_recovery);
}

/// (b) Only a flagged valid zone exists: it is returned, exactly the read a
/// scan without the rank returns. The rank never refuses.
#[test]
fn a_flagged_zone_with_no_alternative_is_still_returned() {
    let (f1, f2) = flagged();
    let data = find_and_parse(&format!("{f1}\n{f2}")).expect("parses");
    assert_eq!(data.mrz_lines, format!("{f1}\n{f2}"));
    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.issuing_country, "QQQ");
    assert_eq!(data.document_number, "CD987654");
    assert!(data.valid());
    assert!(!data.damaged_recovery);
    // The same read as parsing the two lines directly.
    assert_eq!(data, parse_td3(&f1, &f2).expect("parses"));
}

/// (b) The flagged zone is also returned when noise follows it.
#[test]
fn a_flagged_zone_is_returned_through_surrounding_noise() {
    let (f1, f2) = flagged();
    let text = format!("PASSPORT\n\nsome OCR noise\n{f1}\n{f2}\nfooter text");
    let data = find_and_parse(&text).expect("parses");
    assert_eq!(data.mrz_lines, format!("{f1}\n{f2}"));
}

/// (c) The ordinary scan finds only a flagged valid zone; the damaged-capture
/// pass then recovers an unflagged one from a later pair whose line 2 has a
/// misread glyph. The recovery is returned, and says it was a recovery.
///
/// The flagged zone's line 2 sits two rows below its line 1: the ordinary scan
/// reaches that far, the damaged-capture pass (adjacent rows only) does not, so
/// the recovery is the only reading the pass finds.
#[test]
fn an_unflagged_damaged_recovery_beats_a_flagged_ordinary_zone() {
    let (f1, f2) = flagged();
    let (u1, u2) = unflagged();
    let text = format!("{f1}\nnoise\n{f2}\n\n\n\n{u1}\n{}", misread(&u2));
    let data = find_and_parse(&text).expect("parses");
    assert_eq!(data.mrz_lines, format!("{u1}\n{u2}"));
    assert_eq!(data.document_number, "AB123457");
    assert!(data.valid());
    assert!(data.damaged_recovery);
}

/// (c) A recovery that is itself flagged is returned when it is all there is:
/// the damaged-capture pass's flagged zone stands in for the missing ordinary
/// one.
#[test]
fn a_flagged_damaged_recovery_is_returned_when_nothing_else_is() {
    let (g1, g2) = td3("QQR", "EF162483");
    let data = find_and_parse(&format!("{g1}\n{}", misread(&g2))).expect("parses");
    assert_eq!(data.mrz_lines, format!("{g1}\n{g2}"));
    assert_eq!(data.issuing_country, "QQR");
    assert!(data.valid());
    assert!(data.damaged_recovery);
}

/// (c) A flagged recovery does not replace a flagged ordinary-scan zone: the
/// ordinary scan's is the first flagged zone and is kept.
#[test]
fn a_flagged_recovery_does_not_replace_a_flagged_ordinary_zone() {
    let (f1, f2) = flagged();
    let (g1, g2) = td3("QQR", "EF162483");
    let text = format!("{f1}\nnoise\n{f2}\n\n\n\n{g1}\n{}", misread(&g2));
    let data = find_and_parse(&text).expect("parses");
    assert_eq!(data.mrz_lines, format!("{f1}\n{f2}"));
    assert!(!data.damaged_recovery);
}

/// (d) None of the checks fires: the first valid zone is returned, as before.
#[test]
fn an_unflagged_zone_is_returned_as_before() {
    let (u1, u2) = unflagged();
    let data = find_and_parse(&format!("noise\n{u1}\n{u2}\nnoise")).expect("parses");
    assert_eq!(data.mrz_lines, format!("{u1}\n{u2}"));
    assert!(data.valid());
    assert!(!data.damaged_recovery);
    assert_eq!(data, parse_td3(&u1, &u2).expect("parses"));
}

/// (d) Two unflagged valid zones: the first still wins.
#[test]
fn the_first_of_two_unflagged_zones_still_wins() {
    let (a1, a2) = unflagged();
    let (b1, b2) = td3("UTO", "GH135791");
    let data = find_and_parse(&format!("{a1}\n{a2}\n\n\n\n{b1}\n{b2}")).expect("parses");
    assert_eq!(data.mrz_lines, format!("{a1}\n{a2}"));
}

/// The first flagged zone wins among several, and an unflagged zone after all
/// of them still beats every one.
#[test]
fn the_first_flagged_zone_is_the_one_kept() {
    let (f1, f2) = flagged();
    let (g1, g2) = td3("QQR", "EF162483");
    let data = find_and_parse(&format!("{f1}\n{f2}\n\n\n\n{g1}\n{g2}")).expect("parses");
    assert_eq!(data.mrz_lines, format!("{f1}\n{f2}"));
    let (u1, u2) = unflagged();
    let data =
        find_and_parse(&format!("{f1}\n{f2}\n\n\n\n{g1}\n{g2}\n\n\n\n{u1}\n{u2}")).expect("parses");
    assert_eq!(data.mrz_lines, format!("{u1}\n{u2}"));
}

// --- Real specimens ---------------------------------------------------------
//
// The OCR text the real-specimen bench dumped for three public specimens. They
// are pinned whole, with the exact lines the read returns.

/// Kosovo passport 2023 (TD3). The first pass reads the printed line 2 as
/// `PO00000005RKS...`; the next pass's own line 1 (`P<RKSBERISHA<<VLORA...`)
/// and line 2 follow. Before the rank, the scan paired the earlier pass's line 2
/// with the later pass's line 2 (line 2 read twice) and returned that.
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

/// Sweden visa 2024 (MRV-B). The visual-zone header `VISERING/VISA ... SWE
/// 987654321` starts with `V` and has line 2 within reach, so the scan tried it
/// as line 1 first.
const SWEDEN_VISA_2024: &str = r##"VISERING/VISA S WE 98765432
JEDESWE
 T3456
AB
aRtason ann
EU
GoPend
Bt
90
24:094
VCSWETESTSSON<<ANNIKA<<<<<<<<<<<<<<<
0202217109xxx8704645F25041541901001
WE
70237138/00875-2024-00428 $ 70237138/00875-2024-00428
987654321    987654321
S
WN
VCSWETESTSSONZANNIKA<<CKKKK
0202217109XXX3704045F255011541901001
702373800875202440042870237138008775202400428
UCSH2TESTSSONANNESASTNYSOGO
070237138100875202400428702371381
VISERINGVISASNE987654321
SVERIGESWFDENISUEDESWE
VCSWETESTSSONANNIKA<<<<<<<KK<<<K
0202217109XXX6704045F2501154<1901001
702371387008754202400428770237138700875202400428"##;

/// Somaliland passport 2023 (TD3, non-ISO). Its issuer `RSL` is in no registry
/// and no other reading exists, so the zone is flagged and is still the answer.
const SOMALILAND_2023: &str = r##"De svroe eet n foab
ic d
aasaboorkan
SAR
This passport' is valid
walba
waa
llau
31
couintries
kara
R
Ta
Baasaboor ambar
Passport No.
P00040973
JAMHUURIYADDA SOMALILAND
Mebca Type Astaanta Daka/ Country Code Baassboor tambor  Passpot
Astganta
RSL PO0040973
Macaca Awoowwa Sumwme
AvocITa
SOMALILAND SPECIMEN
Madaca Givon Mamiea
MUSTAFE
Mationalty
SOMALILANDER
hshada / Date
Taarikhda Dhalashada/ Date of Bith
01 JAN 2000
Genobta
Jieai Ser GGoo?ta Ohalashada Plece of Birth
HARGEISA
Tanrchoa a Rieivf Date Assue Awooda Rixigta 
DDste 
ofue
19 OCT 2023 SOMALIL AND IMMIGRATION
Dhncako / 0ote
Fonrikha uu Ohacayo Oate of EXpiy
OCT 2028
REPUBLIC OF SOMALILAND
Raanaboo
BAASABOOR
PASSPORT
Jinalyadda 
lachada
esU
Autnority
Holder's
Samnexn
Seha
Sionatore
PSRSLSOMALILAND<SPECIMEN<<MUSTAFE<<<<<<<<<<<
P000409734RSL0001018M28101790000033<<<<<<<42"##;

/// Line 2 read twice was returned as the zone; the true line 1 is now used.
#[test]
fn kosovo_2023_returns_the_printed_line_1() {
    let data = find_and_parse(KOSOVO_2023).expect("parses");
    assert_eq!(data.format, Format::Td3);
    assert_eq!(
        data.mrz_lines,
        "P<RKSBERISHA<<VLORA<<<<<<<<<<<<<<<<<<<<<<<<<\n\
         P000000005RKS0108308F33073081001234567<<<<14"
    );
    assert_eq!(data.issuing_country, "RKS");
    assert_eq!(data.surname, "BERISHA");
    assert!(data.valid());
    assert!(!data.damaged_recovery);
}

/// A visual-zone header was returned as line 1; the printed line 1 is now used.
#[test]
fn sweden_visa_2024_returns_the_printed_line_1() {
    let data = find_and_parse(SWEDEN_VISA_2024).expect("parses");
    assert_eq!(data.format, Format::MrvB);
    assert_eq!(
        data.mrz_lines,
        "VCSWETESTSSONANNIKA<<<<<<<<<<<<<<<<<\n\
         0202217109XXX6704045F2501154<1901001"
    );
    assert_eq!(data.issuing_country, "SWE");
    assert!(data.valid());
    assert!(!data.damaged_recovery);
}

/// No alternative exists for a non-ISO issuer, so the read is unchanged.
#[test]
fn somaliland_2023_is_unchanged() {
    let data = find_and_parse(SOMALILAND_2023).expect("parses");
    assert_eq!(data.format, Format::Td3);
    assert_eq!(
        data.mrz_lines,
        "PSRSLSOMALILAND<SPECIMEN<<MUSTAFE<<<<<<<<<<<\n\
         P000409734RSL0001018M28101790000033<<<<<<<42"
    );
    assert_eq!(data.issuing_country, "RSL");
    assert!(data.valid());
    assert!(!data.damaged_recovery);
}
