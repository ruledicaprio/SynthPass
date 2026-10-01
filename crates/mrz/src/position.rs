//! [`position_class`]: which characters a format's layout allows at one cell.

use crate::Format;

/// Which of the 37 printed MRZ characters (`0`-`9`, `A`-`Z` and the filler `<`)
/// a format's ordinary layout allows at one cell.
///
/// A class can be wider than the cell's exact alphabet: the sex cell admits only
/// `M`, `F` and `<`, and the first document-code cell only its format's own
/// codes, yet both report a letter class. It is never narrower: a character the
/// layout admits at a cell is always in that cell's class.
///
/// # Examples
///
/// ```
/// use mrz::{position_class, Format, PositionClass};
///
/// // TD1 line 2, column 7 is the sex cell: the layout admits `M`, `F` and `<`,
/// // and reports the letter-or-filler class.
/// let sex = position_class(Format::Td1, 1, 7).unwrap();
/// assert_eq!(sex, PositionClass::LetterOrFiller);
/// assert!(sex.allows('F') && sex.allows('<') && !sex.allows('7'));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PositionClass {
    /// `0`-`9` only: the check digits of the document number, both dates and
    /// the composite.
    Digit,
    /// `0`-`9` or `<`: the date cells, and TD3's personal-number check digit,
    /// which an issuer may print as `<` when the personal number is unused.
    DigitOrFiller,
    /// `A`-`Z` only: the first document-code cell.
    Letter,
    /// `A`-`Z` or `<`: the second document-code cell, the issuing state, the
    /// nationality, the sex cell and the name.
    LetterOrFiller,
    /// Any of the 37 characters: the document number and the optional data.
    Any,
}

impl PositionClass {
    /// Whether `c` belongs to this class. Only the 37 printed MRZ characters
    /// can; lower case and every other character never do.
    ///
    /// # Examples
    ///
    /// ```
    /// use mrz::PositionClass;
    ///
    /// assert!(PositionClass::DigitOrFiller.allows('<'));
    /// assert!(!PositionClass::DigitOrFiller.allows('O'));
    /// assert!(!PositionClass::Any.allows('a'));
    /// ```
    pub fn allows(self, c: char) -> bool {
        let digit = c.is_ascii_digit();
        let letter = c.is_ascii_uppercase();
        let filler = c == '<';
        match self {
            Self::Digit => digit,
            Self::DigitOrFiller => digit || filler,
            Self::Letter => letter,
            Self::LetterOrFiller => letter || filler,
            Self::Any => digit || letter || filler,
        }
    }
}

/// The class of characters `format`'s ordinary layout allows at zero-based
/// `line` and `column`, or `None` outside the format's grid.
///
/// It describes the layout, not a reading, so it needs no text. One conditional
/// layout is not modelled: when a document number is longer than nine
/// characters, the cell after its first nine holds the filler instead of a
/// check digit, and the number continues in the optional data (TD1 and TD2, and
/// this crate's TD3 extension). A reader of a zone that may carry such a number
/// should expect the filler there.
///
/// The digit classes are exactly the cells in which
/// [`find_and_parse`](crate::find_and_parse)'s repair turns a misread letter
/// into a digit; a test keeps the two from drifting apart.
///
/// # Examples
///
/// ```
/// use mrz::{position_class, Format, PositionClass};
///
/// // TD3 line 2: the nationality is letters, the date of birth digits.
/// assert_eq!(position_class(Format::Td3, 1, 10), Some(PositionClass::LetterOrFiller));
/// assert_eq!(position_class(Format::Td3, 1, 13), Some(PositionClass::DigitOrFiller));
/// assert_eq!(position_class(Format::Td3, 1, 19), Some(PositionClass::Digit));
/// assert!(PositionClass::Digit.allows('7'));
/// assert!(!PositionClass::Digit.allows('<'));
/// // TD3 has two lines of 44 characters.
/// assert_eq!(position_class(Format::Td3, 2, 0), None);
/// assert_eq!(position_class(Format::Td3, 0, 44), None);
/// ```
pub fn position_class(format: Format, line: usize, column: usize) -> Option<PositionClass> {
    crate::strip::ordinary_cell(format, line, column).map(|cell| cell.position_class())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORMATS: [(Format, usize, usize); 5] = [
        (Format::Td1, 30, 3),
        (Format::Td2, 36, 2),
        (Format::Td3, 44, 2),
        (Format::MrvA, 44, 2),
        (Format::MrvB, 36, 2),
    ];

    /// One letter per class, so a whole line reads as a string: `D` digit,
    /// `d` digit or filler, `L` letter, `l` letter or filler, `A` any.
    fn line_classes(format: Format, line: usize, width: usize) -> String {
        (0..width)
            .map(|column| match position_class(format, line, column) {
                Some(PositionClass::Digit) => 'D',
                Some(PositionClass::DigitOrFiller) => 'd',
                Some(PositionClass::Letter) => 'L',
                Some(PositionClass::LetterOrFiller) => 'l',
                Some(PositionClass::Any) => 'A',
                None => '-',
            })
            .collect()
    }

    fn repeat(c: char, n: usize) -> String {
        c.to_string().repeat(n)
    }

    #[test]
    fn every_cell_of_every_grid_has_a_class_and_nothing_outside_it_does() {
        for (format, width, lines) in FORMATS {
            for line in 0..lines {
                for column in 0..width {
                    assert!(
                        position_class(format, line, column).is_some(),
                        "{format:?} line {line} column {column}"
                    );
                }
                assert_eq!(
                    position_class(format, line, width),
                    None,
                    "{format:?} line {line}"
                );
            }
            assert_eq!(position_class(format, lines, 0), None, "{format:?}");
            assert_eq!(
                position_class(format, usize::MAX, usize::MAX),
                None,
                "{format:?}"
            );
        }
    }

    // Each expected line is written field by field from the format's layout, as
    // literal text, not from the constants that build the cells.
    #[test]
    fn every_line_of_every_format_reads_as_its_layout() {
        let td3_line1 = format!("L{}", repeat('l', 43));
        let td3_line2 = format!(
            "{}Dlll{}Dl{}D{}dD",
            repeat('A', 9),
            repeat('d', 6),
            repeat('d', 6),
            repeat('A', 14)
        );
        let td2_line1 = format!("L{}", repeat('l', 35));
        let td2_line2 = format!(
            "{}Dlll{}Dl{}D{}D",
            repeat('A', 9),
            repeat('d', 6),
            repeat('d', 6),
            repeat('A', 7)
        );
        let td1_line1 = format!("Lllll{}D{}", repeat('A', 9), repeat('A', 15));
        let td1_line2 = format!(
            "{}Dl{}Dlll{}D",
            repeat('d', 6),
            repeat('d', 6),
            repeat('A', 11)
        );
        let td1_line3 = repeat('l', 30);
        let mrv_a_line2 = format!(
            "{}Dlll{}Dl{}D{}",
            repeat('A', 9),
            repeat('d', 6),
            repeat('d', 6),
            repeat('A', 16)
        );
        let mrv_b_line2 = format!(
            "{}Dlll{}Dl{}D{}",
            repeat('A', 9),
            repeat('d', 6),
            repeat('d', 6),
            repeat('A', 8)
        );
        let expected: [(Format, usize, usize, &str); 11] = [
            (Format::Td3, 0, 44, &td3_line1),
            (Format::Td3, 1, 44, &td3_line2),
            (Format::Td2, 0, 36, &td2_line1),
            (Format::Td2, 1, 36, &td2_line2),
            (Format::Td1, 0, 30, &td1_line1),
            (Format::Td1, 1, 30, &td1_line2),
            (Format::Td1, 2, 30, &td1_line3),
            (Format::MrvA, 0, 44, &td3_line1),
            (Format::MrvA, 1, 44, &mrv_a_line2),
            (Format::MrvB, 0, 36, &td2_line1),
            (Format::MrvB, 1, 36, &mrv_b_line2),
        ];
        for (format, line, width, want) in expected {
            assert_eq!(
                want.len(),
                width,
                "{format:?} line {line}: the expectation's width"
            );
            assert_eq!(
                line_classes(format, line, width),
                want,
                "{format:?} line {line}"
            );
        }
    }

    /// The height motif `knowledge/research/mrz-geometric-elimination.md` reads
    /// off line 2: `S` for a letter-or-filler cell (short ink), `T` for a digit
    /// cell (tall ink). TD3, TD2 and both visas share it at columns 10 to 27;
    /// TD1 carries it mirrored, nationality last, at columns 0 to 17.
    #[test]
    fn line_two_carries_the_height_motif_of_the_elimination_note() {
        let motif = |format: Format, line: usize, columns: std::ops::RangeInclusive<usize>| {
            columns
                .map(|column| match position_class(format, line, column) {
                    Some(PositionClass::Digit | PositionClass::DigitOrFiller) => 'T',
                    Some(PositionClass::LetterOrFiller) => 'S',
                    _ => '?',
                })
                .collect::<String>()
        };
        for format in [Format::Td3, Format::Td2, Format::MrvA, Format::MrvB] {
            assert_eq!(
                motif(format, 1, 10..=27),
                "SSSTTTTTTTSTTTTTTT",
                "{format:?}"
            );
        }
        assert_eq!(motif(Format::Td1, 1, 0..=17), "TTTTTTTSTTTTTTTSSS");
    }

    // Every character up to U+07FF, in code-point order, against literal lists:
    // the digits, then `<` (U+003C), then the capitals.
    #[test]
    fn allows_admits_exactly_the_literal_characters_of_its_class() {
        let digits = "0123456789";
        let letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let expected = [
            (PositionClass::Digit, digits.to_string()),
            (PositionClass::DigitOrFiller, format!("{digits}<")),
            (PositionClass::Letter, letters.to_string()),
            (PositionClass::LetterOrFiller, format!("<{letters}")),
            (PositionClass::Any, format!("{digits}<{letters}")),
        ];
        for (class, want) in expected {
            let got: String = (0u32..0x800)
                .filter_map(char::from_u32)
                .filter(|&c| class.allows(c))
                .collect();
            assert_eq!(got, want, "{class:?}");
        }
    }
}
