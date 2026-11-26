//! A `Json` file (spec §8.0 **Output.**): the contract's value written by the stable JSON emitter,
//! keys in name order, two-space indent and a final line feed (§6). It has no header (§4.3).

use std::collections::BTreeMap;

use alz_adapter::JsonValue;

use crate::json::{JsonNumber, JsonObject, JsonValue as WrittenValue};
use crate::render::json::emit;
use crate::render::tree::GeneratedFile;

/// The file that holds `value`.
pub(super) fn file(value: &JsonValue) -> GeneratedFile {
    GeneratedFile::text(&emit(&written(value)))
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
        JsonValue::Object(members) => WrittenValue::Object(JsonObject::from(
            members
                .entries()
                .map(|(key, value)| (key.to_owned(), written(value)))
                .collect::<BTreeMap<_, _>>(),
        )),
    }
}
