//! The protected-variable checks of a passthrough file (spec §7.9), by how the file is covered.
//!
//! Each check works on a parsed tree, never on the text of the layer's file, so a problem names a
//! key path chosen here and never a key or value the layer wrote. A covered file that cannot be
//! parsed cannot be checked and is refused.

use crate::diagnostic::{Code, Diagnostics, Where};
use crate::item::agent::tree_of;
use crate::item::protected_variable::{env_keys, paths, refused, server_envs, unchecked};
use crate::json::{self, JsonNumber, JsonValue};
use crate::markdown::MarkdownDoc;
use crate::merge::{FloatBits, Tree};

use super::coverage::Coverage;
use super::toml_text;

/// The problems of the file at `inner` (its path after `targets/<target>/`) with these bytes.
pub(super) fn problems(inner: &str, bytes: &[u8], at: &Where) -> Diagnostics {
    match Coverage::of(inner) {
        Coverage::Json => json_file(bytes, at),
        Coverage::ClaudeAgent => agent_file(bytes, at),
        Coverage::Toml => toml_file(bytes, at),
        Coverage::Uncovered => Diagnostics::new(),
    }
}

/// Claude settings, `.mcp.json` and plugin files: the root `env`, the `env` of each server of the
/// root `mcpServers`, and any `env` mapping nested below. A root that is not an object holds
/// nothing to check.
fn json_file(bytes: &[u8], at: &Where) -> Diagnostics {
    match json::parse(bytes) {
        Err(_) => unchecked("this file", "is not valid JSON", at),
        Ok(value) => match tree_of_json(&value) {
            Tree::Map(root) => env_keys(root.get("env"), paths::FILE_ENV, at)
                .merge(mcp_servers(root.get("mcpServers"), at))
                .merge(all(root
                    .iter()
                    .filter(|(key, _)| !matches!(key.as_str(), "env" | "mcpServers"))
                    .map(|(_, value)| nested_envs(value, at)))),
            Tree::Null
            | Tree::Bool(_)
            | Tree::Int(_)
            | Tree::Float(_)
            | Tree::Text(_)
            | Tree::List(_) => Diagnostics::new(),
        },
    }
}

/// A Claude agent file: the `env` of each server of its frontmatter's `mcpServers`.
fn agent_file(bytes: &[u8], at: &Where) -> Diagnostics {
    match MarkdownDoc::parse(bytes) {
        Err(_) => unchecked("this agent file", "has frontmatter that does not parse", at),
        Ok(document) => mcp_servers(document.front().get("mcpServers").map(tree_of).as_ref(), at),
    }
}

/// A TOML file, by the text rule of [`toml_text`].
fn toml_file(bytes: &[u8], at: &Where) -> Diagnostics {
    const FILE: &str = "this .toml file";
    match std::str::from_utf8(bytes) {
        Err(_) => unchecked(FILE, "is not valid UTF-8", at),
        Ok(text) => toml_text::mentions_policy(text)
            .then_some(toml_text::POLICY_KEY)
            .into_iter()
            .chain(toml_text::named_variables(text))
            .map(|variable| Diagnostics::new().with(refused(FILE, variable, at)))
            .chain(toml_text::hides_a_name(text).then(|| {
                unchecked(
                    FILE,
                    "has a \\u, \\U or \\x escape or a line-ending backslash",
                    at,
                )
            }))
            .fold(Diagnostics::new(), Diagnostics::merge),
    }
}

/// The `env` of each server in `servers`, when it has the shape of a server list. Another value,
/// such as the name of a file in a plugin manifest, names no server here and is left alone: the
/// file it names is not read (§7.9, 'Not checked').
fn mcp_servers(servers: Option<&Tree>, at: &Where) -> Diagnostics {
    servers
        .filter(|tree| matches!(tree, Tree::Map(_) | Tree::List(_)))
        .map_or_else(Diagnostics::new, |tree| {
            server_envs(Some(tree), Code::ProtectedVariable, at)
        })
}

/// Every `env` mapping at any depth of `tree`. An `env` that is not a mapping is searched like any
/// other value.
fn nested_envs(tree: &Tree, at: &Where) -> Diagnostics {
    match tree {
        Tree::List(items) => all(items.iter().map(|item| nested_envs(item, at))),
        Tree::Map(entries) => all(entries.iter().map(|(key, value)| {
            if key == "env" && matches!(value, Tree::Map(_)) {
                env_keys(Some(value), paths::FILE_NESTED_ENV, at)
            } else {
                nested_envs(value, at)
            }
        })),
        Tree::Null | Tree::Bool(_) | Tree::Int(_) | Tree::Float(_) | Tree::Text(_) => {
            Diagnostics::new()
        }
    }
}

/// The JSON value as a sorted tree. Nesting is at most [`json::MAX_DEPTH`], which bounds the
/// recursion here and in [`nested_envs`].
fn tree_of_json(value: &JsonValue) -> Tree {
    match value {
        JsonValue::Null => Tree::Null,
        JsonValue::Bool(flag) => Tree::Bool(*flag),
        JsonValue::Number(JsonNumber::Int(number)) => Tree::Int(*number),
        JsonValue::Number(JsonNumber::Float(number)) => Tree::Float(FloatBits::new(*number)),
        JsonValue::Text(text) => Tree::Text(text.clone()),
        JsonValue::Array(items) => Tree::List(items.iter().map(tree_of_json).collect()),
        JsonValue::Object(object) => Tree::Map(
            object
                .iter()
                .map(|(key, value)| (key.to_owned(), tree_of_json(value)))
                .collect(),
        ),
    }
}

/// The diagnostics of each, in order.
fn all(found: impl Iterator<Item = Diagnostics>) -> Diagnostics {
    found.fold(Diagnostics::new(), Diagnostics::merge)
}
