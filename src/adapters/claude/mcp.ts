import type { McpServer } from "../../merge/mcp";

export const MCP_FILE = ".mcp.json";

/** §8.1: enabled servers → `.mcp.json`. Env vars are referenced as `${NAME}`, never stored. */
export function renderMcpServers(servers: McpServer[]): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const server of servers) {
    if (server.transport === "stdio") {
      out[server.id] = {
        command: server.command,
        args: server.args,
        env:
          server.envFrom.length > 0
            ? Object.fromEntries(server.envFrom.map((name) => [name, `\${${name}}`]))
            : undefined,
      };
    } else {
      // An entry with a url but no type is read as stdio, so type is always written.
      out[server.id] = { type: server.transport, url: server.url };
    }
  }
  return { mcpServers: out };
}
