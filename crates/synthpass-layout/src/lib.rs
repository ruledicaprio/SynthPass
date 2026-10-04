//! JSON layout files for `synthpass-gen` (ADR-0022 Decisions 5-7).
//!
//! This crate parses one local file, or embedded bytes, into a
//! [`LoadedLayout`]. It uses no network and is deterministic. Parsing lives
//! here; validation stays in `synthpass-gen`: [`ValidatedLayout::try_from_spec`]
//! is the only way to a layout the renderer accepts, and every path in this
//! crate ends there, the five built-ins included.
//!
//! The format is documented in `knowledge/LAYOUTS.md`.

#![forbid(unsafe_code)]

mod error;
mod schema;

use std::fmt::Write as _;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;

use sha2::{Digest, Sha256};
use synthpass_gen::layout::LayoutField;
use synthpass_gen::{DocumentType, ValidatedLayout};

pub use error::{LayoutFileError, NameError};
pub use schema::SCHEMA_VERSION;

/// Input over this many bytes is refused before parsing (ADR-0022 Decision 4).
pub const MAX_BYTES: usize = 64 * 1024;
/// `description` may hold at most this many bytes.
pub const MAX_DESCRIPTION_BYTES: usize = 1024;

const MAX_NAME_CHARS: usize = 64;

/// A layout that passed every check, with the two strings its file carried.
/// Neither string is ever drawn, and neither is part of the layout's identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedLayout {
    name: String,
    description: Option<String>,
    layout: ValidatedLayout,
}

impl LoadedLayout {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn layout(&self) -> &ValidatedLayout {
        &self.layout
    }
}

fn check_name(name: &str) -> Result<(), NameError> {
    let chars = name.chars().count();
    if chars == 0 {
        return Err(NameError::Empty);
    }
    if chars > MAX_NAME_CHARS {
        return Err(NameError::TooLong { chars });
    }
    let bad = name
        .chars()
        .enumerate()
        .find(|(_, c)| !matches!(c, 'a'..='z' | '0'..='9' | '.' | '_' | '/' | '-'));
    match bad {
        Some((index, ch)) => Err(NameError::BadChar { ch, index }),
        None => Ok(()),
    }
}

/// Up to `BEFORE` bytes of line `line` ending `AFTER` bytes past the 1-based
/// byte `column`, for [`LayoutFileError::Json`]. Empty when serde reports no
/// position.
fn near(text: &str, line: usize, column: usize) -> String {
    const BEFORE: usize = 48;
    const AFTER: usize = 8;
    let Some(row) = line.checked_sub(1).and_then(|i| text.lines().nth(i)) else {
        return String::new();
    };
    let end = column.saturating_add(AFTER).min(row.len());
    let start = column.saturating_sub(BEFORE).min(end);
    String::from_utf8_lossy(&row.as_bytes()[start..end])
        .trim()
        .to_owned()
}

/// Parse and validate a layout file held in memory. Checks, in this order:
/// size, UTF-8, JSON (schema included), `schema_version`, `name`,
/// `description`, then the geometry rules of `ValidatedLayout::try_from_spec`.
pub fn parse_bytes(bytes: &[u8]) -> Result<LoadedLayout, LayoutFileError> {
    if bytes.len() > MAX_BYTES {
        return Err(LayoutFileError::TooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|e| LayoutFileError::NotUtf8 {
        valid_up_to: e.valid_up_to(),
    })?;
    let file: schema::LayoutFile = serde_json::from_str::<schema::MapOnly<_>>(text)
        .map(|file| file.0)
        .map_err(|source| {
            let near = near(text, source.line(), source.column());
            LayoutFileError::Json { source, near }
        })?;
    if file.schema_version != SCHEMA_VERSION {
        return Err(LayoutFileError::SchemaVersion {
            found: file.schema_version,
        });
    }
    check_name(&file.name).map_err(LayoutFileError::Name)?;
    if let Some(description) = &file.description {
        if description.len() > MAX_DESCRIPTION_BYTES {
            return Err(LayoutFileError::Description {
                len: description.len(),
            });
        }
    }
    let layout = ValidatedLayout::try_from_spec(file.spec()).map_err(LayoutFileError::Layout)?;
    Ok(LoadedLayout {
        name: file.name,
        description: file.description,
        layout,
    })
}

/// Read a local file and [`parse_bytes`] it. At most [`MAX_BYTES`] + 1 bytes
/// are read, so an oversized file is refused without being read whole.
pub fn load_path(path: &Path) -> Result<LoadedLayout, LayoutFileError> {
    let io = |source| LayoutFileError::Io {
        path: path.to_path_buf(),
        source,
    };
    let file = File::open(path).map_err(io)?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    parse_bytes(&bytes)
}

const BUILTIN_JSON: [&[u8]; 5] = [
    include_bytes!("../layouts/td1.json"),
    include_bytes!("../layouts/td2.json"),
    include_bytes!("../layouts/td3.json"),
    include_bytes!("../layouts/mrva.json"),
    include_bytes!("../layouts/mrvb.json"),
];

static BUILTINS: [OnceLock<LoadedLayout>; 5] = [const { OnceLock::new() }; 5];

fn slot(format: DocumentType) -> usize {
    match format {
        DocumentType::TD1 => 0,
        DocumentType::TD2 => 1,
        DocumentType::TD3 => 2,
        DocumentType::MrvA => 3,
        DocumentType::MrvB => 4,
    }
}

/// `format`'s built-in layout, parsed from the committed `layouts/*.json`
/// through [`parse_bytes`] like any other file (ADR-0022 Decision 6), once per
/// process.
///
/// A built-in that fails to load is a bug in the committed data, not an input
/// error; `tests/builtins.rs` loads all five.
pub fn builtin(format: DocumentType) -> &'static LoadedLayout {
    let i = slot(format);
    BUILTINS[i].get_or_init(|| {
        parse_bytes(BUILTIN_JSON[i]).expect(match format {
            DocumentType::TD1 => "the committed TD1 built-in layout loads",
            DocumentType::TD2 => "the committed TD2 built-in layout loads",
            DocumentType::TD3 => "the committed TD3 built-in layout loads",
            DocumentType::MrvA => "the committed MRV-A built-in layout loads",
            DocumentType::MrvB => "the committed MRV-B built-in layout loads",
        })
    })
}

/// The one emitter: fixed key order, two-space indent, one rectangle per line,
/// a trailing newline. `parse_bytes(to_json(l).as_bytes())` gives `l` back.
pub fn to_json(loaded: &LoadedLayout) -> String {
    // `to_string` on a `&str` cannot fail.
    let quote = |s: &str| serde_json::to_string(s).unwrap_or_default();
    let spec = loaded.layout.spec();
    let mut out = String::from("{\n");
    let _ = writeln!(out, "  \"schema_version\": {SCHEMA_VERSION},");
    let _ = writeln!(out, "  \"name\": {},", quote(&loaded.name));
    if let Some(description) = &loaded.description {
        let _ = writeln!(out, "  \"description\": {},", quote(description));
    }
    let _ = writeln!(
        out,
        "  \"format\": \"{}\",",
        schema::format_key(spec.format)
    );
    for (i, field) in LayoutField::ALL.into_iter().enumerate() {
        out.push_str(&schema::rect_line(field, spec.rect(field)));
        out.push_str(if i + 1 == LayoutField::ALL.len() {
            "\n"
        } else {
            ",\n"
        });
    }
    out.push_str("}\n");
    out
}

/// The layout's identity: SHA-256 over `canonical_bytes()` (ADR-0022
/// Decision 7), so whitespace, key order, `name` and `description` do not
/// change it.
pub fn identity(layout: &ValidatedLayout) -> [u8; 32] {
    Sha256::digest(layout.canonical_bytes()).into()
}

/// [`identity`] as 64 lowercase hex digits, for the `generate` sidecar.
pub fn identity_hex(layout: &ValidatedLayout) -> String {
    identity(layout)
        .iter()
        .fold(String::with_capacity(64), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}
