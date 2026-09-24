//! The one place the MRZ's sex vocabulary becomes the product's.
//!
//! [`mrz::Sex`] says what the zone printed (ADR-0019, ADR-0020): `M`, `F`, the
//! filler `<` for unspecified, or a non-conformant character kept as read.
//! The product schema speaks the visual zone's vocabulary instead: `M`, `F`,
//! and `X` for unspecified (Doc 9303 Part 4 note p). Every crate that turns an
//! `mrz::MrzData` into product output, a benchmark column or a comparison goes
//! through [`sex`], so the mapping is decided here and nowhere else.

/// The product-schema sex for an MRZ sex cell: `M`, `F`, `X`, or unknown.
///
/// The conformant filler `<` means unspecified and maps to the visual-zone
/// value `X`. A non-conformant cell is unreadable, so it maps to `None`
/// rather than asserting `X` (ADR-0019, step 3).
pub fn sex(sex: mrz::Sex) -> Option<&'static str> {
    match sex {
        mrz::Sex::Male => Some("M"),
        mrz::Sex::Female => Some("F"),
        mrz::Sex::Unspecified => Some("X"),
        mrz::Sex::NonConformant(_) => None,
        _ => Some("X"),
    }
}

#[cfg(test)]
mod tests {
    use super::sex;
    use mrz::Sex;

    #[test]
    fn maps_conformant_sex_and_leaves_nonconformant_unknown() {
        assert_eq!(sex(Sex::Male), Some("M"));
        assert_eq!(sex(Sex::Female), Some("F"));
        assert_eq!(sex(Sex::Unspecified), Some("X"));
        for c in ['X', '1', '0', 'S', '\0'] {
            assert_eq!(sex(Sex::NonConformant(c)), None, "{c:?}");
        }
    }

    #[test]
    fn maps_the_whole_mrz_alphabet_without_inventing_x() {
        for c in ('0'..='9').chain('A'..='Z').chain(['<']) {
            let expected = match c {
                'M' => Some("M"),
                'F' => Some("F"),
                '<' => Some("X"),
                _ => None,
            };
            assert_eq!(sex(Sex::from_zone(c)), expected, "{c:?}");
        }
    }
}
