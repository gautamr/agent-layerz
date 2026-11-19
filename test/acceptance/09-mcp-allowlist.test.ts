import { describe, expect, it } from "vitest";
import { fixture, layers, readFile, readTree } from "../helpers";

describe("§13.9 MCP allowlist", () => {
  it("a server defined but not enabled does not appear in any output", async () => {
    const ws = fixture("09-mcp-allowlist");
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(0);
    const mcp = JSON.parse(readFile(ws, "repo/.mcp.json"));
    expect(mcp).toEqual({
      mcpServers: { context7: { type: "http", url: "https://mcp.context7.com/mcp" } },
    });
    const outputs = readTree(`${ws.root}/repo`, [".layer", "layer.yaml"]);
    for (const [path, content] of Object.entries(outputs)) {
      expect(content, path).not.toContain("playwright");
    }
  });
});
