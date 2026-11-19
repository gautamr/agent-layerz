import { describe, expect, it } from "vitest";
import { fixture, layers, readFile } from "../helpers";

describe("§13.4 tool narrowing", () => {
  it("an overlay that adds a tool fails", async () => {
    const ws = fixture("04-tool-narrowing");
    const result = await layers(ws, "repo-widen", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toMatch(/error.*builder.*write/);
    expect(result.stderr).toContain("shop-api");
  });

  it("an overlay that removes a tool or narrows a scope passes", async () => {
    const ws = fixture("04-tool-narrowing");
    const result = await layers(ws, "repo-narrow", ["build"]);
    expect(result.code).toBe(0);
    expect(readFile(ws, "repo-narrow/.claude/agents/builder.md")).toMatch(/^tools: Read, Bash$/m);
  });
});
