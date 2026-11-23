import { execFileSync, spawnSync } from "node:child_process";
import {
  chmodSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { buildSync } from "esbuild";
import { describe, expect, it } from "vitest";
import {
  accountFolder,
  alz,
  built,
  envFor,
  hubEntry,
  PERSONAL,
  plain,
  REPO_LAYER,
  type Setup,
  sh,
  stateDir,
  useStack,
  writeAllowed,
} from "../account-workspace";
import { snapshot } from "../helpers";

/**
 * `alz account allow` runs only for a person at a terminal, and this suite spawns no
 * pseudo-terminal yet (R1.8, TASK-55.2.8, waits for R6). What the binary shows without one is
 * here: the refusals, the usage errors, the account that applies after an entry exists, and the
 * absence of the hint. The cases that need a person to answer are in
 * test/reference/32-account-allow.test.ts under their original names, and come back here in R1.8.
 */

const FIXTURE = "32-account-allow";
/** Written out, not imported, so a change to the list in src fails here (LAYERING-SPEC §7.13). */
const AGENT_SESSION_MARKERS = [
  "CLAUDECODE",
  "CLAUDE_CODE_CHILD_SESSION",
  "AI_AGENT",
  "CODEX_THREAD_ID",
  "CODEX_SESSION_ID",
  "CODEX_CI",
];

const ready = () => built("v1", FIXTURE);
/** A copy without git or a build: enough where alz refuses before it reads anything. */
const unbuilt = () => plain(FIXTURE);

const allow = (s: Setup, extra: Record<string, string> = {}, dir = "repo", args: string[] = []) =>
  alz(s, ["account", "allow", ...args], extra, dir);

const remove = (s: Setup, extra: Record<string, string> = {}) =>
  allow(s, extra, "repo", ["--remove"]);

const which = (s: Setup, extra: Record<string, string> = {}) => alz(s, ["account", "which"], extra);

const allowedPath = (s: Setup) => `${stateDir(s)}/allowed.yaml`;

const hintLines = (text: string) =>
  text.split("\n").filter((line) => line.startsWith("note [account-allow]"));

function addHubV3(s: Setup, account: string) {
  const head = "name: client\nkind: client\nvisibility: private\nversion: 0.1.0\n";
  writeFileSync(`${s.hub}/layer/layer.yaml`, `${head}${account}`);
  sh(s.ws, s.hub, ["add", "."]);
  sh(s.ws, s.hub, ["commit", "--quiet", "-m", "hub v3"]);
  sh(s.ws, s.hub, ["tag", "v3"]);
}

const MAP_V3 = "account:\n  claude: client-c\n  codex: client-a-api\n";
const CLAUDE_ONLY = "account:\n  claude: client-a\n";

async function rebuildAt(s: Setup, ref: string) {
  useStack(s, PERSONAL, ["path:../hub/layer", ref], REPO_LAYER);
  const result = await alz(s, ["build"]);
  expect(result.code, result.stderr).toBe(0);
}

describe("§13.20 who may run alz account allow: only a person at a terminal (AC 1)", () => {
  it.each([
    ["allow", false],
    ["allow --remove", true],
  ])("refuses `%s` without a terminal and changes nothing", async (_label, isRemove) => {
    const s = unbuilt();
    const before = snapshot(s.ws.home);
    const result = await (isRemove ? remove(s) : allow(s));
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("must be run by a person at a terminal");
    expect(result.stderr).toContain("standard input is not a terminal");
    expect(result.stderr).toContain("nothing was changed");
    expect(result.stdout).toBe("");
    expect(snapshot(s.ws.home)).toEqual(before);
    expect(existsSync(stateDir(s))).toBe(false);
  });

  const MARKER_VALUE = "marker-value-4f1c9";
  const cases = AGENT_SESSION_MARKERS.flatMap((marker) =>
    [false, true].flatMap((isRemove) =>
      ["", MARKER_VALUE].map((value) => [marker, isRemove, value] as const),
    ),
  );
  it.each(cases)(
    "refuses with %s set (remove: %s, value %j) without a terminal, naming the marker only",
    async (marker, isRemove, value) => {
      const s = unbuilt();
      const before = snapshot(s.ws.home);
      const result = await (isRemove ? remove : allow)(s, { [marker]: value });
      expect(result.code).toBe(1);
      expect(result.stderr).toContain(marker);
      expect(result.stderr).toContain("an agent session is running here");
      expect(result.stderr).toContain("standard input is not a terminal");
      expect(result.stderr + result.stdout).not.toContain(MARKER_VALUE);
      expect(snapshot(s.ws.home)).toEqual(before);
      const ide = "run alz account allow in a terminal window outside the IDE";
      if (marker === "CLAUDECODE") expect(result.stderr).toContain(ide);
      else expect(result.stderr).not.toContain(ide);
    },
  );

  it("names every reason that applies, and markers by name only", async () => {
    const s = unbuilt();
    const result = await allow(s, { CODEX_CI: "1", AI_AGENT: "x" });
    expect(result.stderr).toContain("standard input is not a terminal");
    expect(result.stderr).toContain("AI_AGENT, CODEX_CI set");
  });

  it.each([["--yes"], ["-y"], ["--force"], ["extra"]])(
    "has no way to answer for the person: %s is a usage error",
    async (arg) => {
      const s = unbuilt();
      const before = snapshot(s.ws.home);
      const result = await allow(s, {}, "repo", [arg]);
      expect(result.code).toBe(2);
      expect(result.stderr).toContain("usage: alz");
      expect(result.stderr).not.toContain("must be run by a person at a terminal");
      expect(snapshot(s.ws.home)).toEqual(before);
    },
  );
});

describe("§13.20 states of allowed.yaml that alz account allow refuses, as account which shows them (decision-20)", () => {
  const abs = (s: Setup) => `${s.real}/repo-b`;
  const entryOf = (s: Setup, account: Record<string, string>, root = abs(s)) => ({
    root,
    account,
    source: "s",
  });

  const states: [string, (s: Setup) => Record<string, string>, string][] = [
    [
      "a file that group can write",
      (s) => {
        writeAllowed(s, [hubEntry(s)]);
        chmodSync(allowedPath(s), 0o660);
        return {};
      },
      "group or others can write it",
    ],
    [
      "a file that others can write",
      (s) => {
        writeAllowed(s, [hubEntry(s)]);
        chmodSync(allowedPath(s), 0o606);
        return {};
      },
      "group or others can write it",
    ],
    [
      "a directory that group can write",
      (s) => {
        writeAllowed(s, [hubEntry(s)]);
        chmodSync(stateDir(s), 0o770);
        return {};
      },
      "can be written by group or others",
    ],
    [
      "XDG_STATE_HOME inside the repository",
      (s) => ({ XDG_STATE_HOME: `${s.real}/repo/state` }),
      "it lies inside the current repository",
    ],
    [
      "XDG_STATE_HOME inside an account folder",
      (s) => ({ XDG_STATE_HOME: `${s.ws.home}/.claude-x/state` }),
      "it lies inside an account folder",
    ],
    [
      "a folder in place of the file",
      (s) => {
        mkdirSync(`${stateDir(s)}/allowed.yaml`, { recursive: true, mode: 0o700 });
        return {};
      },
      "not a regular file",
    ],
    [
      "a FIFO in place of the file",
      (s) => {
        mkdirSync(stateDir(s), { recursive: true, mode: 0o700 });
        execFileSync("mkfifo", [allowedPath(s)]);
        return {};
      },
      "not a regular file",
    ],
    [
      "a file over 1 MiB",
      (s) => {
        writeAllowed(s, [], `#${"x".repeat(1024 * 1024)}\n`);
        return {};
      },
      "at most 1 MiB",
    ],
    [
      "invalid YAML",
      (s) => {
        writeAllowed(s, [], "root: [\n");
        return {};
      },
      "not valid YAML",
    ],
    [
      "a mapping at the top",
      (s) => {
        writeAllowed(s, [], "a: 1\n");
        return {};
      },
      "is not a list",
    ],
    [
      "an entry with a relative root",
      (s) => {
        writeAllowed(s, [entryOf(s, { claude: "x" }, "relative/path")]);
        return {};
      },
      "has entry 1 that alz ignores",
    ],
    [
      "an entry with a bad account name",
      (s) => {
        writeAllowed(s, [entryOf(s, { claude: "Bad Name" })]);
        return {};
      },
      "has entry 1 that alz ignores",
    ],
    [
      "two entries for one root",
      (s) => {
        writeAllowed(s, [entryOf(s, { claude: "x" }), entryOf(s, { claude: "x" })]);
        return {};
      },
      "has entries 1, 2 that alz ignores",
    ],
  ];

  /** The states whose reason `alz account which` prints: the others show only in the reference suite. */
  const shownByWhich = states.filter(([label]) =>
    [
      "a file that group can write",
      "a file that others can write",
      "a directory that group can write",
      "XDG_STATE_HOME inside an account folder",
      "a folder in place of the file",
      "a FIFO in place of the file",
      "a file over 1 MiB",
      "invalid YAML",
    ].includes(label),
  );

  it.each(shownByWhich)(
    "shows %s as ignored with the reason, and the default account applies",
    async (_label, prepare, reason) => {
      const s = await ready();
      const extra = prepare(s);
      const result = await which(s, extra);
      expect(result.stdout + result.stderr).toContain(reason);
      expect(result.stdout).toContain("claude: default (source: default)");
    },
  );

  it.each(states.map((state) => [state[0], state[1]] as const))(
    "leaves %s untouched when alz account allow is run without a terminal",
    async (_label, prepare) => {
      const s = unbuilt();
      const extra = prepare(s);
      const before = { home: snapshot(s.ws.home), repo: snapshot(`${s.real}/repo`) };
      for (const run of [allow, remove]) {
        const result = await run(s, extra);
        expect(result.code).toBe(1);
        expect(result.stderr).toContain("standard input is not a terminal");
      }
      expect(snapshot(s.ws.home)).toEqual(before.home);
      expect(snapshot(`${s.real}/repo`)).toEqual(before.repo);
    },
  );
});

describe("§13.20 a layer's account applies only for an allowed repository, and the hint is for a terminal only (AC 2)", () => {
  it("starts a fresh clone on the default account until the repository is allowed", async () => {
    const s = await ready();
    accountFolder(s, ".claude-client-a", ".codex-client-a-api");
    const fresh = await which(s);
    expect(fresh.stdout).toContain("claude: default (source: default)");
    expect(fresh.stdout).toContain("layer step skipped (no allowed.yaml)");
    writeAllowed(s, [hubEntry(s)]);
    const after = await which(s);
    expect(after.code, after.stderr).toBe(0);
    expect(after.stdout).toContain("claude: client-a (source: layer)");
    expect(after.stdout).toContain("codex: client-a-api (source: layer)");
    expect(after.stdout).toContain("(read; this repository is allowed)");
  });

  it("keeps the approval across a ref bump, and skips the account when the map changes", async () => {
    const s = await ready();
    accountFolder(s, ".claude-client-a", ".codex-client-a-api", ".claude-client-c");
    writeAllowed(s, [hubEntry(s)]);
    await rebuildAt(s, "v2");
    const bumped = await which(s);
    expect(bumped.code, bumped.stderr).toBe(0);
    expect(bumped.stdout).toContain("claude: client-a (source: layer)");

    addHubV3(s, MAP_V3);
    await rebuildAt(s, "v3");
    const closed = await which(s);
    expect(closed.code).toBe(1);
    expect(closed.stderr).toContain("now claude=client-c, codex=client-a-api");
    writeAllowed(s, [hubEntry(s, { account: { claude: "client-c", codex: "client-a-api" } })]);
    const applied = await which(s);
    expect(applied.code, applied.stderr).toBe(0);
    expect(applied.stdout).toContain("claude: client-c (source: layer)");
  });

  it("skips the account when a target is removed from the map, or the declaring source changes", async () => {
    const removed = await ready();
    writeAllowed(removed, [hubEntry(removed)]);
    addHubV3(removed, CLAUDE_ONLY);
    await rebuildAt(removed, "v3");
    expect((await which(removed)).code).toBe(1);

    const moved = await ready();
    writeAllowed(moved, [hubEntry(moved)]);
    cpSync(moved.hub, `${moved.ws.root}/hub2`, { recursive: true });
    useStack(moved, PERSONAL, ["path:../hub2/layer", "v1"], REPO_LAYER);
    expect((await alz(moved, ["build"])).code).toBe(0);
    const result = await which(moved);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain(`from "${moved.real}/hub2/layer"`);
  });

  it("prints no allow hint from build and check without a terminal, with or without a marker, and changes nothing else", async () => {
    const s = await ready();
    const quiet = await alz(s, ["build"]);
    expect(quiet.code, quiet.stderr).toBe(0);
    const tree = snapshot(`${s.real}/repo`);
    expect(hintLines(quiet.stderr)).toEqual([]);
    expect(quiet.stderr).not.toContain("alz account allow");
    for (const marker of [undefined, ...AGENT_SESSION_MARKERS]) {
      for (const command of ["build", "check"]) {
        const result = await alz(s, [command], marker === undefined ? {} : { [marker]: "" });
        expect(result.code, result.stderr).toBe(0);
        expect(result.stderr, `${command} with ${marker}`).not.toContain("alz account allow");
      }
    }
    expect(snapshot(`${s.real}/repo`)).toEqual(tree);
  });

  it("prints no allow hint whatever allowed.yaml holds, when there is no terminal", async () => {
    const s = await ready();
    for (const raw of ["a: 1\n", "root: [\n", "[]\n"]) {
      writeAllowed(s, [], raw);
      for (const command of ["build", "check"]) {
        const result = await alz(s, [command]);
        expect(hintLines(result.stderr), `${command} with ${JSON.stringify(raw)}`).toEqual([]);
      }
    }
  });
});

describe("§13.20 where allowed.yaml may live, and what a link may point at (decision-20 amendment)", () => {
  it("shows a state directory inside a git work tree as ignored, and allow changes nothing without a terminal", async () => {
    const s = await ready();
    writeAllowed(s, []);
    mkdirSync(`${s.ws.home}/.local/state/.git`);
    const before = snapshot(s.ws.home);
    const listed = await which(s);
    expect(listed.stdout + listed.stderr).toContain(
      "it lies inside a git work tree (~/.local/state/.git)",
    );
    for (const run of [allow, remove]) {
      const refused = await run(s);
      expect(refused.code).toBe(1);
      expect(refused.stderr).toContain("standard input is not a terminal");
    }
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  it("shows a link placed inside a git work tree as ignored even when its target is outside every work tree", async () => {
    const s = await ready();
    const tree = `${s.ws.home}/tree`;
    const outside = `${s.ws.home}/outside/state`;
    mkdirSync(`${tree}/.git`, { recursive: true, mode: 0o700 });
    chmodSync(tree, 0o700);
    mkdirSync(`${outside}/agent-layerz`, { recursive: true, mode: 0o700 });
    chmodSync(`${s.ws.home}/outside`, 0o700);
    chmodSync(outside, 0o700);
    chmodSync(`${outside}/agent-layerz`, 0o700);
    writeFileSync(`${outside}/agent-layerz/allowed.yaml`, "[]\n", { mode: 0o600 });
    symlinkSync(outside, `${tree}/state`);
    const extra = { XDG_STATE_HOME: `${tree}/state` };
    const before = snapshot(s.ws.home);
    for (const run of [allow, remove]) {
      const result = await run(s, extra);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("standard input is not a terminal");
    }
    expect(snapshot(s.ws.home)).toEqual(before);
    const listed = await which(s, extra);
    expect(listed.stdout).toContain("ignored: it lies inside a git work tree (~/tree/.git)");
  });
});

describe("§13.20 the real wiring: a pipe is not a terminal", () => {
  const out = mkdtempSync(join(tmpdir(), "alz-bin-"));
  const bundle = `${out}/alz.mjs`;
  /** R1.7 replaces this bundle of the TypeScript entry point with the binary under test (ALZ_BIN). */
  const bundled = () => {
    if (!existsSync(bundle)) {
      buildSync({
        entryPoints: [join(import.meta.dirname, "../../src/cli/bin.ts")],
        bundle: true,
        platform: "node",
        format: "esm",
        target: "node22",
        outfile: bundle,
        banner: {
          js: 'import{createRequire}from"node:module";const require=createRequire(import.meta.url);',
        },
        logLevel: "silent",
      });
    }
    return bundle;
  };

  const wirings: [string, { input?: string; stdio?: ("ignore" | "pipe")[] }][] = [
    ["a pipe that carries the account names", { input: "client-a\nclient-a-api\n" }],
    ["no standard input at all", { stdio: ["ignore", "pipe", "pipe"] }],
  ];

  it.each(wirings)("refuses with %s and writes nothing", async (_label, how) => {
    const s = unbuilt();
    const env = { HOME: s.ws.home, PATH: process.env.PATH ?? "", ...envFor(s) };
    for (const args of [
      ["account", "allow"],
      ["account", "allow", "--remove"],
    ]) {
      const result = spawnSync(process.execPath, [bundled(), ...args], {
        cwd: `${s.ws.root}/repo`,
        env,
        encoding: "utf8",
        ...how,
      });
      expect(result.status).toBe(1);
      expect(result.stderr).toContain("standard input is not a terminal");
      expect(result.stderr).not.toContain("agent session");
      expect(existsSync(stateDir(s))).toBe(false);
    }
  });
});
