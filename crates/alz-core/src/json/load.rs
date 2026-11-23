//! The one place `serde_json` is called (decision-33): its deserializer runs through alz's own
//! visitor, which builds [`JsonValue`] and refuses a duplicate key. `serde_json`'s error text is
//! never read, only its category and position.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt;

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};

use crate::diagnostic::Position;
use crate::json::error::{JsonError, JsonErrorKind};
use crate::json::tree::{JsonNumber, JsonObject, JsonValue};
use crate::untrusted_text::QuotedKey;

/// The largest JSON file alz reads: 1 MiB (1048576 bytes).
pub const MAX_BYTES: usize = 1_048_576;

/// The deepest nesting of arrays and objects alz reads. `serde_json`'s recursion limit is 128 and
/// counts the root value as the first level, so a document of 127 containers inside one another
/// is read and the 128th is refused. The limit stays on because `unbounded_depth` stays off
/// (decision-33).
pub const MAX_DEPTH: usize = 127;

/// Parses `bytes` as one JSON document: at most [`MAX_BYTES`], strict UTF-8, one value and
/// nothing after it, nested at most [`MAX_DEPTH`] deep, with no duplicate key.
///
/// # Errors
///
/// The first problem found, as a [`JsonError`].
pub fn parse(bytes: &[u8]) -> Result<JsonValue, JsonError> {
    if bytes.len() > MAX_BYTES {
        return Err(JsonError::new(JsonErrorKind::TooLarge, None));
    }
    let text = std::str::from_utf8(bytes).map_err(|failure| {
        let valid = bytes.get(..failure.valid_up_to()).unwrap_or_default();
        let prefix = std::str::from_utf8(valid).unwrap_or_default();
        JsonError::new(JsonErrorKind::InvalidUtf8, Some(end_of(prefix)))
    })?;
    let failure = RefCell::new(None);
    read(text, &failure).map_err(|error| {
        let at = Position::new(
            u32::try_from(error.line()).unwrap_or(u32::MAX),
            u32::try_from(characters_before(text, error.line(), error.column()))
                .unwrap_or(u32::MAX),
        );
        JsonError::new(failure.take().unwrap_or(JsonErrorKind::Syntax), at)
    })
}

/// `serde_json` counts a column in bytes of the line, up to and including the last byte it read;
/// the spec counts it in characters (§10.1). This is the number of characters of line `line` that
/// start within the first `byte_column` bytes, which is the column in characters. A line the text
/// does not have keeps the byte column.
fn characters_before(text: &str, line: usize, byte_column: usize) -> usize {
    line.checked_sub(1)
        .and_then(|index| text.split('\n').nth(index))
        .map_or(byte_column, |line_text| {
            line_text
                .char_indices()
                .take_while(|&(offset, _)| offset < byte_column)
                .count()
        })
}

/// The position just after `prefix`, where the next character would be: lines and columns count
/// from 1, the column in characters.
fn end_of(prefix: &str) -> Position {
    let line = prefix.matches('\n').count().saturating_add(1);
    let column = prefix
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count())
        .saturating_add(1);
    Position::new(
        u32::try_from(line).unwrap_or(u32::MAX),
        u32::try_from(column).unwrap_or(u32::MAX),
    )
    .unwrap_or(Position::FIRST)
}

#[expect(
    clippy::disallowed_methods,
    reason = "the one entry point to serde_json (decision-33): every JSON file alz reads goes through json::parse"
)]
fn read(
    text: &str,
    failure: &RefCell<Option<JsonErrorKind>>,
) -> Result<JsonValue, serde_json::Error> {
    let mut reader = serde_json::Deserializer::from_str(text);
    let value = ValueSeed(failure).deserialize(&mut reader)?;
    reader.end()?;
    Ok(value)
}

/// Keeps the first problem the tree itself found, which `serde_json`'s error cannot carry, and
/// gives the error that stops `serde_json`.
fn fail<E: de::Error>(failure: &RefCell<Option<JsonErrorKind>>, kind: JsonErrorKind) -> E {
    failure.borrow_mut().get_or_insert(kind);
    E::custom("stopped")
}

struct ValueSeed<'a>(&'a RefCell<Option<JsonErrorKind>>);

impl<'de> DeserializeSeed<'de> for ValueSeed<'_> {
    type Value = JsonValue;

    fn deserialize<D: de::Deserializer<'de>>(self, reader: D) -> Result<JsonValue, D::Error> {
        reader.deserialize_any(ValueVisitor(self.0))
    }
}

struct ValueVisitor<'a>(&'a RefCell<Option<JsonErrorKind>>);

impl<'de> Visitor<'de> for ValueVisitor<'_> {
    type Value = JsonValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<JsonValue, E> {
        Ok(JsonValue::Null)
    }

    fn visit_bool<E: de::Error>(self, flag: bool) -> Result<JsonValue, E> {
        Ok(JsonValue::Bool(flag))
    }

    fn visit_i64<E: de::Error>(self, number: i64) -> Result<JsonValue, E> {
        Ok(JsonValue::Number(JsonNumber::Int(i128::from(number))))
    }

    fn visit_u64<E: de::Error>(self, number: u64) -> Result<JsonValue, E> {
        Ok(JsonValue::Number(JsonNumber::Int(i128::from(number))))
    }

    fn visit_f64<E: de::Error>(self, number: f64) -> Result<JsonValue, E> {
        Ok(JsonValue::Number(JsonNumber::Float(number)))
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<JsonValue, E> {
        Ok(JsonValue::Text(text.to_owned()))
    }

    fn visit_string<E: de::Error>(self, text: String) -> Result<JsonValue, E> {
        Ok(JsonValue::Text(text))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut items: A) -> Result<JsonValue, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = items.next_element_seed(ValueSeed(self.0))? {
            values.push(value);
        }
        Ok(JsonValue::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut entries: A) -> Result<JsonValue, A::Error> {
        let mut seen = BTreeSet::new();
        let mut pairs = Vec::new();
        while let Some(key) = entries.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(fail(
                    self.0,
                    JsonErrorKind::DuplicateKey(QuotedKey::from_untrusted(&key)),
                ));
            }
            let value = entries.next_value_seed(ValueSeed(self.0))?;
            pairs.push((key, value));
        }
        JsonObject::from_entries(pairs)
            .map(JsonValue::Object)
            .map_err(|duplicate| {
                fail(
                    self.0,
                    JsonErrorKind::DuplicateKey(QuotedKey::from_untrusted(duplicate.key())),
                )
            })
    }
}
