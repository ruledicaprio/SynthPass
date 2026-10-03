//! [`LayoutFileError`]: everything loading a layout can refuse, with a message
//! that names the key or field and the rule (ADR-0022 Decision 4).

use std::fmt;
use std::path::PathBuf;

use synthpass_gen::LayoutError;

/// Why a layout file or byte string was refused. `Display` names the key or
/// field and the rule; JSON errors keep serde_json's line and column.
#[derive(Debug)]
pub enum LayoutFileError {
    /// The input is longer than [`crate::MAX_BYTES`]; refused before parsing.
    TooLarge { len: usize },
    /// The input is not UTF-8.
    NotUtf8 { valid_up_to: usize },
    /// Not JSON, or JSON that breaks the schema: an unknown, duplicate or
    /// missing key, or a value that is not a `u32` (or a known `format`).
    Json {
        source: serde_json::Error,
        /// The input around the error's line and column. serde reports a bad
        /// value (`-1`, `1.5`, a string) without the key it sat under; the
        /// text next to it does.
        near: String,
    },
    /// `schema_version` is not 1.
    SchemaVersion { found: u32 },
    /// `name` is not 1 to 64 characters from `[a-z0-9._/-]`.
    Name(NameError),
    /// `description` is longer than [`crate::MAX_DESCRIPTION_BYTES`] bytes.
    Description { len: usize },
    /// The file parsed but `ValidatedLayout::try_from_spec` refused it.
    Layout(LayoutError),
    /// The file could not be read.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Which `name` rule failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    Empty,
    TooLong { chars: usize },
    BadChar { ch: char, index: usize },
}

impl fmt::Display for LayoutFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutFileError::TooLarge { len } => write!(
                f,
                "input: {len} bytes exceeds the {}-byte limit",
                crate::MAX_BYTES
            ),
            LayoutFileError::NotUtf8 { valid_up_to } => {
                write!(
                    f,
                    "input: not valid UTF-8 (first bad byte at offset {valid_up_to})"
                )
            }
            LayoutFileError::Json { source, near } => {
                write!(f, "layout JSON: {source} (near `{near}`)")
            }
            LayoutFileError::SchemaVersion { found } => write!(
                f,
                "schema_version: {found} is not supported (this build reads only 1)"
            ),
            LayoutFileError::Name(NameError::Empty) => {
                f.write_str("name: must be 1 to 64 characters, found 0")
            }
            LayoutFileError::Name(NameError::TooLong { chars }) => {
                write!(f, "name: must be 1 to 64 characters, found {chars}")
            }
            LayoutFileError::Name(NameError::BadChar { ch, index }) => write!(
                f,
                "name: character {ch:?} at index {index} is not in [a-z0-9._/-]"
            ),
            LayoutFileError::Description { len } => write!(
                f,
                "description: {len} bytes exceeds the {}-byte limit",
                crate::MAX_DESCRIPTION_BYTES
            ),
            LayoutFileError::Layout(e) => write!(f, "{e} [rule {}]", e.rule()),
            LayoutFileError::Io { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for LayoutFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LayoutFileError::Json { source, .. } => Some(source),
            LayoutFileError::Layout(e) => Some(e),
            LayoutFileError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
