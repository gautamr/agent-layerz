//! The values adoption moves between formats: JSON and YAML read from native files and layer
//! files become [`EmitValue`]s, the one value type the emitter of §10.2 writes, and values of one
//! layer file merge into what is on disk.

use crate::json::{JsonNumber, JsonValue};
use crate::render::yaml::{EmitMap, EmitValue};
use crate::yaml::{Node, Value};

/// Why a value cannot be written to a layer file: the emitter of §10.2 writes text, booleans,
/// integers of 64 bits, lists and mappings with text keys, and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Unwritable {
    /// `null`.
    Null,
    /// A number that is not a whole number.
    Fraction,
    /// A whole number beyond the 64-bit range.
    TooLarge,
    /// A mapping key that is not text.
    KeyNotText,
    /// A mapping that holds a key twice.
    SameKeyTwice,
}

impl Unwritable {
    /// What the value is, for a message.
    pub(super) const fn describe(self) -> &'static str {
        match self {
            Self::Null => "a null",
            Self::Fraction => "a number that is not a whole number",
            Self::TooLarge => "a whole number beyond 64 bits",
            Self::KeyNotText => "a key that is not text",
            Self::SameKeyTwice => "a key twice",
        }
    }
}

/// A text value.
pub(super) fn text(value: &str) -> EmitValue {
    EmitValue::Text(value.to_owned())
}

/// A list of texts.
pub(super) fn texts<'a>(values: impl IntoIterator<Item = &'a str>) -> EmitValue {
    EmitValue::Seq(values.into_iter().map(text).collect())
}

/// A mapping of the given entries, in order. The keys are literals of the program, so a key
/// that comes twice is a bug: the later entry is dropped, which keeps the function total.
pub(super) fn map<K: Into<String>>(entries: impl IntoIterator<Item = (K, EmitValue)>) -> EmitValue {
    let unique = entries.into_iter().fold(
        Vec::<(String, EmitValue)>::new(),
        |mut kept, (key, value)| {
            let key = key.into();
            if kept.iter().all(|(seen, _)| *seen != key) {
                kept.push((key, value));
            }
            kept
        },
    );
    EmitValue::Map(EmitMap::from_entries(unique).unwrap_or_default())
}

/// The JSON value as a value the emitter writes.
pub(super) fn from_json(value: &JsonValue) -> Result<EmitValue, Unwritable> {
    match value {
        JsonValue::Null => Err(Unwritable::Null),
        JsonValue::Bool(flag) => Ok(EmitValue::Bool(*flag)),
        JsonValue::Number(JsonNumber::Int(number)) => i64::try_from(*number)
            .map(EmitValue::Int)
            .map_err(|_beyond_64_bits| Unwritable::TooLarge),
        JsonValue::Number(JsonNumber::Float(_)) => Err(Unwritable::Fraction),
        JsonValue::Text(text) => Ok(EmitValue::Text(text.clone())),
        JsonValue::Array(items) => items
            .iter()
            .map(from_json)
            .collect::<Result<Vec<_>, _>>()
            .map(EmitValue::Seq),
        JsonValue::Object(object) => object
            .iter()
            .map(|(key, value)| from_json(value).map(|value| (key.to_owned(), value)))
            .collect::<Result<Vec<_>, _>>()
            .and_then(entries_map),
    }
}

/// The YAML node as a value the emitter writes.
pub(super) fn from_node(node: &Node) -> Result<EmitValue, Unwritable> {
    match node.value() {
        Value::Null => Err(Unwritable::Null),
        Value::Bool(flag) => Ok(EmitValue::Bool(*flag)),
        Value::Int(number) => i64::try_from(*number)
            .map(EmitValue::Int)
            .map_err(|_beyond_64_bits| Unwritable::TooLarge),
        Value::Float(_) => Err(Unwritable::Fraction),
        Value::Text(text) => Ok(EmitValue::Text(text.clone())),
        Value::Seq(items) => items
            .iter()
            .map(from_node)
            .collect::<Result<Vec<_>, _>>()
            .map(EmitValue::Seq),
        Value::Map(mapping) => mapping
            .entries()
            .map(|(key, value)| match key.value() {
                Value::Text(key) => from_node(value).map(|value| (key.clone(), value)),
                Value::Null
                | Value::Bool(_)
                | Value::Int(_)
                | Value::Float(_)
                | Value::Seq(_)
                | Value::Map(_) => Err(Unwritable::KeyNotText),
            })
            .collect::<Result<Vec<_>, _>>()
            .and_then(entries_map),
    }
}

fn entries_map(entries: Vec<(String, EmitValue)>) -> Result<EmitValue, Unwritable> {
    EmitMap::from_entries(entries)
        .map(EmitValue::Map)
        .map_err(|_twice| Unwritable::SameKeyTwice)
}

/// `next` merged into `current`: mappings key by key, lists with the items of `next` that
/// `current` lacks appended, and any other value only when equal. `None` when the shapes differ.
pub(super) fn merge(current: EmitValue, next: EmitValue) -> Option<EmitValue> {
    match (current, next) {
        (EmitValue::Seq(held), EmitValue::Seq(added)) => Some(EmitValue::Seq(
            added.into_iter().fold(held, |mut items, item| {
                if !items.contains(&item) {
                    items.push(item);
                }
                items
            }),
        )),
        (EmitValue::Map(held), EmitValue::Map(added)) => {
            let start = held
                .entries()
                .map(|(key, value)| (key.to_owned(), value.clone()))
                .collect::<Vec<_>>();
            let entries = added.entries().try_fold(start, |entries, (key, value)| {
                merge_entry(entries, key, value)
            })?;
            EmitMap::from_entries(entries).ok().map(EmitValue::Map)
        }
        (current, next) => (current == next).then_some(current),
    }
}

/// `entries` with `key` set to `value`, merged into the value it holds.
fn merge_entry(
    entries: Vec<(String, EmitValue)>,
    key: &str,
    value: &EmitValue,
) -> Option<Vec<(String, EmitValue)>> {
    if entries.iter().any(|(seen, _)| seen == key) {
        entries
            .into_iter()
            .map(|(seen, held)| {
                if seen == key {
                    merge(held, value.clone()).map(|merged| (seen, merged))
                } else {
                    Some((seen, held))
                }
            })
            .collect()
    } else {
        Some(
            entries
                .into_iter()
                .chain([(key.to_owned(), value.clone())])
                .collect(),
        )
    }
}
