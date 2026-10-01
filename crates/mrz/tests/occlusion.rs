//! `apply_occlusion` (#565, ADR-0026): a covered cell's value is never
//! returned as a field value. `mrz_lines` is the exception by design: it
//! keeps the read, covered cells included, and is pinned unchanged by
//! (e) below.
//!
//! This file rebuilds the crate's per-cell coverage table independently
//! (never importing `strip`'s private types), in the spirit of
//! `strip.rs`'s own `assert_agreement`/`reference_program`: a shared bug in
//! the production table and its test would otherwise agree with itself.
//! Every range below is cited against `README.md`'s "What a passing parse
//! guarantees" section and `src/strip.rs`'s field layout (`td1`/`two_line`).
//!
//! (a) `every_single_cell_mask_agrees_with_the_independently_built_coverage_table`
//! (b) `name_grammar_table`
//! (c) `overflow_layout_refuses_any_non_name_mask`
//! (d) `empty_mask_is_the_identity_over_every_corpus_zone`
//! (e) `mrz_lines_is_unchanged_by_occlusion`

mod support;

use mrz::{
    apply_occlusion, format_td1, format_td2, format_td3, parse_mrv_a, parse_mrv_b, parse_td1,
    parse_td2, parse_td3, CellMask, Format, MrzData, MrzError, Sex, Td1Fields, Td2Fields,
    Td3Fields, ZoneField,
};

// ---- Shared fixtures: the same ICAO Doc 9303 Utopia/Eriksson specimen used
// throughout this crate's own test suite (`src/lib.rs`, `tests/icao_vectors.rs`),
// one per format. `crates/mrz/tests` cannot reach `samples/ocr_fixtures/` in a
// published-crate-safe way (that directory sits outside the package this
// crate ships to crates.io), so every fixture below is either one of these
// worked specimens or built from this crate's own `format_*` emitters.

const TD1: [&str; 3] = [
    "I<UTOD231458907<<<<<<<<<<<<<<<",
    "7408122F1204159UTO<<<<<<<<<<<6",
    "ERIKSSON<<ANNA<MARIA<<<<<<<<<<",
];
const TD2: [&str; 2] = [
    "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
    "D231458907UTO7408122F1204159<<<<<<<6",
];
const TD3: [&str; 2] = [
    "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
    "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
];
const MRV_A: [&str; 2] = [
    "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
    "L898902C<3UTO6908061F9406236ZE184226B<<<<<<<",
];
const MRV_B: [&str; 2] = [
    "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
    "L898902C<3UTO6908061F9406236ZE184226",
];

fn canonical_zone(format: Format) -> (Vec<&'static str>, MrzData) {
    let (lines, parsed): (Vec<&str>, MrzData) = match format {
        Format::Td1 => (
            TD1.to_vec(),
            parse_td1(TD1[0], TD1[1], TD1[2]).expect("ICAO TD1 specimen parses"),
        ),
        Format::Td2 => (
            TD2.to_vec(),
            parse_td2(TD2[0], TD2[1]).expect("ICAO TD2 specimen parses"),
        ),
        Format::Td3 => (
            TD3.to_vec(),
            parse_td3(TD3[0], TD3[1]).expect("ICAO TD3 specimen parses"),
        ),
        Format::MrvA => (
            MRV_A.to_vec(),
            parse_mrv_a(MRV_A[0], MRV_A[1]).expect("ICAO MRV-A specimen parses"),
        ),
        Format::MrvB => (
            MRV_B.to_vec(),
            parse_mrv_b(MRV_B[0], MRV_B[1]).expect("ICAO MRV-B specimen parses"),
        ),
        other => panic!("unexpected format {other:?}"),
    };
    assert!(parsed.valid(), "fixture must start checksum-consistent");
    (lines, parsed)
}

const ALL_FORMATS: [Format; 5] = [
    Format::Td1,
    Format::Td2,
    Format::Td3,
    Format::MrvA,
    Format::MrvB,
];

fn width(format: Format) -> usize {
    match format {
        Format::Td1 => 30,
        Format::Td2 | Format::MrvB => 36,
        Format::Td3 | Format::MrvA => 44,
        other => panic!("unexpected format {other:?}"),
    }
}

// ---- (a): an independently built coverage table ----

/// A cell's expected disposition, named without reference to `strip`'s
/// private `CellClass`/`Field`.
#[derive(Clone, Copy)]
enum Kind {
    /// Line 1's first cell on every format.
    Structural,
    /// Feeds a check digit (local or composite).
    Checked,
    /// Unverifiable, and maps onto exactly one [`ZoneField`].
    Field(ZoneField),
    /// Unverifiable, and belongs to the name field -- graded by the grammar,
    /// not a single field.
    Name,
}

/// `(line, start_col, end_col, kind)` ranges, `end_col` exclusive. Built
/// directly from `src/strip.rs`'s `td1`/`two_line` layouts and cross-checked
/// against `README.md`'s "What a passing parse guarantees" table, not
/// imported from either.
fn layout(format: Format) -> Vec<(usize, usize, usize, Kind)> {
    use ZoneField::{DocumentCode, IssuingCountry, Nationality, OptionalData1, Sex as SexField};
    match format {
        Format::Td1 => vec![
            (0, 0, 1, Kind::Structural),
            (0, 1, 2, Kind::Field(DocumentCode)),
            (0, 2, 5, Kind::Field(IssuingCountry)),
            (0, 5, 15, Kind::Checked),  // document number + its check digit
            (0, 15, 30, Kind::Checked), // optional data 1 (composite-covered)
            (1, 0, 7, Kind::Checked),   // date of birth + its check digit
            (1, 7, 8, Kind::Field(SexField)),
            (1, 8, 15, Kind::Checked), // date of expiry + its check digit
            (1, 15, 18, Kind::Field(Nationality)),
            (1, 18, 30, Kind::Checked), // optional data 2 + composite check
            (2, 0, 30, Kind::Name),
        ],
        Format::Td2 | Format::Td3 | Format::MrvA | Format::MrvB => {
            let w = width(format);
            let mut out = vec![
                (0, 0, 1, Kind::Structural),
                (0, 1, 2, Kind::Field(DocumentCode)),
                (0, 2, 5, Kind::Field(IssuingCountry)),
                (0, 5, w, Kind::Name),
                (1, 0, 10, Kind::Checked), // document number + its check digit
                (1, 10, 13, Kind::Field(Nationality)),
                (1, 13, 20, Kind::Checked), // date of birth + its check digit
                (1, 20, 21, Kind::Field(SexField)),
                (1, 21, 28, Kind::Checked), // date of expiry + its check digit
            ];
            let tail = match format {
                // TD2: optional data feeds the composite alone.
                Format::Td2 => Kind::Checked,
                // TD3: optional data (personal number) has its own check
                // digit and feeds the composite too.
                Format::Td3 => Kind::Checked,
                // Visas print no composite and no personal-number digit at
                // all (Doc 9303 Part 7): this is the one field genuinely
                // unverifiable on these two formats.
                Format::MrvA | Format::MrvB => Kind::Field(OptionalData1),
                other => panic!("unexpected format {other:?}"),
            };
            out.push((1, 28, w, tail));
            out
        }
        other => panic!("unexpected format {other:?}"),
    }
}

fn field_value_is_blank(data: &MrzData, field: ZoneField) -> bool {
    match field {
        ZoneField::DocumentCode => data.document_type.is_empty(),
        ZoneField::IssuingCountry => data.issuing_country.is_empty(),
        ZoneField::Surname => data.surname.is_empty(),
        ZoneField::GivenNames => data.given_names.is_empty(),
        ZoneField::Nationality => data.nationality.is_empty(),
        ZoneField::Sex => data.sex == Sex::Unspecified,
        ZoneField::OptionalData1 => data.optional_data_1.is_none(),
        ZoneField::OptionalData2 => data.optional_data_2.is_none(),
        _ => panic!("unexpected ZoneField variant"),
    }
}

const NAME_BOTH: &[ZoneField] = &[ZoneField::Surname, ZoneField::GivenNames];
const NAME_GIVEN: &[ZoneField] = &[ZoneField::GivenNames];
const NAME_PADDING: &[ZoneField] = &[];

// Per-column expected fields for the literal canonical name spans. The three
// formats have the same `ERIKSSON<<ANNA<MARIA` prefix and different padding.
const TD1_NAME_COLUMNS: [&[ZoneField]; 30] = [
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
];
const SHORT_NAME_COLUMNS: [&[ZoneField]; 31] = [
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
];
const LONG_NAME_COLUMNS: [&[ZoneField]; 39] = [
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_BOTH,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_GIVEN,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
    NAME_PADDING,
];

#[test]
fn every_single_cell_mask_agrees_with_the_independently_built_coverage_table() {
    for format in ALL_FORMATS {
        let (_, parsed) = canonical_zone(format);
        for &(line, start, end, kind) in &layout(format) {
            for column in start..end {
                let mask = CellMask::EMPTY.with(line, column);
                let ctx = format!("{format:?} line {line} column {column}");
                match kind {
                    Kind::Structural | Kind::Checked => {
                        let err = apply_occlusion(&parsed, mask)
                            .expect_err(&format!("{ctx}: check-covered/structural must refuse"));
                        assert_eq!(
                            err,
                            MrzError::OccludedCheckedCell {
                                line,
                                position: column,
                            },
                            "{ctx}"
                        );
                    }
                    Kind::Field(zone_field) => {
                        let occluded = apply_occlusion(&parsed, mask).unwrap_or_else(|e| {
                            panic!("{ctx}: unverifiable must be Ok, got {e:?}")
                        });
                        assert_eq!(occluded.fields, vec![zone_field], "{ctx}");
                        assert!(
                            field_value_is_blank(&occluded.data, zone_field),
                            "{ctx}: {zone_field:?} must be blanked"
                        );
                    }
                    Kind::Name => {
                        let masked_index = column - start;
                        // Direct per-column vectors from the documented spans,
                        // independent of the helper that inferred this grammar.
                        let expected = match format {
                            Format::Td1 => TD1_NAME_COLUMNS[masked_index],
                            Format::Td2 | Format::MrvB => SHORT_NAME_COLUMNS[masked_index],
                            Format::Td3 | Format::MrvA => LONG_NAME_COLUMNS[masked_index],
                            other => unreachable!("unexpected name format {other:?}"),
                        };

                        let occluded = apply_occlusion(&parsed, mask)
                            .unwrap_or_else(|e| panic!("{ctx}: name cell must be Ok, got {e:?}"));
                        assert_eq!(occluded.fields.as_slice(), expected, "{ctx}");
                        for field in expected {
                            assert!(
                                field_value_is_blank(&occluded.data, *field),
                                "{ctx}: {field:?} must be blanked"
                            );
                        }
                        if expected.is_empty() {
                            assert_eq!(
                                occluded.data, parsed,
                                "{ctx}: a provably-padding-only mask changes nothing"
                            );
                        }
                    }
                }
            }
        }
    }
}

// ---- (b): the name grammar, as a curated table ----

struct NameCase {
    label: &'static str,
    format: Format,
    /// (line, column) cells to mask.
    mask: &'static [(usize, usize)],
    expected: &'static [ZoneField],
}

#[test]
fn name_grammar_table() {
    let cases = [
        NameCase {
            label: "a visible terminator: masking inside given names leaves the surname alone",
            format: Format::Td3,
            // Column 20 is inside "MARIA" (given names), well after the
            // fully visible `<<` at columns 13-14.
            mask: &[(0, 20)],
            expected: &[ZoneField::GivenNames],
        },
        NameCase {
            label: "a covered terminator: masking the `<<` itself withholds both components",
            format: Format::Td3,
            mask: &[(0, 13)],
            expected: &[ZoneField::Surname, ZoneField::GivenNames],
        },
        NameCase {
            label:
                "padding only: masking trailing filler after a visible terminator changes nothing",
            format: Format::Td3,
            // "ERIKSSON<<ANNA<MARIA" ends at column 24 (5 + 19); everything
            // from column 25 on is trailing filler.
            mask: &[(0, 30)],
            expected: &[],
        },
        NameCase {
            label: "a covered surname start withholds both components",
            format: Format::Td3,
            mask: &[(0, 5)],
            expected: &[ZoneField::Surname, ZoneField::GivenNames],
        },
        NameCase {
            label: "TD1's name line (line 2) follows the same grammar",
            format: Format::Td1,
            mask: &[(2, 15)], // inside "MARIA"
            expected: &[ZoneField::GivenNames],
        },
        NameCase {
            label: "TD3/MRV line 1's name field follows the same grammar (MRV-A)",
            format: Format::MrvA,
            mask: &[(0, 5)], // the first letter of the surname
            expected: &[ZoneField::Surname, ZoneField::GivenNames],
        },
    ];

    for case in cases {
        let (_, parsed) = canonical_zone(case.format);
        let mut mask = CellMask::EMPTY;
        for &(line, column) in case.mask {
            mask = mask.with(line, column);
        }
        let occluded = apply_occlusion(&parsed, mask)
            .unwrap_or_else(|e| panic!("{}: expected Ok, got {e:?}", case.label));
        assert_eq!(occluded.fields, case.expected, "{}", case.label);
        if case.expected.is_empty() {
            assert_eq!(occluded.data, parsed, "{}: nothing withheld", case.label);
        }
    }
}

/// A name that fills its whole field, with no trailing filler run at all: the
/// only terminator available for the given names is "the end of the field"
/// itself. Built through the emitter so the fixture cannot drift from what
/// `format_td3` actually produces for a maximal name.
#[test]
fn a_name_that_fills_its_whole_field() {
    // Surname (8) + separator (2) + given names must total exactly 39, the
    // TD3 name field width, with no trailing `<` at all -- a single-token
    // given name of the right length, built by repetition so the fixture
    // can't drift from a miscounted literal.
    let surname = "ERIKSSON";
    let given_names = "B".repeat(39 - surname.len() - 2);
    let lines = format_td3(&Td3Fields {
        document_number: "L898902C3".into(),
        surname: surname.into(),
        given_names,
        issuing_country: "UTO".into(),
        nationality: "UTO".into(),
        date_of_birth: support::birth("740812"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("120415"),
        ..Default::default()
    });
    let (l1, l2) = lines.split_once('\n').unwrap();
    let name_field = &l1[5..44];
    assert!(
        !name_field.ends_with('<'),
        "sanity: the fixture must fill the field with no trailing filler: {name_field:?}"
    );
    let parsed = parse_td3(l1, l2).expect("emitted TD3 must parse");
    assert!(parsed.valid());

    // The last cell of the field: no `<<` exists anywhere after the
    // surname/given-names separator, so only "end of field" can terminate it,
    // and this masked cell sits right at that end.
    let occluded = apply_occlusion(&parsed, CellMask::EMPTY.with(0, 43))
        .expect("an unverifiable name cell must be Ok");
    assert_eq!(occluded.fields, vec![ZoneField::GivenNames]);
    assert_eq!(occluded.data.given_names, "");
    assert_eq!(occluded.data.surname, "ERIKSSON");
}

// ---- (c): an overflow layout refuses any non-name mask ----

fn overflow_td1() -> MrzData {
    let lines = format_td1(&Td1Fields {
        issuing_country: "UTO".into(),
        document_number: "D231458901234".into(), // 13 chars: overflows the 9-char field
        surname: "ERIKSSON".into(),
        given_names: "ANNA MARIA".into(),
        nationality: "UTO".into(),
        date_of_birth: support::birth("740812"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("120415"),
        ..Default::default()
    });
    let l: Vec<&str> = lines.lines().collect();
    let parsed = parse_td1(l[0], l[1], l[2]).expect("emitted overflow TD1 must parse");
    assert!(
        parsed.document_number_full.is_some(),
        "sanity: overflow triggered"
    );
    assert!(parsed.valid());
    parsed
}

fn overflow_td2() -> MrzData {
    let lines = format_td2(&Td2Fields {
        document_code: "I".into(),
        issuing_country: "UTO".into(),
        document_number: "D2314589012".into(), // 11 chars: overflows the 9-char field
        surname: "ERIKSSON".into(),
        given_names: "ANNA MARIA".into(),
        nationality: "UTO".into(),
        date_of_birth: support::birth("740812"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("120415"),
        ..Default::default()
    });
    let l: Vec<&str> = lines.lines().collect();
    let parsed = parse_td2(l[0], l[1]).expect("emitted overflow TD2 must parse");
    assert!(
        parsed.document_number_full.is_some(),
        "sanity: overflow triggered"
    );
    assert!(parsed.valid());
    parsed
}

fn overflow_td3() -> MrzData {
    let lines = format_td3(&Td3Fields {
        document_number: "L898902C31234".into(), // 13 chars: overflows the 9-char field
        surname: "ERIKSSON".into(),
        given_names: "ANNA MARIA".into(),
        issuing_country: "UTO".into(),
        nationality: "UTO".into(),
        date_of_birth: support::birth("740812"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("120415"),
        ..Default::default()
    });
    let (l1, l2) = lines.split_once('\n').unwrap();
    let parsed = parse_td3(l1, l2).expect("emitted overflow TD3 must parse");
    assert!(
        parsed.document_number_full.is_some(),
        "sanity: overflow triggered"
    );
    assert!(parsed.valid());
    parsed
}

#[test]
fn overflow_layout_refuses_any_non_name_mask() {
    for parsed in [overflow_td1(), overflow_td2(), overflow_td3()] {
        // The sex cell: on an *ordinary* layout it is `Unverifiable` (no
        // check digit covers it on any format), so masking it alone would
        // give `Ok` with `fields == [ZoneField::Sex]` -- the honesty test
        // above confirms exactly that for every format's ordinary layout.
        // On this *overflow* layout, `for_lines` never touches the sex
        // cell's own classification at all (only the relocated
        // document-number/marker/remainder cells change, and they stay
        // `CheckCovered` in every overflow case this crate builds — see
        // `src/strip.rs`'s `for_lines`). So this specific assertion is only
        // true *because* `apply_occlusion` applies the overflow-outside-name
        // rule ahead of the coverage-map class, not as a coincidence of the
        // class itself.
        let (line, column) = match parsed.format {
            Format::Td1 => (1, 7),
            _ => (1, 20),
        };
        let mask = CellMask::EMPTY.with(line, column);
        let err = apply_occlusion(&parsed, mask).expect_err(&format!(
            "{:?}: a masked cell outside the name field must refuse on an overflow layout",
            parsed.format
        ));
        assert_eq!(
            err,
            MrzError::OccludedCheckedCell {
                line,
                position: column,
            },
            "{:?}",
            parsed.format
        );
    }
}

// ---- (d): the empty mask is the identity ----

#[test]
fn empty_mask_is_the_identity_over_every_corpus_zone() {
    let mut zones: Vec<MrzData> = ALL_FORMATS
        .iter()
        .map(|&format| canonical_zone(format).1)
        .collect();
    zones.push(overflow_td1());
    zones.push(overflow_td2());
    zones.push(overflow_td3());

    for parsed in zones {
        let occluded =
            apply_occlusion(&parsed, CellMask::EMPTY).expect("an empty mask must never refuse");
        assert_eq!(
            occluded.data, parsed,
            "{:?}: data must be unchanged",
            parsed.format
        );
        assert!(
            occluded.fields.is_empty(),
            "{:?}: an empty mask withholds nothing",
            parsed.format
        );
    }
}

// ---- Input the mask or the zone cannot honestly describe ----

#[test]
fn a_coordinate_no_zone_has_never_lands_on_another_cell() {
    // With a fixed stride of 44, an unchecked `line * 44 + column` would put
    // (0, 50) on (1, 6)'s bit. No format has a column 44 or a fourth line.
    for (line, column) in [
        (0, 44),
        (0, 50),
        (1, 44),
        (2, 40),
        (3, 0),
        (usize::MAX, usize::MAX),
    ] {
        let mask = CellMask::EMPTY.with(line, column);
        assert!(mask.is_empty(), "({line}, {column}) must be ignored");
        assert!(!mask.contains(line, column), "({line}, {column})");
    }
    assert!(!CellMask::EMPTY.with(0, 50).contains(1, 6));

    // Every real cell of every format keeps a bit of its own.
    for (line_count, line_width) in [(3, 30), (2, 36), (2, 44)] {
        for line in 0..line_count {
            for column in 0..line_width {
                let mask = CellMask::EMPTY.with(line, column);
                for other_line in 0..line_count {
                    for other_column in 0..line_width {
                        assert_eq!(
                            mask.contains(other_line, other_column),
                            (other_line, other_column) == (line, column),
                            "({line}, {column}) vs ({other_line}, {other_column})"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn a_zone_edited_after_parsing_is_refused_not_panicked() {
    // `MrzData`'s fields are public (and deserializable under `serde`), so a
    // caller can hand over a zone whose `mrz_lines` no longer fit its format.
    let (_, parsed) = canonical_zone(Format::Td3);
    let mask = CellMask::EMPTY.with(0, 20);
    for broken in [
        String::new(),
        TD3[0].to_owned(),                 // one line of two
        format!("{}\n{}", TD1[0], TD1[1]), // two lines, TD1's width
        format!("{}\n{}", TD3[0].replacen('E', "\u{c9}", 1), TD3[1]), // right width, not ASCII
    ] {
        let mut edited = parsed.clone();
        edited.mrz_lines = broken.clone();
        assert_eq!(
            apply_occlusion(&edited, mask),
            Err(MrzError::NotFound),
            "{broken:?}"
        );
    }
}

// ---- (e): `mrz_lines` is the validated read, and occlusion leaves it alone ----

/// The documented contract on `Occluded`: withheld fields are blanked in
/// `data`, but `data.mrz_lines` keeps every cell as read, covered ones
/// included (ADR-0026 rejects rewriting covered cells). Every `Ok` result
/// over every single-cell mask must return `mrz_lines` byte for byte.
#[test]
fn mrz_lines_is_unchanged_by_occlusion() {
    for format in ALL_FORMATS {
        let (_, parsed) = canonical_zone(format);
        let mut masks = Vec::new();
        for line in 0..3 {
            for column in 0..width(format) {
                masks.push(CellMask::EMPTY.with(line, column));
            }
        }
        let mut checked_ok = 0;
        for mask in masks {
            if let Ok(occluded) = apply_occlusion(&parsed, mask) {
                assert_eq!(
                    occluded.data.mrz_lines, parsed.mrz_lines,
                    "{format:?}: occlusion rewrote mrz_lines"
                );
                checked_ok += 1;
            }
        }
        assert!(
            checked_ok > 0,
            "{format:?}: no mask was accepted, so nothing was pinned"
        );
    }
}
