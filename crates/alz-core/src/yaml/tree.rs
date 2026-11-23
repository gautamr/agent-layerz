//! alz's own YAML tree (decision-28). A parsed document is a [`Node`]: a [`Value`] and the
//! [`Position`] it was read at. The tree is read from the parser through `deserialize_any`, so a
//! quoted `"true"` stays text, and domain types are made from it in alz's own code.

use std::collections::BTreeSet;

use crate::diagnostic::Position;
use crate::untrusted_text::QuotedKey;
use crate::yaml::error::{YamlError, YamlErrorKind};

/// One value of a document and where it was read. A node that an alias repeats has the position
/// of the alias.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    value: Value,
    at: Position,
}

impl Node {
    /// A node.
    #[must_use]
    pub(crate) const fn new(value: Value, at: Position) -> Self {
        Self { value, at }
    }

    /// The value.
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.value
    }

    /// Where the value was read.
    #[must_use]
    pub const fn at(&self) -> Position {
        self.at
    }

    /// The node's value and position, for a caller that takes the value apart.
    #[must_use]
    pub fn into_parts(self) -> (Value, Position) {
        (self.value, self.at)
    }
}

/// What a YAML node holds, by the YAML 1.2 core schema: `true` and `false` (and their capitalised
/// forms) are booleans, `null` and `~` and an empty value are null, numbers are integers or
/// floats, and everything else is text. Quoted scalars are always text.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `null`, `~` or no value.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An integer, exact up to the range of `i128`.
    Int(i128),
    /// A float, which may be infinite or not a number (`.inf`, `.nan`).
    Float(f64),
    /// Text.
    Text(String),
    /// A sequence.
    Seq(Vec<Node>),
    /// A mapping.
    Map(Mapping),
}

impl Value {
    /// The word for the kind of value, for a message that says what was found.
    #[must_use]
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "a boolean",
            Self::Int(_) | Self::Float(_) => "a number",
            Self::Text(_) => "text",
            Self::Seq(_) => "a list",
            Self::Map(_) => "a mapping",
        }
    }
}

/// The identity of a scalar key. Two keys are equal when their kind and value are: `1` and `"1"`
/// differ, and a float is equal to another by its bits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum KeyIdentity {
    Null,
    Bool(bool),
    Int(i128),
    Float(u64),
    Text(String),
}

impl KeyIdentity {
    fn of(value: &Value) -> Option<Self> {
        match value {
            Value::Null => Some(Self::Null),
            Value::Bool(flag) => Some(Self::Bool(*flag)),
            Value::Int(number) => Some(Self::Int(*number)),
            Value::Float(number) => Some(Self::Float(number.to_bits())),
            Value::Text(text) => Some(Self::Text(text.clone())),
            Value::Seq(_) | Value::Map(_) => None,
        }
    }

    /// The key as text for a diagnostic, which strips and cuts it.
    fn shown(&self) -> String {
        match self {
            Self::Null => "null".to_owned(),
            Self::Bool(flag) => flag.to_string(),
            Self::Int(number) => number.to_string(),
            Self::Float(bits) => f64::from_bits(*bits).to_string(),
            Self::Text(text) => text.clone(),
        }
    }
}

/// A mapping: entries in document order, with unique scalar keys. Which order a later phase
/// writes them in is decided there (TASK-66).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mapping(Vec<(Node, Node)>);

impl Mapping {
    /// The mapping of `entries`, kept in the order given.
    ///
    /// # Errors
    ///
    /// [`YamlErrorKind::NonScalarKey`] for a key that is a list or a mapping, and
    /// [`YamlErrorKind::DuplicateKey`], with the key quoted, for a key that comes twice. Both are
    /// placed at the second key.
    pub(crate) fn from_entries(entries: Vec<(Node, Node)>) -> Result<Self, YamlError> {
        entries
            .iter()
            .try_fold(BTreeSet::new(), |seen, (key, _)| {
                let identity = KeyIdentity::of(key.value())
                    .ok_or_else(|| YamlError::new(YamlErrorKind::NonScalarKey, Some(key.at())))?;
                let shown = identity.shown();
                let mut seen = seen;
                if seen.insert(identity) {
                    Ok(seen)
                } else {
                    Err(YamlError::new(
                        YamlErrorKind::DuplicateKey(Some(QuotedKey::from_untrusted(&shown))),
                        Some(key.at()),
                    ))
                }
            })
            .map(|_| Self(entries))
    }

    /// The entries in document order.
    pub fn entries(&self) -> impl Iterator<Item = (&Node, &Node)> + '_ {
        self.0.iter().map(|(key, value)| (key, value))
    }

    /// The value of the entry whose key is the text `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Node> {
        self.entries()
            .find(|(candidate, _)| matches!(candidate.value(), Value::Text(text) if text == key))
            .map(|(_, value)| value)
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
