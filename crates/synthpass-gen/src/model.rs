//! The fictional identity model and generator configuration.

pub use mrz::Date;

/// Sex marker, as printed on a TD3 MRZ (position 20 of line 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sex {
    M,
    F,
    /// Unspecified — ICAO 9303 allows `X` but [`crate::data::generate_passport`]
    /// never produces it (kept for completeness / future use).
    X,
}

impl Sex {
    /// The single MRZ character for this value.
    pub fn as_mrz_char(self) -> char {
        match self {
            Sex::M => 'M',
            Sex::F => 'F',
            Sex::X => 'X',
        }
    }
}

/// A synthetic, fictional TD3 passport identity.
///
/// Every field describes an invented person drawn deterministically from a
/// seed (see [`crate::data`]) — never a real one. Dates are internally
/// consistent (date of birth strictly precedes date of expiry) by
/// construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Passport {
    /// MRZ document code, e.g. `"P"` for a passport.
    pub document_type: String,
    /// 3-letter ICAO/ISO 3166-1 issuing-state code.
    pub issuing_country: String,
    pub surname: String,
    pub given_names: String,
    /// Document number, already trimmed of MRZ filler padding.
    pub document_number: String,
    /// 3-letter ICAO/ISO 3166-1 nationality code.
    pub nationality: String,
    pub date_of_birth: Date,
    pub sex: Sex,
    pub date_of_expiry: Date,
    /// TD3 personal-number field; `None` when left blank (an all-filler field
    /// is a legitimate, checksum-valid TD3 MRZ — see `mrz::parser`).
    pub personal_number: Option<String>,
}

/// Document type variants for ICAO 9303 formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentType {
    /// TD1 - ID card format (3 lines × 30 characters)
    TD1,
    /// TD2 - Official travel document format (2 lines × 36 characters)
    TD2,
    /// TD3 - Passport format (2 lines × 44 characters)
    TD3,
}

impl DocumentType {
    /// Get the MRZ document code character for this type.
    ///
    /// **This is not a format separator.** TD1 and TD2 both correctly emit
    /// `"I"` per ICAO 9303 (an identity card and an official travel document
    /// share the same document-code letter) — only the MRZ *format*
    /// (line count and width, [`mrz::Format`] via [`DocumentType::into`])
    /// distinguishes them. Any consumer that needs to tell a TD1 from a TD2
    /// must read the format, never this code; see [`Self::as_str`] and the
    /// `From<DocumentType> for mrz::Format` bridge below.
    pub fn document_code(self) -> &'static str {
        match self {
            DocumentType::TD1 => "I", // Identity card
            DocumentType::TD2 => "I", // Official travel document (also uses 'I')
            DocumentType::TD3 => "P", // Passport
        }
    }

    /// Get the number of MRZ lines for this document type.
    pub fn mrz_lines(self) -> usize {
        match self {
            DocumentType::TD1 => 3,
            DocumentType::TD2 => 2,
            DocumentType::TD3 => 2,
        }
    }

    /// Stable string form, used for CLI flags, sidecar JSON (`"mrz_format"`),
    /// and bench report keys. Round-trips through [`Self::parse`].
    pub fn as_str(self) -> &'static str {
        match self {
            DocumentType::TD1 => "TD1",
            DocumentType::TD2 => "TD2",
            DocumentType::TD3 => "TD3",
        }
    }

    /// Parses a document type from a case-insensitive string (`"td1"`,
    /// `"TD1"`, ... all accepted). Mirrors `synthpass_bench::SpecimenClass::parse`'s
    /// style: lowercase-and-match, with an error message listing every valid
    /// value.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "td1" => Ok(Self::TD1),
            "td2" => Ok(Self::TD2),
            "td3" => Ok(Self::TD3),
            other => Err(format!(
                "unknown document type '{other}' (valid: td1, td2, td3)"
            )),
        }
    }
}

/// The generator's MRZ format is a subset of the parser's: [`mrz::Format`]
/// is `#[non_exhaustive]` and also names `MrvA`/`MrvB` (machine-readable
/// visas), which this generator does not render. This conversion is
/// infallible in the direction the generator actually needs.
impl From<DocumentType> for mrz::Format {
    fn from(doc_type: DocumentType) -> Self {
        match doc_type {
            DocumentType::TD1 => mrz::Format::Td1,
            DocumentType::TD2 => mrz::Format::Td2,
            DocumentType::TD3 => mrz::Format::Td3,
        }
    }
}

/// The reverse bridge is fallible: a checksum-valid read can come back as
/// `MrvA`/`MrvB`, or (since `mrz::Format` is `#[non_exhaustive]`) some future
/// variant this generator was never taught to render. Both must be errors,
/// not silently coerced to the nearest TDn — the generator must never claim
/// it produced a format it did not.
impl TryFrom<mrz::Format> for DocumentType {
    type Error = String;

    fn try_from(format: mrz::Format) -> Result<Self, Self::Error> {
        match format {
            mrz::Format::Td1 => Ok(Self::TD1),
            mrz::Format::Td2 => Ok(Self::TD2),
            mrz::Format::Td3 => Ok(Self::TD3),
            other => Err(format!(
                "{other:?} has no synthpass_gen::DocumentType equivalent — this generator only \
                 renders TD1/TD2/TD3"
            )),
        }
    }
}

/// Generation parameters: the seed plus render options.
///
/// The seed is the only thing that determines the generated identity and
/// pixels — see the determinism test in `tests/`. `include_personal_number`
/// is a render/content option, not a source of extra randomness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratorConfig {
    /// Seed for the deterministic identity generator (`ChaCha8Rng`).
    pub seed: u64,
    /// Document type to generate (defaults to TD3 for backward compatibility).
    pub document_type: DocumentType,
    /// Whether to populate the personal-number field. When `false` the
    /// field is left blank (all filler), which is a valid MRZ state.
    pub include_personal_number: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            document_type: DocumentType::TD3,
            include_personal_number: true,
        }
    }
}

impl GeneratorConfig {
    /// A config with the personal-number field populated and TD3 document type.
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            document_type: DocumentType::TD3,
            include_personal_number: true,
        }
    }

    /// Create a config for a specific document type.
    pub fn with_document_type(seed: u64, doc_type: DocumentType) -> Self {
        Self {
            seed,
            document_type: doc_type,
            include_personal_number: true,
        }
    }
}
