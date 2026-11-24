//! What Claude Code says it loaded (target-facts §0.2): the `system` `init` event of a print-mode
//! session in `stream-json` form, reduced to names and statuses.
//!
//! The text comes from a process that alz did not write, so it is untrusted: only strings are
//! taken from the event, every line that is shown goes through [`Stripped`], and nothing else of
//! the event is kept.

use std::collections::BTreeSet;

use thiserror::Error;

use crate::json::{self, JsonValue};
use crate::untrusted_text::Stripped;

/// The scope of an MCP server that a repository's `.mcp.json` defines.
const PROJECT_SCOPE: &str = "project";

/// One server of the `mcp_servers` list of the event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpState {
    name: String,
    scope: String,
    status: Stripped,
}

impl McpState {
    /// The server's name, as Claude gave it.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the server comes from the repository's `.mcp.json`.
    #[must_use]
    pub fn is_project(&self) -> bool {
        self.scope == PROJECT_SCOPE
    }

    /// The status Claude gave, such as `connected`, stripped.
    #[must_use]
    pub const fn status(&self) -> &Stripped {
        &self.status
    }

    /// Whether the server is connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.status.as_str() == "connected"
    }
}

/// What a probe found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    agents: BTreeSet<String>,
    skills: BTreeSet<String>,
    mcp: Vec<McpState>,
    warnings: Vec<Stripped>,
}

/// A probe whose output holds no `init` event.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("printed no init event")]
pub struct NoInitEvent {
    first_line: Stripped,
}

impl NoInitEvent {
    /// The first line Claude printed on its standard error, stripped; empty when there was none.
    #[must_use]
    pub const fn first_line(&self) -> &Stripped {
        &self.first_line
    }
}

fn field<'a>(event: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    match event {
        JsonValue::Object(object) => object.get(key),
        JsonValue::Null
        | JsonValue::Bool(_)
        | JsonValue::Number(_)
        | JsonValue::Text(_)
        | JsonValue::Array(_) => None,
    }
}

fn items<'a>(event: &'a JsonValue, key: &str) -> &'a [JsonValue] {
    match field(event, key) {
        Some(JsonValue::Array(items)) => items,
        Some(
            JsonValue::Null
            | JsonValue::Bool(_)
            | JsonValue::Number(_)
            | JsonValue::Text(_)
            | JsonValue::Object(_),
        )
        | None => &[],
    }
}

fn text_of<'a>(entry: &'a JsonValue, key: &str) -> Option<&'a str> {
    match field(entry, key) {
        Some(JsonValue::Text(text)) => Some(text),
        Some(
            JsonValue::Null
            | JsonValue::Bool(_)
            | JsonValue::Number(_)
            | JsonValue::Array(_)
            | JsonValue::Object(_),
        )
        | None => None,
    }
}

fn strings_of(event: &JsonValue, key: &str) -> BTreeSet<String> {
    items(event, key)
        .iter()
        .filter_map(|entry| match entry {
            JsonValue::Text(text) => Some(text.clone()),
            JsonValue::Null
            | JsonValue::Bool(_)
            | JsonValue::Number(_)
            | JsonValue::Array(_)
            | JsonValue::Object(_) => None,
        })
        .collect()
}

fn mcp_of(event: &JsonValue) -> Vec<McpState> {
    items(event, "mcp_servers")
        .iter()
        .filter_map(|entry| {
            Some(McpState {
                name: text_of(entry, "name")?.to_owned(),
                scope: text_of(entry, "source").unwrap_or_default().to_owned(),
                status: Stripped::from_untrusted(text_of(entry, "status").unwrap_or("unknown")),
            })
        })
        .collect()
}

fn is_init(event: &JsonValue) -> bool {
    text_of(event, "type") == Some("system") && text_of(event, "subtype") == Some("init")
}

impl Probe {
    /// Reads the output of one session. The first `init` event of standard output counts, a line
    /// that is not JSON is skipped, and each line of either stream that is not JSON, trimmed and
    /// stripped, is a warning (standard error first).
    ///
    /// # Errors
    ///
    /// [`NoInitEvent`] when no line of standard output is an `init` event.
    pub fn parse(stdout: &str, stderr: &str) -> Result<Self, NoInitEvent> {
        let init = stdout
            .lines()
            .filter(|line| line.starts_with('{'))
            .filter_map(|line| json::parse(line.as_bytes()).ok())
            .find(is_init);
        let Some(init) = init else {
            return Err(NoInitEvent {
                first_line: Stripped::from_untrusted(stderr.lines().next().map_or("", str::trim)),
            });
        };
        let skills = strings_of(&init, "skills")
            .union(&strings_of(&init, "slash_commands"))
            .cloned()
            .collect();
        Ok(Self {
            agents: strings_of(&init, "agents"),
            skills,
            mcp: mcp_of(&init),
            warnings: stderr
                .lines()
                .chain(stdout.lines())
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('{'))
                .map(Stripped::from_untrusted)
                .filter(|line| !line.as_str().is_empty())
                .collect(),
        })
    }

    /// The agents Claude loaded.
    #[must_use]
    pub const fn agents(&self) -> &BTreeSet<String> {
        &self.agents
    }

    /// The skills and commands Claude loaded.
    #[must_use]
    pub const fn skills(&self) -> &BTreeSet<String> {
        &self.skills
    }

    /// The MCP servers Claude reported.
    #[must_use]
    pub fn mcp(&self) -> &[McpState] {
        &self.mcp
    }

    /// The other lines Claude printed, stripped.
    #[must_use]
    pub fn warnings(&self) -> &[Stripped] {
        &self.warnings
    }
}

#[cfg(test)]
mod tests;
