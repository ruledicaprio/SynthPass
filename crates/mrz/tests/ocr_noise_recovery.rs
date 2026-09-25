//! Synthetic reproductions of #408. The base is ICAO's published Utopia TD3
//! example; the noise shapes were observed on a real passport specimen.

use mrz::{
    check_digit, find_and_parse, format_mrv_a, format_mrv_b, format_td1, format_td2, format_td3,
    parse_td3, Format, MrvAFields, MrvBFields, Td1Fields, Td2Fields, Td3Fields, UNKNOWN,
};

#[test]
fn an_invalid_nationality_variant_does_not_hide_a_later_clean_zone() {
    let rows = [
        "P<UTOY AMAMORI?< JOHAN<<<<<<<<<<<<<<<<<<<<<<<",
        "9QD7EO 1NZ0UT0730113 1M34 1 12 4 1M4 WE23PNSNMVD354",
        "P<UTOYAMAMORI<<JOHAN<<<<<<<<<<<<<<<<<<<<<",
        "9QD7EO1NZOUT07301131M3411241M4WE23PNSNMVD354",
    ];
    let detected = find_and_parse(&rows.join("\n")).expect("later clean zone survives");
    assert!(detected.valid());
    assert_eq!(detected.surname, "YAMAMORI");
    assert_eq!(detected.given_names, "JOHAN");
    assert_eq!(detected.nationality, "UTO");
}

#[test]
fn conflicting_recovery_alternatives_do_not_hide_a_later_clean_zone() {
    let mut alternate_line2 = TD3_L2.to_string();
    alternate_line2.replace_range(10..13, "USA");
    assert!(parse_td3(TD3_L1, &alternate_line2)
        .expect("nationality is outside the checks")
        .valid());

    let noisy_only = format!("{}\n{TD3_L2}\n{alternate_line2}", noisy_name());
    assert!(
        find_and_parse(&noisy_only).is_err(),
        "ambiguous recovery alternatives are refused"
    );

    let with_clean = format!("{noisy_only}\nNOISE\nNOISE\n{TD3_L1}\n{TD3_L2}");
    let detected = find_and_parse(&with_clean).expect("later clean zone stands on its own");
    assert!(detected.valid());
    assert_eq!(detected.surname, "ERIKSSON");
    assert_eq!(detected.nationality, "UTO");
}

const TD3_L1: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
const TD3_L2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";

fn noisy_name() -> String {
    let mut line = TD3_L1.to_string();
    line.replace_range(7..8, &UNKNOWN.to_string());
    line
}

fn split_second_line() -> String {
    format!("{} {}", &TD3_L2[..20], &TD3_L2[20..])
}

#[test]
fn unknown_name_cell_does_not_hide_checked_fields() {
    assert!(parse_td3(TD3_L1, TD3_L2)
        .expect("ICAO example parses")
        .valid());
    let detected = find_and_parse(&format!("{}\n{TD3_L2}", noisy_name()))
        .expect("the checked fields remain readable");
    assert_eq!(detected.format, Format::Td3);
    assert!(detected.valid());
    assert_eq!(detected.document_number, "L898902C3");
    assert!(detected.surname.is_empty(), "the name is not certified");
}

#[test]
fn one_space_inside_line_two_is_rejoined_at_exact_width() {
    let detected = find_and_parse(&format!("{TD3_L1}\n{}", split_second_line()))
        .expect("the exact-width TD3 line can be rejoined");
    assert_eq!(detected.format, Format::Td3);
    assert!(detected.valid());
    assert_eq!(detected.document_number, "L898902C3");
}

#[test]
fn unknown_name_cell_and_space_can_coexist() {
    let detected = find_and_parse(&format!("{}\n{}", noisy_name(), split_second_line()))
        .expect("independent name and whitespace noise should not hide the zone");
    assert_eq!(detected.format, Format::Td3);
    assert!(detected.valid());
    assert!(detected.surname.is_empty());
}

#[test]
fn merged_zone_with_unreadable_name_cell_still_parses() {
    let detected = find_and_parse(&format!("{}{TD3_L2}", noisy_name()))
        .expect("a merged pair still keeps its name uncertainty");
    assert_eq!(detected.format, Format::Td3);
    assert!(detected.valid());
    assert!(detected.surname.is_empty());
}

#[test]
fn non_ascii_name_glyph_becomes_one_unknown_cell() {
    let mut line = TD3_L1.to_string();
    line.replace_range(7..8, "É");
    let detected = find_and_parse(&format!("{line}\n{TD3_L2}"))
        .expect("an unreadable name glyph need not hide checked fields");
    assert!(detected.valid());
    assert!(detected.surname.is_empty());
}

#[test]
fn complete_name_reading_wins_over_earlier_unknown() {
    let text = format!("{}\n{TD3_L1}\n{TD3_L2}", noisy_name());
    let detected = find_and_parse(&text).expect("a later complete OCR pass validates");
    assert!(detected.valid());
    assert_eq!(detected.surname, "ERIKSSON");
}

#[test]
fn ordinary_complete_zone_wins_over_joined_uncertain_name() {
    let wrong_name = format!("{}<<", TD3_L1.replacen("<<", "", 1));
    assert_eq!(wrong_name.len(), 44);
    let text = format!("{wrong_name}\n{}\n{TD3_L1}\n{TD3_L2}", split_second_line());
    let detected = find_and_parse(&text).expect("the ordinary zone validates");
    assert!(detected.valid());
    assert_eq!(detected.surname, "ERIKSSON");
    assert_eq!(detected.given_names, "ANNA MARIA");
}

#[test]
fn unknown_and_complete_names_with_different_document_numbers_are_refused() {
    let mut other_line2 = TD3_L2.to_string();
    other_line2.replace_range(0..1, "X");
    let doc_check = check_digit(&other_line2[0..9]).expect("MRZ document number");
    other_line2.replace_range(9..10, &doc_check.to_string());
    let composite = format!(
        "{}{}{}",
        &other_line2[0..10],
        &other_line2[13..20],
        &other_line2[21..43]
    );
    let composite_check = check_digit(&composite).expect("MRZ composite");
    other_line2.replace_range(43..44, &composite_check.to_string());
    assert!(parse_td3(TD3_L1, &other_line2)
        .expect("second zone parses")
        .valid());

    let text = format!(
        "{}\n{TD3_L2}\nNOISE\nNOISE\n{TD3_L1}\n{other_line2}",
        noisy_name()
    );
    assert!(
        find_and_parse(&text).is_err(),
        "two checksum-valid records that disagree on the document number must be refused"
    );
}

#[test]
fn an_unknown_name_cell_is_preserved_in_all_five_formats() {
    let cases = [
        (
            Format::Td1,
            format_td1(&Td1Fields {
                surname: "DOE".into(),
                ..Td1Fields::default()
            }),
            2,
            0,
        ),
        (
            Format::Td2,
            format_td2(&Td2Fields {
                surname: "DOE".into(),
                ..Td2Fields::default()
            }),
            0,
            5,
        ),
        (
            Format::Td3,
            format_td3(&Td3Fields {
                surname: "DOE".into(),
                ..Td3Fields::default()
            }),
            0,
            5,
        ),
        (
            Format::MrvA,
            format_mrv_a(&MrvAFields {
                surname: "DOE".into(),
                ..MrvAFields::default()
            }),
            0,
            5,
        ),
        (
            Format::MrvB,
            format_mrv_b(&MrvBFields {
                surname: "DOE".into(),
                ..MrvBFields::default()
            }),
            0,
            5,
        ),
    ];
    for (format, zone, line_index, name_cell) in cases {
        for glyph in ["?", "É"] {
            let mut lines: Vec<String> = zone.lines().map(str::to_string).collect();
            lines[line_index].replace_range(name_cell..name_cell + 1, glyph);
            let detected = find_and_parse(&lines.join("\n"))
                .unwrap_or_else(|error| panic!("{format:?} {glyph} name: {error}"));
            assert_eq!(detected.format, format);
            assert!(detected.valid(), "checks outside the name still validate");
            assert!(detected.surname.is_empty(), "name stays uncertified");
        }
    }
}

#[test]
fn a_split_row_is_rejoined_at_each_format_width() {
    let cases = [
        (Format::Td1, format_td1(&Td1Fields::default())),
        (Format::Td2, format_td2(&Td2Fields::default())),
        (Format::Td3, format_td3(&Td3Fields::default())),
        (Format::MrvA, format_mrv_a(&MrvAFields::default())),
        (Format::MrvB, format_mrv_b(&MrvBFields::default())),
    ];
    for (format, zone) in cases {
        let mut lines: Vec<String> = zone.lines().map(str::to_string).collect();
        let middle = lines[1].len() / 2;
        lines[1].insert(middle, ' ');
        let detected = find_and_parse(&lines.join("\n")).expect("exact-width row rejoined");
        assert_eq!(detected.format, format);
        assert!(detected.valid());
    }
}

#[test]
fn unknown_in_checked_document_number_is_not_accepted() {
    let mut line = TD3_L2.to_string();
    line.replace_range(2..3, &UNKNOWN.to_string());
    let result = find_and_parse(&format!("{TD3_L1}\n{line}"));
    assert!(result.is_err() || !result.expect("checked field parses").valid());
}

#[test]
fn unknown_outside_name_is_not_mapped() {
    let mut line = TD3_L1.to_string();
    line.replace_range(2..3, &UNKNOWN.to_string());
    assert!(find_and_parse(&format!("{line}\n{TD3_L2}")).is_err());
}
