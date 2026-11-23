//! The MCP servers of a layer (spec §7.7): `mcp.yaml`, a mapping of server id to definition.
//!
//! The file is read into an ordered map from [`McpServerId`] to [`McpServer`]. A key that is not
//! a server id is an `mcp` error naming the file, whatever the key, printed as a quoted key, and
//! it is never a server; the other servers of the file are still read. A server with a problem
//! gives no item. A server that is refused because `env_from` names a protected variable is
//! remembered, so that `mcp.enable` does not add a second error for it.

mod server;
mod server_id;

use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::layer::ItemPath;
use crate::schema::SchemaError;
use crate::stack::ResolvedLayer;
use crate::untrusted_text::QuotedKey;
use crate::yaml::{self, Node, Value, YamlText};

use super::{place, schema_diagnostics};
use server::{Rejected, read_server};

pub use server::{McpServer, Transport};
pub use server_id::{McpRefusal, McpServerId};

/// What one layer holds of §7.7: the servers of its `mcp.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpItems {
    servers: BTreeMap<McpServerId, McpServer>,
    refused: BTreeSet<McpServerId>,
}

impl McpItems {
    /// The servers that were read, by id.
    #[must_use]
    pub const fn servers(&self) -> &BTreeMap<McpServerId, McpServer> {
        &self.servers
    }

    /// The servers that were refused for a protected variable in `env_from`.
    #[must_use]
    pub const fn refused(&self) -> &BTreeSet<McpServerId> {
        &self.refused
    }

    /// The servers and the refused ids.
    #[must_use]
    pub fn into_parts(self) -> (BTreeMap<McpServerId, McpServer>, BTreeSet<McpServerId>) {
        (self.servers, self.refused)
    }

    /// Reads the servers of `layer`. Every problem of the file is reported; a server with a
    /// problem gives no item, and the other servers are still read.
    #[must_use]
    pub fn read(layer: &ResolvedLayer) -> (Self, Diagnostics) {
        layer
            .contents()
            .iter()
            .find(|(path, _)| matches!(ItemPath::of(path), ItemPath::Mcp))
            .map_or_else(
                || (Self::default(), Diagnostics::new()),
                |(path, bytes)| read_file(&place(layer.meta().name(), path), bytes),
            )
    }
}

fn read_file(at: &Where, bytes: &[u8]) -> (McpItems, Diagnostics) {
    let parsed = YamlText::from_bytes(bytes)
        .and_then(|text| yaml::parse(&text))
        .map_err(|problem| Diagnostics::new().with(problem.into_diagnostic(at.clone())));
    match parsed {
        Err(found) => (McpItems::default(), found),
        Ok(node) => match node.value() {
            Value::Null => (McpItems::default(), Diagnostics::new()),
            Value::Map(mapping) => mapping.entries().fold(
                (McpItems::default(), Diagnostics::new()),
                |(items, found), (key, definition)| read_entry(items, found, key, definition, at),
            ),
            Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Text(_) | Value::Seq(_) => (
                McpItems::default(),
                schema_diagnostics(Code::Mcp, at, &SchemaError::NotAMapping(node.at()).into()),
            ),
        },
    }
}

/// The state after one entry of the file: a server, or the problem of its key or definition.
fn read_entry(
    items: McpItems,
    found: Diagnostics,
    key: &Node,
    definition: &Node,
    at: &Where,
) -> (McpItems, Diagnostics) {
    let id = match key.value() {
        Value::Text(text) => McpServerId::try_from(text.as_str()).map_err(|refusal| {
            Message::new(QuotedKey::from_untrusted(text))
                .then(" is not a server id: it ")
                .then(refusal.rule())
        }),
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Seq(_)
        | Value::Map(_) => Err(Message::new("a key that is ")
            .then(key.value().kind_name())
            .then(" is not a server id")),
    };
    match id {
        Err(message) => (
            items,
            found.with(Diagnostic::error(
                Code::Mcp,
                Some(at.clone().at(key.at())),
                message,
            )),
        ),
        Ok(id) => match read_server(&id, definition, at) {
            Ok(server) => (
                McpItems {
                    servers: items.servers.into_iter().chain([(id, server)]).collect(),
                    refused: items.refused,
                },
                found,
            ),
            Err(Rejected::Reported(problems)) => (items, found.merge(problems)),
            Err(Rejected::ProtectedVariable(problems)) => (
                McpItems {
                    servers: items.servers,
                    refused: items.refused.into_iter().chain([id]).collect(),
                },
                found.merge(problems),
            ),
        },
    }
}

#[cfg(test)]
mod tests;
