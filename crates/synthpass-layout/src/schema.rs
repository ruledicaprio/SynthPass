//! The file schema (ADR-0022 Decisions 1, 2 and 4): two plain structs with
//! `deny_unknown_fields`, no `flatten`, no maps and no defaults for required
//! keys, so an unknown, duplicate or missing key and any number that is not a
//! `u32` is a serde error.

use serde::{Deserialize, Deserializer};
use synthpass_gen::layout::{LayoutField, LayoutSpec, Rect};
use synthpass_gen::DocumentType;

/// The one `schema_version` this crate reads.
pub const SCHEMA_VERSION: u32 = 1;

/// A rectangle in absolute pixels on the format's canvas.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// The `format` key. Spelled lowercase in the file.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FileFormat {
    Td1,
    Td2,
    Td3,
    Mrva,
    Mrvb,
}

impl FileFormat {
    pub(crate) fn document_type(self) -> DocumentType {
        match self {
            FileFormat::Td1 => DocumentType::TD1,
            FileFormat::Td2 => DocumentType::TD2,
            FileFormat::Td3 => DocumentType::TD3,
            FileFormat::Mrva => DocumentType::MrvA,
            FileFormat::Mrvb => DocumentType::MrvB,
        }
    }
}

/// The file's key for `format`.
pub(crate) fn format_key(format: DocumentType) -> &'static str {
    match format {
        DocumentType::TD1 => "td1",
        DocumentType::TD2 => "td2",
        DocumentType::TD3 => "td3",
        DocumentType::MrvA => "mrva",
        DocumentType::MrvB => "mrvb",
    }
}

/// `description` is optional, but an explicit `null` is not "absent": it is a
/// type error, so a key either carries a string or is not there.
fn string_not_null<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}

/// One layout file. Field names are the file's keys; the eleven rectangle keys
/// equal `LayoutField::name()` (pinned by `tests/key_lock.rs`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LayoutFile {
    pub(crate) schema_version: u32,
    pub(crate) name: String,
    #[serde(default, deserialize_with = "string_not_null")]
    pub(crate) description: Option<String>,
    pub(crate) format: FileFormat,
    portrait: FileRect,
    document_type: FileRect,
    issuing_country: FileRect,
    surname: FileRect,
    given_names: FileRect,
    document_number: FileRect,
    nationality: FileRect,
    date_of_birth: FileRect,
    sex: FileRect,
    date_of_expiry: FileRect,
    personal_number: FileRect,
}

fn rect(r: FileRect) -> Rect {
    Rect::new(r.x, r.y, r.width, r.height)
}

impl LayoutFile {
    /// The geometry the file states, ready for `ValidatedLayout::try_from_spec`.
    pub(crate) fn spec(&self) -> LayoutSpec {
        LayoutSpec {
            format: self.format.document_type(),
            portrait: rect(self.portrait),
            document_type: rect(self.document_type),
            issuing_country: rect(self.issuing_country),
            surname: rect(self.surname),
            given_names: rect(self.given_names),
            document_number: rect(self.document_number),
            nationality: rect(self.nationality),
            date_of_birth: rect(self.date_of_birth),
            sex: rect(self.sex),
            date_of_expiry: rect(self.date_of_expiry),
            personal_number: rect(self.personal_number),
        }
    }
}

/// The emitter's line for one rectangle key.
pub(crate) fn rect_line(field: LayoutField, r: Rect) -> String {
    format!(
        "  \"{}\": {{\"x\": {}, \"y\": {}, \"width\": {}, \"height\": {}}}",
        field.name(),
        r.x,
        r.y,
        r.width,
        r.height
    )
}
