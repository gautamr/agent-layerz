import { describe, expect, it } from "vitest";
import { fixture, layers, readFile } from "../helpers";

describe("§7.5 command hooks and native events", () => {
  it("emits command hooks as written, with PROJECT_DIR mapped, and native events", async () => {
    const ws = fixture("19-hook-commands");
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(0);
    const { hooks } = JSON.parse(readFile(ws, "repo/.claude/settings.json"));
    expect(hooks.PreToolUse).toEqual([
      {
        matcher: "Edit|Write",
        hooks: [
          {
            type: "command",
            // biome-ignore lint/suspicious/noTemplateCurlyInString: Claude expands this literal itself.
            command: '"${CLAUDE_PROJECT_DIR}"/.harness/scripts/guard.sh',
            timeout: 10,
            onFailure: "block",
          },
        ],
      },
    ]);
    expect(hooks.Notification).toEqual([
      {
        matcher: "permission_prompt",
        hooks: [
          {
            type: "command",
            // biome-ignore lint/suspicious/noTemplateCurlyInString: Claude expands this literal itself.
            command: '"${CLAUDE_PROJECT_DIR}"/.harness/scripts/notify.sh',
          },
        ],
      },
    ]);
  });

  it("rejects a hook with both run and command", async () => {
    const ws = fixture("19-hook-commands");
    const result = await layers(ws, "bad", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("exactly one of 'run'");
  });
});
