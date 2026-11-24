//! The model routing of a layer (spec §7.8): `models.yaml`, with `aliases`, `roles` and `default`.
//!
//! An alias is a logical name that maps each target to a concrete model id, so the concrete ids
//! live only here. The map of an alias is keyed by [`TargetName`]: a key that is not a target is a
//! `models` error naming the file, whatever the key. A role names the alias it routes to, and
//! `default` names the alias of the session. Whether those aliases exist is known only after the
//! merge, because a more general layer may define them.
//!
//! A file with a problem gives no item, so a merged model never holds half a file.

use std::collections::BTreeMap;
use std::fmt;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::layer::ItemPath;
use crate::merge::Replace;
use crate::path::SafeRelPath;
use crate::schema::{SchemaError, unknown_fields};
use crate::stack::ResolvedLayer;
use crate::target::TargetName;
use crate::untrusted_text::QuotedKey;
use crate::yaml::fields::{KeyPath, Section};
use crate::yaml::{self, Mapping, Node, Value, YamlText};

use super::{Role, place, schema_diagnostics};

/// The fields of `models.yaml`.
const FIELDS: [&str; 3] = ["aliases", "roles", "default"];

/// A name that is blank. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlankModelName;

macro_rules! model_name {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// The text as written.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<&str> for $name {
            type Error = BlankModelName;

            fn try_from(text: &str) -> Result<Self, Self::Error> {
                if text.trim().is_empty() {
                    Err(BlankModelName)
                } else {
                    Ok(Self(text.to_owned()))
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

model_name! {
    /// The logical name of a model, such as `strong`: what an agent's `model`, a role and
    /// `default` name (§7.8). Text with a character that is not white space.
    ModelAlias
}

model_name! {
    /// A concrete model id of a target, such as `opus`: text with a character that is not white
    /// space, passed to the target as written.
    ConcreteModel
}

/// What one `models.yaml` says.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Models {
    aliases: BTreeMap<ModelAlias, Replace<TargetName, ConcreteModel>>,
    alias_order: Vec<ModelAlias>,
    roles: BTreeMap<Role, ModelAlias>,
    default: Option<ModelAlias>,
}

impl Models {
    /// The aliases, each with the concrete model of each target it names.
    #[must_use]
    pub const fn aliases(&self) -> &BTreeMap<ModelAlias, Replace<TargetName, ConcreteModel>> {
        &self.aliases
    }

    /// The aliases in the order the file writes them.
    #[must_use]
    pub fn alias_order(&self) -> &[ModelAlias] {
        &self.alias_order
    }

    /// The alias each role routes to.
    #[must_use]
    pub const fn roles(&self) -> &BTreeMap<Role, ModelAlias> {
        &self.roles
    }

    /// The alias of the session, when the file sets one.
    #[must_use]
    pub const fn default(&self) -> Option<&ModelAlias> {
        self.default.as_ref()
    }
}

/// What one layer holds of §7.8: its `models.yaml`, read, with the path it was read from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModelItems {
    file: Option<(SafeRelPath, Models)>,
}

impl ModelItems {
    /// The path of the file and what it says, `None` when the layer has no `models.yaml`, the file
    /// is empty or it has a problem.
    #[must_use]
    pub fn into_file(self) -> Option<(SafeRelPath, Models)> {
        self.file
    }

    /// Reads the models of `layer`. Every problem of the file is reported; a file with a problem
    /// gives no item.
    #[must_use]
    pub fn read(layer: &ResolvedLayer) -> (Self, Diagnostics) {
        layer
            .contents()
            .iter()
            .find(|(path, _)| matches!(ItemPath::of(path), ItemPath::Models))
            .map_or_else(
                || (Self::default(), Diagnostics::new()),
                |(path, bytes)| {
                    let at = place(layer.meta().name(), path);
                    match read_file(&at, bytes) {
                        Ok(file) => (
                            Self {
                                file: file.map(|models| (path.clone(), models)),
                            },
                            Diagnostics::new(),
                        ),
                        Err(found) => (Self::default(), found),
                    }
                },
            )
    }
}

fn problem(at: &Where, node: &Node, message: Message) -> Diagnostics {
    Diagnostics::new().with(Diagnostic::error(
        Code::Models,
        Some(at.clone().at(node.at())),
        message,
    ))
}

/// The models of the file, `None` for an empty file.
fn read_file(at: &Where, bytes: &[u8]) -> Result<Option<Models>, Diagnostics> {
    let node = YamlText::from_bytes(bytes)
        .and_then(|text| yaml::parse(&text))
        .map_err(|found| Diagnostics::new().with(found.into_diagnostic(at.clone())))?;
    match node.value() {
        Value::Null => Ok(None),
        Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Text(_)
        | Value::Seq(_)
        | Value::Map(_) => {
            let section = Section::new(&node, KeyPath::root()).map_err(|found| {
                schema_diagnostics(
                    Code::Models,
                    at,
                    &SchemaError::NotAMapping(found.position()).into(),
                )
            })?;
            let unknown = unknown_fields(&section, &FIELDS)
                .err()
                .map_or_else(Diagnostics::new, |errors| {
                    schema_diagnostics(Code::Models, at, &errors)
                });
            let aliases = section.node("aliases").map_or_else(
                || Ok((BTreeMap::new(), Vec::new())),
                |node| read_aliases(node, at),
            );
            let roles = section
                .node("roles")
                .map_or_else(|| Ok(BTreeMap::new()), |node| read_roles(node, at));
            let default = section
                .node("default")
                .map_or(Ok(None), |node| read_default(node, at).map(Some));
            match (aliases, roles, default) {
                (Ok((aliases, alias_order)), Ok(roles), Ok(default)) if !unknown.has_errors() => {
                    Ok(Some(Models {
                        aliases,
                        alias_order,
                        roles,
                        default,
                    }))
                }
                (aliases, roles, default) => Err([aliases.err(), roles.err(), default.err()]
                    .into_iter()
                    .flatten()
                    .fold(unknown, Diagnostics::merge)),
            }
        }
    }
}

/// The entries of a mapping `node` as `(key, key text, value)`, or the problem that it is not a
/// mapping.
fn entries<'a>(node: &'a Node, what: &'static str, at: &Where) -> Result<&'a Mapping, Diagnostics> {
    match node.value() {
        Value::Map(mapping) => Ok(mapping),
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Text(_)
        | Value::Seq(_) => Err(problem(at, node, Message::new(what))),
    }
}

/// A name of a layer file, read from the key `key` of a mapping, or the problem with the key.
fn named<'a, T: for<'b> TryFrom<&'b str>>(
    key: &'a Node,
    what: &'static str,
    at: &Where,
) -> Result<(T, &'a str), Diagnostics> {
    match key.value() {
        Value::Text(text) => T::try_from(text.as_str())
            .map(|name| (name, text.as_str()))
            .map_err(|_blank| problem(at, key, Message::new(what).then(" must not be empty"))),
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Seq(_)
        | Value::Map(_) => Err(problem(
            at,
            key,
            Message::new(what)
                .then(" must be text, not ")
                .then(key.value().kind_name()),
        )),
    }
}

fn read_aliases(node: &Node, at: &Where) -> Result<AliasesRead, Diagnostics> {
    let mapping = entries(node, "aliases must be a mapping", at)?;
    let (read, found) = mapping.entries().fold(
        (Vec::new(), Diagnostics::new()),
        |(read, found), (key, targets)| match named::<ModelAlias>(key, "an alias name", at)
            .and_then(|(alias, text)| read_targets(text, targets, at).map(|read| (alias, read)))
        {
            Ok(entry) => (read.into_iter().chain([entry]).collect(), found),
            Err(problems) => (read, found.merge(problems)),
        },
    );
    if found.has_errors() {
        Err(found)
    } else {
        let order = read.iter().map(|(alias, _)| alias.clone()).collect();
        Ok((read.into_iter().collect(), order))
    }
}

/// The aliases of a file by name, and the names in the order the file writes them.
type AliasesRead = (
    BTreeMap<ModelAlias, Replace<TargetName, ConcreteModel>>,
    Vec<ModelAlias>,
);

/// The concrete model of each target of the alias `alias`. A key that is not a target, or a value
/// that is not a model id, is a problem.
fn read_targets(
    alias: &str,
    node: &Node,
    at: &Where,
) -> Result<Replace<TargetName, ConcreteModel>, Diagnostics> {
    let mapping = entries(node, "an alias must map each target to a model id", at)?;
    let (targets, found) = mapping.entries().fold(
        (Vec::new(), Diagnostics::new()),
        |(targets, found), (key, value)| {
            let read = match key.value() {
                Value::Text(text) => TargetName::try_from(text.as_str()).map_err(|_not_a_target| {
                    problem(
                        at,
                        key,
                        Message::new("alias ")
                            .then(QuotedKey::from_untrusted(alias))
                            .then(" has a key that is not a target: ")
                            .then(QuotedKey::from_untrusted(text)),
                    )
                }),
                Value::Null
                | Value::Bool(_)
                | Value::Int(_)
                | Value::Float(_)
                | Value::Seq(_)
                | Value::Map(_) => Err(problem(
                    at,
                    key,
                    Message::new("alias ")
                        .then(QuotedKey::from_untrusted(alias))
                        .then(" has a key that is not a target, but ")
                        .then(key.value().kind_name()),
                )),
            }
            .and_then(|target| match value.value() {
                Value::Text(text) => ConcreteModel::try_from(text.as_str())
                    .map(|model| (target, model))
                    .map_err(|_blank| model_problem(alias, target, at, value)),
                Value::Null
                | Value::Bool(_)
                | Value::Int(_)
                | Value::Float(_)
                | Value::Seq(_)
                | Value::Map(_) => Err(model_problem(alias, target, at, value)),
            });
            match read {
                Ok(entry) => (targets.into_iter().chain([entry]).collect(), found),
                Err(problems) => (targets, found.merge(problems)),
            }
        },
    );
    if found.has_errors() {
        Err(found)
    } else {
        Ok(targets.into_iter().collect())
    }
}

fn model_problem(alias: &str, target: TargetName, at: &Where, value: &Node) -> Diagnostics {
    problem(
        at,
        value,
        Message::new("alias ")
            .then(QuotedKey::from_untrusted(alias))
            .then(": the model of ")
            .then(target.as_str())
            .then(" must be a model id, text that is not empty"),
    )
}

fn read_roles(node: &Node, at: &Where) -> Result<BTreeMap<Role, ModelAlias>, Diagnostics> {
    let mapping = entries(node, "roles must map each role to an alias", at)?;
    let (roles, found) = mapping.entries().fold(
        (BTreeMap::new(), Diagnostics::new()),
        |(roles, found), (key, value)| match named::<Role>(key, "a role name", at).and_then(
            |(role, _)| {
                read_alias(
                    value,
                    "a role must route to an alias, text that is not empty",
                    at,
                )
                .map(|alias| (role, alias))
            },
        ) {
            Ok(entry) => (roles.into_iter().chain([entry]).collect(), found),
            Err(problems) => (roles, found.merge(problems)),
        },
    );
    if found.has_errors() {
        Err(found)
    } else {
        Ok(roles)
    }
}

fn read_default(node: &Node, at: &Where) -> Result<ModelAlias, Diagnostics> {
    read_alias(node, "default must be an alias, text that is not empty", at)
}

fn read_alias(node: &Node, what: &'static str, at: &Where) -> Result<ModelAlias, Diagnostics> {
    match node.value() {
        Value::Text(text) => ModelAlias::try_from(text.as_str())
            .map_err(|_blank| problem(at, node, Message::new(what))),
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Seq(_)
        | Value::Map(_) => Err(problem(at, node, Message::new(what))),
    }
}

#[cfg(test)]
mod tests;
