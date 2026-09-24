use mrz::{
    format_mrv_a, format_mrv_b, format_td1, format_td2, format_td3, parse_mrv_a, parse_mrv_b,
    parse_td1, parse_td2, parse_td3, MrvAFields, MrvBFields, MrzError, Td1Fields, Td2Fields,
    Td3Fields,
};

#[test]
fn bad_length_counts_characters() {
    let zone = format_td3(&Td3Fields::default());
    let (line1, line2) = zone.split_once('\n').expect("two TD3 lines");
    let mut line1 = line1.to_string();
    line1.replace_range(5..6, "É");
    assert_eq!(line1.chars().count(), 44);
    assert_eq!(line1.len(), 45);
    assert_eq!(
        parse_td3(&line1, line2),
        Err(MrzError::BadLength {
            expected: 44,
            got: 44
        })
    );
}

#[test]
fn bad_document_code_always_keeps_both_raw_cells() {
    let td3 = format_td3(&Td3Fields::default());
    let td2 = format_td2(&Td2Fields::default());
    let td1 = format_td1(&Td1Fields::default());
    let mrv_a = format_mrv_a(&MrvAFields::default());
    let mrv_b = format_mrv_b(&MrvBFields::default());
    for (zone, parse) in [
        (&td3, parse_td3 as fn(&str, &str) -> Result<_, _>),
        (&td2, parse_td2),
        (&mrv_a, parse_mrv_a),
        (&mrv_b, parse_mrv_b),
    ] {
        let (line1, line2) = zone.split_once('\n').expect("two lines");
        let mut line1 = line1.to_string();
        line1.replace_range(0..1, "Z");
        assert_eq!(
            parse(&line1, line2),
            Err(MrzError::BadDocumentCode("Z<".into()))
        );
    }
    let mut lines = td1.lines();
    let mut line1 = lines.next().expect("TD1 line 1").to_string();
    line1.replace_range(0..1, "Z");
    assert_eq!(
        parse_td1(
            &line1,
            lines.next().expect("TD1 line 2"),
            lines.next().expect("TD1 line 3")
        ),
        Err(MrzError::BadDocumentCode("Z<".into()))
    );
}
