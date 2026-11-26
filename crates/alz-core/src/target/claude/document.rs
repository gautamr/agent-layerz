//! The JSON documents of the adapter (`settings.json`, `.mcp.json`): built as contract JSON values
//! and written by the stable JSON emitter, which puts the keys in code point order (spec §10.2).

use std::collections::BTreeMap;

use alz_adapter::{JsonObject, JsonValue};

use crate::json::{JsonNumber, JsonObject as WrittenObject, JsonValue as WrittenValue};
use crate::render::json::emit;
use crate::render::tree::GeneratedFile;

/// An object of the given entries. A key that comes twice has its last value, and the entries are
/// in key order.
pub(super) fn map<'a>(entries: impl IntoIterator<Item = (&'a str, JsonValue)>) -> JsonValue {
    object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value)),
    )
}

/// An object of the given entries, with owned keys.
pub(super) fn object(entries: impl IntoIterator<Item = (String, JsonValue)>) -> JsonValue {
    let unique: BTreeMap<String, JsonValue> = entries.into_iter().collect();
    // The keys of a map are unique, so the object cannot be refused.
    JsonValue::Object(JsonObject::from_entries(unique.into_iter().collect()).unwrap_or_default())
}

/// A text value.
pub(super) fn text(value: &str) -> JsonValue {
    JsonValue::Text(value.to_owned())
}

/// A list of texts.
pub(super) fn texts<'a>(values: impl IntoIterator<Item = &'a str>) -> JsonValue {
    JsonValue::Array(values.into_iter().map(text).collect())
}

/// The entries of an object, by key.
fn entries(object: &JsonObject) -> BTreeMap<String, JsonValue> {
    object
        .entries()
        .map(|(key, value)| (key.to_owned(), value.clone()))
        .collect()
}

/// The deep merge of §7.9, with `later` the more specific value: two objects merge key by key, two
/// lists are concatenated keeping the first occurrence of an item, and in every other case `later`
/// replaces `earlier`.
pub(super) fn deep_merge(earlier: JsonValue, later: JsonValue) -> JsonValue {
    match (earlier, later) {
        (JsonValue::Object(before), JsonValue::Object(after)) => {
            let merged = after
                .entries()
                .fold(entries(&before), |mut kept, (key, value)| {
                    let combined = match kept.remove(key) {
                        Some(known) => deep_merge(known, value.clone()),
                        None => value.clone(),
                    };
                    kept.insert(key.to_owned(), combined);
                    kept
                });
            object(merged)
        }
        (JsonValue::Array(before), JsonValue::Array(after)) => {
            JsonValue::Array(before.into_iter().chain(after).fold(
                Vec::new(),
                |mut kept: Vec<JsonValue>, item| {
                    if !kept.contains(&item) {
                        kept.push(item);
                    }
                    kept
                },
            ))
        }
        (_, later) => later,
    }
}

/// `value` as the JSON value the emitter writes.
fn written(value: &JsonValue) -> WrittenValue {
    match value {
        JsonValue::Null => WrittenValue::Null,
        JsonValue::Bool(flag) => WrittenValue::Bool(*flag),
        JsonValue::Int(whole) => WrittenValue::Number(JsonNumber::Int(*whole)),
        JsonValue::Float(float) => WrittenValue::Number(JsonNumber::Float(float.get())),
        JsonValue::Text(value) => WrittenValue::Text(value.clone()),
        JsonValue::Array(items) => WrittenValue::Array(items.iter().map(written).collect()),
        JsonValue::Object(members) => WrittenValue::Object(WrittenObject::from(
            members
                .entries()
                .map(|(key, value)| (key.to_owned(), written(value)))
                .collect::<BTreeMap<_, _>>(),
        )),
    }
}

/// The file that holds `value`: stable JSON with two-space indent and a final line feed.
pub(super) fn file(value: &JsonValue) -> GeneratedFile {
    GeneratedFile::text(&emit(&written(value)))
}
