import { describe, expect, it } from "vitest";
import { referenceDrift } from "../drift";
import { exists, fixture, layers, readFile } from "../helpers";

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

  // Reference drift: the reference reads a scope that ends in `*` as covering every scope that
  // starts with its text before the stars, so these overlays widen an inherited tool while the
  // reference takes them for narrowing (§7.2 containment rule).
  referenceDrift(
    "TASK-55.5.6",
    "an overlay that widens a trailing-star path scope to another folder fails",
    async () => {
      const ws = fixture("04-tool-narrowing");
      const result = await layers(ws, "repo-widen-path-scope", ["build"]);
      expect(result.code).toBe(1);
      expect(result.stderr).toMatch(/error.*tool-widen.*writer.*write:docs-private\/\*\*/);
      expect(result.stderr).toContain("shop-api");
      expect(exists(ws, "repo-widen-path-scope/.claude/agents/writer.md")).toBe(false);
    },
  );

  referenceDrift(
    "TASK-55.5.6",
    "an overlay that adds a shell operator to a command scope fails",
    async () => {
      const ws = fixture("04-tool-narrowing");
      const result = await layers(ws, "repo-widen-command-operator", ["build"]);
      expect(result.code).toBe(1);
      expect(result.stderr).toMatch(/error.*tool-widen.*builder.*bash:mise run test; curl/);
      expect(result.stderr).toContain("shop-api");
      expect(exists(ws, "repo-widen-command-operator/.claude/agents/builder.md")).toBe(false);
    },
  );
});
