import { describe, expect, it } from "vitest";
import { fixture, layers, readFile } from "../helpers";

describe("§13.5 skill override", () => {
  it("the repo skill replaces the personal one, and doctor reports the user-dir collision", async () => {
    const ws = fixture("05-skill-override");
    const build = await layers(ws, "repo", ["build"]);
    expect(build.code).toBe(0);
    const skill = readFile(ws, "repo/.claude/skills/stack-conventions/SKILL.md");
    expect(skill).toContain("REPO-STACK");
    expect(skill).not.toContain("PERSONAL-STACK");

    const account = `${ws.root}/account`;
    const doctor = await layers(ws, "repo", ["doctor"], { CLAUDE_CONFIG_DIR: account });
    expect(doctor.code).toBe(1);
    expect(doctor.stdout).toMatch(/collision.*skill.*stack-conventions/);
    expect(doctor.stdout).toContain(account);
  });
});
