import { describe, expect, it } from "vitest";
import { fixture, layers, readFile, readTree } from "../helpers";

const PROTECTED = [
  ".claude/agent-memory/reviewer/MEMORY.md",
  ".claude/agent-memory-local/reviewer/MEMORY.md",
  ".claude/settings.local.json",
  "backlog/tasks/task-1.md",
  "docs/guide.md",
  "specs/001-slice/spec.md",
  "notes/team.md",
];

describe("§13.15 protected paths", () => {
  it("files under protected paths are byte-identical after a build, even when a layer ships them", async () => {
    const ws = fixture("15-protected-paths");
    const before = readTree(`${ws.root}/repo`);
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(0);

    const after = readTree(`${ws.root}/repo`);
    for (const path of PROTECTED) {
      expect(after[path], path).toBe(before[path]);
      expect(after[path], path).toContain("REPO-OWNED");
      expect(result.stderr).toMatch(
        new RegExp(`warning \\[protected\\] ${path.replaceAll(".", "\\.")}`),
      );
    }
    const lock = readFile(ws, "repo/layer.lock");
    for (const path of PROTECTED) expect(lock).not.toContain(path);
    expect((await layers(ws, "repo", ["check"])).code).toBe(0);
  });
});
