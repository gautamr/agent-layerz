import { describe, expect, it } from "vitest";
import { fixture, layers } from "../helpers";

describe("§10 alz ls", () => {
  it("lists each built item with the layer that decides it", async () => {
    const ws = fixture("22-ls");
    const result = await layers(ws, "repo", ["ls"]);
    expect(result.code).toBe(0);
    expect(result.stdout).toMatch(/agents \(1\)\n {2}architect +personal\n/);
  });

  it("--live passes when Claude loaded everything, and shows Claude's warnings", async () => {
    const ws = fixture("22-ls");
    const env = { ALZ_CLAUDE_BIN: `${ws.root}/bin/claude-ok` };
    const result = await layers(ws, "repo", ["ls", "--live"], env);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("Claude matches the build");
    expect(result.stdout).toContain("not from layers, agents: Explore");
    expect(result.stderr).toContain(
      "warning [claude] Ignoring 1 permissions.additionalDirectories",
    );
  });

  it("--live fails when a built agent did not load", async () => {
    const ws = fixture("22-ls");
    const env = { ALZ_CLAUDE_BIN: `${ws.root}/bin/claude-missing` };
    const result = await layers(ws, "repo", ["ls", "--live"], env);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("agent 'architect' was built but Claude did not load it");
  });
});
