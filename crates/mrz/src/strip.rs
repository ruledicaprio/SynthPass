//! Crate-private, behavior-neutral check program for the five MRZ grids.
//!
//! A cell can be constrained, check-covered, and a comparison operand at the
//! same time. Weights count positions in each concatenated checksum input,
//! not positions in the printed strip. This describes checksum consistency,
//! never identity with the printed zone. A name can fill its line completely;
//! no layout here assumes a trailing filler run.

use crate::checksum::verify;
use crate::Format;

// One bit per printed MRZ glyph: filler, 0-9, A-Z. The mask is
// positional metadata, independent of what the current direct parser accepts.
const FILLER: u64 = 1;
const DIGIT: u64 = ((1u64 << 10) - 1) << 1;
const LETTER: u64 = ((1u64 << 26) - 1) << 11;
const ANY: u64 = DIGIT | LETTER | FILLER;
const fn glyph(letter: u8) -> u64 {
    1u64 << (11 + (letter - b'A'))
}
const SEX: u64 = FILLER | glyph(b'M') | glyph(b'F');
const TD3_CODE: u64 = glyph(b'P');
const VISA_CODE: u64 = glyph(b'V');
const ID_CODE: u64 = glyph(b'I') | glyph(b'A') | glyph(b'C');
const WEIGHTS: [u8; 3] = [7, 3, 1];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    DocumentCode,
    Issuer,
    Name,
    DocumentNumber,
    DocumentNumberMarker,
    NumberRemainder,
    DocumentNumberCheck,
    Nationality,
    Birth,
    BirthCheck,
    Sex,
    Expiry,
    ExpiryCheck,
    Optional1,
    Optional2,
    PersonalCheck,
    CompositeCheck,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditPolicy {
    Prefix,
    CheckedData,
    UncheckedData,
    Comparison,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Check {
    DocumentNumber,
    Birth,
    Expiry,
    Personal,
    Composite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WeightedCheck {
    check: Check,
    weight: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CellSpec {
    line: u8,
    column: u8,
    field: Field,
    allowed: u64,
    local: Option<WeightedCheck>,
    composite_weight: Option<u8>,
    compared_against: Option<Check>,
    edit_policy: EditPolicy,
}

impl CellSpec {
    const EMPTY: Self = Self {
        line: 0,
        column: 0,
        field: Field::Name,
        allowed: ANY,
        local: None,
        composite_weight: None,
        compared_against: None,
        edit_policy: EditPolicy::UncheckedData,
    };

    fn unchecked_data(self) -> bool {
        self.local.is_none() && self.composite_weight.is_none() && self.compared_against.is_none()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OverflowEncoding {
    Spec,
    Legacy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LayoutMode {
    Ordinary,
    Td1Overflow(OverflowEncoding),
    Td2Overflow(OverflowEncoding),
    Td3Extension(OverflowEncoding),
}

#[derive(Debug)]
struct Template {
    format: Format,
    mode: LayoutMode,
    line_width: usize,
    line_count: usize,
    cells: Vec<CellSpec>,
}

const fn profile(field: Field) -> (u64, EditPolicy) {
    match field {
        Field::DocumentCode | Field::Issuer => (LETTER | FILLER, EditPolicy::Prefix),
        Field::Birth | Field::Expiry => (DIGIT | FILLER, EditPolicy::CheckedData),
        Field::BirthCheck
        | Field::ExpiryCheck
        | Field::DocumentNumberCheck
        | Field::PersonalCheck
        | Field::CompositeCheck => (DIGIT, EditPolicy::Comparison),
        Field::Sex => (SEX, EditPolicy::UncheckedData),
        Field::DocumentNumberMarker => (FILLER, EditPolicy::Comparison),
        Field::Name | Field::Nationality => (LETTER | FILLER, EditPolicy::UncheckedData),
        Field::Optional1 | Field::Optional2 => (ANY, EditPolicy::UncheckedData),
        Field::DocumentNumber | Field::NumberRemainder => (ANY, EditPolicy::CheckedData),
    }
}

const fn fields<const N: usize>(
    mut cells: [CellSpec; N],
    start: usize,
    end: usize,
    field: Field,
) -> [CellSpec; N] {
    let (allowed, edit_policy) = profile(field);
    let mut i = start;
    while i < end {
        cells[i].field = field;
        cells[i].allowed = allowed;
        cells[i].edit_policy = edit_policy;
        i += 1;
    }
    cells
}

const fn local<const N: usize>(
    mut cells: [CellSpec; N],
    start: usize,
    end: usize,
    check_cell: usize,
    check: Check,
) -> [CellSpec; N] {
    let mut i = start;
    while i < end {
        cells[i].local = Some(WeightedCheck {
            check,
            weight: WEIGHTS[(i - start) % 3],
        });
        i += 1;
    }
    cells[check_cell].compared_against = Some(check);
    cells
}

const fn composite<const N: usize>(
    mut cells: [CellSpec; N],
    spans: &[(usize, usize)],
    check_cell: usize,
) -> [CellSpec; N] {
    let mut span = 0;
    let mut position = 0;
    while span < spans.len() {
        let (start, end) = spans[span];
        let mut i = start;
        while i < end {
            cells[i].composite_weight = Some(WEIGHTS[position % 3]);
            position += 1;
            i += 1;
        }
        span += 1;
    }
    cells[check_cell].compared_against = Some(Check::Composite);
    cells
}

const fn grid<const N: usize>(width: usize) -> [CellSpec; N] {
    let mut cells = [CellSpec::EMPTY; N];
    let mut i = 0;
    while i < N {
        cells[i].line = (i / width) as u8;
        cells[i].column = (i % width) as u8;
        i += 1;
    }
    cells
}

const fn td1() -> [CellSpec; 90] {
    let mut c = grid::<90>(30);
    c = fields(c, 0, 2, Field::DocumentCode);
    c[0].allowed = ID_CODE;
    c = fields(c, 2, 5, Field::Issuer);
    c = fields(c, 5, 14, Field::DocumentNumber);
    c = fields(c, 14, 15, Field::DocumentNumberCheck);
    c = fields(c, 15, 30, Field::Optional1);
    c = fields(c, 30, 36, Field::Birth);
    c = fields(c, 36, 37, Field::BirthCheck);
    c = fields(c, 37, 38, Field::Sex);
    c = fields(c, 38, 44, Field::Expiry);
    c = fields(c, 44, 45, Field::ExpiryCheck);
    c = fields(c, 45, 48, Field::Nationality);
    c = fields(c, 48, 59, Field::Optional2);
    c = fields(c, 59, 60, Field::CompositeCheck);
    c = fields(c, 60, 90, Field::Name);
    c = local(c, 5, 14, 14, Check::DocumentNumber);
    c = local(c, 30, 36, 36, Check::Birth);
    c = local(c, 38, 44, 44, Check::Expiry);
    composite(c, &[(5, 30), (30, 37), (38, 45), (48, 59)], 59)
}

const fn two_line<const N: usize>(width: usize, passport: bool, visa: bool) -> [CellSpec; N] {
    let mut c = grid::<N>(width);
    c = fields(c, 0, 2, Field::DocumentCode);
    c[0].allowed = if passport {
        TD3_CODE
    } else if visa {
        VISA_CODE
    } else {
        ID_CODE
    };
    c = fields(c, 2, 5, Field::Issuer);
    c = fields(c, 5, width, Field::Name);
    c = fields(c, width, width + 9, Field::DocumentNumber);
    c = fields(c, width + 9, width + 10, Field::DocumentNumberCheck);
    c = fields(c, width + 10, width + 13, Field::Nationality);
    c = fields(c, width + 13, width + 19, Field::Birth);
    c = fields(c, width + 19, width + 20, Field::BirthCheck);
    c = fields(c, width + 20, width + 21, Field::Sex);
    c = fields(c, width + 21, width + 27, Field::Expiry);
    c = fields(c, width + 27, width + 28, Field::ExpiryCheck);
    c = fields(c, width + 28, N, Field::Optional1);
    c = local(c, width, width + 9, width + 9, Check::DocumentNumber);
    c = local(c, width + 13, width + 19, width + 19, Check::Birth);
    c = local(c, width + 21, width + 27, width + 27, Check::Expiry);
    if passport {
        c = fields(c, N - 2, N - 1, Field::PersonalCheck);
        c = fields(c, N - 1, N, Field::CompositeCheck);
        c = local(c, width + 28, N - 2, N - 2, Check::Personal);
        c = composite(
            c,
            &[
                (width, width + 10),
                (width + 13, width + 20),
                (width + 21, N - 1),
            ],
            N - 1,
        );
    } else if !visa {
        c = fields(c, N - 1, N, Field::CompositeCheck);
        c = composite(
            c,
            &[
                (width, width + 10),
                (width + 13, width + 20),
                (width + 21, N - 1),
            ],
            N - 1,
        );
    }
    c
}

const TD1: [CellSpec; 90] = td1();
const TD2: [CellSpec; 72] = two_line::<72>(36, false, false);
const TD3: [CellSpec; 88] = two_line::<88>(44, true, false);
const MRV_A: [CellSpec; 88] = two_line::<88>(44, false, true);
const MRV_B: [CellSpec; 72] = two_line::<72>(36, false, true);

/// Resolve the parser's conditional layout. TD3's overflow is a crate extension
/// by analogy with Parts 5/6; Part 4 defines no such encoding. Visas never
/// select overflow. Legacy mode reflects this crate's pre-0.6 emission only.
fn for_lines(format: Format, lines: &[&str]) -> Option<Template> {
    let (width, count, base): (usize, usize, &[CellSpec]) = match format {
        Format::Td1 => (30, 3, &TD1),
        Format::Td2 => (36, 2, &TD2),
        Format::Td3 => (44, 2, &TD3),
        Format::MrvA => (44, 2, &MRV_A),
        Format::MrvB => (36, 2, &MRV_B),
    };
    if lines.len() != count
        || lines
            .iter()
            .any(|line| !line.is_ascii() || line.len() != width)
    {
        return None;
    }
    let mut template = Template {
        format,
        mode: LayoutMode::Ordinary,
        line_width: width,
        line_count: count,
        cells: base.to_vec(),
    };
    let (number_start, optional_start, optional_end) = match format {
        Format::Td1 => (5, 15, 30),
        Format::Td2 => (36, 64, 71),
        Format::Td3 => (44, 72, 86),
        Format::MrvA | Format::MrvB => return Some(template),
    };
    let strip: String = lines.concat();
    let bytes = strip.as_bytes();
    let marker = number_start + 9;
    let number = &strip[number_start..marker];
    let optional = &strip[optional_start..optional_end];
    if bytes[marker] != b'<' || number.trim_end_matches('<').is_empty() {
        return Some(template);
    }
    let Some(end) = optional.find('<') else {
        return Some(template);
    };
    if end < 2 || !optional.as_bytes()[end - 1].is_ascii_digit() {
        return Some(template);
    }
    let check_cell = optional_start + end - 1;
    let remainder_end = check_cell;
    let spec_full = format!("{}{}", number, &strip[optional_start..remainder_end]);
    let printed = bytes[check_cell] as char;
    let encoding = if verify(&spec_full, printed) {
        OverflowEncoding::Spec
    } else if number.ends_with('<')
        && verify(
            &format!(
                "{}{}",
                number[0..8].trim_end_matches('<'),
                &strip[optional_start..remainder_end]
            ),
            printed,
        )
    {
        OverflowEncoding::Legacy
    } else {
        OverflowEncoding::Spec
    };
    template.mode = match (format, encoding) {
        (Format::Td1, e) => LayoutMode::Td1Overflow(e),
        (Format::Td2, e) => LayoutMode::Td2Overflow(e),
        (Format::Td3, e) => LayoutMode::Td3Extension(e),
        (Format::MrvA | Format::MrvB, _) => return None,
    };
    template.cells[marker].field = Field::DocumentNumberMarker;
    template.cells[marker].allowed = FILLER;
    template.cells[marker].compared_against = None;
    template.cells[marker].local = None;
    let principal = if encoding == OverflowEncoding::Legacy {
        8
    } else {
        9
    };
    for i in number_start..marker {
        template.cells[i].local = (i - number_start < principal).then_some(WeightedCheck {
            check: Check::DocumentNumber,
            weight: WEIGHTS[(i - number_start) % 3],
        });
    }
    for i in optional_start..remainder_end {
        let cell = &mut template.cells[i];
        cell.field = Field::NumberRemainder;
        cell.allowed = ANY;
        cell.edit_policy = EditPolicy::CheckedData;
        cell.local = Some(WeightedCheck {
            check: Check::DocumentNumber,
            weight: WEIGHTS[(principal + i - optional_start) % 3],
        });
    }
    let check = &mut template.cells[check_cell];
    check.field = Field::DocumentNumberCheck;
    check.allowed = DIGIT;
    check.edit_policy = EditPolicy::Comparison;
    check.compared_against = Some(Check::DocumentNumber);
    Some(template)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_mrv_a, parse_mrv_b, parse_td1, parse_td2, parse_td3, Checks, MrzData};

    // Deliberately independent of `CellSpec`, `WEIGHTS`, and the production
    // checksum helper. These are parser/Doc 9303 coordinates, not a second
    // traversal of the candidate program under test.
    type ReferenceSpan = (Check, Vec<(usize, usize)>, usize);

    fn spans(format: Format) -> Vec<ReferenceSpan> {
        match format {
            Format::Td1 => vec![
                (Check::DocumentNumber, vec![(5, 14)], 14),
                (Check::Birth, vec![(30, 36)], 36),
                (Check::Expiry, vec![(38, 44)], 44),
                (
                    Check::Composite,
                    vec![(5, 30), (30, 37), (38, 45), (48, 59)],
                    59,
                ),
            ],
            Format::Td2 => vec![
                (Check::DocumentNumber, vec![(36, 45)], 45),
                (Check::Birth, vec![(49, 55)], 55),
                (Check::Expiry, vec![(57, 63)], 63),
                (Check::Composite, vec![(36, 46), (49, 56), (57, 71)], 71),
            ],
            Format::Td3 => vec![
                (Check::DocumentNumber, vec![(44, 53)], 53),
                (Check::Birth, vec![(57, 63)], 63),
                (Check::Expiry, vec![(65, 71)], 71),
                (Check::Personal, vec![(72, 86)], 86),
                (Check::Composite, vec![(44, 54), (57, 64), (65, 87)], 87),
            ],
            Format::MrvA => vec![
                (Check::DocumentNumber, vec![(44, 53)], 53),
                (Check::Birth, vec![(57, 63)], 63),
                (Check::Expiry, vec![(65, 71)], 71),
            ],
            Format::MrvB => vec![
                (Check::DocumentNumber, vec![(36, 45)], 45),
                (Check::Birth, vec![(49, 55)], 55),
                (Check::Expiry, vec![(57, 63)], 63),
            ],
        }
    }

    fn reference_value(byte: u8) -> u32 {
        match byte {
            b'0'..=b'9' => u32::from(byte - b'0'),
            b'A'..=b'Z' => u32::from(byte - b'A') + 10,
            b'<' => 0,
            _ => panic!("test input must be MRZ alphabet"),
        }
    }

    fn reference_digit(bytes: &[u8]) -> u8 {
        let sum: u32 = bytes
            .iter()
            .enumerate()
            .map(|(i, b)| reference_value(*b) * [7, 3, 1][i % 3])
            .sum();
        b'0' + (sum % 10) as u8
    }

    fn reference_program(
        format: Format,
        strip: &[u8],
        mode: LayoutMode,
    ) -> Vec<(Check, Vec<usize>, usize)> {
        let mut checks: Vec<_> = spans(format)
            .into_iter()
            .map(|(check, ranges, comparison)| {
                let data = ranges
                    .into_iter()
                    .flat_map(|(start, end)| start..end)
                    .collect();
                (check, data, comparison)
            })
            .collect();
        if mode != LayoutMode::Ordinary {
            let (number, optional) = match format {
                Format::Td1 => (5, 15),
                Format::Td2 => (36, 64),
                Format::Td3 => (44, 72),
                _ => panic!("visa has no overflow"),
            };
            let remainder_len = strip[optional..].iter().position(|b| *b == b'<').unwrap() - 1;
            let principal = match mode {
                LayoutMode::Td1Overflow(OverflowEncoding::Legacy)
                | LayoutMode::Td2Overflow(OverflowEncoding::Legacy)
                | LayoutMode::Td3Extension(OverflowEncoding::Legacy) => 8,
                _ => 9,
            };
            checks[0] = (
                Check::DocumentNumber,
                (number..number + principal)
                    .chain(optional..optional + remainder_len)
                    .collect(),
                optional + remainder_len,
            );
        }
        checks
    }

    fn expected_fields(format: Format) -> Vec<(Field, usize, usize)> {
        match format {
            Format::Td1 => vec![
                (Field::DocumentCode, 0, 2),
                (Field::Issuer, 2, 5),
                (Field::DocumentNumber, 5, 14),
                (Field::DocumentNumberCheck, 14, 15),
                (Field::Optional1, 15, 30),
                (Field::Birth, 30, 36),
                (Field::BirthCheck, 36, 37),
                (Field::Sex, 37, 38),
                (Field::Expiry, 38, 44),
                (Field::ExpiryCheck, 44, 45),
                (Field::Nationality, 45, 48),
                (Field::Optional2, 48, 59),
                (Field::CompositeCheck, 59, 60),
                (Field::Name, 60, 90),
            ],
            Format::Td2 | Format::MrvB => {
                let final_field = if format == Format::Td2 {
                    Field::CompositeCheck
                } else {
                    Field::Optional1
                };
                vec![
                    (Field::DocumentCode, 0, 2),
                    (Field::Issuer, 2, 5),
                    (Field::Name, 5, 36),
                    (Field::DocumentNumber, 36, 45),
                    (Field::DocumentNumberCheck, 45, 46),
                    (Field::Nationality, 46, 49),
                    (Field::Birth, 49, 55),
                    (Field::BirthCheck, 55, 56),
                    (Field::Sex, 56, 57),
                    (Field::Expiry, 57, 63),
                    (Field::ExpiryCheck, 63, 64),
                    (Field::Optional1, 64, 71),
                    (final_field, 71, 72),
                ]
            }
            Format::Td3 | Format::MrvA => {
                let mut zones = vec![
                    (Field::DocumentCode, 0, 2),
                    (Field::Issuer, 2, 5),
                    (Field::Name, 5, 44),
                    (Field::DocumentNumber, 44, 53),
                    (Field::DocumentNumberCheck, 53, 54),
                    (Field::Nationality, 54, 57),
                    (Field::Birth, 57, 63),
                    (Field::BirthCheck, 63, 64),
                    (Field::Sex, 64, 65),
                    (Field::Expiry, 65, 71),
                    (Field::ExpiryCheck, 71, 72),
                ];
                if format == Format::Td3 {
                    zones.extend([
                        (Field::Optional1, 72, 86),
                        (Field::PersonalCheck, 86, 87),
                        (Field::CompositeCheck, 87, 88),
                    ]);
                } else {
                    zones.push((Field::Optional1, 72, 88));
                }
                zones
            }
        }
    }

    fn parse(format: Format, strip: &str) -> Result<MrzData, crate::MrzError> {
        let width = match format {
            Format::Td1 => 30,
            Format::Td2 | Format::MrvB => 36,
            Format::Td3 | Format::MrvA => 44,
        };
        match format {
            Format::Td1 => parse_td1(
                &strip[..width],
                &strip[width..width * 2],
                &strip[width * 2..],
            ),
            Format::Td2 => parse_td2(&strip[..width], &strip[width..]),
            Format::Td3 => parse_td3(&strip[..width], &strip[width..]),
            Format::MrvA => parse_mrv_a(&strip[..width], &strip[width..]),
            Format::MrvB => parse_mrv_b(&strip[..width], &strip[width..]),
        }
    }

    fn state(checks: &Checks, check: Check) -> Option<bool> {
        match check {
            Check::DocumentNumber => checks.document_number,
            Check::Birth => checks.date_of_birth,
            Check::Expiry => checks.date_of_expiry,
            Check::Personal => checks.personal_number,
            Check::Composite => checks.composite,
        }
    }

    fn assert_agreement(format: Format, lines: &[&str], expected_mode: LayoutMode) {
        let template = for_lines(format, lines).expect("well-formed fixture");
        assert_eq!(template.mode, expected_mode);
        assert_eq!(
            template.cells.len(),
            template.line_width * template.line_count
        );
        let document_code = match format {
            Format::Td3 => TD3_CODE,
            Format::MrvA | Format::MrvB => VISA_CODE,
            Format::Td1 | Format::Td2 => ID_CODE,
        };
        assert_eq!(template.cells[0].allowed, document_code);
        for cell in &template.cells {
            assert_ne!(cell.allowed, 0, "every cell has an alphabet");
            if cell.field == Field::Sex {
                assert_eq!(cell.allowed, SEX);
            }
            if matches!(cell.field, Field::Name | Field::Nationality) {
                assert_eq!(cell.allowed, LETTER | FILLER);
            }
        }
        let original = lines.concat().into_bytes();
        let program = reference_program(format, &original, expected_mode);
        if expected_mode == LayoutMode::Ordinary {
            let zones = expected_fields(format);
            assert_eq!(zones.first().map(|z| z.1), Some(0));
            assert_eq!(zones.last().map(|z| z.2), Some(template.cells.len()));
            for &(field, start, end) in &zones {
                for cell in &template.cells[start..end] {
                    assert_eq!(cell.field, field, "{format:?} zone {start}..{end}");
                }
            }
        }
        for (index, cell) in template.cells.iter().enumerate() {
            assert_eq!(
                (usize::from(cell.line), usize::from(cell.column)),
                (index / template.line_width, index % template.line_width)
            );
            let local = program.iter().find_map(|(check, data, _)| {
                if *check == Check::Composite {
                    return None;
                }
                data.iter()
                    .position(|pos| *pos == index)
                    .map(|i| WeightedCheck {
                        check: *check,
                        weight: [7, 3, 1][i % 3],
                    })
            });
            let composite = program
                .iter()
                .find(|(check, _, _)| *check == Check::Composite)
                .and_then(|(_, data, _)| data.iter().position(|pos| *pos == index))
                .map(|i| [7, 3, 1][i % 3]);
            let comparison = program
                .iter()
                .find(|(_, _, pos)| *pos == index)
                .map(|(check, _, _)| *check);
            assert_eq!(
                cell.local, local,
                "{format:?} {expected_mode:?} cell {index} local"
            );
            assert_eq!(
                cell.composite_weight, composite,
                "{format:?} cell {index} composite"
            );
            assert_eq!(
                cell.compared_against, comparison,
                "{format:?} cell {index} comparison"
            );
            assert_eq!(
                cell.unchecked_data(),
                local.is_none() && composite.is_none() && comparison.is_none()
            );
        }
        let baseline = parse(format, std::str::from_utf8(&original).unwrap()).expect("base parses");
        assert!(baseline.valid(), "fixture must start checksum-consistent");
        for index in 0..original.len() {
            let mut mutated = original.clone();
            mutated[index] = if original[index] == b'0' { b'1' } else { b'0' };
            let text = std::str::from_utf8(&mutated).unwrap();
            let parsed = parse(format, text);
            let Ok(parsed) = parsed else {
                // Direct parsers reject a changed document-code first cell.
                assert_eq!(
                    index, 0,
                    "unexpected parser error at {format:?} cell {index}: {parsed:?}"
                );
                continue;
            };
            // A mutation can remove an overflow signature or cause the parser
            // to choose the other encoding. That is a distinct layout, never
            // an ordinary-map prediction dressed as an overflow result.
            let width = template.line_width;
            let mutant_lines: Vec<&str> = text
                .as_bytes()
                .chunks(width)
                .map(|part| std::str::from_utf8(part).unwrap())
                .collect();
            let mutant_template = for_lines(format, &mutant_lines).unwrap();
            let mutant_program = reference_program(format, &mutated, mutant_template.mode);
            for (check, data, operand) in mutant_program {
                let input: Vec<u8> = data.iter().map(|pos| mutated[*pos]).collect();
                let expected = reference_digit(&input) == mutated[operand];
                if mutant_template.mode == expected_mode {
                    let (_, baseline_data, baseline_operand) = program
                        .iter()
                        .find(|(kind, _, _)| *kind == check)
                        .expect("check exists");
                    if &data == baseline_data && operand == *baseline_operand {
                        let old_digit = reference_digit(
                            &baseline_data
                                .iter()
                                .map(|pos| original[*pos])
                                .collect::<Vec<_>>(),
                        );
                        let delta = baseline_data
                            .iter()
                            .position(|pos| *pos == index)
                            .map(|position| {
                                (reference_value(mutated[index]) as i32
                                    - reference_value(original[index]) as i32)
                                    * [7, 3, 1][position % 3]
                            })
                            .unwrap_or(0);
                        let predicted =
                            b'0' + ((i32::from(old_digit - b'0') + delta).rem_euclid(10) as u8);
                        assert_eq!(
                            reference_digit(&input),
                            predicted,
                            "independent weighted delta at {format:?} cell {index}, {check:?}"
                        );
                    }
                }
                assert_eq!(
                    state(&parsed.checks, check),
                    Some(expected),
                    "{format:?} cell {index}, {check:?}, mode {:?}",
                    mutant_template.mode
                );
            }
        }
    }

    #[test]
    fn ordinary_program_matches_all_five_direct_parsers_cell_by_cell() {
        assert_agreement(
            Format::Td1,
            &[
                "I<UTOD231458907<<<<<<<<<<<<<<<",
                "7408122F1204159UTO<<<<<<<<<<<6",
                "ERIKSSON<<ANNA<MARIA<<<<<<<<<<",
            ],
            LayoutMode::Ordinary,
        );
        assert_agreement(
            Format::Td2,
            &[
                "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
                "D231458907UTO7408122F1204159<<<<<<<6",
            ],
            LayoutMode::Ordinary,
        );
        assert_agreement(
            Format::Td3,
            &[
                "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
                "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
            ],
            LayoutMode::Ordinary,
        );
        assert_agreement(
            Format::MrvA,
            &[
                "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
                "L898902C<3UTO6908061F9406236ZE184226B<<<<<<<",
            ],
            LayoutMode::Ordinary,
        );
        assert_agreement(
            Format::MrvB,
            &[
                "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
                "L898902C<3UTO6908061F9406236ZE184226",
            ],
            LayoutMode::Ordinary,
        );
    }

    fn overflow_lower(format: Format, legacy: bool) -> String {
        let (number, remainder, optional_len) = if legacy {
            ("L898902C<", "31234", 14)
        } else {
            (
                "D23145890",
                "XYZ",
                if format == Format::Td2 { 7 } else { 14 },
            )
        };
        let full = if legacy {
            format!("{}{}", &number[..8], remainder)
        } else {
            format!("{number}{remainder}")
        };
        let mut optional = format!("{remainder}{}<", reference_digit(full.as_bytes()) as char);
        optional.extend(std::iter::repeat_n('<', optional_len - optional.len()));
        let mut body = format!("{number}<UTO7408122F1204159{optional}");
        if format == Format::Td3 {
            body.push(reference_digit(optional.as_bytes()) as char);
        }
        let composite_data = if format == Format::Td3 {
            format!("{}{}{}", &body[0..10], &body[13..20], &body[21..43])
        } else {
            format!("{}{}{}", &body[0..10], &body[13..20], &body[21..35])
        };
        body.push(reference_digit(composite_data.as_bytes()) as char);
        body
    }

    #[test]
    fn overflow_modes_keep_relocated_check_and_legacy_weights() {
        assert_agreement(
            Format::Td1,
            &[
                "I<UTOD23145890<XYZ5<<<<<<<<<<<",
                "7408122F1204159UTO<<<<<<<<<<<0",
                "ERIKSSON<<ANNA<MARIA<<<<<<<<<<",
            ],
            LayoutMode::Td1Overflow(OverflowEncoding::Spec),
        );
        let td2 = overflow_lower(Format::Td2, false);
        assert_agreement(
            Format::Td2,
            &["I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<", &td2],
            LayoutMode::Td2Overflow(OverflowEncoding::Spec),
        );
        let td3 = overflow_lower(Format::Td3, false);
        assert_agreement(
            Format::Td3,
            &["P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<", &td3],
            LayoutMode::Td3Extension(OverflowEncoding::Spec),
        );
        let legacy = overflow_lower(Format::Td3, true);
        assert_agreement(
            Format::Td3,
            &["P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<", &legacy],
            LayoutMode::Td3Extension(OverflowEncoding::Legacy),
        );
    }

    #[test]
    fn blind_substitution_is_not_called_a_rejection() {
        let line1 = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        let original = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";
        let mutated = original.replacen('L', "B", 1); // L=21 and B=11: same residue.
        let template = for_lines(Format::Td3, &[line1, original]).unwrap();
        assert_eq!(template.cells[44].local.unwrap().weight, 7);
        let parsed = parse_td3(line1, &mutated).unwrap();
        assert_eq!(parsed.checks.document_number, Some(true));
        assert_eq!(parsed.checks.composite, Some(true));
    }
}
