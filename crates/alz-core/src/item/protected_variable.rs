//! The checks for the variables no layer may set (spec §7.9), over the settings and native fields
//! a layer wrote, as sorted [`Tree`]s.
//!
//! Every problem is a `protected-variable` error at the file, with a key path chosen here and never
//! a key or value from the layer. A refused protected name is printed as the fixed name it
//! stands for, in its fixed spelling, so the text the layer wrote (`codex_home`) does not reach the
//! output. A list or map of the wrong type cannot be checked and is refused.
//!
//! The checks cover `claude.env`, the `env` of the servers under `claude.mcpServers` and
//! `codex.mcp_servers`, and `shell_environment_policy` wherever it appears under `codex`.

use std::collections::BTreeMap;

use crate::account::matched_protected_names;
use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::merge::Tree;

use super::EnvVarName;

/// The key paths the checks name.
pub(super) mod paths {
    /// `claude.env` of `settings.yaml`.
    pub(in crate::item) const CLAUDE_ENV: &str = "claude.env";
    /// The `env` of a server under `claude.mcpServers`.
    pub(super) const CLAUDE_SERVER_ENV: &str = "claude.mcpServers.env";
    /// The `env` of a server under `codex.mcp_servers`.
    pub(super) const CODEX_SERVER_ENV: &str = "codex.mcp_servers.env";
    /// The `env_from` of a server of `mcp.yaml`.
    pub(in crate::item) const MCP_ENV_FROM: &str = "mcp.env_from";
    /// The root `env` of a passthrough JSON file.
    pub(in crate::item) const FILE_ENV: &str = "env";
    /// An `env` mapping nested anywhere below the root of a passthrough JSON file.
    pub(in crate::item) const FILE_NESTED_ENV: &str = "a nested env";
    /// The `env` of a server under `mcpServers` in a passthrough file.
    pub(super) const FILE_SERVER_ENV: &str = "mcpServers.env";
    /// A `shell_environment_policy` under `codex`.
    pub(super) const POLICY: &str = "codex.shell_environment_policy";
    pub(super) const POLICY_SET: &str = "codex.shell_environment_policy.set";
    pub(super) const POLICY_EXCLUDE: &str = "codex.shell_environment_policy.exclude";
    pub(super) const POLICY_FILTERS: &str = "codex.shell_environment_policy.filters";
}

/// The key `shell_environment_policy` of Codex's configuration.
const POLICY: &str = "shell_environment_policy";

/// The problems of a mapping of variables (`claude.env`, a server's `env`, a policy's `set`): its
/// keys are variable names. An absent or `null` value is nothing to check.
pub(super) fn env_keys(env: Option<&Tree>, path: &'static str, at: &Where) -> Diagnostics {
    match env {
        None | Some(Tree::Null) => Diagnostics::new(),
        Some(Tree::Map(variables)) => variables
            .keys()
            .filter_map(|key| name_problem(key, path, at))
            .collect(),
        Some(_) => unchecked(path, "is not a mapping", at),
    }
}

/// The problems of the names in an `env_from` list: each must be a plain variable name and none
/// may be protected.
pub(super) fn env_from_names<'a>(
    names: impl IntoIterator<Item = &'a str>,
    at: &Where,
) -> Diagnostics {
    names
        .into_iter()
        .filter_map(|name| name_problem(name, paths::MCP_ENV_FROM, at))
        .collect()
}

/// The field that holds MCP servers in a target's native fields.
#[derive(Debug, Clone, Copy)]
pub(super) enum ServerField {
    /// `claude.mcpServers`: a map of name to definition, or a list of names and one-entry maps.
    Claude,
    /// `codex.mcp_servers`: a map of name to definition.
    Codex,
    /// `mcpServers` at the root of a passthrough JSON file or of a passthrough agent file's
    /// frontmatter: shaped as for Claude, named by itself.
    File,
}

impl ServerField {
    const fn path(self) -> &'static str {
        match self {
            Self::Claude => "claude.mcpServers",
            Self::Codex => "codex.mcp_servers",
            Self::File => "mcpServers",
        }
    }

    const fn env_path(self) -> &'static str {
        match self {
            Self::Claude => paths::CLAUDE_SERVER_ENV,
            Self::Codex => paths::CODEX_SERVER_ENV,
            Self::File => paths::FILE_SERVER_ENV,
        }
    }
}

/// The problems of the `env` of each server in `servers`. A value of another type than the field
/// takes could carry variables that cannot be checked, so it is an error of `shape` (the code of
/// the file kind that holds the field), naming the field.
pub(super) fn server_envs(
    servers: Option<&Tree>,
    field: ServerField,
    shape: Code,
    at: &Where,
) -> Diagnostics {
    let path = field.env_path();
    let definitions: Result<Vec<&Tree>, Diagnostics> = match (servers, field) {
        (None | Some(Tree::Null), _) => Ok(Vec::new()),
        (Some(Tree::List(items)), ServerField::Claude | ServerField::File) => Ok(items
            .iter()
            .filter_map(|item| match item {
                Tree::Map(entries) => Some(entries.values()),
                Tree::Null
                | Tree::Bool(_)
                | Tree::Int(_)
                | Tree::Float(_)
                | Tree::Text(_)
                | Tree::List(_) => None,
            })
            .flatten()
            .collect()),
        (
            Some(Tree::Map(entries)),
            ServerField::Claude | ServerField::Codex | ServerField::File,
        ) => Ok(entries.values().collect()),
        (
            Some(Tree::Bool(_) | Tree::Int(_) | Tree::Float(_) | Tree::Text(_) | Tree::List(_)),
            ServerField::Claude | ServerField::Codex | ServerField::File,
        ) => Err(wrong_shape(
            field.path(),
            match field {
                ServerField::Claude | ServerField::File => " must be a mapping or a list",
                ServerField::Codex => " must be a mapping",
            },
            shape,
            at,
        )),
    };
    match definitions {
        Err(found) => found,
        Ok(definitions) => all(definitions.into_iter().map(|definition| match definition {
            Tree::Map(fields) => env_keys(fields.get("env"), path, at),
            Tree::Null
            | Tree::Bool(_)
            | Tree::Int(_)
            | Tree::Float(_)
            | Tree::Text(_)
            | Tree::List(_) => Diagnostics::new(),
        })),
    }
}

/// The problems of every `shell_environment_policy` found anywhere in `entries`, profiles
/// included: its `set` keys, its `exclude` patterns and its `filters` patterns that are not set to
/// `include`.
pub(super) fn codex_policies(
    entries: &BTreeMap<String, Tree>,
    shape: Code,
    at: &Where,
) -> Diagnostics {
    all(entries.iter().map(|(key, value)| {
        if key == POLICY {
            policy(value, shape, at)
        } else {
            policies_in(value, shape, at)
        }
    }))
}

fn policies_in(tree: &Tree, shape: Code, at: &Where) -> Diagnostics {
    match tree {
        Tree::List(items) => all(items.iter().map(|item| policies_in(item, shape, at))),
        Tree::Map(entries) => codex_policies(entries, shape, at),
        Tree::Null | Tree::Bool(_) | Tree::Int(_) | Tree::Float(_) | Tree::Text(_) => {
            Diagnostics::new()
        }
    }
}

/// One `shell_environment_policy`. One that is not a mapping, or `null`, could carry settings that
/// cannot be checked, so it is an error of `shape`.
fn policy(policy: &Tree, shape: Code, at: &Where) -> Diagnostics {
    match policy {
        Tree::Map(fields) => env_keys(fields.get("set"), paths::POLICY_SET, at)
            .merge(exclusions(fields.get("exclude"), at))
            .merge(filters(fields.get("filters"), at)),
        Tree::Null => Diagnostics::new(),
        Tree::Bool(_) | Tree::Int(_) | Tree::Float(_) | Tree::Text(_) | Tree::List(_) => {
            wrong_shape(paths::POLICY, " must be a mapping", shape, at)
        }
    }
}

fn exclusions(exclude: Option<&Tree>, at: &Where) -> Diagnostics {
    match exclude {
        None | Some(Tree::Null) => Diagnostics::new(),
        Some(Tree::List(patterns)) => all(patterns.iter().map(|pattern| match pattern {
            Tree::Text(text) => pattern_problems(text, paths::POLICY_EXCLUDE, at),
            Tree::Null
            | Tree::Bool(_)
            | Tree::Int(_)
            | Tree::Float(_)
            | Tree::List(_)
            | Tree::Map(_) => unchecked(
                paths::POLICY_EXCLUDE,
                "holds a pattern that is not a string",
                at,
            ),
        })),
        Some(_) => unchecked(paths::POLICY_EXCLUDE, "is not a list", at),
    }
}

fn filters(filters: Option<&Tree>, at: &Where) -> Diagnostics {
    match filters {
        None | Some(Tree::Null) => Diagnostics::new(),
        Some(Tree::Map(actions)) => all(actions
            .iter()
            .filter(|(_, action)| !matches!(action, Tree::Text(text) if text == "include"))
            .map(|(pattern, _)| pattern_problems(pattern, paths::POLICY_FILTERS, at))),
        Some(_) => unchecked(paths::POLICY_FILTERS, "is not a mapping", at),
    }
}

/// One error for each protected variable that `pattern` matches as Codex matches its patterns.
fn pattern_problems(pattern: &str, path: &'static str, at: &Where) -> Diagnostics {
    matched_protected_names(pattern)
        .map(|variable| refused(path, variable, at))
        .collect()
}

fn name_problem(key: &str, path: &'static str, at: &Where) -> Option<Diagnostic> {
    match EnvVarName::try_from(key) {
        Err(_) => Some(Diagnostic::error(
            Code::ProtectedVariable,
            Some(at.clone()),
            Message::new(path).then(
                " has a key that is not a plain variable name (letters, digits and '_', not starting with a digit) (\u{a7}7.9)",
            ),
        )),
        Ok(name) => name
            .protected()
            .map(|variable| refused(path, variable, at)),
    }
}

pub(in crate::item) fn refused(
    path: &'static str,
    variable: &'static str,
    at: &Where,
) -> Diagnostic {
    Diagnostic::error(
        Code::ProtectedVariable,
        Some(at.clone()),
        Message::new(path)
            .then(" names ")
            .then(variable)
            .then("; no layer may set the variables that choose an account folder or locate alz's files (\u{a7}7.9)"),
    )
}

pub(in crate::item) fn unchecked(path: &'static str, why: &'static str, at: &Where) -> Diagnostics {
    Diagnostics::new().with(Diagnostic::error(
        Code::ProtectedVariable,
        Some(at.clone()),
        Message::new(path)
            .then(" ")
            .then(why)
            .then(", so it cannot be checked (\u{a7}7.9)"),
    ))
}

fn wrong_shape(path: &'static str, rule: &'static str, code: Code, at: &Where) -> Diagnostics {
    Diagnostics::new().with(Diagnostic::error(
        code,
        Some(at.clone()),
        Message::new(path).then(rule),
    ))
}

/// The diagnostics of each, in order.
fn all(found: impl Iterator<Item = Diagnostics>) -> Diagnostics {
    found.fold(Diagnostics::new(), Diagnostics::merge)
}

#[cfg(test)]
mod tests;
