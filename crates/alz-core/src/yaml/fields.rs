//! The helpers every schema reads the tree with (decision-28, spec §7.13 "Types", §10.1 "Schema
//! errors"). An error names the field by its key path and what is wrong with it, and never holds the
//! field's value: the only input text it can hold is a quoted key (an unknown field).

use alz_adapter::TargetId;

use crate::diagnostic::{Message, MessagePart, Position};
use crate::untrusted_text::QuotedKey;
use crate::yaml::tree::{Mapping, Node, Value};

/// One step of a key path: a field the spec names, or the n-th item of a list, counted from 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// A field name that the spec defines.
    Field(&'static str),
    /// A target id used as a key (§5.2 **Target ids**), such as a section of `settings.yaml`.
    Target(TargetId),
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

    /// The path of the section of the target `id` in the mapping at this path.
    #[must_use]
    pub fn target(&self, id: &TargetId) -> Self {
        self.with(Step::Target(id.clone()))
    }

    /// The path of the n-th item of the list at this path.
    #[must_use]
    pub fn item(&self, index: usize) -> Self {
        self.with(Step::Item(index))
    }

    fn with(&self, step: Step) -> Self {
        Self(self.0.iter().cloned().chain([step]).collect())
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
                Step::Target(id) if index == 0 => message.then(MessagePart::printable(id)),
                Step::Target(id) => message.then(".").then(MessagePart::printable(id)),
                Step::Item(item) => message.then("[").then(*item).then("]"),
            })
    }
}

/// What is wrong with a field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldProblem {
    /// A number where text is expected: quote the value to keep it as text (§5.1, §5.2).
    NumberNotText,
    /// Anything but text or a number where text is expected: null, a boolean, a list or a
    /// mapping. Quoting would not make it what the field wants, so no hint is given.
    NotText,
    /// A value that is not a whole number.
    NotWholeNumber,
    /// A float with no fractional part that is too large to be read as a whole number, such as
    /// `1e300`.
    WholeNumberTooLarge,
    /// A value that is not `true` or `false`.
    NotBoolean,
    /// A value that is not a list.
    NotList,
    /// A value that is not a mapping.
    NotMapping,
    /// A key the schema does not know.
    UnknownField(QuotedKey),
    /// A key that is not text, named by its kind. It is no text from the file, so it is printed
    /// as the kind, unquoted: `an unknown key that is a number`.
    UnknownKeyOfKind(&'static str),
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
            FieldProblem::NumberNotText => self
                .path
                .message()
                .then(" must be text; put the value in quotes to keep it as text"),
            FieldProblem::NotText => self.path.message().then(" must be text"),
            FieldProblem::NotWholeNumber => self.path.message().then(" must be a whole number"),
            FieldProblem::WholeNumberTooLarge => self
                .path
                .message()
                .then(" is a whole number that is too large"),
            FieldProblem::NotBoolean => self.path.message().then(" must be true or false"),
            FieldProblem::NotList => self.path.message().then(" must be a list"),
            FieldProblem::NotMapping => self.path.message().then(" must be a mapping"),
            FieldProblem::UnknownField(key) => {
                self.in_section(Message::new("unknown field ").then(key.clone()))
            }
            FieldProblem::UnknownKeyOfKind(kind) => {
                self.in_section(Message::new("an unknown key that is ").then(*kind))
            }
        }
    }
}

impl FieldError {
    /// `message`, followed by ` in <key path>` when the section is not the document's root.
    fn in_section(&self, message: Message) -> Message {
        if self.path.0.is_empty() {
            message
        } else {
            self.path.extend(message.then(" in "))
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

    /// Reads a mapping that is not a node of its own, such as the frontmatter of a Markdown file,
    /// as the section at `path`. `at` is where the mapping starts, for a field it lacks.
    #[must_use]
    pub const fn of_mapping(mapping: &'a Mapping, path: KeyPath, at: Position) -> Self {
        Self { mapping, path, at }
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

    /// The keys that are text, in document order. A key that is not text is left out; the
    /// reader of the section reports it through [`Section::unknown_fields`].
    pub fn text_keys(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.mapping
            .entries()
            .filter_map(|(key, _)| match key.value() {
                Value::Text(name) => Some(name.as_str()),
                Value::Null
                | Value::Bool(_)
                | Value::Int(_)
                | Value::Float(_)
                | Value::Seq(_)
                | Value::Map(_) => None,
            })
    }

    /// The error for every key that is not one of `known`, each as a quoted key, in document
    /// order. A key that is not text counts as unknown.
    #[must_use]
    pub fn unknown_fields(&self, known: &[&str]) -> Vec<FieldError> {
        self.mapping
            .entries()
            .filter_map(|(key, _)| match key.value() {
                Value::Text(name) if known.contains(&name.as_str()) => None,
                Value::Text(name) => Some(self.unknown(
                    FieldProblem::UnknownField(QuotedKey::from_untrusted(name)),
                    key.at(),
                )),
                Value::Null
                | Value::Bool(_)
                | Value::Int(_)
                | Value::Float(_)
                | Value::Seq(_)
                | Value::Map(_) => Some(self.unknown(
                    FieldProblem::UnknownKeyOfKind(key.value().kind_name()),
                    key.at(),
                )),
            })
            .collect()
    }

    fn unknown(&self, problem: FieldProblem, at: Position) -> FieldError {
        FieldError {
            path: self.path.clone(),
            problem,
            at,
        }
    }

    /// The field as text, `None` when it is absent. A number is an error that suggests quoting, so `ref: 1.10` is never read as `"1.1"`.
    ///
    /// # Errors
    ///
    /// [`FieldProblem::NumberNotText`] or [`FieldProblem::NotText`].
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
    /// [`FieldProblem::NotWholeNumber`], or [`FieldProblem::WholeNumberTooLarge`] for a float with
    /// no fractional part that does not fit.
    pub fn whole_number(&self, field: &'static str) -> Result<Option<i128>, FieldError> {
        self.node(field)
            .map(|node| {
                whole_number_of(node.value()).ok_or_else(|| FieldError {
                    path: self.path.field(field),
                    problem: match node.value() {
                        Value::Float(number) if number.is_finite() && number.fract() == 0.0 => {
                            FieldProblem::WholeNumberTooLarge
                        }
                        Value::Float(_)
                        | Value::Null
                        | Value::Bool(_)
                        | Value::Int(_)
                        | Value::Text(_)
                        | Value::Seq(_)
                        | Value::Map(_) => FieldProblem::NotWholeNumber,
                    },
                    at: node.at(),
                })
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
    /// [`FieldProblem::NumberNotText`] or [`FieldProblem::NotText`].
    pub fn text_of(node: &'a Node, path: KeyPath) -> Result<&'a str, FieldError> {
        Self::as_text(node, path)
    }

    fn as_text(node: &Node, path: KeyPath) -> Result<&str, FieldError> {
        let refused = |problem| FieldError {
            path,
            problem,
            at: node.at(),
        };
        match node.value() {
            Value::Text(text) => Ok(text),
            Value::Int(_) | Value::Float(_) => Err(refused(FieldProblem::NumberNotText)),
            Value::Null | Value::Bool(_) | Value::Seq(_) | Value::Map(_) => {
                Err(refused(FieldProblem::NotText))
            }
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
