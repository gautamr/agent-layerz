import { describe, expect, it } from "vitest";
import { expected, fixture, layers, readFile } from "../helpers";

describe("§13.3 agent overlay", () => {
  it("base + skills overlay + body overlay give one merged agent", async () => {
    const ws = fixture("03-agent-overlay");
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(0);
    expect(readFile(ws, "repo/.claude/agents/architect.md")).toBe(
      expected("03-agent-overlay", ".claude/agents/architect.md"),
    );
    // write:docs/** cannot be scoped per agent in Claude: the build says so.
    expect(result.stderr).toMatch(/warning.*architect.*write:docs\/\*\*/);
  });

  it("explain agent architect lists all three sources in order", async () => {
    const ws = fixture("03-agent-overlay");
    const result = await layers(ws, "repo", ["explain", "agent", "architect"]);
    expect(result.code).toBe(0);
    const out = result.stdout;
    const base = out.indexOf("personal@1.0.0:agents/architect.md");
    const client = out.indexOf("acme:agents/architect.overlay.md");
    const repo = out.indexOf("shop-api:agents/architect.overlay.md");
    expect(base).toBeGreaterThanOrEqual(0);
    expect(client).toBeGreaterThan(base);
    expect(repo).toBeGreaterThan(client);
    expect(out).toContain("domain-glossary");
  });
});
