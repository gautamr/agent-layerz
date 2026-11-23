//! The lock (spec §5.3): `layer.lock` as a value. A lock is repository content and untrusted, so
//! it is read once into this type, every field of which is validated; the build never works from
//! the text. It is written as stable JSON, keys in code point order, so that the same lock is the
//! same bytes (spec §6).

use std::collections::BTreeMap;
use std::fmt;

use thiserror::Error;

use crate::diagnostic::{Code, Diagnostic, Message, Where};
use crate::json::{self, JsonError, JsonNumber, JsonObject, JsonValue};
use crate::layer::{LayerName, Visibility};
use crate::lock::content_hash::{ContentHash, FileHash};
use crate::lock::object_id::ObjectId;
use crate::path::SafeRelPath;
use crate::render::json::emit;
use crate::stack::{GitRef, Source};
use crate::untrusted_text::Stripped;

/// The path of the lock from the repository root.
const LOCK_FILE: &str = "layer.lock";

/// One layer as the lock records it, in stack order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedLayer {
    /// The layer's name.
    pub name: LayerName,
    /// The stack entry the layer was read from, as written.
    pub source: Source,
    /// The ref the layer was resolved by, when it had one.
    pub rev: Option<GitRef>,
    /// The commit the layer was resolved to, when it has one. It decides which cached copy is
    /// read, so it is an [`ObjectId`] or the lock is invalid.
    pub sha: Option<ObjectId>,
    /// The hash of the layer's files (§5.3).
    pub content_hash: ContentHash,
}

/// `layer.lock`: the visibility the output inherits, each layer's pin, and the hash of every
/// generated file. The only version is 1, so the lock holds none: a lock of another version does
/// not parse, and [`Lock::emit`] writes `1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lock {
    /// The most private visibility in the stack.
    pub visibility: Visibility,
    /// The layers in stack order.
    pub layers: Vec<LockedLayer>,
    /// Each generated file's path and the hash of its content, in name order.
    pub files: BTreeMap<SafeRelPath, FileHash>,
}

/// The part of the lock that is missing or invalid. A lock holds the value of a field, so the
/// name is all an error says: the value is never printed (§10.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LockField {
    /// The document is not a JSON object.
    Document,
    /// `lockVersion`, which must be 1.
    LockVersion,
    /// `visibility`.
    Visibility,
    /// `layers`, which must be an array of objects.
    Layers,
    /// A layer's `name`.
    LayerName,
    /// A layer's `source`.
    LayerSource,
    /// A layer's `rev`.
    LayerRev,
    /// A layer's `sha`, which must be an object id.
    LayerSha,
    /// A layer's `contentHash`.
    LayerContentHash,
    /// `files`, which must be an object.
    Files,
    /// A key of `files`, which must be a safe relative path.
    FilePath,
    /// A value of `files`, which must be a hash.
    FileHash,
}

impl LockField {
    /// The field as the spec writes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Document => "the document",
            Self::LockVersion => "lockVersion",
            Self::Visibility => "visibility",
            Self::Layers => "layers",
            Self::LayerName => "layers[].name",
            Self::LayerSource => "layers[].source",
            Self::LayerRev => "layers[].rev",
            Self::LayerSha => "layers[].sha",
            Self::LayerContentHash => "layers[].contentHash",
            Self::Files => "files",
            Self::FilePath => "a key of files",
            Self::FileHash => "a value of files",
        }
    }
}

impl fmt::Display for LockField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Why a lock is invalid. It never holds input, except a quoted duplicate key inside a
/// [`JsonError`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LockError {
    /// The file is not valid JSON, is too large or too deep, or has a duplicate key.
    #[error("{0}")]
    Json(JsonError),
    /// A field is missing or has a value the spec does not allow.
    #[error("{0} is missing or not valid")]
    Invalid(LockField),
}

impl From<JsonError> for LockError {
    fn from(error: JsonError) -> Self {
        Self::Json(error)
    }
}

impl LockError {
    /// The error as the diagnostic `lock` at `layer.lock` (§5.3, §10.1), which stops the command
    /// before any value of the lock is used.
    #[must_use]
    pub fn into_diagnostic(self) -> Diagnostic {
        let file = Where::repository_file(Stripped::from_untrusted(LOCK_FILE));
        match self {
            Self::Json(error) => error.into_diagnostic(Code::Lock, file),
            Self::Invalid(field) => Diagnostic::error(
                Code::Lock,
                Some(file),
                Message::new("the lock's ")
                    .then(field.as_str())
                    .then(" is missing or not valid; delete the lock and rebuild"),
            ),
        }
    }
}

fn object(value: &JsonValue, field: LockField) -> Result<&JsonObject, LockError> {
    match value {
        JsonValue::Object(entries) => Ok(entries),
        JsonValue::Null
        | JsonValue::Bool(_)
        | JsonValue::Number(_)
        | JsonValue::Text(_)
        | JsonValue::Array(_) => Err(LockError::Invalid(field)),
    }
}

fn text(value: &JsonValue, field: LockField) -> Result<&str, LockError> {
    match value {
        JsonValue::Text(text) => Ok(text),
        JsonValue::Null
        | JsonValue::Bool(_)
        | JsonValue::Number(_)
        | JsonValue::Array(_)
        | JsonValue::Object(_) => Err(LockError::Invalid(field)),
    }
}

fn required<'a>(
    entries: &'a JsonObject,
    key: &str,
    field: LockField,
) -> Result<&'a JsonValue, LockError> {
    entries.get(key).ok_or(LockError::Invalid(field))
}

/// A field that may be absent; a present field of any other type is invalid.
fn optional<'a>(entries: &'a JsonObject, key: &str) -> Option<&'a JsonValue> {
    entries.get(key)
}

/// The error for a value that breaks the rule of `field`, whatever the value's own error was:
/// the value is never printed.
fn invalid<E>(field: LockField) -> impl FnOnce(E) -> LockError {
    move |_| LockError::Invalid(field)
}

/// Whether `number` is the integer 1, as `JSON.parse` reads `1` and `1.0` alike.
fn is_one(number: JsonNumber) -> bool {
    match number {
        JsonNumber::Int(whole) => whole == 1,
        JsonNumber::Float(float) => float.to_bits() == 1.0_f64.to_bits(),
    }
}

fn locked_layer(value: &JsonValue) -> Result<LockedLayer, LockError> {
    let entries = object(value, LockField::Layers)?;
    let name = text(
        required(entries, "name", LockField::LayerName)?,
        LockField::LayerName,
    )?;
    let source = text(
        required(entries, "source", LockField::LayerSource)?,
        LockField::LayerSource,
    )?;
    let content_hash = text(
        required(entries, "contentHash", LockField::LayerContentHash)?,
        LockField::LayerContentHash,
    )?;
    Ok(LockedLayer {
        name: LayerName::try_from(name).map_err(invalid(LockField::LayerName))?,
        source: Source::try_from(source).map_err(invalid(LockField::LayerSource))?,
        rev: optional(entries, "rev")
            .map(|rev| {
                GitRef::try_from(text(rev, LockField::LayerRev)?)
                    .map_err(invalid(LockField::LayerRev))
            })
            .transpose()?,
        sha: optional(entries, "sha")
            .map(|sha| {
                ObjectId::try_from(text(sha, LockField::LayerSha)?)
                    .map_err(invalid(LockField::LayerSha))
            })
            .transpose()?,
        content_hash: ContentHash::try_from(content_hash)
            .map_err(invalid(LockField::LayerContentHash))?,
    })
}

fn file_entry((key, value): (&str, &JsonValue)) -> Result<(SafeRelPath, FileHash), LockError> {
    Ok((
        SafeRelPath::try_from(key).map_err(invalid(LockField::FilePath))?,
        FileHash::try_from(text(value, LockField::FileHash)?)
            .map_err(invalid(LockField::FileHash))?,
    ))
}

impl Lock {
    /// Reads the lock from the bytes of `layer.lock`: at most 1 MiB of strict UTF-8, one JSON
    /// document nested at most 128 deep, no duplicate key (§5.3). Keys the lock does not know are
    /// ignored, as the reference does, and not written back.
    ///
    /// # Errors
    ///
    /// [`LockError::Json`] for text that is not such a document, and [`LockError::Invalid`] naming
    /// the first field that is missing or breaks its rule: `lockVersion` other than 1, a layer
    /// `sha` that is not 40 or 64 lowercase hexadecimal characters, a `contentHash` or file hash
    /// that is not `sha256:` and 64 of them, a file path that is not a safe relative path.
    pub fn parse(bytes: &[u8]) -> Result<Self, LockError> {
        Self::from_json(&json::parse(bytes)?)
    }

    /// The lock held by a parsed document.
    ///
    /// # Errors
    ///
    /// [`LockError::Invalid`], as for [`Lock::parse`].
    pub fn from_json(document: &JsonValue) -> Result<Self, LockError> {
        let entries = object(document, LockField::Document)?;
        match required(entries, "lockVersion", LockField::LockVersion)? {
            JsonValue::Number(number) if is_one(*number) => {}
            JsonValue::Null
            | JsonValue::Bool(_)
            | JsonValue::Number(_)
            | JsonValue::Text(_)
            | JsonValue::Array(_)
            | JsonValue::Object(_) => return Err(LockError::Invalid(LockField::LockVersion)),
        }
        let visibility = text(
            required(entries, "visibility", LockField::Visibility)?,
            LockField::Visibility,
        )?;
        let layers = match required(entries, "layers", LockField::Layers)? {
            JsonValue::Array(items) => items
                .iter()
                .map(locked_layer)
                .collect::<Result<Vec<_>, _>>()?,
            JsonValue::Null
            | JsonValue::Bool(_)
            | JsonValue::Number(_)
            | JsonValue::Text(_)
            | JsonValue::Object(_) => return Err(LockError::Invalid(LockField::Layers)),
        };
        let files = object(
            required(entries, "files", LockField::Files)?,
            LockField::Files,
        )?
        .iter()
        .map(file_entry)
        .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(Self {
            visibility: Visibility::try_from(visibility).map_err(invalid(LockField::Visibility))?,
            layers,
            files,
        })
    }

    /// The lock as a JSON document. Keys are in no particular order; [`Lock::emit`] orders them.
    #[must_use]
    pub fn to_json(&self) -> JsonValue {
        let text = |value: &dyn fmt::Display| JsonValue::Text(value.to_string());
        let layer = |layer: &LockedLayer| {
            let optional = [
                ("rev", layer.rev.as_ref().map(|rev| text(rev))),
                ("sha", layer.sha.as_ref().map(|sha| text(sha))),
            ];
            let entries = [
                ("name", Some(text(&layer.name))),
                ("source", Some(text(&layer.source))),
                ("contentHash", Some(text(&layer.content_hash))),
            ]
            .into_iter()
            .chain(optional)
            .filter_map(|(key, value)| value.map(|value| (key.to_owned(), value)))
            .collect::<BTreeMap<_, _>>();
            JsonValue::Object(entries.into())
        };
        let files = self
            .files
            .iter()
            .map(|(path, hash)| (path.to_string(), text(hash)))
            .collect::<BTreeMap<_, _>>();
        let top = BTreeMap::from([
            (
                "lockVersion".to_owned(),
                JsonValue::Number(JsonNumber::Int(1)),
            ),
            ("visibility".to_owned(), text(&self.visibility.as_str())),
            (
                "layers".to_owned(),
                JsonValue::Array(self.layers.iter().map(layer).collect()),
            ),
            ("files".to_owned(), JsonValue::Object(files.into())),
        ]);
        JsonValue::Object(top.into())
    }

    /// The bytes of `layer.lock`: stable JSON with keys in code point order, two-space indent and
    /// a final line feed.
    #[must_use]
    pub fn emit(&self) -> String {
        emit(&self.to_json())
    }
}

#[cfg(test)]
mod tests;
