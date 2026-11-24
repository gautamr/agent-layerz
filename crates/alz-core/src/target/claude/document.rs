//! The JSON documents of the adapter (`settings.json`, `.mcp.json`): built as a sorted
//! [`Tree`], so the keys are in code point order by construction, and written by the stable JSON
//! emitter (spec §10.2).

use std::collections::BTreeMap;

use crate::json::{JsonNumber, JsonObject, JsonValue};
use crate::merge::Tree;
use crate::render::json::emit;
use crate::render::tree::GeneratedFile;

/// A mapping of the given entries.
pub(super) fn map<'a>(entries: impl IntoIterator<Item = (&'a str, Tree)>) -> Tree {
    Tree::Map(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

/// A text value.
pub(super) fn text(value: &str) -> Tree {
    Tree::Text(value.to_owned())
}

/// A list of texts.
pub(super) fn texts<'a>(values: impl IntoIterator<Item = &'a str>) -> Tree {
    Tree::List(values.into_iter().map(text).collect())
}

/// `tree` as the JSON value it stands for.
fn json(tree: &Tree) -> JsonValue {
    match tree {
        Tree::Null => JsonValue::Null,
        Tree::Bool(flag) => JsonValue::Bool(*flag),
        Tree::Int(whole) => JsonValue::Number(JsonNumber::Int(*whole)),
        Tree::Float(float) => JsonValue::Number(JsonNumber::Float(float.get())),
        Tree::Text(value) => JsonValue::Text(value.clone()),
        Tree::List(items) => JsonValue::Array(items.iter().map(json).collect()),
        Tree::Map(entries) => JsonValue::Object(JsonObject::from(
            entries
                .iter()
                .map(|(key, value)| (key.clone(), json(value)))
                .collect::<BTreeMap<_, _>>(),
        )),
    }
}

/// The file that holds `tree`: stable JSON with two-space indent and a final line feed.
pub(super) fn file(tree: &Tree) -> GeneratedFile {
    GeneratedFile::text(&emit(&json(tree)))
}
