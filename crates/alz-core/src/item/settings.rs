//! The settings of a layer (spec §7.9): `settings.yaml`, namespaced by target.
//!
//! The top level takes only the known targets, `claude` and `codex`. Any other key, whatever it
//! is, is a `settings` error, and none of the file's settings are merged: a file is read into an
//! ordered map from [`TargetId`] to [`SettingsTree`], so no key of the layer can name anything
//! but a target. Each target's value must be a mapping, whose keys must not duplicate a
//! first-class item (permissions, hooks, MCP, model), and which must not set a protected
//! variable. For Codex that also holds below the top level where Codex reads the same setting
//! again: `features.hooks` and `features.codex_hooks` at any depth, and `profile`
//! and `profiles`, which a project's Codex config does not read.
//!
//! For Claude every key and value is also checked against the table of typed keys
//! ([`CLAUDE_SETTINGS`]): a key that is not in it, and a value of the wrong type, are errors.
//!
//! A file with a problem gives no item, so a merged model never holds half a file.

use std::collections::BTreeMap;

use crate::diagnostic::{Code, Diagnostics, Where};
use crate::layer::ItemPath;
use crate::merge::Tree;
use crate::path::SafeRelPath;
use crate::schema::{SchemaError, unknown_fields};
use crate::stack::ResolvedLayer;
use crate::yaml::fields::{KeyPath, Section};
use crate::yaml::{self, Node, Value, YamlText};
use alz_adapter::TargetId;

use super::agent::tree_of;
use super::native_keys::{Step, refusal, refusal_error};
use super::native_types::{CLAUDE_SETTINGS, Typed};
use super::protected_variable::{codex_policies, env_keys, paths};
use super::{place, schema_diagnostics};

/// The settings of one target: a mapping of native keys to values, in key order, which the target
/// is given without validation beyond the checks of §7.9.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SettingsTree(BTreeMap<String, Tree>);

impl SettingsTree {
    /// The value of the native key `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Tree> {
        self.0.get(key)
    }

    /// The entries in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Tree)> + '_ {
        self.0.iter().map(|(key, value)| (key.as_str(), value))
    }

    /// How many keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there is no key.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The same settings as a sorted tree.
    #[must_use]
    pub fn to_tree(&self) -> Tree {
        Tree::Map(self.0.clone())
    }

    /// The deep merge of §7.9 with `later` the more specific settings ([`Tree::deep_merge`] at
    /// the top level): associative over kind-consistent trees, with the empty settings as identity.
    #[must_use]
    pub fn deep_merge(self, later: Self) -> Self {
        Self(
            later
                .0
                .into_iter()
                .fold(self.0, |mut merged, (key, value)| {
                    let combined = match merged.remove(&key) {
                        Some(before) => before.deep_merge(value),
                        None => value,
                    };
                    merged.insert(key, combined);
                    merged
                }),
        )
    }
}

impl From<BTreeMap<String, Tree>> for SettingsTree {
    fn from(entries: BTreeMap<String, Tree>) -> Self {
        Self(entries)
    }
}

/// What one `settings.yaml` says: the settings of each target that it names.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Settings(BTreeMap<TargetId, SettingsTree>);

impl Settings {
    /// The settings of `target`, when the file names it.
    #[must_use]
    pub fn of(&self, target: &TargetId) -> Option<&SettingsTree> {
        self.0.get(target)
    }

    /// The targets the file names, in name order, with their settings.
    pub fn iter(&self) -> impl Iterator<Item = (&TargetId, &SettingsTree)> + '_ {
        self.0.iter()
    }

    /// The settings of each target in `entries`. For the tests of the merge.
    #[cfg(test)]
    pub(crate) fn new(entries: impl IntoIterator<Item = (TargetId, SettingsTree)>) -> Self {
        Self(entries.into_iter().collect())
    }
}

/// What one layer holds of §7.9: its `settings.yaml`, read, with the path it was read from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SettingsItems {
    file: Option<(SafeRelPath, Settings)>,
}

impl SettingsItems {
    /// The path of the file and what it says, `None` when the layer has no `settings.yaml`, the
    /// file is empty or it has a problem.
    #[must_use]
    pub fn into_file(self) -> Option<(SafeRelPath, Settings)> {
        self.file
    }

    /// Reads the settings of `layer`. Every problem of the file is reported; a file with a problem
    /// gives no item.
    #[must_use]
    pub fn read(layer: &ResolvedLayer) -> (Self, Diagnostics) {
        layer
            .contents()
            .iter()
            .find(|(path, _)| matches!(ItemPath::of(path), ItemPath::Settings))
            .map_or_else(
                || (Self::default(), Diagnostics::new()),
                |(path, bytes)| {
                    let at = place(layer.meta().name(), path);
                    match read_file(&at, bytes) {
                        Ok(file) => (
                            Self {
                                file: file.map(|settings| (path.clone(), settings)),
                            },
                            Diagnostics::new(),
                        ),
                        Err(found) => (Self::default(), found),
                    }
                },
            )
    }
}

/// The settings of the file, `None` for an empty file.
fn read_file(at: &Where, bytes: &[u8]) -> Result<Option<Settings>, Diagnostics> {
    let node = YamlText::from_bytes(bytes)
        .and_then(|text| yaml::parse(&text))
        .map_err(|problem| Diagnostics::new().with(problem.into_diagnostic(at.clone())))?;
    match node.value() {
        Value::Null => Ok(None),
        Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Text(_)
        | Value::Seq(_)
        | Value::Map(_) => {
            let section = Section::new(&node, KeyPath::root()).map_err(|problem| {
                schema_diagnostics(
                    Code::Settings,
                    at,
                    &SchemaError::NotAMapping(problem.position()).into(),
                )
            })?;
            // Any target id names a section; a key that is not one is unknown (§7.9).
            let named: Vec<(&str, TargetId)> = section
                .text_keys()
                .filter_map(|key| TargetId::try_from(key).ok().map(|target| (key, target)))
                .collect();
            let known: Vec<&str> = named.iter().map(|(key, _)| *key).collect();
            let unknown = unknown_fields(&section, &known)
                .err()
                .map_or_else(Diagnostics::new, |errors| {
                    schema_diagnostics(Code::Settings, at, &errors)
                });
            let (targets, problems) = named
                .into_iter()
                .filter_map(|(key, target)| section.written_node(key).map(|node| (target, node)))
                .fold(
                    (BTreeMap::new(), unknown),
                    |(targets, problems), (target, node)| match read_target(&target, node, at) {
                        Ok(tree) => (
                            targets.into_iter().chain([(target, tree)]).collect(),
                            problems,
                        ),
                        Err(found) => (targets, problems.merge(found)),
                    },
                );
            if problems.has_errors() {
                Err(problems)
            } else {
                Ok(Some(Settings(targets)))
            }
        }
    }
}

/// The settings of one target: a mapping, without the keys of a first-class item and without a
/// protected variable.
fn read_target(target: &TargetId, node: &Node, at: &Where) -> Result<SettingsTree, Diagnostics> {
    let section = Section::new(node, KeyPath::root().target(target)).map_err(|problem| {
        schema_diagnostics(Code::Settings, at, &SchemaError::from(problem).into())
    })?;
    let entries = match tree_of(node) {
        Tree::Map(entries) => entries,
        Tree::Null
        | Tree::Bool(_)
        | Tree::Int(_)
        | Tree::Float(_)
        | Tree::Text(_)
        | Tree::List(_) => {
            return Err(schema_diagnostics(
                Code::Settings,
                at,
                &SchemaError::NotAMapping(section.at()).into(),
            ));
        }
    };
    // A section of a target with no checks of its own is read as a mapping and nothing more.
    let found = refused_keys(target, node, at).merge(match target.as_str() {
        "claude" => typed_keys(node, at).merge(env_keys(entries.get("env"), paths::CLAUDE_ENV, at)),
        "codex" => codex_policies(&entries, Code::Settings, at),
        _ => Diagnostics::new(),
    });
    if found.has_errors() {
        Err(found)
    } else {
        Ok(SettingsTree(entries))
    }
}

/// An error for each key of Claude's settings that is not in the table of typed keys, and for each
/// value of the wrong type (§7.9). A key that the table of native keys refuses is reported by
/// [`refused_keys`], so it is skipped here.
fn typed_keys(node: &Node, at: &Where) -> Diagnostics {
    let reported = |steps: &[Step<'_>]| refusal(crate::target::claude(), steps).is_some();
    Typed::new(
        "claude",
        Code::Settings,
        at,
        " is not a Claude Code settings key that alz accepts (docs/target-facts.md section 14)",
        &reported,
    )
    .check(CLAUDE_SETTINGS, node)
}

/// An error for each key of the settings of `target`, at any depth, that the table of native keys
/// refuses: a key that duplicates a first-class item or is reserved (§7.9). The error prints the
/// key path of the table, never text of the layer.
fn refused_keys(target: &TargetId, node: &Node, at: &Where) -> Diagnostics {
    walk(target, node, &[], at)
}

fn walk(target: &TargetId, node: &Node, steps: &[Step<'_>], at: &Where) -> Diagnostics {
    match node.value() {
        Value::Map(mapping) => mapping
            .entries()
            .map(|(key, value)| {
                let step = match key.value() {
                    Value::Text(name) => Step::Key(name.as_str()),
                    Value::Null
                    | Value::Bool(_)
                    | Value::Int(_)
                    | Value::Float(_)
                    | Value::Seq(_)
                    | Value::Map(_) => Step::Unnamed,
                };
                let below = [steps, &[step]].concat();
                refusal(target, &below)
                    .and_then(|row| refusal_error(row, &at.clone().at(key.at())))
                    .into_iter()
                    .collect::<Diagnostics>()
                    .merge(walk(target, value, &below, at))
            })
            .fold(Diagnostics::new(), Diagnostics::merge),
        Value::Seq(items) => items
            .iter()
            .map(|item| walk(target, item, &[steps, &[Step::Unnamed]].concat(), at))
            .fold(Diagnostics::new(), Diagnostics::merge),
        Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Text(_) => {
            Diagnostics::new()
        }
    }
}

#[cfg(test)]
mod tests;
