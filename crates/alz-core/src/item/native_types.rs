//! The check of the types of the native keys alz accepts (spec §7.9, §7.2): the table that says
//! which keys of a target's settings and of an agent's native fields a layer may carry, and of
//! what type each value is. The rows belong to the target: its descriptor states them
//! (`alz_adapter::NativeVocabulary::settings_keys` and `agent_fields`) as [`Shape`]s, and the core
//! reads them through the registry. Every row of Claude's is recorded with its type in
//! `docs/target-facts.md` §14; the tests read that file and fail for a row it lacks.
//!
//! [`Typed::check`] reads a YAML node against a shape and reports each problem: a value of the
//! wrong type, a key that is not in the table. A message names the key path of the table
//! (`claude.sandbox.enabled`, `*` for the key of a map) and never prints a value; a key that is not
//! in the table is printed as a quoted key, so that the error says which one to remove. The checks
//! of the security table ([`super::native_keys`]) apply on top: a key that table refuses is reported
//! there, so [`Typed`] skips it.

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::untrusted_text::{QuotedKey, Stripped};
use crate::yaml::{Mapping, Node, Value};
use alz_adapter::{Ends, Field, Shape};

use super::native_keys::Step;

/// `message` followed by the whole number `number`, in digits.
fn signed(message: Message, number: i128) -> Message {
    let sign = if number < 0 {
        message.then("-")
    } else {
        message
    };
    sign.then(u64::try_from(number.unsigned_abs()).unwrap_or(u64::MAX))
}

/// Whether `name` is a name of an MCP server that Claude Code accepts in an allowlist entry.
fn is_server_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|letter| letter.is_ascii_alphanumeric() || letter == '-' || letter == '_')
}

/// Whether the shape is a list and `value` a list, or a mapping shape and `value` a mapping.
const fn is_container_for(shape: Shape, value: &Value) -> bool {
    match shape {
        Shape::List(_) | Shape::NonEmptyList(_) => matches!(value, Value::Seq(_)),
        Shape::Map(_) | Shape::Object(_) | Shape::Single(_) => matches!(value, Value::Map(_)),
        Shape::Bool
        | Shape::False
        | Shape::Text
        | Shape::Whole
        | Shape::WholeIn(_)
        | Shape::Number
        | Shape::Fraction(_)
        | Shape::McpServerName
        | Shape::Choice(_)
        | Shape::Either(_) => false,
    }
}

/// What the shape asks for, after `message`.
fn expectation(shape: Shape, message: Message) -> Message {
    match shape {
        Shape::Bool => message.then("a boolean"),
        Shape::False => message.then("the boolean false"),
        Shape::Text => message.then("text"),
        Shape::Whole => message.then("a whole number"),
        Shape::WholeIn(span) => match span.max() {
            None => signed(message.then("a whole number of at least "), span.min()),
            Some(max) => signed(
                signed(message.then("a whole number from "), span.min()).then(" to "),
                max,
            ),
        },
        Shape::Number => message.then("a finite number"),
        Shape::Fraction(Ends::AboveZero) => message.then("a number above 0 and at most 1"),
        Shape::Fraction(Ends::FromZero) => message.then("a number from 0 to 1"),
        Shape::McpServerName => message.then("a name of letters, digits, hyphens and underscores"),
        Shape::Choice(values) => {
            values
                .iter()
                .enumerate()
                .fold(message.then("one of "), |message, (index, value)| {
                    let separated = if index == 0 {
                        message
                    } else {
                        message.then(", ")
                    };
                    separated.then(*value)
                })
        }
        Shape::List(item) => expectation(*item, message.then("a list whose items are ")),
        Shape::NonEmptyList(item) => expectation(
            *item,
            message.then("a list of at least one item, whose items are "),
        ),
        Shape::Map(item) => expectation(*item, message.then("a mapping whose values are ")),
        Shape::Object(_) => message.then("a mapping"),
        Shape::Single(fields) => fields.iter().enumerate().fold(
            message.then("a mapping with exactly one of the keys "),
            |message, (index, field)| {
                let separated = if index == 0 {
                    message
                } else {
                    message.then(", ")
                };
                separated.then(field.name())
            },
        ),
        Shape::Either(options) => {
            options
                .iter()
                .enumerate()
                .fold(message, |message, (index, option)| {
                    let separated = if index == 0 {
                        message
                    } else {
                        message.then(", or ")
                    };
                    expectation(*option, separated)
                })
        }
    }
}

/// A check of YAML nodes against shapes, for one place that holds native keys.
#[derive(Clone, Copy)]
pub struct Typed<'a> {
    prefix: &'a str,
    code: Code,
    at: &'a Where,
    unknown: &'a str,
    reported: &'a dyn Fn(&[Step<'_>]) -> bool,
}

impl<'a> Typed<'a> {
    /// A check whose messages start the key path with `prefix` and carry `code`, located at `at`.
    /// `unknown` ends the message about a key that the table does not list. `reported` says which
    /// key paths another check reports already, so that this one skips them.
    #[must_use]
    pub const fn new(
        prefix: &'a str,
        code: Code,
        at: &'a Where,
        unknown: &'a str,
        reported: &'a dyn Fn(&[Step<'_>]) -> bool,
    ) -> Self {
        Self {
            prefix,
            code,
            at,
            unknown,
            reported,
        }
    }

    /// The problems of `node` against the mapping of `fields`.
    #[must_use]
    pub fn check(&self, fields: &'static [Field], node: &Node) -> Diagnostics {
        self.walk(Shape::Object(fields), node, &[], self.prefix)
    }

    fn error(&self, node: &Node, message: Message) -> Diagnostics {
        Diagnostics::new().with(Diagnostic::error(
            self.code,
            Some(self.at.clone().at(node.at())),
            message,
        ))
    }

    fn wrong(&self, shape: Shape, node: &Node, shown: &str) -> Diagnostics {
        self.error(
            node,
            expectation(
                shape,
                Message::new(QuotedKey::from_untrusted(shown)).then(" must be "),
            ),
        )
    }

    fn walk(&self, shape: Shape, node: &Node, steps: &[Step<'_>], shown: &str) -> Diagnostics {
        let value = node.value();
        let fits = |fitting: bool| {
            if fitting {
                Diagnostics::new()
            } else {
                self.wrong(shape, node, shown)
            }
        };
        match shape {
            Shape::Bool => fits(matches!(value, Value::Bool(_))),
            Shape::False => fits(matches!(value, Value::Bool(false))),
            Shape::Text => fits(matches!(value, Value::Text(_))),
            Shape::Whole => fits(matches!(value, Value::Int(_))),
            Shape::WholeIn(span) => {
                fits(matches!(value, Value::Int(number) if span.holds(*number)))
            }
            Shape::Number => fits(match value {
                Value::Int(_) => true,
                Value::Float(number) => number.is_finite(),
                Value::Null | Value::Bool(_) | Value::Text(_) | Value::Seq(_) | Value::Map(_) => {
                    false
                }
            }),
            Shape::Fraction(ends) => fits(match value {
                Value::Int(number) => {
                    i32::try_from(*number).is_ok_and(|whole| ends.holds(f64::from(whole)))
                }
                Value::Float(number) => ends.holds(*number),
                Value::Null | Value::Bool(_) | Value::Text(_) | Value::Seq(_) | Value::Map(_) => {
                    false
                }
            }),
            Shape::McpServerName => {
                fits(matches!(value, Value::Text(name) if is_server_name(name)))
            }
            Shape::Choice(values) => {
                fits(matches!(value, Value::Text(text) if values.contains(&text.as_str())))
            }
            Shape::List(item) => self.list(shape, *item, node, steps, shown),
            Shape::NonEmptyList(item) => {
                if matches!(value, Value::Seq(items) if items.is_empty()) {
                    self.wrong(shape, node, shown)
                } else {
                    self.list(shape, *item, node, steps, shown)
                }
            }
            Shape::Map(item) => self.map(shape, *item, node, steps, shown),
            Shape::Object(fields) => self.object(shape, fields, node, steps, shown),
            Shape::Single(fields) => self.single(shape, fields, node, steps, shown),
            Shape::Either(options) => self.either(shape, options, node, steps, shown),
        }
    }

    fn list(
        &self,
        shape: Shape,
        item: Shape,
        node: &Node,
        steps: &[Step<'_>],
        shown: &str,
    ) -> Diagnostics {
        if let Value::Seq(items) = node.value() {
            let below = [steps, &[Step::Unnamed]].concat();
            let shown_item = format!("{shown}[]");
            items
                .iter()
                .map(|entry| self.walk(item, entry, &below, &shown_item))
                .fold(Diagnostics::new(), Diagnostics::merge)
        } else {
            self.wrong(shape, node, shown)
        }
    }

    fn map(
        &self,
        shape: Shape,
        item: Shape,
        node: &Node,
        steps: &[Step<'_>],
        shown: &str,
    ) -> Diagnostics {
        if let Value::Map(mapping) = node.value() {
            self.entries(mapping, shown, |_, name, entry| {
                let below = [steps, &[Step::Key(name)]].concat();
                self.walk(item, entry, &below, &format!("{shown}.*"))
            })
        } else {
            self.wrong(shape, node, shown)
        }
    }

    fn object(
        &self,
        shape: Shape,
        fields: &[Field],
        node: &Node,
        steps: &[Step<'_>],
        shown: &str,
    ) -> Diagnostics {
        if let Value::Map(mapping) = node.value() {
            let missing = fields
                .iter()
                .filter(|field| field.is_required())
                .filter(|field| {
                    !mapping.entries().any(
                        |(key, _)| matches!(key.value(), Value::Text(name) if *name == field.name()),
                    )
                })
                .map(|field| {
                    self.error(
                        node,
                        Message::new(QuotedKey::from_untrusted(&format!(
                            "{shown}.{}",
                            field.name()
                        )))
                        .then(" is required"),
                    )
                })
                .fold(Diagnostics::new(), Diagnostics::merge);
            missing.merge(self.entries(mapping, shown, |key, name, entry| {
                let below = [steps, &[Step::Key(name)]].concat();
                if (self.reported)(&below) {
                    Diagnostics::new()
                } else if let Some(field) = fields.iter().find(|field| field.name() == name) {
                    self.walk(
                        field.shape(),
                        entry,
                        &below,
                        &format!("{shown}.{}", field.name()),
                    )
                } else {
                    self.error(
                        key,
                        Message::new(QuotedKey::from_untrusted(&format!("{shown}.{name}")))
                            .then(Stripped::from_untrusted(self.unknown)),
                    )
                }
            }))
        } else {
            self.wrong(shape, node, shown)
        }
    }

    fn single(
        &self,
        shape: Shape,
        fields: &[Field],
        node: &Node,
        steps: &[Step<'_>],
        shown: &str,
    ) -> Diagnostics {
        let entry = match node.value() {
            Value::Map(mapping) => match mapping.entries().collect::<Vec<_>>().as_slice() {
                [(key, entry)] => match key.value() {
                    Value::Text(name) => fields
                        .iter()
                        .find(|field| field.name() == name)
                        .map(|field| (field, *entry)),
                    Value::Null
                    | Value::Bool(_)
                    | Value::Int(_)
                    | Value::Float(_)
                    | Value::Seq(_)
                    | Value::Map(_) => None,
                },
                _ => None,
            },
            Value::Null
            | Value::Bool(_)
            | Value::Int(_)
            | Value::Float(_)
            | Value::Text(_)
            | Value::Seq(_) => None,
        };
        entry.map_or_else(
            || self.wrong(shape, node, shown),
            |(field, value)| {
                let below = [steps, &[Step::Key(field.name())]].concat();
                self.walk(
                    field.shape(),
                    value,
                    &below,
                    &format!("{shown}.{}", field.name()),
                )
            },
        )
    }

    fn either(
        &self,
        shape: Shape,
        options: &[Shape],
        node: &Node,
        steps: &[Step<'_>],
        shown: &str,
    ) -> Diagnostics {
        let tried: Vec<Diagnostics> = options
            .iter()
            .map(|option| self.walk(*option, node, steps, shown))
            .collect();
        if tried.iter().any(|found| !found.has_errors()) {
            return Diagnostics::new();
        }
        // A value that is a list or a mapping gets the problems of the one option of its kind,
        // when there is exactly one.
        let of_its_kind: Vec<usize> = options
            .iter()
            .enumerate()
            .filter(|(_, option)| is_container_for(**option, node.value()))
            .map(|(index, _)| index)
            .collect();
        match of_its_kind.as_slice() {
            [only] => tried.into_iter().nth(*only).unwrap_or_default(),
            _ => self.wrong(shape, node, shown),
        }
    }

    /// The problems of the entries of `mapping`, each read by `value` (the key, its text and the
    /// value), and an error for a key that is not text.
    fn entries(
        &self,
        mapping: &Mapping,
        shown: &str,
        value: impl Fn(&Node, &str, &Node) -> Diagnostics,
    ) -> Diagnostics {
        mapping
            .entries()
            .map(|(key, entry)| match key.value() {
                Value::Text(name) => value(key, name.as_str(), entry),
                Value::Null
                | Value::Bool(_)
                | Value::Int(_)
                | Value::Float(_)
                | Value::Seq(_)
                | Value::Map(_) => self.error(
                    key,
                    Message::new(QuotedKey::from_untrusted(shown))
                        .then(" has a key that is not text"),
                ),
            })
            .fold(Diagnostics::new(), Diagnostics::merge)
    }
}

impl std::fmt::Debug for Typed<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Typed")
            .field("prefix", &self.prefix)
            .field("code", &self.code)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
