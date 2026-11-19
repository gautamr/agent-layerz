import { describe, expect, it } from "vitest";
import { expected, fixture, layers, readFile } from "../helpers";

describe("§13.2 instruction stacking", () => {
  it("personal + client + repo give one AGENTS.md with three sections; mode: remove drops a block", async () => {
    const ws = fixture("02-instructions");
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(0);

    const agentsMd = readFile(ws, "repo/AGENTS.md");
    expect(agentsMd).toBe(expected("02-instructions", "AGENTS.md"));
    expect(agentsMd).not.toContain("Prefer short sentences");
    expect(readFile(ws, "repo/CLAUDE.md")).toBe(expected("02-instructions", "CLAUDE.md"));
  });
});
