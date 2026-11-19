import { describe, expect, it } from "vitest";
import { fixture, layers, readFile } from "../helpers";

describe("§13.7 permission floor", () => {
  it("a repo layer removing a personal deny fails", async () => {
    const ws = fixture("07-permission-floor");
    const result = await layers(ws, "repo-remove", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toMatch(/error.*shop-api.*read:\.env\*/);
  });

  it("allow, ask and deny lists combine across layers", async () => {
    const ws = fixture("07-permission-floor");
    const result = await layers(ws, "repo-combine", ["build"]);
    expect(result.code).toBe(0);
    const settings = JSON.parse(readFile(ws, "repo-combine/.claude/settings.json"));
    expect(settings.permissions.allow).toEqual(["Bash(git status)", "Bash(mise run *)"]);
    expect(settings.permissions.deny).toEqual([
      "Read(.env*)",
      "Bash(rm -rf *)",
      "Bash(git push --force*)",
    ]);
  });
});
