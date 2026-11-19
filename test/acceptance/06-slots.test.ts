import { describe, expect, it } from "vitest";
import { exists, fixture, layers, readFile } from "../helpers";

describe("§13.6 slots", () => {
  it("a missing slot fails with strict_slots: true", async () => {
    const ws = fixture("06-slots");
    const result = await layers(ws, "repo-strict", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toMatch(/error.*domain-glossary/);
    expect(exists(ws, "repo-strict/.claude/skills/domain-glossary/SKILL.md")).toBe(false);
  });

  it("a missing slot gives a stub skill and a warning with strict_slots: false", async () => {
    const ws = fixture("06-slots");
    const result = await layers(ws, "repo-loose", ["build"]);
    expect(result.code).toBe(0);
    expect(result.stderr).toMatch(/warning.*domain-glossary/);
    const stub = readFile(ws, "repo-loose/.claude/skills/domain-glossary/SKILL.md");
    expect(stub).toContain("not defined for this repo");
  });
});
