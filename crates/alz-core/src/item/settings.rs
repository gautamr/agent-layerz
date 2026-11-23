//! The settings of a layer (spec §7.9): `settings.yaml`, namespaced by target.
//!
//! The top level takes only the known targets, `claude` and `codex`. Any other key, whatever it
//! is, is a `settings` error, and none of the file's settings are merged: a file is read into an
//! ordered map from [`TargetName`] to [`SettingsTree`], so no key of the layer can name anything
//! but a target. Each target's value must be a mapping, whose keys must not duplicate a
//! first-class item (permissions, hooks, MCP, model), and which must not set a protected
//! variable. For Codex that also holds below the top level where Codex reads the same setting
//! again: `features.hooks` and `features.codex_hooks` at any depth, `profile`, and
//! `approval_policy` and `sandbox_mode` under `profiles.*`.
//!
//! A file with a problem gives no item, so a merged model never holds half a file.

use std::collections::BTreeMap;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::layer::ItemPath;
use crate::merge::Tree;
use crate::path::SafeRelPath;
use crate::schema::{SchemaError, unknown_fields};
use crate::stack::ResolvedLayer;
use crate::target::TargetName;
use crate::untrusted_text::QuotedKey;
use crate::yaml::fields::{KeyPath, Section};
use crate::yaml::{self, Node, Value, YamlText};

use super::agent::tree_of;
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
pub struct Settings(BTreeMap<TargetName, SettingsTree>);

impl Settings {
    /// The settings of `target`, when the file names it.
    #[must_use]
    pub fn of(&self, target: TargetName) -> Option<&SettingsTree> {
        self.0.get(&target)
    }

    /// The targets the file names, in name order, with their settings.
    pub fn iter(&self) -> impl Iterator<Item = (TargetName, &SettingsTree)> + '_ {
        self.0.iter().map(|(target, tree)| (*target, tree))
    }

    /// The settings of each target in `entries`. For the tests of the merge.
    #[cfg(test)]
    pub(crate) fn new(entries: impl IntoIterator<Item = (TargetName, SettingsTree)>) -> Self {
        Self(entries.into_iter().collect())
    }
}

/// The keys of a target that duplicate a first-class item: each with the place that sets it (§7.9).
const fn reserved(target: TargetName) -> &'static [(&'static str, &'static str)] {
    match target {
        TargetName::Claude => &[
            ("permissions", "permissions.yaml"),
            ("hooks", "hooks/hooks.yaml"),
            ("model", "models.yaml (default)"),
            ("mcpServers", "mcp.yaml"),
            ("enabledMcpjsonServers", "mcp.enable in the repo manifest"),
            ("disabledMcpjsonServers", "mcp.enable in the repo manifest"),
            (
                "enableAllProjectMcpServers",
                "mcp.enable in the repo manifest",
            ),
            (
                "disableAllHooks",
                "hooks.yaml `disabled` (locked hooks cannot be switched off)",
            ),
        ],
        TargetName::Codex => &[
            ("model", "models.yaml"),
            ("mcp_servers", "mcp.yaml"),
            ("hooks", "hooks/hooks.yaml"),
            ("profile", "permissions.yaml"),
            ("approval_policy", "permissions.yaml"),
            ("sandbox_mode", "permissions.yaml"),
        ],
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
            let unknown = unknown_fields(&section, &TargetName::ALL.map(TargetName::as_str))
                .err()
                .map_or_else(Diagnostics::new, |errors| {
                    schema_diagnostics(Code::Settings, at, &errors)
                });
            let (targets, problems) = TargetName::ALL
                .into_iter()
                .filter_map(|target| {
                    section
                        .written_node(target.as_str())
                        .map(|node| (target, node))
                })
                .fold(
                    (BTreeMap::new(), unknown),
                    |(targets, problems), (target, node)| match read_target(target, node, at) {
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
fn read_target(target: TargetName, node: &Node, at: &Where) -> Result<SettingsTree, Diagnostics> {
    let section =
        Section::new(node, KeyPath::root().field(target.as_str())).map_err(|problem| {
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
    let found = refused_keys(target, node, at)
        .merge(nested_refusals(target, node, at))
        .merge(match target {
            TargetName::Claude => env_keys(entries.get("env"), paths::CLAUDE_ENV, at),
            TargetName::Codex => codex_policies(&entries, Code::Settings, at),
        });
    if found.has_errors() {
        Err(found)
    } else {
        Ok(SettingsTree(entries))
    }
}

/// An error for each key of `node`, a mapping, that duplicates a first-class item.
fn refused_keys(target: TargetName, node: &Node, at: &Where) -> Diagnostics {
    match node.value() {
        Value::Map(mapping) => mapping
            .entries()
            .filter_map(|(key, _)| match key.value() {
                Value::Text(name) => reserved(target)
                    .iter()
                    .find(|(reserved_key, _)| reserved_key == name)
                    .map(|(reserved_key, owner)| {
                        reserved_key_error(&format!("{target}.{reserved_key}"), owner, key, at)
                    }),
                Value::Null
                | Value::Bool(_)
                | Value::Int(_)
                | Value::Float(_)
                | Value::Seq(_)
                | Value::Map(_) => None,
            })
            .collect(),
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Text(_)
        | Value::Seq(_) => Diagnostics::new(),
    }
}

/// The error for the key `key`, which duplicates the first-class item set through `owner`. `path`
/// is a fixed key path, never text of the layer: no value and no profile name reaches the output.
fn reserved_key_error(path: &str, owner: &'static str, key: &Node, at: &Where) -> Diagnostic {
    Diagnostic::error(
        Code::Settings,
        Some(at.clone().at(key.at())),
        Message::new(QuotedKey::from_untrusted(path))
            .then(" is set through ")
            .then(owner),
    )
}

/// Where a mapping of Codex settings stands, for the keys Codex reads again below the top level.
#[derive(Debug, Clone, Copy)]
enum Level {
    /// The mapping under `codex`.
    Top,
    /// The mapping of `profiles`.
    Profiles,
    /// The mapping of one profile.
    Profile,
    /// A `features` mapping, with the fixed path that leads to it.
    Features(&'static str),
    /// Any other mapping or list.
    Other,
}

impl Level {
    /// The level of the value under `key`.
    const fn below(self, key: &str) -> Self {
        match (self, key.as_bytes()) {
            (Self::Top, b"features") => Self::Features("codex"),
            (Self::Profile, b"features") => Self::Features("codex.profiles.*"),
            (Self::Profiles, _) => Self::Profile,
            (Self::Top, b"profiles") => Self::Profiles,
            (Self::Features(_) | Self::Other, b"features") => Self::Features("codex.**"),
            (Self::Top | Self::Profile | Self::Features(_) | Self::Other, _) => Self::Other,
        }
    }

    /// The fixed key path and the owner of `key` when it duplicates a first-class item here.
    /// The top level is checked by [`reserved`].
    fn reserved(self, key: &str) -> Option<(String, &'static str)> {
        match (self, key) {
            (Self::Features(prefix), "hooks" | "codex_hooks") => {
                Some((format!("{prefix}.features.{key}"), "hooks/hooks.yaml"))
            }
            (Self::Profile, "approval_policy" | "sandbox_mode") => {
                Some((format!("codex.profiles.*.{key}"), "permissions.yaml"))
            }
            (Self::Top | Self::Profiles | Self::Profile | Self::Features(_) | Self::Other, _) => {
                None
            }
        }
    }
}

/// An error for each key below the top level of the settings of `target` that duplicates a
/// first-class item (§7.9). Only Codex has such keys.
fn nested_refusals(target: TargetName, node: &Node, at: &Where) -> Diagnostics {
    match target {
        TargetName::Claude => Diagnostics::new(),
        TargetName::Codex => walk(node, Level::Top, at),
    }
}

fn walk(node: &Node, level: Level, at: &Where) -> Diagnostics {
    match node.value() {
        Value::Map(mapping) => mapping
            .entries()
            .map(|(key, value)| {
                let name = match key.value() {
                    Value::Text(name) => Some(name.as_str()),
                    Value::Null
                    | Value::Bool(_)
                    | Value::Int(_)
                    | Value::Float(_)
                    | Value::Seq(_)
                    | Value::Map(_) => None,
                };
                name.and_then(|name| level.reserved(name))
                    .map(|(path, owner)| reserved_key_error(&path, owner, key, at))
                    .into_iter()
                    .collect::<Diagnostics>()
                    .merge(walk(
                        value,
                        name.map_or(Level::Other, |name| level.below(name)),
                        at,
                    ))
            })
            .fold(Diagnostics::new(), Diagnostics::merge),
        Value::Seq(items) => items
            .iter()
            .map(|item| walk(item, Level::Other, at))
            .fold(Diagnostics::new(), Diagnostics::merge),
        Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Text(_) => {
            Diagnostics::new()
        }
    }
}

#[cfg(test)]
mod tests;
