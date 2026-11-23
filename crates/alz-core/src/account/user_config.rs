//! The owner's `accounts.yaml` as account values (spec §7.13 "User config"): the shape only.
//! What needs the file system or the environment (the trust checks of "Both files", `~`, real
//! paths, account folders, two keys that resolve to one directory, the longest match for a
//! working directory) is the capability layer's (R6, TASK-55.7).
//!
//! The file is read as far as it can be. The whole file is ignored when its top level is not a
//! mapping, and each part that does not fit is ignored with one warning. A warning holds no
//! value: the only input text it can hold is a key, through [`QuotedKey`].

use crate::account::declaration::AccountDeclaration;
use crate::account::folder_key::FolderKey;
use crate::account::name::AccountName;
use crate::account::reading::{Reading, tree, unreadable, warning};
use crate::diagnostic::{Code, Message, Position, Where};
use crate::untrusted_text::QuotedKey;
use crate::yaml::fields::KeyPath;
use crate::yaml::{Mapping, Node, Value};

/// A `folders` entry: a folder and the account it stands for, for every target or for each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderRule {
    key: FolderKey,
    choice: AccountDeclaration,
}

impl FolderRule {
    /// A rule.
    #[must_use]
    pub const fn new(key: FolderKey, choice: AccountDeclaration) -> Self {
        Self { key, choice }
    }

    /// The folder, as written.
    #[must_use]
    pub const fn key(&self) -> &FolderKey {
        &self.key
    }

    /// The account the folder and everything below it stands for. `default` is allowed.
    #[must_use]
    pub const fn choice(&self) -> &AccountDeclaration {
        &self.choice
    }
}

/// The usable content of `accounts.yaml`. A file that is absent or wholly ignored is the empty
/// value, which is also the default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserConfig {
    default_name: Option<AccountName>,
    folders: Vec<FolderRule>,
}

impl UserConfig {
    /// What the default account is called on this machine, when the file says so.
    #[must_use]
    pub const fn default_name(&self) -> Option<&AccountName> {
        self.default_name.as_ref()
    }

    /// The `folders` entries that fit their shape, in the order of the file.
    #[must_use]
    pub fn folders(&self) -> &[FolderRule] {
        &self.folders
    }

    /// Reads the bytes of `accounts.yaml`, shown as `file` in warnings. A file past a YAML limit
    /// or that is not valid YAML is ignored with one warning.
    #[must_use]
    pub fn from_bytes(bytes: &[u8], file: &Where) -> Reading<Self> {
        tree(bytes).map_or_else(
            |error| Reading::new(Self::default()).warned(unreadable(&error, file)),
            |node| Self::from_node(&node, file),
        )
    }

    /// Reads a parsed document, shown as `file` in warnings. A null document (an empty file) is
    /// an empty config.
    #[must_use]
    pub fn from_node(node: &Node, file: &Where) -> Reading<Self> {
        match node.value() {
            Value::Null => Reading::new(Self::default()),
            Value::Map(mapping) => mapping
                .entries()
                .map(|(key, value)| top_level_entry(key, value, file))
                .fold(Reading::new(Self::default()), |config, entry| {
                    config.then(entry, Self::with)
                }),
            Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Text(_) | Value::Seq(_) => {
                Reading::new(Self::default()).warned(warning(
                    Code::AccountConfig,
                    file,
                    node.at(),
                    Message::new("the file is ignored: its top level must be a mapping"),
                ))
            }
        }
    }

    fn with(self, part: Part) -> Self {
        match part {
            Part::Nothing => self,
            Part::DefaultName(name) => Self {
                default_name: Some(name),
                ..self
            },
            Part::Folders(folders) => Self { folders, ..self },
        }
    }
}

/// What one top-level key of the file contributes.
enum Part {
    Nothing,
    DefaultName(AccountName),
    Folders(Vec<FolderRule>),
}

/// The key as text for a warning: its text, or the kind of value for a key that is not text.
fn shown(key: &Node) -> QuotedKey {
    QuotedKey::from_untrusted(match key.value() {
        Value::Text(text) => text,
        kind @ (Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Seq(_)
        | Value::Map(_)) => kind.kind_name(),
    })
}

fn config_warning(file: &Where, at: Position, message: Message) -> Reading<Part> {
    Reading::new(Part::Nothing).warned(warning(Code::AccountConfig, file, at, message))
}

fn top_level_entry(key: &Node, value: &Node, file: &Where) -> Reading<Part> {
    match key.value() {
        Value::Text(name) if name == "default_name" => default_name(value, file),
        Value::Text(name) if name == "folders" => folders(value, file),
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Text(_)
        | Value::Seq(_)
        | Value::Map(_) => config_warning(
            file,
            key.at(),
            Message::new("the key ")
                .then(shown(key))
                .then(" is not known and is ignored"),
        ),
    }
}

/// `default_name` is read as written: a null is not an account name either.
fn default_name(value: &Node, file: &Where) -> Reading<Part> {
    match value.value() {
        Value::Text(text) => AccountName::try_from(text.as_str()).map_or_else(
            |_not_a_name| not_a_default_name(value, file),
            |name| Reading::new(Part::DefaultName(name)),
        ),
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Seq(_)
        | Value::Map(_) => not_a_default_name(value, file),
    }
}

fn not_a_default_name(value: &Node, file: &Where) -> Reading<Part> {
    config_warning(
        file,
        value.at(),
        Message::new("default_name must be an account name; it is ignored"),
    )
}

/// `folders` that is absent (a null) is nothing; one that is not a mapping is ignored.
fn folders(value: &Node, file: &Where) -> Reading<Part> {
    match value.value() {
        Value::Null => Reading::new(Part::Nothing),
        Value::Map(mapping) => folder_rules(mapping, file),
        Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Text(_) | Value::Seq(_) => {
            config_warning(
                file,
                value.at(),
                Message::new("'folders' must be a mapping; it is ignored"),
            )
        }
    }
}

fn folder_rules(mapping: &Mapping, file: &Where) -> Reading<Part> {
    mapping
        .entries()
        .map(|(key, value)| folder_rule(key, value, file))
        .fold(Reading::new(Vec::new()), |rules, rule| {
            rules.then(rule, |mut rules, rule| {
                rules.extend(rule);
                rules
            })
        })
        .map(Part::Folders)
}

fn folder_rule(key: &Node, value: &Node, file: &Where) -> Reading<Option<FolderRule>> {
    let ignored = |at: Position, message: Message| {
        Reading::new(None).warned(warning(Code::AccountConfig, file, at, message))
    };
    let folder = match key.value() {
        Value::Text(text) => FolderKey::try_from(text.as_str()).ok(),
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Seq(_)
        | Value::Map(_) => None,
    };
    match folder {
        None => ignored(
            key.at(),
            Message::new("the folders key ")
                .then(shown(key))
                .then(" is not an absolute path or ~/..., so it is ignored"),
        ),
        Some(folder) => AccountDeclaration::from_node(value, KeyPath::root().field("folders"))
            .map_or_else(
                |_refused| {
                    ignored(
                        key.at(),
                        Message::new("the folders entry ").then(shown(key)).then(
                            " must be an account name or a mapping of target to name, so it is ignored",
                        ),
                    )
                },
                |choice| Reading::new(Some(FolderRule::new(folder, choice))),
            ),
    }
}

#[cfg(test)]
mod tests;
