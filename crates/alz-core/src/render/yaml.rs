//! alz's own YAML emitter (decision-28): the YAML of generated agent, rule and skill frontmatter,
//! adopted layer files and `allowed.yaml`, written byte for byte as the TypeScript reference
//! writes it (`stringifyYaml` in `src/core/yaml.ts`: block style, no line folding) until cutover.
//!
//! The input is an [`EmitValue`]: text, booleans, integers, lists and maps with text keys. The
//! output is a [`String`] ending in a line feed. [`emit`] is pure and total. The golden samples
//! in `tests/fixtures/yaml-emit/samples.json` come from the reference
//! (`crates/alz-core/tools/yaml-emit-samples.mjs`); how text is quoted is in `text`.

mod text;

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use thiserror::Error;

/// One level of indentation: two spaces.
const INDENT_STEP: &str = "  ";

/// A key longer than this many UTF-16 code units is written as an explicit `? key` entry, the way
/// the reference does (a YAML implicit key may not be longer than 1024 characters).
const MAX_IMPLICIT_KEY_UNITS: usize = 1024;

/// A value to write as YAML.
///
/// Integers are written exactly. The reference holds numbers as floating point, so its samples
/// stop at 2^53; beyond that this emitter is exact and the reference is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmitValue {
    /// Text of any content: it is quoted or written as a block when YAML needs it.
    Text(String),
    /// `true` or `false`.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A list, written one `- item` per line.
    Seq(Vec<EmitValue>),
    /// A mapping with text keys.
    Map(EmitMap),
}

/// A mapping: entries in the order given, with unique keys. The caller decides the order (spec
/// §6, name order, where a section is sorted).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EmitMap(Vec<(String, EmitValue)>);

/// Two entries of a mapping have the same key. Its text is never part of the message, so the
/// error is safe to show; [`DuplicateKey::key`] gives it to a caller that quotes it properly.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("a mapping has the same key twice")]
pub struct DuplicateKey(String);

impl DuplicateKey {
    /// The key that came twice.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.0
    }
}

impl EmitMap {
    /// The mapping of `entries`, in the order given.
    ///
    /// # Errors
    ///
    /// [`DuplicateKey`] for the second entry with a key that came before.
    pub fn from_entries(entries: Vec<(String, EmitValue)>) -> Result<Self, DuplicateKey> {
        entries
            .iter()
            .try_fold(BTreeSet::new(), |mut seen, (key, _)| {
                if seen.insert(key.clone()) {
                    Ok(seen)
                } else {
                    Err(DuplicateKey(key.clone()))
                }
            })
            .map(|_| Self(entries))
    }

    /// The entries in order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &EmitValue)> + '_ {
        self.0.iter().map(|(key, value)| (key.as_str(), value))
    }

    /// How many entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there is no entry.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The YAML document of `value`: block style, two spaces per level, ending in a line feed. An
/// empty list is `[]` and an empty mapping is `{}`.
#[must_use]
pub fn emit(value: &EmitValue) -> String {
    format!("{}\n", node(value, ""))
}

/// `value` as the reference writes a node whose context indent is `indent`.
fn node(value: &EmitValue, indent: &str) -> String {
    match value {
        EmitValue::Text(text) => text::scalar(text, false, indent),
        EmitValue::Bool(flag) => flag.to_string(),
        EmitValue::Int(number) => number.to_string(),
        EmitValue::Seq(items) if items.is_empty() => "[]".to_owned(),
        EmitValue::Seq(items) => {
            let item_indent = format!("{indent}{INDENT_STEP}");
            block_lines(
                indent,
                items
                    .iter()
                    .map(|item| format!("- {}", node(item, &item_indent))),
            )
        }
        EmitValue::Map(map) if map.is_empty() => "{}".to_owned(),
        EmitValue::Map(map) => block_lines(
            indent,
            map.entries().map(|(key, value)| entry(key, value, indent)),
        ),
    }
}

/// The lines of a block collection: the first as it is, every later one after the indent.
fn block_lines(indent: &str, lines: impl Iterator<Item = String>) -> String {
    lines
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                line
            } else {
                format!("\n{indent}{line}")
            }
        })
        .collect()
}

/// One `key: value` entry of a mapping whose entries are at `indent`.
fn entry(key: &str, value: &EmitValue, indent: &str) -> String {
    let inner = format!("{indent}{INDENT_STEP}");
    let key_text = text::scalar(key, true, &inner);
    let explicit = key_text.encode_utf16().count() > MAX_IMPLICIT_KEY_UNITS;
    let head = if explicit {
        format!("? {key_text}\n{indent}:")
    } else {
        format!("{key_text}:")
    };
    // A non-empty collection under an implicit key starts on the next line; everything else,
    // an empty `[]` or `{}` included, follows the colon.
    let separator = match value {
        EmitValue::Seq(items) if !explicit && !items.is_empty() => format!("\n{inner}"),
        EmitValue::Map(map) if !explicit && !map.is_empty() => format!("\n{inner}"),
        EmitValue::Seq(_)
        | EmitValue::Map(_)
        | EmitValue::Text(_)
        | EmitValue::Bool(_)
        | EmitValue::Int(_) => " ".to_owned(),
    };
    format!("{head}{separator}{}", node(value, &inner))
}
