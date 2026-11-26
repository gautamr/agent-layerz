//! The enabled MCP servers (spec §7.7).

use alz_adapter::{McpServer, McpTransport};

use crate::item::Transport;
use crate::merge::Merged;

/// The servers whose id is in `mcp.enable`, in the order the merge gives them.
pub(super) fn of(merged: &Merged) -> Vec<McpServer> {
    merged
        .mcp()
        .enabled_servers()
        .map(|(id, server)| McpServer::new(id.as_str(), transport(server.value().transport())))
        .collect()
}

fn transport(transport: &Transport) -> McpTransport {
    match transport {
        Transport::Stdio {
            command,
            args,
            env_from,
        } => McpTransport::Stdio {
            command: command.clone(),
            args: args.clone(),
            env_from: env_from
                .iter()
                .map(|name| name.as_str().to_owned())
                .collect(),
        },
        Transport::Http { url } => McpTransport::Http { url: url.clone() },
        Transport::Sse { url } => McpTransport::Sse { url: url.clone() },
    }
}
