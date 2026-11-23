//! The merge of MCP servers and the allowlist (spec §7.7).
//!
//! Servers are an ordered map from server id to definition, and the most specific layer that
//! defines an id replaces the whole definition ([`Replace`]); the layers that defined it before
//! stay in its provenance, marked as replaced. Only the ids in the manifest's `mcp.enable` are
//! emitted, in the order the manifest lists them, each once. An enabled id that no layer defines
//! is an `mcp` error, unless a layer's definition of it was refused for a protected variable: that
//! error was already reported (§7.9).

use std::collections::BTreeSet;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::item::{McpItems, McpServer, McpServerId};
use crate::merge::algebra::Replace;
use crate::merge::provenance::{Action, Traced};
use crate::merge::stack::LayerContext;
use crate::stack::Manifest;
use crate::untrusted_text::Stripped;

/// The servers after the merge: every definition, and the ids that are on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MergedMcp {
    defined: Replace<McpServerId, Traced<McpServer>>,
    enabled: Vec<McpServerId>,
}

impl MergedMcp {
    /// Every server a layer defined, by id, each with the layers that defined it. A definition of
    /// a more general layer that a more specific one replaced is in the provenance only.
    #[must_use]
    pub const fn defined(&self) -> &Replace<McpServerId, Traced<McpServer>> {
        &self.defined
    }

    /// The ids that are on, in the order of `mcp.enable`, each once. Each is defined.
    #[must_use]
    pub fn enabled(&self) -> &[McpServerId] {
        &self.enabled
    }

    /// The servers that are on, in the order of `mcp.enable`, with their definitions. Only these
    /// are emitted (the allowlist, §7.7).
    pub fn enabled_servers(&self) -> impl Iterator<Item = (&McpServerId, &Traced<McpServer>)> + '_ {
        self.enabled
            .iter()
            .filter_map(|id| self.defined.get(id).map(|server| (id, server)))
    }
}

/// The state of the fold for MCP servers: what is defined so far, and the ids refused for a
/// protected variable in any layer.
#[derive(Debug, Default)]
pub(super) struct McpState {
    defined: Replace<McpServerId, Traced<McpServer>>,
    refused: BTreeSet<McpServerId>,
}

impl McpState {
    /// The state after one more, more specific, layer: its servers replace those of the same id.
    pub(super) fn absorb(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        let layer = context.layer().meta().name();
        let (items, found) = McpItems::read(context.layer());
        let (servers, refused) = items.into_parts();
        let arrived: Replace<McpServerId, Traced<McpServer>> = servers
            .into_iter()
            .map(|(id, server)| {
                let traced = match self.defined.get(&id) {
                    Some(previous) => {
                        previous
                            .clone()
                            .revised(server, layer.clone(), Action::Replace)
                    }
                    None => Traced::new(server, layer.clone(), Action::Define),
                };
                (id, traced)
            })
            .collect();
        (
            Self {
                defined: self.defined.merge(arrived),
                refused: self.refused.into_iter().chain(refused).collect(),
            },
            found,
        )
    }

    /// The finished servers, and an error for each id of `mcp.enable` that no layer defines.
    pub(super) fn finish(self, manifest: &Manifest) -> (MergedMcp, Diagnostics) {
        let (enabled, found) = manifest.mcp_enable().iter().fold(
            (Vec::<McpServerId>::new(), Diagnostics::new()),
            |(enabled, found), id| {
                if enabled.contains(id) {
                    (enabled, found)
                } else if self.defined.get(id).is_some() {
                    (enabled.into_iter().chain([id.clone()]).collect(), found)
                } else if self.refused.contains(id) {
                    (enabled, found)
                } else {
                    (enabled, found.with(not_defined(id)))
                }
            },
        );
        (
            MergedMcp {
                defined: self.defined,
                enabled,
            },
            found,
        )
    }
}

fn not_defined(id: &McpServerId) -> Diagnostic {
    Diagnostic::error(
        Code::Mcp,
        Some(Where::repository_file(Stripped::from_untrusted(
            "layer.yaml",
        ))),
        Message::new("mcp.enable lists ")
            .then(MessagePart::printable(id))
            .then(", but no layer defines it"),
    )
}

#[cfg(test)]
mod tests;
