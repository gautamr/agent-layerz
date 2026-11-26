//! Native values (spec §7.2, §7.9) as the JSON value of the contract.
//!
//! The contract's number is finite, as JSON writes it. alz writes a number that is not finite as
//! `null` (`render::json`), and an agent writer refuses a null, so such a number becomes
//! [`JsonValue::Null`] here and the output is the one it always was.

use alz_adapter::{Finite, JsonObject, JsonValue};

use crate::item::NativeValue;
use crate::merge::Tree;

/// A float as the contract holds it: null when it is not finite.
fn float(number: f64) -> JsonValue {
    Finite::try_from(number).map_or(JsonValue::Null, JsonValue::Float)
}

/// An object of entries whose keys are unique, as the keys of a mapping are.
fn object(entries: Vec<(String, JsonValue)>) -> JsonValue {
    JsonValue::Object(JsonObject::from_entries(entries).unwrap_or_default())
}

/// A native value of an agent.
pub(super) fn of_native(value: &NativeValue) -> JsonValue {
    match value {
        NativeValue::Null => JsonValue::Null,
        NativeValue::Bool(flag) => JsonValue::Bool(*flag),
        NativeValue::Int(number) => JsonValue::Int(*number),
        NativeValue::Float(number) => float(number.get()),
        NativeValue::Text(text) => JsonValue::Text(text.clone()),
        NativeValue::List(items) => JsonValue::Array(items.iter().map(of_native).collect()),
        NativeValue::Map(map) => object(
            map.iter()
                .map(|(key, value)| (key.to_owned(), of_native(value)))
                .collect(),
        ),
    }
}

/// A settings value.
pub(super) fn of_tree(tree: &Tree) -> JsonValue {
    match tree {
        Tree::Null => JsonValue::Null,
        Tree::Bool(flag) => JsonValue::Bool(*flag),
        Tree::Int(number) => JsonValue::Int(*number),
        Tree::Float(number) => float(number.get()),
        Tree::Text(text) => JsonValue::Text(text.clone()),
        Tree::List(items) => JsonValue::Array(items.iter().map(of_tree).collect()),
        Tree::Map(entries) => object(
            entries
                .iter()
                .map(|(key, value)| (key.clone(), of_tree(value)))
                .collect(),
        ),
    }
}
