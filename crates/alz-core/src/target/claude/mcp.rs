//! `.mcp.json` (spec §7.7, §8.1, target-facts §7): the enabled servers, and only those.

use crate::diagnostic::Diagnostics;
use crate::item::Transport;
use crate::merge::{Merged, Tree};

use super::document::{file, map, text, texts};
use super::{Output, output_path};

/// The path of the file.
const MCP_FILE: &str = ".mcp.json";

/// The file of the enabled servers, none when no server is on.
pub(super) fn render(merged: &Merged) -> Output {
    let servers: Vec<(&str, Tree)> = merged
        .mcp()
        .enabled_servers()
        .map(|(id, server)| (id.as_str(), definition(server.value().transport())))
        .collect();
    if servers.is_empty() {
        Output::default()
    } else {
        Output::one(
            output_path(MCP_FILE),
            file(&map([("mcpServers", map(servers))])),
            Diagnostics::new(),
        )
    }
}

/// One server: `command`, `args` and `env` for stdio, and for a URL the `type`, always written
/// because an entry with a URL and no type is read as stdio. A variable is referenced as
/// `${NAME}` and never stored.
fn definition(transport: &Transport) -> Tree {
    match transport {
        Transport::Stdio {
            command,
            args,
            env_from,
        } => {
            let references: Vec<(String, Tree)> = env_from
                .iter()
                .map(|name| {
                    (
                        name.as_str().to_owned(),
                        Tree::Text(format!("${{{}}}", name.as_str())),
                    )
                })
                .collect();
            map([
                Some(("command", text(command))),
                args.as_ref()
                    .map(|written| ("args", texts(written.iter().map(String::as_str)))),
                (!references.is_empty())
                    .then(|| ("env", Tree::Map(references.into_iter().collect()))),
            ]
            .into_iter()
            .flatten())
        }
        Transport::Http { url } => map([("type", text("http")), ("url", text(url))]),
        Transport::Sse { url } => map([("type", text("sse")), ("url", text(url))]),
    }
}
