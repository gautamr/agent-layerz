import { describe, expect, it } from "vitest";
import { exists, fixture, layers, readFile } from "../helpers";

describe("§13.8 locked hook", () => {
  it("disabling a locked hook fails", async () => {
    const ws = fixture("08-locked-hook");
    const result = await layers(ws, "repo-locked", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toMatch(/error.*block-secrets.*locked/);
    expect(result.stderr).toContain("shop-api");
  });

  it("disabling an unlocked hook works", async () => {
    const ws = fixture("08-locked-hook");
    const result = await layers(ws, "repo-unlocked", ["build"]);
    expect(result.code).toBe(0);
    const settings = JSON.parse(readFile(ws, "repo-unlocked/.claude/settings.json"));
    expect(Object.keys(settings.hooks)).toEqual(["PreToolUse"]);
    const [group] = settings.hooks.PreToolUse;
    expect(group.matcher).toBe("Read");
    expect(group.hooks[0]).toMatchObject({
      type: "command",
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Claude expands this literal itself.
      command: '"${CLAUDE_PROJECT_DIR}/.claude/hooks/personal/scripts/block-secrets.sh"',
      if: "Read(.env*)",
      onFailure: "block",
    });
    expect(exists(ws, "repo-unlocked/.claude/hooks/personal/scripts/block-secrets.sh")).toBe(true);
    expect(exists(ws, "repo-unlocked/.claude/hooks/personal/scripts/format.sh")).toBe(false);
  });
});
