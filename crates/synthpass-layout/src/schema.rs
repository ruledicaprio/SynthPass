//! The file schema (ADR-0022 Decisions 1, 2 and 4): two plain structs with
//! `deny_unknown_fields`, no `flatten`, no maps and no defaults for required
//! keys, so an unknown, duplicate or missing key and any number that is not a
//! `u32` is a serde error.

use std::fmt;
use std::marker::PhantomData;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use synthpass_gen::layout::{LayoutField, LayoutSpec, Rect};
use synthpass_gen::DocumentType;

/// The one `schema_version` this crate reads.
pub const SCHEMA_VERSION: u32 = 1;

/// Deserializes `T` from a JSON object and from nothing else. A derived struct
/// also accepts serde's sequence form (`[60, 120, 700, 34]`), which the format
/// does not have: this visitor implements only `visit_map`, so a sequence is
/// an "invalid type" error, and forwards the map to `T`'s derived impl, which
/// keeps `deny_unknown_fields`, duplicate-key detection and the line/column.
#[derive(Debug)]
pub(crate) struct MapOnly<T>(pub(crate) T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for MapOnly<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct MapVisitor<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for MapVisitor<T> {
            type Value = MapOnly<T>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a JSON object")
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                T::deserialize(MapAccessDeserializer::new(map)).map(MapOnly)
            }
        }

        d.deserialize_map(MapVisitor(PhantomData))
    }
}

/// A rectangle in absolute pixels on the format's canvas.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// The `format` key: one of five lowercase strings. Read as a string and
/// matched by hand, because a derived enum also accepts the externally tagged
/// map form (`{"td3": null}`).
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(try_from = "String")]
pub(crate) enum FileFormat {
    Td1,
    Td2,
    Td3,
    Mrva,
    Mrvb,
}

impl TryFrom<String> for FileFormat {
    type Error = String;

    fn try_from(key: String) -> Result<Self, String> {
        match key.as_str() {
            "td1" => Ok(FileFormat::Td1),
            "td2" => Ok(FileFormat::Td2),
            "td3" => Ok(FileFormat::Td3),
            "mrva" => Ok(FileFormat::Mrva),
            "mrvb" => Ok(FileFormat::Mrvb),
            _ => Err(format!(
                "unknown variant `{key}`, expected one of `td1`, `td2`, `td3`, `mrva`, `mrvb`"
            )),
        }
    }
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
    portrait: MapOnly<FileRect>,
    document_type: MapOnly<FileRect>,
    issuing_country: MapOnly<FileRect>,
    surname: MapOnly<FileRect>,
    given_names: MapOnly<FileRect>,
    document_number: MapOnly<FileRect>,
    nationality: MapOnly<FileRect>,
    date_of_birth: MapOnly<FileRect>,
    sex: MapOnly<FileRect>,
    date_of_expiry: MapOnly<FileRect>,
    personal_number: MapOnly<FileRect>,
}

fn rect(r: &MapOnly<FileRect>) -> Rect {
    let r = &r.0;
    Rect::new(r.x, r.y, r.width, r.height)
}

impl LayoutFile {
    /// The geometry the file states, ready for `ValidatedLayout::try_from_spec`.
    pub(crate) fn spec(&self) -> LayoutSpec {
        LayoutSpec {
            format: self.format.document_type(),
            portrait: rect(&self.portrait),
            document_type: rect(&self.document_type),
            issuing_country: rect(&self.issuing_country),
            surname: rect(&self.surname),
            given_names: rect(&self.given_names),
            document_number: rect(&self.document_number),
            nationality: rect(&self.nationality),
            date_of_birth: rect(&self.date_of_birth),
            sex: rect(&self.sex),
            date_of_expiry: rect(&self.date_of_expiry),
            personal_number: rect(&self.personal_number),
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
