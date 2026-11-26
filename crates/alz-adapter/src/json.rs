//! The JSON value of the contract, for a `Json` file (spec §8.0 **Output.**), for native settings
//! (§7.9) and for the native fields of an agent (§7.2). It is the value, not its text: the core
//! writes it with its stable JSON emitter, keys in name order (§6).

use std::collections::BTreeSet;

use thiserror::Error;

use crate::number::Finite;

/// One JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A whole number, exact up to the range of `i128`.
    Int(i128),
    /// Any other number.
    Float(Finite),
    /// Text.
    Text(String),
    /// An array.
    Array(Vec<JsonValue>),
    /// An object.
    Object(JsonObject),
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

/// An object: entries in the order given, with unique keys. The order is the source's order, which
/// the native fields of an agent keep (§7.2); a writer decides the order it writes in.
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

    /// The entries, in order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &JsonValue)> + '_ {
        self.0.iter().map(|(key, value)| (key.as_str(), value))
    }

    /// The value of `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.0
            .iter()
            .find_map(|(entry, value)| (entry == key).then_some(value))
    }

    /// Whether the object has no entry.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{JsonObject, JsonValue};

    fn entry(key: &str, value: bool) -> (String, JsonValue) {
        (key.to_owned(), JsonValue::Bool(value))
    }

    #[test]
    fn an_object_keeps_the_order_it_was_given() {
        let object =
            JsonObject::from_entries(vec![entry("b", true), entry("a", false)]).expect("unique");
        let keys: Vec<&str> = object.entries().map(|(key, _)| key).collect();
        assert_eq!(keys, ["b", "a"]);
        assert_eq!(object.get("a"), Some(&JsonValue::Bool(false)));
        assert_eq!(object.get("c"), None);
    }

    #[test]
    fn the_first_key_that_comes_twice_is_the_error() {
        let error = JsonObject::from_entries(vec![
            entry("zq", true),
            entry("b", true),
            entry("zq", false),
        ])
        .expect_err("zq is twice");
        assert_eq!(error.key(), "zq");
        assert!(!error.to_string().contains("zq"));
    }
}
