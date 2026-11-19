import { execFileSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { fixture, layers, readTree } from "../helpers";

// Re-specified per docs/target-facts.md §8.11: an empty CLAUDE_CONFIG_DIR has no login, so the
// model-driven listing only runs with ALZ_SMOKE_CLAUDE=1 and an API key in the environment.
describe("§13.12 account independence", () => {
  it("build output and doctor results are identical under two empty account dirs", async () => {
    const results = [];
    for (const name of ["account-a", "account-b"]) {
      const ws = fixture("01-determinism");
      const account = `${ws.root}/${name}`;
      mkdirSync(account);
      const env = { CLAUDE_CONFIG_DIR: account };
      const build = await layers(ws, "repo", ["build"], env);
      expect(build.code).toBe(0);
      const doctor = await layers(ws, "repo", ["doctor"], env);
      expect(doctor.code).toBe(0);
      expect(doctor.stdout).toContain("no collisions");
      expect(doctor.stdout).not.toMatch(/collision:/);
      results.push({
        tree: readTree(`${ws.root}/repo`),
        doctor: doctor.stdout.replaceAll(ws.root, "<ws>").replaceAll(name, "<account>"),
      });
    }
    expect(results[1]).toEqual(results[0]);
  });

  const smoke = process.env.ALZ_SMOKE_CLAUDE === "1" && Boolean(process.env.ANTHROPIC_API_KEY);
  it.skipIf(!smoke)("Claude lists the same agents and skills under both account dirs", async () => {
    const listings = [];
    for (const name of ["account-a", "account-b"]) {
      const ws = fixture("01-determinism");
      const account = `${ws.root}/${name}`;
      mkdirSync(account);
      expect((await layers(ws, "repo", ["build"])).code).toBe(0);
      const out = execFileSync(
        "claude",
        ["-p", "Reply with OK.", "--output-format", "stream-json", "--verbose", "--max-turns", "1"],
        { cwd: `${ws.root}/repo`, env: { ...process.env, CLAUDE_CONFIG_DIR: account } },
      ).toString();
      const init = out
        .split("\n")
        .filter(Boolean)
        .map((line) => JSON.parse(line))
        .find((event) => event.type === "system" && event.subtype === "init");
      listings.push({ agents: [...init.agents].sort(), skills: [...init.skills].sort() });
    }
    expect(listings[1]).toEqual(listings[0]);
    expect(listings[0]?.agents).toContain("reviewer");
  });
});
