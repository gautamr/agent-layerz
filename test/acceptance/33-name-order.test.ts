import { mkdirSync, writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { referenceDrift } from "../drift";
import { fixture, layers, readFile, type Workspace } from "../helpers";
import { contentHash, hashedFiles } from "../spec/content-hash";

const FIXTURE = "33-name-order";

/**
 * U+FF61 is a single UTF-16 unit, larger than U+D83D, the high surrogate of U+1F600: UTF-16 unit
 * order puts the emoji first, code point order puts it last. The TypeScript reference sorts by
 * UTF-16 unit, so these cases are reference drift (TASK-64, decision-30).
 */
const HALF_WIDTH = "\u{FF61}";
const EMOJI = "\u{1F600}";

/** A fixture whose personal layer has the exotic names; the committed fixture stays ASCII. */
function workspace(): Workspace {
  const ws = fixture(FIXTURE);
  const instructions = `${ws.root}/personal/instructions`;
  mkdirSync(instructions, { recursive: true });
  for (const [name, marker] of [
    ["z", "MARK-Z"],
    [HALF_WIDTH, "MARK-FF61"],
    [EMOJI, "MARK-1F600"],
  ] as const) {
    writeFileSync(`${instructions}/${name}.md`, `${marker}\n`);
  }
  return ws;
}

/**
 * The keys of `sandbox.ignoreViolations` in the text of a settings.json, in the order the text has
 * them. That mapping takes any text key, so it is where whole-number and exotic keys can sit in a
 * settings file that alz accepts (a settings key outside the typed table is an error).
 */
function ignoreViolationKeys(text: string): string[] {
  return [...text.matchAll(/^ {6}("(?:[^"\\]|\\.)*"):/gm)].map(
    (match) => JSON.parse(match[1] as string) as string,
  );
}

/** A settings.yaml whose `sandbox.ignoreViolations` has the given keys. */
function exoticSettings(keys: readonly string[]): string {
  const entries = keys.map((key) => `      "${key}": [x]`);
  return `claude:\n  sandbox:\n    ignoreViolations:\n${entries.join("\n")}\n`;
}

function personalLockEntry(ws: Workspace) {
  const lock = JSON.parse(readFile(ws, "repo/layer.lock")) as {
    layers: { name: string; contentHash: string }[];
  };
  return lock.layers.find((layer) => layer.name === "personal");
}

describe("§13.25 name order (§6)", () => {
  referenceDrift(
    "TASK-64",
    "orders instruction blocks by code point, not by UTF-16 unit",
    async () => {
      const ws = workspace();
      const result = await layers(ws, "repo", ["build"]);
      expect(result.code).toBe(0);
      const text = readFile(ws, "repo/AGENTS.md");
      const at = (marker: string) => text.indexOf(marker);
      expect(at("MARK-Z")).toBeGreaterThanOrEqual(0);
      expect(at("MARK-FF61")).toBeGreaterThan(at("MARK-Z"));
      expect(at("MARK-1F600")).toBeGreaterThan(at("MARK-FF61"));
    },
  );

  referenceDrift("TASK-64", "hashes the layer with its files in code point order", async () => {
    const ws = workspace();
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(0);
    const dir = `${ws.root}/personal`;
    const files = hashedFiles(dir);
    expect(files.indexOf(`instructions/${HALF_WIDTH}.md`)).toBeLessThan(
      files.indexOf(`instructions/${EMOJI}.md`),
    );
    expect(personalLockEntry(ws)?.contentHash).toBe(`sha256:${contentHash(dir)}`);
  });

  referenceDrift("TASK-64", "sorts JSON keys by code point, whole numbers included", async () => {
    const ws = workspace();
    writeFileSync(
      `${ws.root}/personal/settings.yaml`,
      exoticSettings([HALF_WIDTH, EMOJI, "10", "9", "-x", "a"]),
    );
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(0);
    // Read the key order from the text: JSON.parse would move "9" and "10" to the front.
    const keys = ignoreViolationKeys(readFile(ws, "repo/.claude/settings.json"));
    expect(keys).toEqual(["-x", "10", "9", "a", HALF_WIDTH, EMOJI]);
  });

  it("passes alz check after a build with such names", async () => {
    const ws = workspace();
    writeFileSync(`${ws.root}/personal/settings.yaml`, exoticSettings([EMOJI, "10", "9"]));
    expect((await layers(ws, "repo", ["build"])).code).toBe(0);
    const result = await layers(ws, "repo", ["check"]);
    expect(result.code).toBe(0);
  });
});
