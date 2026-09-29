//! #408: OCR can insert whitespace inside one otherwise complete MRZ line.
//!
//! `find_and_parse` already splits a physical OCR line into long candidate
//! tokens. These tests pin the narrower rule needed before that filter:
//! adjacent fragments are rejoined only when their combined width is one of
//! the three MRZ line widths (30, 36 or 44 cells).

use mrz::{
    find_and_parse, format_mrv_a, format_td1, format_td2, format_td3, MrvAFields, MrzData,
    Td1Fields, Td2Fields, Td3Fields,
};

fn clean_read(zone: &str) -> MrzData {
    find_and_parse(zone).expect("the emitter's clean zone parses")
}

fn split_at(line: &str, cell: usize) -> String {
    format!("{} {}", &line[..cell], &line[cell..])
}

fn td3_zone() -> String {
    format_td3(&Td3Fields {
        document_number: "E00000000".to_string(),
        ..Td3Fields::default()
    })
}

fn td2_zone() -> String {
    format_td2(&Td2Fields {
        document_number: "E00000000".to_string(),
        ..Td2Fields::default()
    })
}

fn mrv_a_zone() -> String {
    format_mrv_a(&MrvAFields {
        document_number: "E00000000".to_string(),
        ..MrvAFields::default()
    })
}

fn td1_zone() -> String {
    format_td1(&Td1Fields {
        document_number: "E00000000".to_string(),
        ..Td1Fields::default()
    })
}

#[test]
fn td3_line_2_rejoins_at_every_split_from_cell_20_through_24() {
    let zone = td3_zone();
    let (line1, line2) = zone.split_once('\n').expect("TD3 has two lines");
    let expected = clean_read(&zone);

    for cell in 20..=24 {
        let text = format!("{line1}\n{}", split_at(line2, cell));
        assert_eq!(
            find_and_parse(&text).expect("the exact-width TD3 fragments rejoin"),
            expected,
            "split at cell {cell}"
        );
    }
}

#[test]
fn td2_line_2_rejoins_to_36_cells_on_a_merged_physical_line() {
    let zone = td2_zone();
    let (line1, line2) = zone.split_once('\n').expect("TD2 has two lines");
    let expected = clean_read(&zone);
    let text = format!("{line1} {}", split_at(line2, 16));

    assert_eq!(
        find_and_parse(&text).expect("the exact-width TD2 fragments rejoin"),
        expected
    );
}

#[test]
fn mrv_a_line_2_rejoins_to_44_cells_on_a_merged_physical_line() {
    let zone = mrv_a_zone();
    let (line1, line2) = zone.split_once('\n').expect("MRV-A has two lines");
    let expected = clean_read(&zone);
    let text = format!("{line1} {}", split_at(line2, 20));

    assert_eq!(
        find_and_parse(&text).expect("the exact-width MRV-A fragments rejoin"),
        expected
    );
}

#[test]
fn td1_line_2_rejoins_to_30_cells_on_a_merged_physical_line() {
    let zone = td1_zone();
    let mut lines = zone.lines();
    let line1 = lines.next().expect("TD1 line 1");
    let line2 = lines.next().expect("TD1 line 2");
    let line3 = lines.next().expect("TD1 line 3");
    let expected = clean_read(&zone);
    let text = format!("{line1} {} {line3}", split_at(line2, 14));

    assert_eq!(
        find_and_parse(&text).expect("the exact-width TD1 fragments rejoin"),
        expected
    );
}

#[test]
fn a_split_line_2_can_follow_line_1_on_the_same_physical_line() {
    let zone = td3_zone();
    let (line1, line2) = zone.split_once('\n').expect("TD3 has two lines");
    let expected = clean_read(&zone);
    let text = format!("{line1} {}", split_at(line2, 22));

    assert_eq!(
        find_and_parse(&text).expect("the merged physical line is recovered"),
        expected
    );
}

#[test]
fn fragments_that_do_not_join_to_an_mrz_width_stay_separate() {
    let zone = td3_zone();
    let (line1, line2) = zone.split_once('\n').expect("TD3 has two lines");
    let mut short_line2 = line2.to_string();
    let filler = short_line2
        .rfind('<')
        .expect("the emitted TD3 line 2 has optional-data filler");
    short_line2.remove(filler);
    assert_eq!(short_line2.len(), 43);
    let text = format!("{line1}\n{}", split_at(&short_line2, 20));

    assert!(
        find_and_parse(&text).is_err(),
        "a 43-cell fragment pair must not be joined and length-repaired"
    );
}

#[test]
fn two_complete_mrz_lines_separated_by_space_stay_two_lines() {
    let zone = td3_zone();
    let expected = clean_read(&zone);
    let one_physical_line = zone.replace('\n', " ");

    assert_eq!(
        find_and_parse(&one_physical_line).expect("two complete tokens stay separate"),
        expected
    );
}

#[test]
fn an_already_clean_zone_parses_exactly_as_before() {
    let zone = td3_zone();
    let expected = clean_read(&zone);

    assert_eq!(find_and_parse(&zone).expect("clean TD3 parses"), expected);
}
