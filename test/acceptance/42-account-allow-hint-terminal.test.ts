import { appendFileSync, chmodSync, cpSync, readFileSync, writeFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import {
  accountFolder,
  alz,
  hubEntry,
  PERSONAL,
  REPO_LAYER,
  remoteStack,
  sh,
  useStack,
  writeAllowed,
} from "../account-workspace";
import {
  AGENT_SESSION_MARKERS,
  addHubV3,
  allowed,
  allowedPath,
  atTerminal,
  CLAUDE_ONLY,
  hintLines,
  MAP_V3,
  NAMES,
  onTerminal,
  readAllowed,
  ready,
  rebuildAt,
  TERMINAL_CASE_MS,
  which,
} from "../allow-terminal";
import { referenceDrift } from "../drift";
import { snapshot } from "../helpers";

vi.setConfig({ testTimeout: TERMINAL_CASE_MS });

/**
 * The hint of `alz build` and `alz check` at a terminal, what applies only after allow, and how
 * the person's decision is escaped (R6.1b, TASK-55.7.17). A person at a terminal is a
 * pseudo-terminal (test/pty.ts); the cases without one are in file 32 and the reference copies in
 * test/reference/32.
 */

describe("§13.20 a layer's account applies only after allow, and the hint on build and check at a terminal (AC 2)", () => {
  it("starts a fresh clone on the default account and applies the named one after allow", async () => {
    const s = await ready();
    accountFolder(s, ".claude-client-a", ".codex-client-a-api");
    const fresh = await which(s);
    expect(fresh.stdout).toContain("claude: default (source: default)");
    expect(fresh.stdout).toContain("layer step skipped (no allowed.yaml)");
    await allowed(s);
    const after = await which(s);
    expect(after.code, after.stderr).toBe(0);
    expect(after.stdout).toContain("claude: client-a (source: layer)");
    expect(after.stdout).toContain("codex: client-a-api (source: layer)");
    expect(after.stdout).toContain("(read; this repository is allowed)");
  });

  it(
    "prints the allow command from build and check on a terminal only, and changes nothing else",
    async () => {
      const s = await ready();
      const quiet = await alz(s, ["build"]);
      const tree = snapshot(`${s.real}/repo`);
      const withTerminal = await atTerminal(s, ["build"]);
      expect(withTerminal.code, withTerminal.stderr).toBe(0);
      const lines = hintLines(withTerminal.stderr);
      expect(lines).toHaveLength(1);
      expect(lines[0]).toContain("claude=client-a, codex=client-a-api");
      expect(lines[0]).toContain('"client"');
      expect(lines[0]).toContain("it applies only after you run: alz account allow");
      expect(withTerminal.stdout).toBe(quiet.stdout);
      expect(snapshot(`${s.real}/repo`)).toEqual(tree);

      const check = await atTerminal(s, ["check"]);
      expect(check.code, check.stderr).toBe(0);
      expect(hintLines(check.stderr)).toHaveLength(1);
      expect(check.stdout).toBe((await alz(s, ["check"])).stdout);

      // No terminal, or any marker, prints nothing about it.
      expect(hintLines(quiet.stderr)).toEqual([]);
      expect(quiet.stderr).not.toContain("alz account allow");
      for (const marker of AGENT_SESSION_MARKERS) {
        for (const command of ["build", "check"]) {
          const marked = await atTerminal(s, [command], { [marker]: "" });
          expect(marked.code, marked.stderr).toBe(0);
          expect(marked.stderr, `${command} with ${marker}`).not.toContain("alz account allow");
        }
      }
      expect(snapshot(`${s.real}/repo`)).toEqual(tree);
    },
    3 * TERMINAL_CASE_MS,
  );

  it("prints no hint once allowed, and a ref bump keeps the approval", async () => {
    const s = await ready();
    await allowed(s);
    const build = await atTerminal(s, ["build"]);
    expect(hintLines(build.stderr)).toEqual([]);
    await rebuildAt(s, "v2");
    const bumped = await atTerminal(s, ["build"]);
    expect(hintLines(bumped.stderr)).toEqual([]);
    const check = await atTerminal(s, ["check"]);
    expect(hintLines(check.stderr)).toEqual([]);
    accountFolder(s, ".claude-client-a", ".codex-client-a-api");
    const resolved = await which(s);
    expect(resolved.code, resolved.stderr).toBe(0);
    expect(resolved.stdout).toContain("claude: client-a (source: layer)");
  });

  it("skips the account again when the map changes, with both values in the hint, until allowed again", async () => {
    const s = await ready();
    await allowed(s);
    addHubV3(s, MAP_V3);
    useStack(s, PERSONAL, ["path:../hub/layer", "v3"], REPO_LAYER);
    const build = await atTerminal(s, ["build"]);
    expect(build.code, build.stderr).toBe(0);
    const [line] = hintLines(build.stderr);
    expect(line).toContain("was claude=client-a, codex=client-a-api");
    expect(line).toContain("now claude=client-c, codex=client-a-api");
    expect(line).toContain("run: alz account allow");
    const check = await atTerminal(s, ["check"]);
    expect(hintLines(check.stderr)).toHaveLength(1);

    const closed = await which(s);
    expect(closed.code).toBe(1);
    expect(closed.stderr).toContain("now claude=client-c, codex=client-a-api");

    const again = await allowed(s, ["client-c", "client-a-api"]);
    expect(again.stdout).toContain("currently allowed: claude=client-a, codex=client-a-api");
    accountFolder(s, ".claude-client-c", ".codex-client-a-api");
    const applied = await which(s);
    expect(applied.code, applied.stderr).toBe(0);
    expect(applied.stdout).toContain("claude: client-c (source: layer)");
    expect(hintLines((await atTerminal(s, ["build"])).stderr)).toEqual([]);
  });

  it("skips the account when a target is removed from the map", async () => {
    const s = await ready();
    await allowed(s);
    addHubV3(s, CLAUDE_ONLY);
    await rebuildAt(s, "v3");
    const build = await atTerminal(s, ["build"]);
    expect(hintLines(build.stderr)[0]).toContain("now claude=client-a from");
    expect((await which(s)).code).toBe(1);
    await allowed(s, ["client-a"]);
    expect(readAllowed(s)).not.toContain("codex");
  });

  it("skips the account when the declaring source changes", async () => {
    const s = await ready();
    await allowed(s);
    cpSync(s.hub, `${s.ws.root}/hub2`, { recursive: true });
    useStack(s, PERSONAL, ["path:../hub2/layer", "v1"], REPO_LAYER);
    const build = await atTerminal(s, ["build"]);
    expect(build.code, build.stderr).toBe(0);
    const [line] = hintLines(build.stderr);
    expect(line).toContain(`from "${s.hubLayer}"`);
    expect(line).toContain(`from "${s.real}/hub2/layer"`);
    expect((await which(s)).code).toBe(1);
    await allowed(s);
    expect(readAllowed(s)).toContain(`source: ${s.real}/hub2/layer`);
  });

  it("names --remove when the layers no longer declare an account", async () => {
    const s = await ready();
    await allowed(s);
    addHubV3(s, "");
    await rebuildAt(s, "v3");
    const build = await atTerminal(s, ["build"]);
    expect(hintLines(build.stderr)[0]).toContain("run: alz account allow --remove");
  });

  it("names the reason in the hint when allowed.yaml cannot be trusted", async () => {
    const s = await ready();
    await allowed(s);
    chmodSync(allowedPath(s), 0o664);
    const build = await atTerminal(s, ["build"]);
    expect(hintLines(build.stderr)[0]).toContain(
      "allowed.yaml is ignored (group or others can write it)",
    );
    expect(hintLines(build.stderr)[0]).toContain("then run: alz account allow");
  });

  it("records a remote layer's URL without its ref", async () => {
    const s = await ready();
    const url = "https://example.invalid/hub.git";
    remoteStack(s, [
      {
        url: "https://example.invalid/first.git",
        ref: "v1",
        sha: "3".repeat(40),
        files: {
          "layer.yaml": "name: first\nkind: personal\nvisibility: public\nversion: 1.0.0\n",
          "instructions/10-x.md": "- first.\n",
        },
      },
      {
        url,
        ref: "v1",
        sha: "1".repeat(40),
        files: {
          "layer.yaml":
            "name: client\nkind: client\nvisibility: private\nversion: 0.1.0\naccount:\n  claude: client-a\n  codex: client-a-api\n",
          "instructions/10-x.md": "- client.\n",
        },
      },
    ]);
    const result = await allowed(s);
    expect(result.stdout).toContain('source as written: "git:https://example.invalid/hub.git@v1"');
    expect(result.stdout).toContain(`revision: ${"1".repeat(40)}`);
    expect(readAllowed(s)).toContain(`  source: ${url}\n`);
    expect(readAllowed(s)).not.toContain("@v1");
  });
});

describe("§13.20 the hint on build and check at a terminal, edge cases", () => {
  it("prints no hint from check when its own comparison failed, and exits 1", async () => {
    const s = await ready();
    const lock = JSON.parse(readFileSync(`${s.real}/repo/layer.lock`, "utf8")) as {
      files: Record<string, string>;
    };
    const [first] = Object.keys(lock.files);
    // Appended to, so the file still carries alz's generated header and differs from a fresh build.
    appendFileSync(`${s.real}/repo/${first}`, "tampered\n");
    const result = await atTerminal(s, ["check"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain(`error [check] ${first}`);
    expect(hintLines(result.stderr)).toEqual([]);
  });

  it("still prints the hint from check when only the git identity differs", async () => {
    const s = await ready();
    appendFileSync(
      `${s.real}/personal/layer.yaml`,
      "commit_identity:\n  name: Owner Example\n  email: owner@example.com\n",
    );
    expect((await alz(s, ["build"])).code).toBe(0);
    const repo = `${s.real}/repo`;
    sh(s.ws, repo, ["init", "--quiet", "--initial-branch=main"]);
    sh(s.ws, repo, ["config", "user.name", "Someone Else"]);
    sh(s.ws, repo, ["config", "user.email", "else@example.invalid"]);
    const result = await atTerminal(s, ["check"]);
    expect(result.code, result.stderr).toBe(1);
    expect(result.stderr).toContain("the git identity differs");
    expect(hintLines(result.stderr)).toHaveLength(1);
  });

  // A directory that touches an account folder stops the command (spec 6, 10.1: exit 1 with an error
  // diagnostic); the reference builds there and exits 0 (decision-30).
  referenceDrift(
    "TASK-55.7",
    "prints no hint for a repository inside the folder CLAUDE_CONFIG_DIR names, and stops with exit 1",
    async () => {
      const s = await ready();
      const result = await atTerminal(s, ["build"], { CLAUDE_CONFIG_DIR: `${s.real}/repo` });
      expect(result.code, result.stderr).toBe(1);
      expect(result.stderr).toContain("error [");
      expect(hintLines(result.stderr)).toEqual([]);
    },
  );

  referenceDrift(
    "TASK-55.7",
    "prints no hint for a repository that is $HOME, and stops with exit 1",
    async () => {
      const s = await ready();
      const result = await atTerminal(s, ["build"], { HOME: `${s.real}/repo` });
      expect(result.code, result.stderr).toBe(1);
      expect(result.stderr).toContain("error [");
      expect(hintLines(result.stderr)).toEqual([]);
    },
  );

  it("prints exactly the miss line when XDG_CACHE_HOME moved under mise and the layers name an account", async () => {
    const s = await ready();
    const result = await atTerminal(s, ["build"], {
      XDG_CACHE_HOME: `${s.real}/othercache`,
      __MISE_DIFF: "x",
    });
    expect(result.code, result.stderr).toBe(0);
    expect(hintLines(result.stderr)).toEqual([
      "note [account-allow] the account declaration was not checked: XDG_CACHE_HOME points away from ~/.cache and mise or direnv is active here",
    ]);
  });

  it("prints nothing about a cache miss when no layer names an account: off by default", async () => {
    const s = await ready();
    useStack(s, PERSONAL, REPO_LAYER);
    const result = await atTerminal(s, ["build"], {
      XDG_CACHE_HOME: `${s.real}/othercache`,
      __MISE_DIFF: "x",
    });
    expect(result.code, result.stderr).toBe(0);
    expect(hintLines(result.stderr)).toEqual([]);
  });

  it.each([
    ["a mapping at the top", "a: 1\n", "is not a list"],
    [
      "an ignored entry",
      "- root: relative/path\n  account:\n    claude: x\n  source: s\n",
      "has entry 1 that alz ignores",
    ],
  ])("names the problem and 'then run' for %s", async (_label, raw, reason) => {
    const s = await ready();
    writeAllowed(s, [], raw);
    const result = await atTerminal(s, ["build"]);
    const [line] = hintLines(result.stderr);
    expect(line).toContain(reason);
    expect(line).toContain("then run: alz account allow");
  });

  it("names two entries for one root by position", async () => {
    const s = await ready();
    const entry = hubEntry(s);
    writeAllowed(s, [entry, entry]);
    const result = await atTerminal(s, ["build"]);
    expect(hintLines(result.stderr)[0]).toContain("has entries 1, 2 that alz ignores");
  });
});

describe("§13.20 what the person decides on is escaped, not stripped, at a terminal", () => {
  const ZERO_WIDTH = "​";
  const CYRILLIC_A = "а";

  /** Copies the hub's layer under a folder whose name holds `mark`, and points the stack at it. */
  function pointAt(s: Awaited<ReturnType<typeof ready>>, mark: string) {
    const folder = `hub${mark}x`;
    cpSync(`${s.ws.root}/hub/layer`, `${s.ws.root}/${folder}/layer`, { recursive: true });
    writeFileSync(
      `${s.ws.root}/repo/layer.yaml`,
      `stack:\n  - source: path:../personal\n  - source: ${JSON.stringify(`path:../${folder}/layer`)}\n  - source: path:./.layer\ntargets: [claude]\n`,
    );
    return folder;
  }

  it.each([
    ["a zero-width character", ZERO_WIDTH, "\\u{200B}"],
    ["a Cyrillic homoglyph", CYRILLIC_A, "\\u{0430}"],
  ])(
    "shows %s in the source as \\u{XXXX}, in the display, the prompt and the hint",
    async (_label, mark, shown) => {
      const s = await ready();
      const folder = pointAt(s, mark);
      const build = await alz(s, ["build"]);
      expect(build.code, build.stderr).toBe(0);
      const session = onTerminal(s, ["account", "allow"]);
      for (const name of NAMES) {
        await session.waitForStderr(`Type ${name} to allow`);
        session.type(`${name}\n`);
      }
      const result = await session.done();
      expect(result.code, result.stderr).toBe(0);
      expect(result.stdout).toContain(`source as written: "path:../hub${shown}x/layer"`);
      expect(result.stderr).toContain(`hub${shown}x/layer`);
      expect(`${result.stdout}${result.stderr}`).not.toContain(mark);
      expect(readAllowed(s)).toContain("source:");

      // A change to a look-alike source is visible in the hint, never as identical text.
      const plainCopy = "hubx";
      cpSync(`${s.ws.root}/hub/layer`, `${s.ws.root}/${plainCopy}/layer`, { recursive: true });
      writeFileSync(
        `${s.ws.root}/repo/layer.yaml`,
        `stack:\n  - source: path:../personal\n  - source: path:../${plainCopy}/layer\n  - source: path:./.layer\ntargets: [claude]\n`,
      );
      const changed = await atTerminal(s, ["build"]);
      const [line] = hintLines(changed.stderr);
      expect(line).toContain(`${s.real}/${plainCopy}/layer`);
      expect(line).toContain(`${s.real}/hub${shown}x/layer`);
      expect(line).not.toContain(mark);
      expect(folder).toContain(mark);
    },
  );
});
