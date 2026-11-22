import { execFileSync, spawn, spawnSync } from "node:child_process";
import {
  appendFileSync,
  chmodSync,
  constants,
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  readlinkSync,
  realpathSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { PassThrough } from "node:stream";
import { buildSync } from "esbuild";
import { describe, expect, it, vi } from "vitest";
import { createTerminal, type Terminal } from "../../src/cli/terminal";
import { parseYamlLimited } from "../../src/core/yaml";
import { updateAllowed } from "../../src/resolve/account-allow-writer";
import {
  accountFolder,
  alz,
  built,
  envFor,
  hubEntry,
  PERSONAL,
  REPO_LAYER,
  remoteStack,
  type Setup,
  sh,
  stateDir,
  useStack,
  writeAllowed,
} from "../account-workspace";
import { type TraceCall, tracedAsync } from "../fs-trace";
import { importedSpecifiers, SRC, valueImports } from "../import-graph";

vi.mock("node:fs", async (importOriginal) =>
  (await import("../fs-trace")).intercept(
    "fs",
    (await importOriginal()) as Record<string, unknown>,
  ),
);
vi.mock("node:child_process", async (importOriginal) =>
  (await import("../fs-trace")).intercept(
    "child_process",
    (await importOriginal()) as Record<string, unknown>,
  ),
);

const FIXTURE = "32-account-allow";
const HEADER = "# written by alz account allow; deleting the file withdraws every approval\n";
const NAMES = ["client-a", "client-a-api"];
/** Written out, not imported, so a change to the list in src fails here (LAYERING-SPEC §7.13). */
const AGENT_SESSION_MARKERS = [
  "CLAUDECODE",
  "CLAUDE_CODE_CHILD_SESSION",
  "AI_AGENT",
  "CODEX_THREAD_ID",
  "CODEX_SESSION_ID",
  "CODEX_CI",
];

/** A fake terminal: the answers are given in order, and every prompt is recorded. */
function terminal(...answers: (string | undefined)[]): { term: Terminal; asked: string[] } {
  const asked: string[] = [];
  return {
    asked,
    term: {
      ask: async (prompt) => {
        asked.push(prompt);
        return answers.shift();
      },
    },
  };
}

const ready = () => built("v1", FIXTURE);

const allow = (
  s: Setup,
  term: Terminal | undefined,
  extra: Record<string, string> = {},
  dir = "repo",
  args: string[] = [],
) => alz(s, ["account", "allow", ...args], extra, dir, term);

const remove = (s: Setup, term: Terminal | undefined, extra: Record<string, string> = {}) =>
  allow(s, term, extra, "repo", ["--remove"]);

/** Everything under `dir` without following links: kind, mode, content or link target. */
function snapshot(dir: string): Record<string, string> {
  const out: Record<string, string> = {};
  const walk = (current: string) => {
    let names: string[];
    try {
      names = readdirSync(current);
    } catch {
      return;
    }
    for (const name of names) {
      const full = join(current, name);
      const stat = lstatSync(full);
      const rel = full.slice(dir.length);
      const mode = (stat.mode & 0o7777).toString(8);
      if (stat.isSymbolicLink()) out[rel] = `link:${readlinkSync(full)}`;
      else if (stat.isDirectory()) {
        out[rel] = `dir:${mode}`;
        walk(full);
      } else if (stat.isFile()) out[rel] = `file:${mode}:${readFileSync(full, "utf8")}`;
      else out[rel] = `other:${mode}`;
    }
  };
  walk(dir);
  return out;
}

const allowedPath = (s: Setup) => `${stateDir(s)}/allowed.yaml`;
const readAllowed = (s: Setup) => readFileSync(allowedPath(s), "utf8");
const modeOf = (path: string) => (statSync(path).mode & 0o777).toString(8);

/** The state directory holds nothing but allowed.yaml: no lock, no temporary file. */
function expectNoLeftovers(s: Setup) {
  expect(readdirSync(stateDir(s)).filter((name) => name !== "allowed.yaml")).toEqual([]);
}

/** Any C0, DEL or C1 character other than a newline. */
const hasControl = (text: string): boolean =>
  [...text].some((char) => {
    const code = char.codePointAt(0) ?? 0;
    return code !== 0x0a && (code <= 0x1f || (code >= 0x7f && code <= 0x9f));
  });

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

/** Runs allow to the end with the right answers. */
async function allowed(s: Setup, names = NAMES) {
  const t = terminal(...names);
  const result = await allow(s, t.term);
  expect(result.code, result.stderr).toBe(0);
  return result;
}

describe("§13.20 who may run alz account allow: only a person at a terminal (AC 1)", () => {
  it.each([
    ["allow", false],
    ["allow --remove", true],
  ])(
    "refuses `%s` without a terminal before any file system call, and changes nothing",
    async (_label, isRemove) => {
      const s = await ready();
      const before = snapshot(s.ws.home);
      const { result, calls } = await tracedAsync(() =>
        isRemove ? remove(s, undefined) : allow(s, undefined),
      );
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("must be run by a person at a terminal");
      expect(result.stderr).toContain("standard input is not a terminal");
      expect(result.stdout).toBe("");
      expect(calls).toEqual([]);
      expect(snapshot(s.ws.home)).toEqual(before);
    },
  );

  const MARKER_VALUE = "marker-value-4f1c9";
  const cases = AGENT_SESSION_MARKERS.flatMap((marker) =>
    [false, true].flatMap((isRemove) =>
      ["", MARKER_VALUE].map((value) => [marker, isRemove, value] as const),
    ),
  );
  it.each(cases)(
    "refuses with %s set (remove: %s, value %j) even on a terminal, without asking",
    async (marker, isRemove, value) => {
      const s = await ready();
      const t = terminal(...NAMES);
      const before = snapshot(s.ws.home);
      const result = await (isRemove ? remove : allow)(s, t.term, { [marker]: value });
      expect(result.code).toBe(1);
      expect(result.stderr).toContain(marker);
      expect(result.stderr).toContain("an agent session is running here");
      expect(result.stderr + result.stdout).not.toContain(MARKER_VALUE);
      expect(t.asked).toEqual([]);
      expect(snapshot(s.ws.home)).toEqual(before);
      const ide = "run alz account allow in a terminal window outside the IDE";
      if (marker === "CLAUDECODE") expect(result.stderr).toContain(ide);
      else expect(result.stderr).not.toContain(ide);
    },
  );

  it("names every reason that applies, and markers by name only", async () => {
    const s = await ready();
    const result = await allow(s, undefined, { CODEX_CI: "1", AI_AGENT: "x" });
    expect(result.stderr).toContain("standard input is not a terminal");
    expect(result.stderr).toContain("AI_AGENT, CODEX_CI set");
  });

  it.each([["--yes"], ["-y"], ["--force"], ["extra"]])(
    "has no way to answer for the person: %s is a usage error",
    async (arg) => {
      const s = await ready();
      const t = terminal(...NAMES);
      const before = snapshot(s.ws.home);
      const result = await allow(s, t.term, {}, "repo", [arg]);
      expect(result.code).toBe(2);
      expect(result.stderr).toContain("usage: alz");
      expect(t.asked).toEqual([]);
      expect(snapshot(s.ws.home)).toEqual(before);
    },
  );
});

describe("§13.20 the prompt and what is written (AC 1, AC 2)", () => {
  it("shows the root, the accounts, the layer, its source, ref and revision, and the recorded source", async () => {
    const s = await ready();
    accountFolder(s, ".claude-client-a");
    const t = terminal(...NAMES);
    const result = await allow(s, t.term);
    expect(result.code, result.stderr).toBe(0);
    const text = result.stdout;
    expect(text).toContain(`repository: ${s.real}/repo\n`);
    expect(text).toContain("claude: client-a (the account folder exists)");
    expect(text).toContain("codex: client-a-api (the account folder does not exist yet)");
    expect(text).toContain('declared by the layer "client", stack entry 2');
    expect(text).toContain('source as written: "path:../hub/layer"');
    expect(text).toContain('ref: "v1"');
    expect(text).toContain(`revision: ${s.v1}`);
    expect(text).toContain(`recorded source: "${s.hubLayer}"`);
    expect(text).toContain("will use these accounts' logins and MCP connections");
    expect(hasControl(text + result.stderr)).toBe(false);
    expect(hasControl("a\u001b[31mb")).toBe(true);
    // The answer is asked for once per distinct name, in target order; the name is in the prompt only.
    // The prompt itself names the root and the source, so it is never asked blind.
    expect(t.asked).toEqual([
      `Type client-a to allow it for "${s.real}/repo" (source "${s.hubLayer}"): `,
      `Type client-a-api to allow it for "${s.real}/repo" (source "${s.hubLayer}"): `,
    ]);
  });

  it.each([
    ["y", ["y", "y"]],
    ["yes", ["yes", "yes"]],
    ["an empty line", ["", ""]],
    ["a wrong name", ["client-b", "client-a-api"]],
    ["the end of input", [undefined]],
    ["a wrong second answer", ["client-a", "y"]],
    ["a name in another case", ["CLIENT-A", "client-a-api"]],
    ["a name inside other text", ["client-a now", "client-a-api"]],
  ])("writes nothing for %s", async (_label, answers) => {
    const s = await ready();
    const before = snapshot(s.ws.home);
    const t = terminal(...answers);
    const result = await allow(s, t.term);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("not confirmed; nothing changed");
    expect(snapshot(s.ws.home)).toEqual(before);
    expect(existsSync(stateDir(s))).toBe(false);
  });

  it("accepts the exact names, with spaces and tabs around them, and writes the exact file", async () => {
    const s = await ready();
    const t = terminal(" client-a\t", "client-a-api  ");
    const result = await allow(s, t.term);
    expect(result.code, result.stderr).toBe(0);
    expect(readAllowed(s)).toBe(
      `${HEADER}- root: ${s.real}/repo\n  account:\n    claude: client-a\n    codex: client-a-api\n  source: ${s.hubLayer}\n`,
    );
    expect(modeOf(allowedPath(s))).toBe("600");
    expect(modeOf(stateDir(s))).toBe("700");
    // The directories alz created on the way are private too.
    expect(modeOf(`${s.ws.home}/.local/state`)).toBe("700");
    expect(modeOf(`${s.ws.home}/.local`)).toBe("700");
    expectNoLeftovers(s);
    expect(result.stdout).toContain(
      `allowed: ${s.real}/repo uses claude=client-a, codex=client-a-api`,
    );
    expect(result.stdout).toContain("alz account init client-a");
  });

  it("records the real root when run from a subdirectory or from a symbolic link", async () => {
    const s = await ready();
    symlinkSync(`${s.real}/repo`, `${s.ws.root}/link-to-repo`);
    const t = terminal(...NAMES);
    const result = await allow(s, t.term, {}, "link-to-repo/sub/deeper");
    expect(result.code, result.stderr).toBe(0);
    expect(readAllowed(s)).toContain(`- root: ${s.real}/repo\n`);
    expect(readAllowed(s)).not.toContain("link-to-repo");
  });

  it("replaces the entry for the same root and keeps every other one, sorted, unresolved roots included", async () => {
    const s = await ready();
    const other = { root: `${s.real}/repo-b`, account: { claude: "b-account" }, source: "/x/b" };
    const gone = {
      root: "/zzz/does/not/exist",
      account: { codex: "z" },
      source: "https://example.invalid/z.git",
    };
    const old = hubEntry(s, { account: { claude: "old-name" }, source: "/old/source" });
    writeAllowed(s, [gone, old, other]);
    const result = await allowed(s);
    const entries = parseYamlLimited(readAllowed(s)) as { root: string }[];
    expect(entries.map((entry) => entry.root)).toEqual(
      [`${s.real}/repo`, `${s.real}/repo-b`, "/zzz/does/not/exist"].sort(),
    );
    expect(entries.find((entry) => entry.root === `${s.real}/repo`)).toEqual(hubEntry(s));
    expect(entries.find((entry) => entry.root === other.root)).toEqual(other);
    expect(entries.find((entry) => entry.root === gone.root)).toEqual(gone);
    expect(result.stdout).toContain("allowed:");
    expectNoLeftovers(s);
  });

  it("records only the targets the declaration names", async () => {
    const s = await ready();
    addHubV3(s, CLAUDE_ONLY);
    await rebuildAt(s, "v3");
    await allowed(s, ["client-a"]);
    expect(readAllowed(s)).toBe(
      `${HEADER}- root: ${s.real}/repo\n  account:\n    claude: client-a\n  source: ${s.hubLayer}\n`,
    );
  });

  it("keeps a linked allowed.yaml a link and writes its target; a dangling link refuses", async () => {
    const s = await ready();
    const store = `${s.ws.home}/store`;
    mkdirSync(store, { mode: 0o700 });
    writeFileSync(`${store}/target.yaml`, "[]\n", { mode: 0o600 });
    mkdirSync(stateDir(s), { recursive: true, mode: 0o700 });
    symlinkSync(`${store}/target.yaml`, allowedPath(s));
    await allowed(s);
    expect(lstatSync(allowedPath(s)).isSymbolicLink()).toBe(true);
    expect(readFileSync(`${store}/target.yaml`, "utf8")).toContain(`- root: ${s.real}/repo\n`);
    expect(readdirSync(store)).toEqual(["target.yaml"]);

    const dangling = await ready();
    mkdirSync(stateDir(dangling), { recursive: true, mode: 0o700 });
    symlinkSync(`${dangling.ws.home}/nowhere.yaml`, allowedPath(dangling));
    const before = snapshot(dangling.ws.home);
    const t = terminal(...NAMES);
    const result = await allow(dangling, t.term);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("it is a link that does not resolve");
    expect(t.asked).toEqual([]);
    expect(snapshot(dangling.ws.home)).toEqual(before);
  });

  it("is already allowed when the entry is the same: exit 0, no prompt, the file untouched", async () => {
    const s = await ready();
    await allowed(s);
    const before = snapshot(s.ws.home);
    const inode = statSync(allowedPath(s)).ino;
    const t = terminal(...NAMES);
    const result = await allow(s, t.term);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("already allowed");
    expect(t.asked).toEqual([]);
    expect(statSync(allowedPath(s)).ino).toBe(inode);
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  it("refuses when the declaration changes while the person confirms", async () => {
    const s = await ready();
    addHubV3(s, MAP_V3);
    const asked: string[] = [];
    const term: Terminal = {
      ask: async (prompt) => {
        asked.push(prompt);
        if (asked.length === 1) {
          await rebuildAt(s, "v3");
        }
        return NAMES[asked.length - 1];
      },
    };
    const before = snapshot(s.ws.home);
    const result = await allow(s, term);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("the declaration changed while you were confirming");
    expect(snapshot(s.ws.home)).toEqual(before);
  });
});

describe("§13.20 states alz account allow refuses, and changes nothing (decision-20)", () => {
  const abs = (s: Setup) => `${s.real}/repo-b`;

  /** A state that changes files only; the run then uses the normal environment. */
  const setUp =
    (prepare: (s: Setup) => void) =>
    (s: Setup): Record<string, string> => {
      prepare(s);
      return {};
    };
  const entryOf = (s: Setup, account: Record<string, string>, root = abs(s)) => ({
    root,
    account,
    source: "s",
  });

  const states: [string, (s: Setup) => Record<string, string>, string][] = [
    [
      "a file that group can write",
      setUp((s) => {
        writeAllowed(s, []);
        chmodSync(allowedPath(s), 0o660);
      }),
      "group or others can write it",
    ],
    [
      "a file that others can write",
      setUp((s) => {
        writeAllowed(s, []);
        chmodSync(allowedPath(s), 0o606);
      }),
      "group or others can write it",
    ],
    [
      "a directory that group can write",
      setUp((s) => {
        writeAllowed(s, []);
        chmodSync(stateDir(s), 0o770);
      }),
      "can be written by group or others",
    ],
    [
      "a parent below HOME that group can write",
      setUp((s) => {
        writeAllowed(s, []);
        chmodSync(`${s.ws.home}/.local`, 0o775);
      }),
      "can be written by group or others",
    ],
    [
      "a parent below HOME that group can write, with no state folder yet",
      setUp((s) => {
        mkdirSync(`${s.ws.home}/.local/state`, { recursive: true });
        chmodSync(`${s.ws.home}/.local/state`, 0o775);
      }),
      "can be written by group or others",
    ],
    [
      "XDG_STATE_HOME moved while mise is active",
      (s) => ({ XDG_STATE_HOME: `${s.ws.home}/elsewhere`, __MISE_DIFF: "x" }),
      "XDG_STATE_HOME points away from its default and mise or direnv is active here",
    ],
    [
      "XDG_STATE_HOME moved while direnv is active",
      (s) => ({ XDG_STATE_HOME: `${s.ws.home}/elsewhere`, DIRENV_DIFF: "x" }),
      "XDG_STATE_HOME points away from its default and mise or direnv is active here",
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
      setUp((s) => {
        mkdirSync(`${stateDir(s)}/allowed.yaml`, { recursive: true, mode: 0o700 });
      }),
      "not a regular file",
    ],
    [
      "a FIFO in place of the file",
      setUp((s) => {
        mkdirSync(stateDir(s), { recursive: true, mode: 0o700 });
        execFileSync("mkfifo", [allowedPath(s)]);
      }),
      "not a regular file",
    ],
    [
      "a file over 1 MiB",
      setUp((s) => {
        writeAllowed(s, [], `#${"x".repeat(1024 * 1024)}\n`);
      }),
      "at most 1 MiB",
    ],
    [
      "invalid YAML",
      setUp((s) => {
        writeAllowed(s, [], "root: [\n");
      }),
      "not valid YAML",
    ],
    [
      "a mapping at the top",
      setUp((s) => {
        writeAllowed(s, [], "a: 1\n");
      }),
      "is not a list",
    ],
    [
      "an entry with a relative root",
      setUp((s) => {
        writeAllowed(s, [entryOf(s, { claude: "x" }, "relative/path")]);
      }),
      "has entry 1 that alz ignores",
    ],
    [
      "an entry with a bad account name",
      setUp((s) => {
        writeAllowed(s, [entryOf(s, { claude: "Bad Name" })]);
      }),
      "has entry 1 that alz ignores",
    ],
    [
      "an entry whose root is not a real path",
      setUp((s) => {
        symlinkSync(abs(s), `${s.real}/link-b`);
        writeAllowed(s, [entryOf(s, { claude: "x" }, `${s.real}/link-b`)]);
      }),
      "has entry 1 that alz ignores",
    ],
    [
      "two entries for one root",
      setUp((s) => {
        writeAllowed(s, [entryOf(s, { claude: "x" }), entryOf(s, { claude: "x" })]);
      }),
      "has entries 1, 2 that alz ignores",
    ],
  ];

  it.each(states)(
    "refuses %s, with the reason, before any prompt",
    async (_label, prepare, reason) => {
      const s = await ready();
      const extra = prepare(s);
      const before = { home: snapshot(s.ws.home), repo: snapshot(`${s.real}/repo`) };
      const t = terminal(...NAMES);
      const result = await allow(s, t.term, extra);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain(reason);
      expect(t.asked).toEqual([]);
      expect(snapshot(s.ws.home)).toEqual(before.home);
      expect(snapshot(`${s.real}/repo`)).toEqual(before.repo);
    },
  );

  it.each(states.slice(0, 4).concat(states.slice(11)))(
    "refuses `--remove` the same way for %s",
    async (_label, prepare, reason) => {
      const s = await ready();
      const extra = prepare(s);
      const before = snapshot(s.ws.home);
      const t = terminal(...NAMES);
      const result = await remove(s, t.term, extra);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain(reason);
      expect(snapshot(s.ws.home)).toEqual(before);
    },
  );

  it("refuses a relative HOME", async () => {
    const s = await ready();
    const t = terminal(...NAMES);
    const result = await allow(s, t.term, { HOME: "relative/home" });
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("HOME is not an absolute path");
    expect(t.asked).toEqual([]);
  });

  it("refuses in the writer when the user id is unknown", async () => {
    const s = await ready();
    const before = snapshot(s.ws.home);
    const result = await updateAllowed({
      change: { kind: "set", entry: hubEntry(s) },
      repoRoot: `${s.real}/repo`,
      home: s.ws.home,
      env: envFor(s),
      uid: undefined,
    });
    expect(result).toMatchObject({ kind: "refused", written: false });
    expect(snapshot(s.ws.home)).toEqual(before);
  });
});

describe("§13.20 what the layers say decides whether alz account allow can run (AC 1)", () => {
  const refused = async (s: Setup, extra: Record<string, string> = {}, dir = "repo") => {
    const before = snapshot(s.ws.home);
    const t = terminal(...NAMES);
    const result = await allow(s, t.term, extra, dir);
    expect(result.code).toBe(1);
    expect(t.asked).toEqual([]);
    expect(snapshot(s.ws.home)).toEqual(before);
    return result;
  };

  it("says only to run alz build on a cache miss: a lock that is missing", async () => {
    const s = await ready();
    rmSync(`${s.real}/repo/layer.lock`);
    const result = await refused(s);
    expect(result.stderr).toContain(
      "the layer cache cannot be verified (layer.lock is missing); run alz build",
    );
    expect(result.stdout).toBe("");
  });

  it("says only to run alz build on a cache miss: a cached layer that no longer matches the lock", async () => {
    const s = await ready();
    const entry = readdirSync(s.cache).find((name) => name.startsWith("tree2@"));
    expect(entry).toBeDefined();
    writeFileSync(`${s.cache}/${entry}/instructions/10-client.md`, "- Tampered.\n");
    const result = await refused(s);
    expect(result.stderr).toContain("(a layer no longer matches layer.lock); run alz build");
  });

  it("says only to run alz build on a cache miss: a cache that XDG_CACHE_HOME moved under mise", async () => {
    const s = await ready();
    const result = await refused(s, {
      XDG_CACHE_HOME: `${s.ws.home}/othercache`,
      __MISE_DIFF: "x",
    });
    expect(result.stderr).toContain("XDG_CACHE_HOME points away from ~/.cache");
    expect(result.stderr).toContain("run alz build");
  });

  it("refuses when the layers name no account, and names --remove when this root has an entry", async () => {
    const s = await ready();
    addHubV3(s, "");
    await rebuildAt(s, "v3");
    const plain = await refused(s);
    expect(plain.stderr).toContain("the layers name no account here");
    expect(plain.stderr).not.toContain("--remove");

    writeAllowed(s, [hubEntry(s)]);
    const withEntry = await refused(s);
    expect(withEntry.stderr).toContain("the layers name no account here");
    expect(withEntry.stderr).toContain("alz account allow --remove");
  });

  it("refuses the reserved name alz-blocked", async () => {
    const s = await ready();
    addHubV3(s, "account: alz-blocked\n");
    await rebuildAt(s, "v3");
    const result = await refused(s);
    expect(result.stderr).toContain("alz-blocked is reserved");
  });

  it("refuses when several client layers declare", async () => {
    const s = await ready();
    const files = (name: string) => ({
      "layer.yaml": `name: ${name}\nkind: client\nvisibility: private\nversion: 0.1.0\naccount: client-a\n`,
      "instructions/10-x.md": `- ${name}.\n`,
    });
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
        url: "https://example.invalid/one.git",
        ref: "v1",
        sha: "1".repeat(40),
        files: files("client-one"),
      },
      {
        url: "https://example.invalid/two.git",
        ref: "v1",
        sha: "2".repeat(40),
        files: files("client-two"),
      },
    ]);
    const result = await refused(s);
    expect(result.stderr).toContain(
      'several client layers declare an account ("client-one", "client-two")',
    );
    expect(result.stderr).toContain("remove one declaration, then run alz build");
  });

  it("refuses when layer.yaml cannot be read, but --remove still works", async () => {
    const s = await ready();
    writeAllowed(s, [hubEntry(s)]);
    writeFileSync(`${s.real}/repo/layer.yaml`, "stack: [\n");
    const result = await refused(s);
    expect(result.stderr).toContain(`repair ${s.real}/repo/layer.yaml`);

    const t = terminal(...NAMES);
    const removed = await remove(s, t.term);
    expect(removed.code, removed.stderr).toBe(0);
    expect(readAllowed(s)).toBe(`${HEADER}[]\n`);
  });

  it("refuses outside a repository, inside an account folder and inside a configured folder", async () => {
    const s = await ready();
    const none = await refused(s, {}, "personal");
    expect(none.stderr).toContain("repository: none (no repository here)");
    mkdirSync(`${s.ws.home}/.claude-x`);
    const folder = await refused(s, {}, "../home/.claude-x");
    expect(folder.stderr).toContain("the working directory is inside an account folder");
    const configured = await refused(s, { CLAUDE_CONFIG_DIR: `${s.real}/repo` });
    expect(configured.stderr).toContain("inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names");
  });
});

describe("§13.20 alz account allow --remove (AC 1)", () => {
  it("removes only this root's entry, keeps the others sorted, and leaves [] after the last", async () => {
    const s = await ready();
    const other = { root: `${s.real}/repo-b`, account: { claude: "b-account" }, source: "/x/b" };
    writeAllowed(s, [other, hubEntry(s)]);
    const t = terminal("client-a", "client-a-api");
    const result = await remove(s, t.term);
    expect(result.code, result.stderr).toBe(0);
    expect(t.asked).toEqual([
      `Type client-a to remove it for "${s.real}/repo" (source "${s.hubLayer}"): `,
      `Type client-a-api to remove it for "${s.real}/repo" (source "${s.hubLayer}"): `,
    ]);
    expect(result.stdout).toContain(
      "removing it sends this repository back to the default account",
    );
    expect(parseYamlLimited(readAllowed(s))).toEqual([other]);
    expect(readAllowed(s).startsWith(HEADER)).toBe(true);
    expect(modeOf(allowedPath(s))).toBe("600");
    expectNoLeftovers(s);

    // Only the other root's entry is left, so removing the repository's again changes nothing.
    const again = await remove(s, terminal().term);
    expect(again.code).toBe(0);
    expect(again.stdout).toContain("nothing to remove");
    expect(parseYamlLimited(readAllowed(s))).toEqual([other]);

    writeAllowed(s, [hubEntry(s)]);
    const last = await remove(s, terminal(...NAMES).term);
    expect(last.code, last.stderr).toBe(0);
    expect(readAllowed(s)).toBe(`${HEADER}[]\n`);
  });

  it("does nothing, and creates nothing, when there is no file or no entry", async () => {
    const s = await ready();
    const before = snapshot(s.ws.home);
    const t = terminal(...NAMES);
    const none = await remove(s, t.term);
    expect(none.code).toBe(0);
    expect(none.stdout).toContain("nothing to remove");
    expect(t.asked).toEqual([]);
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  it("asks for the recorded names, and a wrong answer changes nothing", async () => {
    const s = await ready();
    writeAllowed(s, [hubEntry(s, { account: { claude: "one-name" } })]);
    const before = snapshot(s.ws.home);
    const wrong = terminal("client-a");
    const refused = await remove(s, wrong.term);
    expect(refused.code).toBe(1);
    expect(wrong.asked).toEqual([
      `Type one-name to remove it for "${s.real}/repo" (source "${s.hubLayer}"): `,
    ]);
    expect(snapshot(s.ws.home)).toEqual(before);
    const right = await remove(s, terminal("one-name").term);
    expect(right.code, right.stderr).toBe(0);
  });

  it("works on a cache miss", async () => {
    const s = await ready();
    writeAllowed(s, [hubEntry(s)]);
    rmSync(`${s.real}/repo/layer.lock`);
    const result = await remove(s, terminal(...NAMES).term);
    expect(result.code, result.stderr).toBe(0);
    expect(readAllowed(s)).toBe(`${HEADER}[]\n`);
  });
});

describe("§13.20 the lock file (AC 1)", () => {
  const input = (s: Setup, waitMs: number) => ({
    change: { kind: "set" as const, entry: hubEntry(s) },
    repoRoot: `${s.real}/repo`,
    home: s.ws.home,
    env: envFor(s),
    uid: process.getuid?.(),
    waitMs,
  });

  it("waits for a lock that goes away, then succeeds", async () => {
    const s = await ready();
    writeAllowed(s, []);
    const lock = `${allowedPath(s)}.lock`;
    writeFileSync(lock, "999999\n", { mode: 0o600 });
    const timer = setTimeout(() => rmSync(lock), 200);
    const result = await allow(s, terminal(...NAMES).term);
    clearTimeout(timer);
    expect(result.code, result.stderr).toBe(0);
    expect(readAllowed(s)).toContain(`- root: ${s.real}/repo\n`);
    expect(existsSync(lock)).toBe(false);
  });

  it("never removes a lock it did not create: it refuses, names it and leaves it", async () => {
    const s = await ready();
    writeAllowed(s, []);
    const lock = `${allowedPath(s)}.lock`;
    writeFileSync(lock, "999999\n", { mode: 0o600 });
    const before = snapshot(s.ws.home);
    const result = await updateAllowed(input(s, 200));
    expect(result.kind).toBe("refused");
    if (result.kind === "refused") {
      expect(result.reason).toContain("allowed.yaml.lock");
      expect(result.reason).toContain("delete that file");
      expect(result.written).toBe(false);
    }
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  it("does not follow a lock that is a link", async () => {
    const s = await ready();
    writeAllowed(s, []);
    const victim = `${s.ws.home}/victim`;
    writeFileSync(victim, "keep", { mode: 0o600 });
    symlinkSync(victim, `${allowedPath(s)}.lock`);
    const result = await updateAllowed(input(s, 100));
    expect(result.kind).toBe("refused");
    expect(readFileSync(victim, "utf8")).toBe("keep");
    expect(lstatSync(`${allowedPath(s)}.lock`).isSymbolicLink()).toBe(true);
  });

  it("is safe across real processes: twelve runs at once leave one sorted entry per root", async () => {
    const s = await ready();
    const out = mkdtempSync(join(tmpdir(), "alz-driver-"));
    const bundle = `${out}/driver.mjs`;
    buildSync({
      entryPoints: [join(import.meta.dirname, "../allow-driver.ts")],
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
    const roots = Array.from({ length: 8 }, (_, index) => `${s.real}/roots/r${index}`);
    for (const root of roots) mkdirSync(root, { recursive: true });
    const shared = `${s.real}/roots/shared`;
    mkdirSync(shared);
    const go = `${out}/go`;
    const runs = [
      ...roots.map((root, index) => ({
        root,
        source: `/src/${index}`,
        account: { claude: `acct-${index}` },
      })),
      ...Array.from({ length: 4 }, (_, index) => ({
        root: shared,
        source: `/shared/${index}`,
        account: { codex: `shared-${index}` },
      })),
    ];
    const env = { HOME: s.ws.home, PATH: process.env.PATH ?? "", ...envFor(s) };
    const children = runs.map(
      (run) =>
        new Promise<{ code: number | null; stdout: string }>((resolve) => {
          const child = spawn(
            process.execPath,
            [bundle, JSON.stringify({ ...run, go, home: s.ws.home, env })],
            {
              stdio: ["ignore", "pipe", "inherit"],
              env: { PATH: process.env.PATH ?? "" },
            },
          );
          let stdout = "";
          child.stdout.on("data", (chunk) => {
            stdout += chunk;
          });
          child.on("close", (code) => resolve({ code, stdout }));
        }),
    );
    await new Promise((resolve) => setTimeout(resolve, 400));
    writeFileSync(go, "");
    const results = await Promise.all(children);
    for (const result of results) expect(result.code, result.stdout).toBe(0);
    // At least one process met a held lock and waited for it: the lock was really contended.
    expect(
      results.filter((result) => JSON.parse(result.stdout).waited === true).length,
    ).toBeGreaterThan(0);

    const entries = parseYamlLimited(readAllowed(s)) as { root: string; source: string }[];
    expect(entries).toHaveLength(9);
    const sorted = [...entries.map((entry) => entry.root)].sort();
    expect(entries.map((entry) => entry.root)).toEqual(sorted);
    expect(new Set(entries.map((entry) => entry.root)).size).toBe(9);
    const sharedEntry = entries.find((entry) => entry.root === shared);
    expect(runs.filter((run) => run.root === shared).map((run) => run.source)).toContain(
      sharedEntry?.source,
    );
    expectNoLeftovers(s);
    const listed = await which(s);
    expect(listed.stderr).not.toContain("ignored");
    rmSync(out, { recursive: true, force: true });
  });
});

const which = (s: Setup, extra: Record<string, string> = {}) => alz(s, ["account", "which"], extra);

describe("§13.20 only the state directory is written (AC 1)", () => {
  const isWrite = (call: TraceCall): boolean => {
    if (
      [
        "writeSync",
        "renameSync",
        "mkdirSync",
        "unlinkSync",
        "chmodSync",
        "fchmodSync",
        "writeFileSync",
        "rmSync",
        "appendFileSync",
        "symlinkSync",
        "linkSync",
        "copyFileSync",
        "rmdirSync",
      ].includes(call.fn)
    ) {
      return true;
    }
    if (call.fn !== "openSync") return false;
    const { flags } = call;
    if (typeof flags === "number") {
      return (flags & (constants.O_WRONLY | constants.O_RDWR | constants.O_CREAT)) !== 0;
    }
    return typeof flags === "string" && /[wa+]/.test(flags);
  };

  it("writes only under the state directory, runs no subprocess and opens nothing in an account folder", async () => {
    const s = await ready();
    mkdirSync(`${s.ws.home}/.claude-client-a`);
    writeFileSync(`${s.ws.home}/.claude-client-a/.credentials.json`, "secret");
    const t = terminal(...NAMES);
    const { result, calls } = await tracedAsync(() => allow(s, t.term));
    expect(result.code, result.stderr).toBe(0);
    const state = `${realpathSync.native(s.ws.home)}/.local`;
    const writes = calls.filter(isWrite);
    // Positive control: the trace sees the rename that puts the file in place.
    expect(writes.some((call) => call.fn === "renameSync")).toBe(true);
    for (const call of writes) {
      for (const path of call.paths) {
        expect(path.startsWith(`${state}/`), `${call.fn} ${path}`).toBe(true);
      }
    }
    expect(calls.filter((call) => call.api === "child_process")).toEqual([]);
    const touched = calls.flatMap((call) => call.paths);
    expect(touched.filter((path) => path.includes(".credentials.json"))).toEqual([]);
    expect(touched.filter((path) => path.includes("/.claude-client-a/"))).toEqual([]);
  });
});

describe("§13.20 import graph and the one place that reads standard input (AC 1)", () => {
  const forbidden = [
    "resolve/git.ts",
    "resolve/tree-cache.ts",
    "resolve/sources.ts",
    "pipeline.ts",
    "node:child_process",
    "child_process",
  ];

  it("keeps git, fetching, the pipeline and child processes out of allow and its writer", () => {
    for (const entry of ["cli/commands/account-allow.ts", "resolve/account-allow-writer.ts"]) {
      const imported = valueImports(entry);
      expect(imported.size).toBeGreaterThan(5);
      expect(
        [...imported].filter((name) => forbidden.includes(name)),
        entry,
      ).toEqual([]);
    }
  });

  const sourceFiles = (dir: string): string[] =>
    readdirSync(dir, { withFileTypes: true }).flatMap((entry) =>
      entry.isDirectory()
        ? sourceFiles(join(dir, entry.name))
        : entry.name.endsWith(".ts")
          ? [join(dir, entry.name)]
          : [],
    );

  it("imports the hint only in build and check", () => {
    const importers = sourceFiles(SRC)
      .filter((file) => /from "[^"]*account-allow-hint"/.test(readFileSync(file, "utf8")))
      .map((file) => file.slice(SRC.length + 1))
      .sort();
    expect(importers).toEqual(["cli/commands/build.ts", "cli/commands/check.ts"]);
  });

  /** Every way a source file could reach standard input or a terminal. */
  const STDIN =
    /process\.stdin|["']node:readline["']|["']readline["']|["']node:tty["']|["']tty["']|isatty|\/dev\/(?:tty|stdin)|readFileSync\(\s*0\b|readSync\(\s*0\b|fd:\s*0\b/;

  it("reads standard input only in terminal.ts", () => {
    const found = sourceFiles(SRC)
      .filter((file) => STDIN.test(readFileSync(file, "utf8")))
      .map((file) => file.slice(SRC.length + 1));
    // The scan sees the file that does read it, so a silent scan cannot pass.
    expect(found).toEqual(["cli/terminal.ts"]);
  });

  it.each([
    "process.stdin.on('data', f)",
    'import { createInterface } from "readline";',
    'import { isatty } from "tty";',
    'import { createInterface } from "node:readline";',
    'openSync("/dev/tty", "r")',
    'readFileSync("/dev/stdin")',
    "readFileSync(0, 'utf8')",
    "readSync(0, buffer)",
  ])("the scan for standard input catches %s", (sample) => {
    expect(STDIN.test(sample)).toBe(true);
  });

  it.each([
    ['await import("node:readline")', ["node:readline"]],
    ['const r = require("tty");', ["tty"]],
    ['import "node:readline";', ["node:readline"]],
    ["await import(name)", ["<dynamic import>"]],
    ["require(name)", ["<dynamic require>"]],
    ['import type { X } from "node:readline";', []],
    ['import { x } from "./y";', ["./y"]],
  ])("the import scan catches %s", (sample, expected) => {
    expect(importedSpecifiers(sample)).toEqual(expected);
  });
});

describe("§13.20 the real wiring: a pipe is not a terminal", () => {
  const out = mkdtempSync(join(tmpdir(), "alz-bin-"));
  const bundle = `${out}/alz.mjs`;
  const bundled = () => {
    if (!existsSync(bundle)) {
      buildSync({
        entryPoints: [join(SRC, "cli/bin.ts")],
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
    const s = await ready();
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

describe("§13.20 a layer's account applies only after allow, and the hint on build and check (AC 2)", () => {
  const hintLines = (text: string) =>
    text.split("\n").filter((line) => line.startsWith("note [account-allow]"));

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

  it("prints the allow command from build and check on a terminal only, and changes nothing else", async () => {
    const s = await ready();
    const quiet = await alz(s, ["build"]);
    const tree = snapshot(`${s.real}/repo`);
    const withTerminal = await alz(s, ["build"], {}, "repo", terminal().term);
    expect(withTerminal.code).toBe(0);
    const lines = hintLines(withTerminal.stderr);
    expect(lines).toHaveLength(1);
    expect(lines[0]).toContain("claude=client-a, codex=client-a-api");
    expect(lines[0]).toContain('"client"');
    expect(lines[0]).toContain("it applies only after you run: alz account allow");
    expect(withTerminal.stdout).toBe(quiet.stdout);
    expect(snapshot(`${s.real}/repo`)).toEqual(tree);

    const check = await alz(s, ["check"], {}, "repo", terminal().term);
    expect(check.code, check.stderr).toBe(0);
    expect(hintLines(check.stderr)).toHaveLength(1);
    expect(check.stdout).toBe((await alz(s, ["check"])).stdout);

    // No terminal, or any marker, prints nothing about it.
    expect(hintLines(quiet.stderr)).toEqual([]);
    expect(quiet.stderr).not.toContain("alz account allow");
    for (const marker of AGENT_SESSION_MARKERS) {
      for (const command of ["build", "check"]) {
        const marked = await alz(s, [command], { [marker]: "" }, "repo", terminal().term);
        expect(marked.code, marked.stderr).toBe(0);
        expect(marked.stderr, `${command} with ${marker}`).not.toContain("alz account allow");
      }
    }
    expect(snapshot(`${s.real}/repo`)).toEqual(tree);
  });

  it("prints no hint once allowed, and a ref bump keeps the approval", async () => {
    const s = await ready();
    await allowed(s);
    const build = await alz(s, ["build"], {}, "repo", terminal().term);
    expect(hintLines(build.stderr)).toEqual([]);
    await rebuildAt(s, "v2");
    const bumped = await alz(s, ["build"], {}, "repo", terminal().term);
    expect(hintLines(bumped.stderr)).toEqual([]);
    const check = await alz(s, ["check"], {}, "repo", terminal().term);
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
    const build = await alz(s, ["build"], {}, "repo", terminal().term);
    expect(build.code, build.stderr).toBe(0);
    const [line] = hintLines(build.stderr);
    expect(line).toContain("was claude=client-a, codex=client-a-api");
    expect(line).toContain("now claude=client-c, codex=client-a-api");
    expect(line).toContain("run: alz account allow");
    const check = await alz(s, ["check"], {}, "repo", terminal().term);
    expect(hintLines(check.stderr)).toHaveLength(1);

    const closed = await which(s);
    expect(closed.code).toBe(1);
    expect(closed.stderr).toContain("now claude=client-c, codex=client-a-api");

    const again = await allow(s, terminal("client-c", "client-a-api").term);
    expect(again.code, again.stderr).toBe(0);
    expect(again.stdout).toContain("currently allowed: claude=client-a, codex=client-a-api");
    accountFolder(s, ".claude-client-c", ".codex-client-a-api");
    const applied = await which(s);
    expect(applied.code, applied.stderr).toBe(0);
    expect(applied.stdout).toContain("claude: client-c (source: layer)");
    expect(hintLines((await alz(s, ["build"], {}, "repo", terminal().term)).stderr)).toEqual([]);
  });

  it("skips the account when a target is removed from the map", async () => {
    const s = await ready();
    await allowed(s);
    addHubV3(s, CLAUDE_ONLY);
    await rebuildAt(s, "v3");
    const build = await alz(s, ["build"], {}, "repo", terminal().term);
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
    const build = await alz(s, ["build"], {}, "repo", terminal().term);
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
    const build = await alz(s, ["build"], {}, "repo", terminal().term);
    expect(hintLines(build.stderr)[0]).toContain("run: alz account allow --remove");
  });

  it("names the reason in the hint when allowed.yaml cannot be trusted", async () => {
    const s = await ready();
    await allowed(s);
    chmodSync(allowedPath(s), 0o664);
    const build = await alz(s, ["build"], {}, "repo", terminal().term);
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
    const t = terminal(...NAMES);
    const result = await allow(s, t.term);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain('source as written: "git:https://example.invalid/hub.git@v1"');
    expect(result.stdout).toContain(`revision: ${"1".repeat(40)}`);
    expect(readAllowed(s)).toContain(`  source: ${url}\n`);
    expect(readAllowed(s)).not.toContain("@v1");
  });
});

describe("§13.20 where allowed.yaml may live, and what a link may point at (decision-20 amendment)", () => {
  it("refuses a state directory inside a git work tree, for a missing file and an existing one", async () => {
    const missing = await ready();
    mkdirSync(`${missing.ws.home}/.local/state/.git`, { recursive: true });
    const before = snapshot(missing.ws.home);
    const t = terminal(...NAMES);
    const result = await allow(missing, t.term);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("it lies inside a git work tree (~/.local/state/.git)");
    expect(t.asked).toEqual([]);
    expect(snapshot(missing.ws.home)).toEqual(before);

    const existing = await ready();
    writeAllowed(existing, []);
    mkdirSync(`${existing.ws.home}/.local/state/.git`);
    const beforeExisting = snapshot(existing.ws.home);
    const second = terminal(...NAMES);
    for (const run of [allow, remove]) {
      const refused = await run(existing, second.term);
      expect(refused.code).toBe(1);
      expect(refused.stderr).toContain(
        "allowed.yaml is ignored (it lies inside a git work tree (~/.local/state/.git))",
      );
    }
    expect(second.asked).toEqual([]);
    expect(snapshot(existing.ws.home)).toEqual(beforeExisting);
  });

  it("refuses a link whose target directory is inside a git work tree", async () => {
    const s = await ready();
    const store = `${s.ws.home}/store`;
    mkdirSync(`${store}/.git`, { recursive: true, mode: 0o700 });
    chmodSync(store, 0o700);
    writeFileSync(`${store}/target.yaml`, "[]\n", { mode: 0o600 });
    mkdirSync(stateDir(s), { recursive: true, mode: 0o700 });
    symlinkSync(`${store}/target.yaml`, allowedPath(s));
    const before = snapshot(s.ws.home);
    const result = await allow(s, terminal(...NAMES).term);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("git work tree");
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  const target = (s: Setup) => `${s.ws.home}/store/target.yaml`;
  const linked = (s: Setup, content: string | undefined) => {
    mkdirSync(`${s.ws.home}/store`, { recursive: true, mode: 0o700 });
    chmodSync(`${s.ws.home}/store`, 0o700);
    if (content !== undefined) writeFileSync(target(s), content, { mode: 0o600 });
    else mkdirSync(target(s), { mode: 0o700 });
    mkdirSync(stateDir(s), { recursive: true, mode: 0o700 });
    symlinkSync(target(s), allowedPath(s));
  };

  it.each([
    ["an empty file", ""],
    ["a file that holds only []", "[]\n"],
    ["a file that starts with alz's own header", `${HEADER}[]\n`],
  ])("follows a link to %s", async (_label, content) => {
    const s = await ready();
    linked(s, content);
    const result = await allow(s, terminal(...NAMES).term);
    expect(result.code, result.stderr).toBe(0);
    expect(lstatSync(allowedPath(s)).isSymbolicLink()).toBe(true);
    expect(readFileSync(target(s), "utf8")).toContain(`- root: ${s.real}/repo\n`);
  });

  it.each([
    ["a file somebody else wrote", "- root: /somewhere\n  account:\n    claude: x\n  source: /y\n"],
    ["a file with a comment of another program", "# another tool's file\n[]\n"],
  ])("refuses a link to %s, and changes nothing", async (_label, content) => {
    const s = await ready();
    linked(s, content);
    const before = snapshot(s.ws.home);
    const t = terminal(...NAMES);
    const result = await allow(s, t.term);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("not empty, [] or written by alz account allow");
    // Known before the prompt: nobody is asked to confirm what cannot be written.
    expect(t.asked).toEqual([]);
    expect(snapshot(s.ws.home)).toEqual(before);
    // --remove reads the same file: this root has no entry in it, so there is nothing to remove.
    const removed = await remove(s, terminal(...NAMES).term);
    expect(removed.code, removed.stderr).toBe(0);
    expect(removed.stdout).toContain("nothing to remove");
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  /** The state folder's parent is a link: the file's real path differs from where it is looked for. */
  const linkedParent = (s: Setup, content: string) => {
    const real = `${s.ws.home}/realstate`;
    mkdirSync(`${real}/agent-layerz`, { recursive: true, mode: 0o700 });
    chmodSync(real, 0o700);
    chmodSync(`${real}/agent-layerz`, 0o700);
    writeFileSync(`${real}/agent-layerz/allowed.yaml`, content, { mode: 0o600 });
    mkdirSync(`${s.ws.home}/.local`, { recursive: true, mode: 0o700 });
    chmodSync(`${s.ws.home}/.local`, 0o700);
    symlinkSync(real, `${s.ws.home}/.local/state`);
    return `${real}/agent-layerz/allowed.yaml`;
  };

  it("refuses a file reached through a linked folder when it is not alz's, before any prompt", async () => {
    const s = await ready();
    linkedParent(s, "- root: /somewhere\n  account:\n    claude: x\n  source: /y\n");
    const before = snapshot(s.ws.home);
    const t = terminal(...NAMES);
    const result = await allow(s, t.term);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("not empty, [] or written by alz account allow");
    expect(t.asked).toEqual([]);
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  it("writes a file reached through a linked folder when it is empty, and keeps the link", async () => {
    const s = await ready();
    const file = linkedParent(s, "");
    const result = await allow(s, terminal(...NAMES).term);
    expect(result.code, result.stderr).toBe(0);
    expect(lstatSync(`${s.ws.home}/.local/state`).isSymbolicLink()).toBe(true);
    expect(readFileSync(file, "utf8")).toContain(`- root: ${s.real}/repo\n`);
    expect(readdirSync(`${s.ws.home}/realstate/agent-layerz`)).toEqual(["allowed.yaml"]);
  });

  it("refuses a link placed inside a git work tree even when its target is outside every work tree", async () => {
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
    const t = terminal(...NAMES);
    for (const run of [allow, remove]) {
      const result = await run(s, t.term, extra);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("git work tree (~/tree/.git)");
    }
    expect(t.asked).toEqual([]);
    expect(snapshot(s.ws.home)).toEqual(before);
    const listed = await which(s, extra);
    expect(listed.stdout).toContain("ignored: it lies inside a git work tree (~/tree/.git)");
  });

  it("refuses a link to a directory", async () => {
    const s = await ready();
    linked(s, undefined);
    const before = snapshot(s.ws.home);
    const result = await allow(s, terminal(...NAMES).term);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("not a regular file");
    expect(snapshot(s.ws.home)).toEqual(before);
  });
});

describe("§13.20 what the person decides on is escaped, not stripped", () => {
  const ZERO_WIDTH = "​";
  const CYRILLIC_A = "а";

  /** Copies the hub's layer under a folder whose name holds `mark`, and points the stack at it. */
  async function pointAt(s: Setup, mark: string) {
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
      const folder = await pointAt(s, mark);
      const build = await alz(s, ["build"], {}, "repo", terminal().term);
      expect(build.code, build.stderr).toBe(0);
      const t = terminal(...NAMES);
      const result = await allow(s, t.term);
      expect(result.code, result.stderr).toBe(0);
      const text = `${result.stdout}${result.stderr}${t.asked.join("")}`;
      expect(result.stdout).toContain(`source as written: "path:../hub${shown}x/layer"`);
      expect(t.asked[0]).toContain(`hub${shown}x/layer`);
      expect(text).not.toContain(mark);
      expect(readAllowed(s)).toContain("source:");

      // A change to a look-alike source is visible in the hint, never as identical text.
      const plainCopy = "hubx";
      cpSync(`${s.ws.root}/hub/layer`, `${s.ws.root}/${plainCopy}/layer`, { recursive: true });
      writeFileSync(
        `${s.ws.root}/repo/layer.yaml`,
        `stack:\n  - source: path:../personal\n  - source: path:../${plainCopy}/layer\n  - source: path:./.layer\ntargets: [claude]\n`,
      );
      const changed = await alz(s, ["build"], {}, "repo", terminal().term);
      const [line] = changed.stderr.split("\n").filter((l) => l.startsWith("note [account-allow]"));
      expect(line).toContain(`${s.real}/${plainCopy}/layer`);
      expect(line).toContain(`${s.real}/hub${shown}x/layer`);
      expect(line).not.toContain(mark);
      expect(folder).toContain(mark);
    },
  );
});

describe("§13.20 one reader for every answer", () => {
  it("reads two answers that arrive in one chunk, one for each question", async () => {
    const input = new PassThrough();
    const output = new PassThrough();
    const term = createTerminal({ input, output, tty: false });
    input.write("client-a\nclient-a-api\n");
    expect(await term.ask("first: ")).toBe("client-a");
    expect(await term.ask("second: ")).toBe("client-a-api");
    input.end();
    expect(await term.ask("third: ")).toBeUndefined();
    expect(output.read()?.toString()).toContain("first: ");
    term.close?.();
  });

  it("ends a pending question with undefined when the input ends", async () => {
    const input = new PassThrough();
    const term = createTerminal({ input, output: new PassThrough(), tty: false });
    const pending = term.ask("name: ");
    input.end();
    expect(await pending).toBeUndefined();
  });

  it("asks one question for one name, whatever the number of targets", async () => {
    const s = await ready();
    addHubV3(s, "account: client-a\n");
    await rebuildAt(s, "v3");
    const t = terminal("client-a");
    const result = await allow(s, t.term);
    expect(result.code, result.stderr).toBe(0);
    expect(t.asked).toHaveLength(1);
    expect(readAllowed(s)).toContain("claude: client-a\n    codex: client-a\n");
  });
});

describe("§13.20 the repository is read again after the confirmation", () => {
  it.each([
    [
      "layer.yaml no longer parses",
      (s: Setup) => writeFileSync(`${s.real}/repo/layer.yaml`, "stack: [\n"),
    ],
    [
      "the stack drops the declaring layer",
      (s: Setup) =>
        writeFileSync(
          `${s.real}/repo/layer.yaml`,
          "stack:\n  - source: path:../personal\n  - source: path:./.layer\ntargets: [claude]\n",
        ),
    ],
  ])("refuses when %s while the person confirms", async (_label, change) => {
    const s = await ready();
    const asked: string[] = [];
    const term: Terminal = {
      ask: async (prompt) => {
        asked.push(prompt);
        change(s);
        return NAMES[asked.length - 1];
      },
    };
    const before = snapshot(s.ws.home);
    const result = await allow(s, term);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("the declaration changed while you were confirming");
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  it("says nothing to remove when the entry went away while the person confirms", async () => {
    const s = await ready();
    writeAllowed(s, [hubEntry(s, { account: { claude: "client-a" } })]);
    const term: Terminal = {
      ask: async () => {
        writeAllowed(s, []);
        return "client-a";
      },
    };
    const result = await remove(s, term);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("nothing to remove");
    expect(result.stdout).not.toContain("removed the entry");
  });
});

describe("§13.20 the hint on build and check, edge cases", () => {
  const hintLines = (text: string) =>
    text.split("\n").filter((line) => line.startsWith("note [account-allow]"));

  it("prints no hint from check when its own comparison failed, and exits 1", async () => {
    const s = await ready();
    const lock = JSON.parse(readFileSync(`${s.real}/repo/layer.lock`, "utf8")) as {
      files: Record<string, string>;
    };
    const [first] = Object.keys(lock.files);
    writeFileSync(`${s.real}/repo/${first}`, "tampered\n");
    const result = await alz(s, ["check"], {}, "repo", terminal().term);
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
    const result = await alz(s, ["check"], {}, "repo", terminal().term);
    expect(result.code, result.stderr).toBe(1);
    expect(result.stderr).toContain("the git identity differs");
    expect(hintLines(result.stderr)).toHaveLength(1);
  });

  it("prints no hint for a repository inside the folder CLAUDE_CONFIG_DIR names", async () => {
    const s = await ready();
    const result = await alz(
      s,
      ["build"],
      { CLAUDE_CONFIG_DIR: `${s.real}/repo` },
      "repo",
      terminal().term,
    );
    expect(result.code, result.stderr).toBe(0);
    expect(hintLines(result.stderr)).toEqual([]);
  });

  it("prints no hint for a repository that is $HOME", async () => {
    const s = await ready();
    const result = await alz(s, ["build"], { HOME: `${s.real}/repo` }, "repo", terminal().term);
    expect(result.code, result.stderr).toBe(0);
    expect(hintLines(result.stderr)).toEqual([]);
  });

  it("prints exactly the miss line when XDG_CACHE_HOME moved under mise and the layers name an account", async () => {
    const s = await ready();
    const result = await alz(
      s,
      ["build"],
      { XDG_CACHE_HOME: `${s.real}/othercache`, __MISE_DIFF: "x" },
      "repo",
      terminal().term,
    );
    expect(result.code, result.stderr).toBe(0);
    expect(hintLines(result.stderr)).toEqual([
      "note [account-allow] the account declaration was not checked: XDG_CACHE_HOME points away from ~/.cache and mise or direnv is active here",
    ]);
  });

  it("prints nothing about a cache miss when no layer names an account: off by default", async () => {
    const s = await ready();
    useStack(s, PERSONAL, REPO_LAYER);
    const result = await alz(
      s,
      ["build"],
      { XDG_CACHE_HOME: `${s.real}/othercache`, __MISE_DIFF: "x" },
      "repo",
      terminal().term,
    );
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
    const result = await alz(s, ["build"], {}, "repo", terminal().term);
    const [line] = hintLines(result.stderr);
    expect(line).toContain(reason);
    expect(line).toContain("then run: alz account allow");
  });

  it("names two entries for one root by position", async () => {
    const s = await ready();
    const entry = hubEntry(s);
    writeAllowed(s, [entry, entry]);
    const result = await alz(s, ["build"], {}, "repo", terminal().term);
    expect(hintLines(result.stderr)[0]).toContain("has entries 1, 2 that alz ignores");
  });
});
