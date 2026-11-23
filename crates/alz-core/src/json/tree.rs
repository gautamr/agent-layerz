//! alz's own JSON tree. An object keeps the order it was read in and holds each key once; the
//! order a writer uses is its own decision (name order, spec §6).

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

/// A JSON number: an integer, exact up to the range of `i128`, or a finite float.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JsonNumber {
    /// A number written without a fraction or an exponent.
    Int(i128),
    /// Any other number.
    Float(f64),
}

/// One JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number.
    Number(JsonNumber),
    /// Text. A JSON string has a UTF-8 form, so a lone surrogate escape is refused when reading.
    Text(String),
    /// An array.
    Array(Vec<JsonValue>),
    /// An object.
    Object(JsonObject),
}

impl JsonValue {
    /// The word for the kind of value, for a message that says what was found.
    #[must_use]
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "a boolean",
            Self::Number(_) => "a number",
            Self::Text(_) => "text",
            Self::Array(_) => "an array",
            Self::Object(_) => "an object",
        }
    }
}

/// Two entries of an object have the same key. The error holds the key for a caller that quotes
/// it properly; its message does not print it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("an object has the same key twice")]
pub struct DuplicateKey(String);

impl DuplicateKey {
    /// The key that came twice.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.0
    }
}

/// An object: entries in the order given, with unique keys.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct JsonObject(Vec<(String, JsonValue)>);

impl JsonObject {
    /// The object of `entries`, in the order given.
    ///
    /// # Errors
    ///
    /// [`DuplicateKey`] for the first key that comes twice.
    pub fn from_entries(entries: Vec<(String, JsonValue)>) -> Result<Self, DuplicateKey> {
        let mut seen = BTreeSet::new();
        let duplicate = entries
            .iter()
            .find(|(key, _)| !seen.insert(key.as_str()))
            .map(|(key, _)| DuplicateKey(key.clone()));
        duplicate.map_or(Ok(Self(entries)), Err)
    }

    /// The value of `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.0
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// The entries in the order they were given.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &JsonValue)> + '_ {
        self.0.iter().map(|(key, value)| (key.as_str(), value))
    }

    /// The number of entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no entries.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<BTreeMap<String, JsonValue>> for JsonObject {
    /// A map holds each key once, so the conversion cannot fail; the entries come in name order.
    fn from(entries: BTreeMap<String, JsonValue>) -> Self {
        Self(entries.into_iter().collect())
    }
}
