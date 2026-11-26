//! `.mcp.json` (spec §7.7, §8.1, target-facts §7): the enabled servers, and only those.

use alz_adapter::{JsonValue, McpTransport, Model};

use super::Made;
use super::document::{file, map, object, text, texts};

/// The path of the file.
const MCP_FILE: &str = ".mcp.json";

/// The file of the enabled servers, none when no server is on.
pub(super) fn render(model: &Model) -> Made {
    let servers: Vec<(&str, JsonValue)> = model
        .mcp_servers
        .iter()
        .map(|server| (server.id.as_str(), definition(&server.transport)))
        .collect();
    if servers.is_empty() {
        Made::default()
    } else {
        Made::file(MCP_FILE, file(map([("mcpServers", map(servers))])))
    }
}

/// One server: `command`, `args` and `env` for stdio, and for a URL the `type`, always written
/// because an entry with a URL and no type is read as stdio. A variable is referenced as
/// `${NAME}` and never stored.
fn definition(transport: &McpTransport) -> JsonValue {
    match transport {
        McpTransport::Stdio {
            command,
            args,
            env_from,
        } => {
            let references: Vec<(String, JsonValue)> = env_from
                .iter()
                .map(|name| (name.clone(), JsonValue::Text(format!("${{{name}}}"))))
                .collect();
            map([
                Some(("command", text(command))),
                args.as_ref()
                    .map(|written| ("args", texts(written.iter().map(String::as_str)))),
                (!references.is_empty()).then(|| ("env", object(references))),
            ]
            .into_iter()
            .flatten())
        }
        McpTransport::Http { url } => map([("type", text("http")), ("url", text(url))]),
        McpTransport::Sse { url } => map([("type", text("sse")), ("url", text(url))]),
    }
}
