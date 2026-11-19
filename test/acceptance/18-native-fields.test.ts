import { describe, expect, it } from "vitest";
import { fixture, layers, readFile } from "../helpers";

describe("§7.2 native agent fields", () => {
  it("passes claude: fields through, and overlays deep-merge them", async () => {
    const ws = fixture("18-native-fields");
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(0);
    const agent = readFile(ws, "repo/.claude/agents/architect.md");
    expect(agent).toContain("effort: high");
    expect(agent).toContain("maxTurns: 60");
    expect(agent).toContain("color: blue");
    expect(agent).not.toContain("claude:");
  });

  it("rejects a claude: field that has a neutral equivalent", async () => {
    const ws = fixture("18-native-fields");
    const result = await layers(ws, "bad", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("'claude.tools' has a neutral field");
  });
});
