//! The MRZ sex cell as a type (ADR-0019), and its text form (ADR-0020).
//!
//! Doc 9303 Part 4 note p puts three values in the zone: `M`, `F`, and the
//! filler `<` for unspecified. `X` is the visual zone's letter for
//! unspecified, not the MRZ's. No check digit covers the cell on any format,
//! so a misread there is invisible to the arithmetic. That is why a
//! non-conformant character is kept as read rather than mapped to a legal
//! value.

use std::fmt;
use std::str::FromStr;

/// What the MRZ sex cell holds.
///
/// ```
/// use mrz::Sex;
///
/// assert_eq!(Sex::from_zone('F'), Sex::Female);
/// assert_eq!(Sex::from_zone('<'), Sex::Unspecified);
/// assert_eq!(Sex::from_zone('1'), Sex::NonConformant('1')); // a misread, kept as read
/// assert_eq!(Sex::from_zone('X'), Sex::NonConformant('X')); // X is the VIZ letter, not the MRZ's
///
/// // The text form is the zone character.
/// assert_eq!(Sex::Unspecified.to_string(), "<");
/// assert_eq!("1".parse::<Sex>(), Ok(Sex::NonConformant('1')));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Sex {
    /// Zone `M`.
    Male,
    /// Zone `F`.
    Female,
    /// Zone `<`: the issuer does not identify the holder's sex (Part 4 note p).
    Unspecified,
    /// Any other zone character, `X` included, kept verbatim. Never `M`, `F`
    /// or `<`: [`Sex::from_zone`] maps those to the variants above.
    NonConformant(char),
}

impl Sex {
    /// Classify the sex cell's character.
    pub fn from_zone(c: char) -> Sex {
        match c {
            'M' => Sex::Male,
            'F' => Sex::Female,
            '<' => Sex::Unspecified,
            other => Sex::NonConformant(other),
        }
    }

    /// The character the zone holds: the inverse of [`Sex::from_zone`].
    pub fn zone_char(self) -> char {
        match self {
            Sex::Male => 'M',
            Sex::Female => 'F',
            Sex::Unspecified => '<',
            Sex::NonConformant(c) => c,
        }
    }
}

/// The ADR-0020 text form: the zone character.
impl fmt::Display for Sex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.zone_char())
    }
}

/// Rejection from parsing a [`Sex`]'s text form: it is exactly one MRZ-alphabet character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseSexError;

impl fmt::Display for ParseSexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an MRZ sex value is exactly one character of 0-9, A-Z or <")
    }
}

impl std::error::Error for ParseSexError {}

/// The inverse of `Display`: exactly one MRZ-alphabet character, classified by
/// [`Sex::from_zone`].
impl FromStr for Sex {
    type Err = ParseSexError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if crate::checksum::char_value(c).is_some() => Ok(Sex::from_zone(c)),
            _ => Err(ParseSexError),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Sex {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Sex {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// Overwrites the whole value. The variant itself carries M/F/<, so wiping
/// only the payload of `NonConformant` would leave those values behind.
#[cfg(feature = "zeroize")]
impl zeroize::Zeroize for Sex {
    fn zeroize(&mut self) {
        // Best-effort: overwrite the discriminant as well as the payload.
        *self = Sex::NonConformant('\0');
        if let Sex::NonConformant(c) = self {
            c.zeroize();
        }
        std::hint::black_box(&*self);
    }
}
