//! The helpers every schema reads the tree with (decision-28, spec §7.13 "Types", §10.1 "Schema
//! errors"). An error names the field by its key path and what is wrong with it, and never holds the
//! field's value: the only input text it can hold is a quoted key (an unknown field).

use crate::diagnostic::{Message, Position};
use crate::untrusted_text::QuotedKey;
use crate::yaml::tree::{Mapping, Node, Value};

/// One step of a key path: a field the spec names, or the n-th item of a list, counted from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// A field name that the spec defines.
    Field(&'static str),
    /// An item of a list.
    Item(usize),
}

/// A key path such as `account.claude` or `stack[1].ref`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyPath(Vec<Step>);

impl KeyPath {
    /// The path of the document itself, which is empty.
    #[must_use]
    pub const fn root() -> Self {
        Self(Vec::new())
    }

    /// The path of a field of the mapping at this path.
    #[must_use]
    pub fn field(&self, name: &'static str) -> Self {
        self.with(Step::Field(name))
    }

    /// The path of the n-th item of the list at this path.
    #[must_use]
    pub fn item(&self, index: usize) -> Self {
        self.with(Step::Item(index))
    }

    fn with(&self, step: Step) -> Self {
        Self(self.0.iter().copied().chain([step]).collect())
    }

    /// The path as the start of a message: names joined with `.`, items as `[n]`.
    #[must_use]
    pub fn message(&self) -> Message {
        self.extend(Message::new(""))
    }

    /// `message` followed by the path.
    #[must_use]
    pub fn extend(&self, message: Message) -> Message {
        self.0
            .iter()
            .enumerate()
            .fold(message, |message, (index, step)| match step {
                Step::Field(name) if index == 0 => message.then(*name),
                Step::Field(name) => message.then(".").then(*name),
                Step::Item(item) => message.then("[").then(*item).then("]"),
            })
    }
}

/// What is wrong with a field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldProblem {
    /// A number or a boolean where text is expected: quote the value to keep it as text.
    NotText,
    /// A value that is not a whole number.
    NotWholeNumber,
    /// A value that is not `true` or `false`.
    NotBoolean,
    /// A value that is not a list.
    NotList,
    /// A value that is not a mapping.
    NotMapping,
    /// A key the schema does not know.
    UnknownField(QuotedKey),
}

/// A field that does not fit its schema, with its key path and the position of its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    path: KeyPath,
    problem: FieldProblem,
    at: Position,
}

impl FieldError {
    /// The key path of the field.
    #[must_use]
    pub const fn path(&self) -> &KeyPath {
        &self.path
    }

    /// What is wrong.
    #[must_use]
    pub const fn problem(&self) -> &FieldProblem {
        &self.problem
    }

    /// Where the field's value is in the file.
    #[must_use]
    pub const fn position(&self) -> Position {
        self.at
    }

    /// The message: the key path, then what is wrong. It names no value.
    #[must_use]
    pub fn message(&self) -> Message {
        match &self.problem {
            FieldProblem::NotText => self
                .path
                .message()
                .then(" must be text; put the value in quotes to keep it as text"),
            FieldProblem::NotWholeNumber => self.path.message().then(" must be a whole number"),
            FieldProblem::NotBoolean => self.path.message().then(" must be true or false"),
            FieldProblem::NotList => self.path.message().then(" must be a list"),
            FieldProblem::NotMapping => self.path.message().then(" must be a mapping"),
            FieldProblem::UnknownField(key) => {
                let message = Message::new("unknown field ").then(key.clone());
                if self.path.0.is_empty() {
                    message
                } else {
                    self.path.extend(message.then(" in "))
                }
            }
        }
    }
}

/// A mapping being read as a schema section: the mapping and its key path.
#[derive(Debug, Clone)]
pub struct Section<'a> {
    mapping: &'a Mapping,
    path: KeyPath,
    at: Position,
}

impl<'a> Section<'a> {
    /// Reads `node` as the section at `path`.
    ///
    /// # Errors
    ///
    /// [`FieldProblem::NotMapping`] when the node is not a mapping.
    pub fn new(node: &'a Node, path: KeyPath) -> Result<Self, FieldError> {
        match node.value() {
            Value::Map(mapping) => Ok(Self {
                mapping,
                path,
                at: node.at(),
            }),
            Value::Null
            | Value::Bool(_)
            | Value::Int(_)
            | Value::Float(_)
            | Value::Text(_)
            | Value::Seq(_) => Err(FieldError {
                path,
                problem: FieldProblem::NotMapping,
                at: node.at(),
            }),
        }
    }

    /// Where the section's mapping starts in the file.
    #[must_use]
    pub const fn at(&self) -> Position {
        self.at
    }

    /// The key path of the section.
    #[must_use]
    pub const fn path(&self) -> &KeyPath {
        &self.path
    }

    /// The key path of a field of this section.
    #[must_use]
    pub fn field_path(&self, field: &'static str) -> KeyPath {
        self.path.field(field)
    }

    /// The node of the field, `None` when it is absent. A field whose value is null, written as
    /// nothing, `~` or `null`, counts as absent (§5.1, §5.2); one that must be present is then
    /// reported as missing. Every reader of a field goes through here.
    #[must_use]
    pub fn node(&self, field: &str) -> Option<&'a Node> {
        self.mapping
            .get(field)
            .filter(|node| !matches!(node.value(), Value::Null))
    }

    /// The node of the field as written, null included, `None` only when the key is absent. The
    /// `account` field uses it because its schema refuses null (§7.13); every other field
    /// goes through [`Section::node`].
    #[must_use]
    pub fn written_node(&self, field: &str) -> Option<&'a Node> {
        self.mapping.get(field)
    }

    /// Where the key `field` is written, `None` when it is absent.
    #[must_use]
    pub fn key_position(&self, field: &str) -> Option<Position> {
        self.mapping
            .entries()
            .find(|(key, _)| matches!(key.value(), Value::Text(text) if text == field))
            .map(|(key, _)| key.at())
    }

    /// The error for every key that is not one of `known`, each as a quoted key, in document
    /// order. A key that is not text counts as unknown.
    #[must_use]
    pub fn unknown_fields(&self, known: &[&str]) -> Vec<FieldError> {
        self.mapping
            .entries()
            .filter_map(|(key, _)| match key.value() {
                Value::Text(name) if known.contains(&name.as_str()) => None,
                Value::Text(name) => Some(self.unknown(name, key.at())),
                Value::Null
                | Value::Bool(_)
                | Value::Int(_)
                | Value::Float(_)
                | Value::Seq(_)
                | Value::Map(_) => Some(self.unknown(key.value().kind_name(), key.at())),
            })
            .collect()
    }

    fn unknown(&self, name: &str, at: Position) -> FieldError {
        FieldError {
            path: self.path.clone(),
            problem: FieldProblem::UnknownField(QuotedKey::from_untrusted(name)),
            at,
        }
    }

    /// The field as text, `None` when it is absent. A number or a boolean is an error that
    /// suggests quoting, so `ref: 1.10` is never read as `"1.1"`.
    ///
    /// # Errors
    ///
    /// [`FieldProblem::NotText`].
    pub fn text(&self, field: &'static str) -> Result<Option<&'a str>, FieldError> {
        self.node(field)
            .map(|node| Self::as_text(node, self.path.field(field)))
            .transpose()
    }

    /// The field as a whole number, `None` when it is absent. A float without a fractional part
    /// (`1e3`, `2.0`) counts as one (§5.1, §5.2).
    ///
    /// # Errors
    ///
    /// [`FieldProblem::NotWholeNumber`].
    pub fn whole_number(&self, field: &'static str) -> Result<Option<i128>, FieldError> {
        self.node(field)
            .map(|node| {
                whole_number_of(node.value()).ok_or_else(|| FieldError {
                    path: self.path.field(field),
                    problem: FieldProblem::NotWholeNumber,
                    at: node.at(),
                })
            })
            .transpose()
    }

    /// The field as a list of text, `None` when it is absent. Every item must be text.
    ///
    /// # Errors
    ///
    /// [`FieldProblem::NotList`], or [`FieldProblem::NotText`] for the first item that is not text,
    /// at the path `<field>[<n>]`.
    pub fn text_list(&self, field: &'static str) -> Result<Option<Vec<&'a str>>, FieldError> {
        let path = self.path.field(field);
        self.node(field)
            .map(|node| match node.value() {
                Value::Seq(items) => items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| Self::as_text(item, path.item(index)))
                    .collect(),
                Value::Null
                | Value::Bool(_)
                | Value::Int(_)
                | Value::Float(_)
                | Value::Text(_)
                | Value::Map(_) => Err(FieldError {
                    path: path.clone(),
                    problem: FieldProblem::NotList,
                    at: node.at(),
                }),
            })
            .transpose()
    }

    /// The field as `true` or `false`, `None` when it is absent.
    ///
    /// # Errors
    ///
    /// [`FieldProblem::NotBoolean`].
    pub fn boolean(&self, field: &'static str) -> Result<Option<bool>, FieldError> {
        self.node(field)
            .map(|node| match node.value() {
                Value::Bool(flag) => Ok(*flag),
                Value::Null
                | Value::Int(_)
                | Value::Float(_)
                | Value::Text(_)
                | Value::Seq(_)
                | Value::Map(_) => Err(FieldError {
                    path: self.path.field(field),
                    problem: FieldProblem::NotBoolean,
                    at: node.at(),
                }),
            })
            .transpose()
    }

    /// The items of the field as nodes with their key paths, `None` when the field is absent.
    ///
    /// # Errors
    ///
    /// [`FieldProblem::NotList`].
    pub fn items(
        &self,
        field: &'static str,
    ) -> Result<Option<Vec<(KeyPath, &'a Node)>>, FieldError> {
        let path = self.path.field(field);
        self.node(field)
            .map(|node| match node.value() {
                Value::Seq(items) => Ok(items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| (path.item(index), item))
                    .collect()),
                Value::Null
                | Value::Bool(_)
                | Value::Int(_)
                | Value::Float(_)
                | Value::Text(_)
                | Value::Map(_) => Err(FieldError {
                    path: path.clone(),
                    problem: FieldProblem::NotList,
                    at: node.at(),
                }),
            })
            .transpose()
    }

    /// `node` as text, at `path`.
    ///
    /// # Errors
    ///
    /// [`FieldProblem::NotText`].
    pub fn text_of(node: &'a Node, path: KeyPath) -> Result<&'a str, FieldError> {
        Self::as_text(node, path)
    }

    fn as_text(node: &Node, path: KeyPath) -> Result<&str, FieldError> {
        match node.value() {
            Value::Text(text) => Ok(text),
            Value::Null
            | Value::Bool(_)
            | Value::Int(_)
            | Value::Float(_)
            | Value::Seq(_)
            | Value::Map(_) => Err(FieldError {
                path,
                problem: FieldProblem::NotText,
                at: node.at(),
            }),
        }
    }
}

/// The whole number a value stands for, if any: an integer, or a finite float with no fractional
/// part. The float is turned into digits and read back, which is exact for every float below
/// 1e38 and refuses the rest, so no cast is needed.
fn whole_number_of(value: &Value) -> Option<i128> {
    match value {
        Value::Int(number) => Some(*number),
        Value::Float(number) if number.is_finite() && number.fract() == 0.0 => {
            format!("{number:.0}").parse().ok()
        }
        Value::Float(_)
        | Value::Null
        | Value::Bool(_)
        | Value::Text(_)
        | Value::Seq(_)
        | Value::Map(_) => None,
    }
}
