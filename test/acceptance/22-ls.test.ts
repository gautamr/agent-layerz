import { describe, expect, it } from "vitest";
import { referenceDrift } from "../drift";
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

  referenceDrift(
    "TASK-55.6",
    "lists the items of a section in name order, whichever layer defines them",
    async () => {
      const ws = fixture("01-determinism");
      const result = await layers(ws, "repo", ["ls"]);
      expect(result.code).toBe(0);
      expect(result.stdout).toMatch(
        /skills \(3\)\n {2}domain-glossary +acme\n {2}review-checklist +personal\n {2}stack-conventions +shop-api\n/,
      );
    },
  );
});

describe("§10.3 alz explain", () => {
  it("shows a rule that a more specific layer replaced as overridden, then the merged file", async () => {
    const ws = fixture("17-rules");
    const result = await layers(ws, "repo", ["explain", "rule", "backend"]);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("rule backend\nsources, general → specific:\n");
    expect(result.stdout).toContain("1. personal@1.0.0:rules/backend.md — rule (overridden)");
    expect(result.stdout).toContain("2. shop-api:rules/backend.md — rule\n");
    expect(result.stdout).toContain("\nmerged result (.claude/rules/backend.md):\n");
  });

  it("is a usage error for a type it does not know", async () => {
    const ws = fixture("17-rules");
    const result = await layers(ws, "repo", ["explain", "widget", "x"]);
    expect(result.code).toBe(2);
    expect(result.stdout).toBe("");
    expect(result.stderr).not.toBe("");
  });

  referenceDrift(
    "TASK-55.6",
    "reports a name the merged result does not have as an error [explain]",
    async () => {
      const ws = fixture("17-rules");
      const result = await layers(ws, "repo", ["explain", "rule", "nobody"]);
      expect(result.code).toBe(1);
      expect(result.stdout).toBe("");
      expect(result.stderr).toContain(
        "error [explain] no rule named 'nobody' in the merged result",
      );
    },
  );
});
