//! The native fields of an agent (spec §7.2): a `claude:` or `codex:` map of target fields the
//! neutral schema does not model, passed through to that target only, and deep-merged by overlays.

use std::collections::BTreeMap;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::merge::FloatBits;
use crate::schema::SchemaErrors;
use crate::target::TargetName;
use crate::untrusted_text::QuotedKey;
use crate::yaml::fields::Section;
use crate::yaml::{Mapping, Node, Value};

use super::super::schema_diagnostics;
use super::ordered_tree::{NativeMap, NativeValue};

/// The keys of a target's map that have a neutral field, and so are refused inside it (§7.2).
/// The spec names `tools`, `model` and `permissionMode` and leaves the rest of the list open; this
/// is the reference's list.
const fn neutral_keys(target: TargetName) -> &'static [&'static str] {
    match target {
        TargetName::Claude => &[
            "name",
            "description",
            "tools",
            "model",
            "permissionMode",
            "skills",
            "isolation",
            "memory",
        ],
        TargetName::Codex => &["name", "description", "model", "developer_instructions"],
    }
}

/// The native fields of an agent or an overlay: for each target that has some, a mapping. A
/// target with no `claude:` or `codex:` map has no entry.
///
/// Keys inside a mapping keep the order they first appeared in across the stack (TASK-66); see
/// [`NativeMap`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NativeFields(BTreeMap<TargetName, NativeMap>);

impl NativeFields {
    /// The fields of `target`, a mapping, when some layer wrote any.
    #[must_use]
    pub fn of(&self, target: TargetName) -> Option<&NativeMap> {
        self.0.get(&target)
    }

    /// The targets that have fields, in name order, with their mappings.
    pub fn iter(&self) -> impl Iterator<Item = (TargetName, &NativeMap)> + '_ {
        self.0.iter().map(|(target, tree)| (*target, tree))
    }

    /// Whether no target has any.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The fields after a more specific layer: each target's mapping is deep-merged in place
    /// (§7.9, [`NativeMap::deep_merge`]).
    #[must_use]
    pub fn merge(self, later: Self) -> Self {
        Self(
            later
                .0
                .into_iter()
                .fold(self.0, |mut merged, (target, tree)| {
                    let combined = match merged.remove(&target) {
                        Some(before) => before.deep_merge(tree),
                        None => tree,
                    };
                    merged.insert(target, combined);
                    merged
                }),
        )
    }

    /// Reads the `claude:` and `codex:` maps of an agent file. Every problem is reported: a value
    /// that is not a mapping, and each key that has a neutral field, at the key.
    pub(super) fn read(section: &Section<'_>, at: &Where) -> Result<Self, Diagnostics> {
        let (fields, problems) = TargetName::ALL.into_iter().fold(
            (BTreeMap::new(), Diagnostics::new()),
            |(fields, problems), target| match read_target(section, target, at) {
                Ok(Some(tree)) => (
                    fields.into_iter().chain([(target, tree)]).collect(),
                    problems,
                ),
                Ok(None) => (fields, problems),
                Err(found) => (fields, problems.merge(found)),
            },
        );
        if problems.has_errors() {
            Err(problems)
        } else {
            Ok(Self(fields))
        }
    }
}

fn read_target(
    section: &Section<'_>,
    target: TargetName,
    at: &Where,
) -> Result<Option<NativeMap>, Diagnostics> {
    let path = section.field_path(target.as_str());
    section
        .node(target.as_str())
        .map(|node| {
            Section::new(node, path)
                .map_err(|error| schema_diagnostics(Code::Agent, at, &SchemaErrors::from(error)))?;
            let refused: Diagnostics = mapping_of(node)
                .into_iter()
                .flat_map(Mapping::entries)
                .filter_map(|(key, _)| {
                    neutral_key(target, key).map(|name| refusal(target, name, at, key))
                })
                .collect();
            if refused.has_errors() {
                Err(refused)
            } else {
                Ok(map_of(node))
            }
        })
        .transpose()
}

fn mapping_of(node: &Node) -> Option<&Mapping> {
    if let Value::Map(mapping) = node.value() {
        Some(mapping)
    } else {
        None
    }
}

/// The text of `key` when it is a key that has a neutral field.
fn neutral_key(target: TargetName, key: &Node) -> Option<&str> {
    if let Value::Text(name) = key.value() {
        neutral_keys(target)
            .contains(&name.as_str())
            .then_some(name.as_str())
    } else {
        None
    }
}

fn refusal(target: TargetName, key: &str, at: &Where, node: &Node) -> Diagnostic {
    Diagnostic::error(
        Code::Agent,
        Some(at.clone().at(node.at())),
        Message::new(QuotedKey::from_untrusted(&format!("{target}.{key}")))
            .then(" has a neutral field; set it outside the target map so every target gets it"),
    )
}

/// The mapping of a YAML node that is a mapping, in document order; empty for any other node,
/// which [`Section::new`] refused before.
fn map_of(node: &Node) -> NativeMap {
    mapping_of(node).map_or_else(NativeMap::default, map_entries)
}

fn map_entries(mapping: &Mapping) -> NativeMap {
    NativeMap::from_entries(
        mapping
            .entries()
            .map(|(key, value)| (key_text(key.value()), value_of(value))),
    )
}

/// The value of a YAML node. A mapping key that is not text is written as YAML writes it.
fn value_of(node: &Node) -> NativeValue {
    match node.value() {
        Value::Null => NativeValue::Null,
        Value::Bool(flag) => NativeValue::Bool(*flag),
        Value::Int(number) => NativeValue::Int(*number),
        Value::Float(number) => NativeValue::Float(FloatBits::new(*number)),
        Value::Text(text) => NativeValue::Text(text.clone()),
        Value::Seq(items) => NativeValue::List(items.iter().map(value_of).collect()),
        Value::Map(mapping) => NativeValue::Map(map_entries(mapping)),
    }
}

fn key_text(key: &Value) -> String {
    match key {
        Value::Null => "null".to_owned(),
        Value::Bool(flag) => flag.to_string(),
        Value::Int(number) => number.to_string(),
        Value::Float(number) => number.to_string(),
        Value::Text(text) => text.clone(),
        // The YAML reader refuses a key that is a list or a mapping.
        Value::Seq(_) | Value::Map(_) => String::new(),
    }
}
