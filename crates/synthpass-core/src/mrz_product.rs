//! The one place the MRZ's sex vocabulary becomes the product's.
//!
//! [`mrz::Sex`] says what the zone printed (ADR-0019, ADR-0020): `M`, `F`, the
//! filler `<` for unspecified, or a non-conformant character kept as read.
//! The product schema speaks the visual zone's vocabulary instead: `M`, `F`,
//! and `X` for unspecified (Doc 9303 Part 4 note p). Every crate that turns an
//! `mrz::MrzData` into product output, a benchmark column or a comparison goes
//! through [`sex`], so the mapping is decided here and nowhere else.

/// The product-schema sex for an MRZ sex cell: `M`, `F` or `X`.
///
/// This reproduces the parser's pre-0.8 collapse exactly. Every cell other
/// than `M`/`F` (the conformant filler `<` and a non-conformant misread
/// alike) becomes `X`, so product output is byte-identical to what it was
/// before `mrz` typed the field. It never returns `None` yet. The `Option` is
/// the shape a later, separately measured change needs, to leave a misread
/// cell empty instead of asserting a confident `X` (ADR-0019, step 3).
pub fn sex(sex: mrz::Sex) -> Option<&'static str> {
    match sex {
        mrz::Sex::Male => Some("M"),
        mrz::Sex::Female => Some("F"),
        mrz::Sex::Unspecified | mrz::Sex::NonConformant(_) => Some("X"),
        _ => Some("X"),
    }
}

#[cfg(test)]
mod tests {
    use super::sex;
    use mrz::Sex;

    #[test]
    fn reproduces_the_pre_0_8_collapse_exactly() {
        assert_eq!(sex(Sex::Male), Some("M"));
        assert_eq!(sex(Sex::Female), Some("F"));
        assert_eq!(sex(Sex::Unspecified), Some("X"));
        for c in ['X', '1', '0', 'S', '\0'] {
            assert_eq!(sex(Sex::NonConformant(c)), Some("X"), "{c:?}");
        }
    }

    #[test]
    fn matches_the_old_clean_sex_over_the_whole_mrz_alphabet() {
        for c in ('0'..='9').chain('A'..='Z').chain(['<']) {
            let old = match c {
                'M' => "M",
                'F' => "F",
                _ => "X",
            };
            assert_eq!(sex(Sex::from_zone(c)), Some(old), "{c:?}");
        }
    }
}
