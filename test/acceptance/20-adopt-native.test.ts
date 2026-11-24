import { describe, expect, it } from "vitest";
import { referenceDrift } from "../drift";
import { exists, fixture, layers, readFile, readTree } from "../helpers";

describe("§4.3 --adopt converts hand-written native config", () => {
  // The twin of fixture 20 whose architect has no native hooks: both binaries adopt it.
  it("turns .claude/, CLAUDE.md and .mcp.json into repo-layer sources with the same effect", async () => {
    const ws = fixture("20-adopt-native-plain");
    const before = JSON.parse(readFile(ws, "repo/.claude/settings.json"));
    const result = await layers(ws, "repo", ["build", "--adopt"]);
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);

    // The rebuilt settings keep every permission and hook.
    const after = JSON.parse(readFile(ws, "repo/.claude/settings.json"));
    expect(after.env).toEqual(before.env);
    expect(after.permissions).toEqual(before.permissions);
    expect(Object.keys(after.hooks).sort()).toEqual(Object.keys(before.hooks).sort());

    // Native agent fields survive; the concrete model resolves through the personal alias.
    const architect = readFile(ws, "repo/.claude/agents/architect.md");
    for (const line of ["model: opus", "effort: xhigh", "maxTurns: 60"]) {
      expect(architect).toContain(line);
    }
    expect(readFile(ws, "repo/.layer/agents/architect.md")).toContain("model: strong");
    expect(readFile(ws, "repo/.claude/agents/reviewer.md")).toContain("permissionMode: plan");

    // Directories and MCP servers land in the manifest, which keeps its comments.
    const manifest = readFile(ws, "repo/layer.yaml");
    expect(manifest).toContain("# the repo layer");
    expect(manifest).toContain("targets: [claude]");
    expect(manifest).toMatch(/add_dirs:\n\s+- \.\./);
    expect(Object.keys(JSON.parse(readFile(ws, "repo/.mcp.json")).mcpServers)).toEqual([
      "backlog",
      "tracker",
    ]);

    // CLAUDE.md keeps only what AGENTS.md does not already give it.
    expect(readFile(ws, "repo/.layer/targets/claude/CLAUDE.extra.md")).toBe(
      "@.harness/PROTOCOL.md\n\n- Reviewers are read-only.\n",
    );
    expect((await layers(ws, "repo", ["check"])).code).toBe(0);
  });

  // §4.3: hook scripts move to .layer/hooks/scripts/ and the hooks that run them use scripts/<path>.
  referenceDrift("TASK-55.6.14", "moves hand-written hook scripts to hooks/scripts/", async () => {
    const ws = fixture("20-adopt-native-plain");
    const result = await layers(ws, "repo", ["build", "--adopt"]);
    expect(result.code).toBe(0);
    expect(exists(ws, "repo/.layer/hooks/scripts/guard.sh")).toBe(true);
    expect(exists(ws, "repo/.claude/hooks/guard.sh")).toBe(false);
    const after = JSON.parse(readFile(ws, "repo/.claude/settings.json"));
    expect(after.hooks.PreToolUse[0].hooks[0].command).toBe(
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Claude expands this literal itself.
      '"${CLAUDE_PROJECT_DIR}/.claude/hooks/shop-hub/scripts/guard.sh"',
    );
  });

  // An agent's own hooks would run for every agent once moved into hooks/hooks.yaml, so the plan
  // is refused (lead decision of TASK-55.6.13).
  referenceDrift("TASK-55.6.13", "adopts nothing when an agent has native hooks", async () => {
    const ws = fixture("20-adopt-native");
    const before = readTree(`${ws.root}/repo`);
    const result = await layers(ws, "repo", ["build", "--adopt"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain(".claude/agents/architect.md");
    expect(result.stderr).toContain("claude.hooks");
    expect(result.stderr).toContain("nothing was adopted");
    expect(readTree(`${ws.root}/repo`)).toEqual(before);
  });

  it("adopts nothing when one file cannot be adopted safely", async () => {
    const ws = fixture("20-adopt-native");
    const before = readTree(`${ws.root}/secret`);
    const result = await layers(ws, "secret", ["build", "--adopt"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("TRACKER_TOKEN");
    expect(result.stderr).toContain("to a value");
    expect(result.stderr).toContain("nothing was adopted");
    expect(readTree(`${ws.root}/secret`)).toEqual(before);
  });
});
