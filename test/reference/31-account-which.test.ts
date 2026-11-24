import { execFileSync } from "node:child_process";
import {
  constants,
  lstatSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { describe, expect, it, vi } from "vitest";
import YAML from "yaml";
import { readRegularFileWithId } from "../../src/core/files";
import { type AccountResolution, resolveAccounts } from "../../src/resolve/account-resolution";
import {
  accountFolder,
  envFor,
  hubEntry,
  PERSONAL,
  plain,
  REPO_LAYER,
  type Setup,
  useStack,
  writeAccounts,
  writeAllowed,
} from "../account-workspace";
import { traced, tracedAsync } from "../fs-trace";
import { valueImports } from "../import-graph";
import { sha256Hex } from "../spec/cache";
import { ENV_NAMES, YAML_ALIAS_LIMIT } from "../spec/limits";
import { alz, built } from "./account-in-process";

/**
 * Account resolution (spec 13.19) as the TypeScript implementation does it: the cases that call
 * `resolveAccounts` with an input the CLI cannot give (a `run:` choice, another uid, no uid, a
 * relative HOME), the cases that read what the process does through the file system trace, the
 * source-graph scans of `account-resolution` and `account` and the `readRegularFileWithId` unit.
 * Each keeps the name it had in the acceptance suite. The cases the binary can show are in
 * test/acceptance/31-account-which.test.ts under their counterpart names; the Rust cover of the
 * rest is the account resolver (R6, TASK-55.7) and the alz-io capability types (R3, TASK-55.4).
 * Deleted with src/ at the cutover.
 */

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

const which = (s: Setup, extra: Record<string, string> = {}, args: string[] = [], dir = "repo") =>
  alz(s, ["account", "which", ...args], extra, dir);

/** Resolver input for the repository of the fixture, with the same XDG environment as the CLI. */
function input(s: Setup, extra: Record<string, string> = {}, cwd = `${s.ws.root}/repo`) {
  return {
    cwd,
    home: s.ws.home,
    env: { HOME: s.ws.home, ...envFor(s, extra) },
    uid: process.getuid?.(),
  };
}

function target(result: AccountResolution, name: "claude" | "codex") {
  const found = result.targets.find((item) => item.target === name);
  if (!found) throw new Error(`no ${name} target`);
  return found;
}

const CLIENT_A = ".claude-client-a";
const CLIENT_A_API = ".codex-client-a-api";
const MARKER = "SECRET-MARKER-5f2c1d";

function removeTreeEntries(s: Setup): void {
  for (const name of readdirSync(s.cache)) {
    if (name.startsWith("tree2@")) rmSync(`${s.cache}/${name}`, { recursive: true, force: true });
  }
}

describe("§13.19 sources: which account each target gets and why (AC 1)", () => {
  it("lets alz run's explicit choice beat alz use's, in the resolver", () => {
    const s = plain();
    accountFolder(s, ".claude-run-a", ".claude-use-a");
    const result = resolveAccounts({
      ...input(s),
      run: { target: "claude", account: "run-a" },
      use: { claude: "use-a" },
    });
    expect(target(result, "claude")).toMatchObject({ source: "run", account: "run-a" });
    expect(target(result, "codex")).toMatchObject({ source: "default" });
  });
});

describe("§13.19 paths: the repository root and the folders entries (AC 4)", () => {
  describe("a layer.yaml on the way that cannot be read stops the walk (decision-19)", () => {
    const aliasList = Array.from({ length: 150 }, () => "*a").join(", ");

    it("proves the alias payload parses within the limit and is refused only past it", () => {
      const ok = `a: &a [x]\nstack:\n  - source: path:x\nrefs: [${Array.from({ length: 50 }, () => "*a").join(", ")}]\n`;
      const parse = (text: string) => YAML.parse(text, { maxAliasCount: YAML_ALIAS_LIMIT });
      expect(() => parse(ok)).not.toThrow();
      expect(() => parse(`a: &a [x]\nrefs: [${aliasList}]\n`)).toThrow();
    });

    it("opens a regular file without waiting for a writer (O_NONBLOCK), so a swapped-in FIFO cannot hang it", () => {
      const s = plain();
      const fifo = `${s.ws.root}/pipe`;
      execFileSync("mkfifo", [fifo]);
      expect(readRegularFileWithId(fifo, 1024)).toBe("invalid");
      writeFileSync(`${s.ws.root}/plain`, "x");
      const { calls, result } = traced(() => readRegularFileWithId(`${s.ws.root}/plain`, 1024));
      expect(typeof result).toBe("object");
      const open = calls.find((call) => call.fn === "openSync");
      expect(typeof open?.flags === "number" && (open.flags & constants.O_NONBLOCK) !== 0).toBe(
        true,
      );
    });
  });
});

describe("§13.19 the two files: where alz reads them from (AC 2)", () => {
  const CONFIG = (s: Setup) => `folders:\n  ${s.real}/repo: client-b\n`;

  it("ignores a file the user does not own, in the resolver", () => {
    const s = plain();
    accountFolder(s, ".claude-client-b");
    writeAccounts(s, CONFIG(s));
    const uid = process.getuid?.() ?? 0;
    const result = resolveAccounts({ ...input(s), uid: uid + 1 });
    expect(result.accountsFile).toMatchObject({
      status: "ignored",
      reason: "it is not owned by you",
    });
    expect(target(result, "claude").source).toBe("default");
    expect(result.warnings.map((warning) => warning.code)).toContain("file-ignored");
  });

  it("ignores both files when the platform has no uid to compare with", () => {
    const s = plain();
    accountFolder(s, ".claude-client-b");
    writeAccounts(s, CONFIG(s));
    const result = resolveAccounts({ ...input(s), uid: undefined });
    expect(result.accountsFile.status).toBe("ignored");
    expect(target(result, "claude").source).toBe("default");
  });

  it("ignores a file whose path names an account folder, before any file system call on it", () => {
    const s = plain();
    const result = resolveAccounts(input(s, { XDG_CONFIG_HOME: `${s.ws.home}/.claude-x` }));
    expect(result.accountsFile).toMatchObject({
      status: "ignored",
      reason: "it lies inside an account folder",
    });
  });
  it("never reads the values of __MISE_DIFF, DIRENV_DIFF or an ALZ_USE_ variable, only the listed names", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAllowed(s, [hubEntry(s)]);
    writeAccounts(s, `folders:\n  ${s.real}/elsewhere: client-b\n`);
    const gets: string[] = [];
    const probes: string[] = [];
    const reads: string[] = [];
    const target0: Record<string, string> = {
      HOME: s.ws.home,
      ...envFor(s),
      ALZ_USE_CLAUDE: "forged",
      CLAUDE_CONFIG_DIR: "/foreign",
    };
    // The two manager variables are accessors: an existence check cannot run the getter, so a
    // read of the value is caught however it is made.
    for (const name of ["__MISE_DIFF", "DIRENV_DIFF"]) {
      Object.defineProperty(target0, name, {
        enumerable: true,
        configurable: true,
        get() {
          reads.push(name);
          return MARKER;
        },
      });
    }
    const env = new Proxy<Record<string, string>>(target0, {
      get(object, key, receiver) {
        gets.push(String(key));
        return Reflect.get(object, key, receiver);
      },
      has(object, key) {
        probes.push(`has:${String(key)}`);
        return Reflect.has(object, key);
      },
      getOwnPropertyDescriptor(object, key) {
        probes.push(`descriptor:${String(key)}`);
        return Reflect.getOwnPropertyDescriptor(object, key);
      },
      ownKeys(object) {
        probes.push("ownKeys");
        return Reflect.ownKeys(object);
      },
    });
    const result = resolveAccounts({
      cwd: `${s.real}/repo`,
      home: s.ws.home,
      env,
      uid: process.getuid?.(),
    });
    expect(target(result, "claude").source).toBe("environment");
    expect(result.warnings.map((warning) => warning.text).join("\n")).toContain(
      "mise or direnv is active here",
    );
    expect(JSON.stringify(result)).not.toContain(MARKER);
    expect(gets.length).toBeGreaterThan(0);
    expect(gets.filter((name) => /MISE|DIRENV|ALZ_USE/.test(name))).toEqual([]);
    const known = ENV_NAMES;
    expect(gets.filter((name) => !known.includes(name))).toEqual([]);
    // The value of a manager variable is never read: its getter never runs, no `get` names it,
    // and nothing enumerates the environment. Existence is the only thing asked.
    expect(reads).toEqual([]);
    expect(probes.filter((probe) => probe === "ownKeys" || probe.startsWith("has:"))).toEqual([]);
    const managerProbes = probes.filter((probe) => /MISE|DIRENV/.test(probe));
    expect(managerProbes.length).toBeGreaterThan(0);
    expect(managerProbes.filter((probe) => !probe.startsWith("descriptor:"))).toEqual([]);
  });
});

describe("§13.19 allow: a layer's account applies only while allowed.yaml holds the same entry (AC 5)", () => {
  it("gives the default account and opens no lock or cache file for a root that has no entry", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAllowed(s, [hubEntry(s, { root: `${s.real}/some-other-repository` })]);
    const { result, calls } = await tracedAsync(() => which(s));
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("claude: default (source: default)");
    expect(result.stdout).toContain("layer step skipped (this repository is not allowed)");
    expect(result.stdout).not.toContain("alz account allow");
    // Exactly these: the manifest on the walk and allowed.yaml. No accounts.yaml exists here, and
    // no lock, cache file or layer of the stack is opened.
    const home = realpathSync.native(s.ws.home);
    const opened = [
      ...new Set(calls.filter((call) => call.fn === "openSync").flatMap((call) => call.paths)),
    ];
    expect(opened.sort()).toEqual(
      [`${s.real}/repo/layer.yaml`, `${home}/.local/state/agent-layerz/allowed.yaml`].sort(),
    );
  });

  it("applies the named account for a matching entry", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAllowed(s, [hubEntry(s)]);
    const result = resolveAccounts(input(s));
    expect(target(result, "claude")).toMatchObject({ source: "layer", account: "client-a" });
    expect(result.allowedFile.allowed).toBe(true);
    expect(result.warnings).toEqual([]);
  });
});

/** Every file, folder and link under `dir` with its size, time and content: unchanged means untouched. */
function listing(dir: string): Record<string, string> {
  const out: Record<string, string> = {};
  const walk = (current: string) => {
    for (const item of readdirSync(current, { withFileTypes: true })) {
      const full = join(current, item.name);
      const stat = lstatSync(full);
      const body = stat.isFile() ? sha256Hex(readFileSync(full)) : "";
      out[full] =
        `${item.isDirectory() ? "dir" : item.isSymbolicLink() ? "link" : "file"}:${stat.size}:${stat.mtimeMs}:${body}`;
      if (item.isDirectory()) walk(full);
    }
  };
  walk(dir);
  return out;
}

describe("§13.19 reads only: no git, no write, no fetch (AC 3)", () => {
  async function prepared() {
    const s = await built();
    accountFolder(s, CLIENT_A_API);
    // A planted account folder with a file in it: alz may only list its name.
    mkdirSync(`${s.ws.home}/${CLIENT_A}`);
    writeFileSync(`${s.ws.home}/${CLIENT_A}/.credentials.json`, "secret");
    writeAccounts(s, `default_name: personal\nfolders:\n  ${s.real}/elsewhere: client-a\n`);
    writeAllowed(s, [hubEntry(s)]);
    return s;
  }

  it("proves the trace works: a build is seen running git and writing", async () => {
    const s = await built("v1");
    useStack(s, PERSONAL, ["path:../hub/layer", "v2"], REPO_LAYER);
    const { result, calls } = await tracedAsync(() => alz(s, ["build"]));
    expect(result.code, result.stderr).toBe(0);
    expect(calls.some((call) => call.api === "child_process")).toBe(true);
    for (const name of ["mkdirSync", "writeFileSync", "renameSync"]) {
      expect(
        calls.some((call) => call.fn === name),
        name,
      ).toBe(true);
    }
  });

  it("never runs git, writes or opens anything it need not, in an allowed repository", async () => {
    const s = await prepared();
    const home = realpathSync.native(s.ws.home);
    const repoBefore = listing(`${s.real}/repo`);
    const cacheBefore = listing(s.cache);
    const homeBefore = listing(s.ws.home);

    // The command starts in the working directory by the name the user's shell gives it: on macOS
    // the temp folder is a link (/var/..., really /private/var/...), so resolving it is a call the
    // entry makes once, apart from the lookups of the already resolved path.
    const named = { ...s, ws: { ...s.ws, root: s.ws.root.replace(/^\/private(?=\/var\/)/, "") } };
    const { result, calls } = await tracedAsync(() => which(named));
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("claude: client-a (source: layer)");

    expect(listing(`${s.real}/repo`)).toEqual(repoBefore);
    expect(listing(s.cache)).toEqual(cacheBefore);
    expect(listing(s.ws.home)).toEqual(homeBefore);

    expect(calls.filter((call) => call.api === "child_process")).toEqual([]);
    const readOnly = new Set([
      "lstatSync",
      "statSync",
      "fstatSync",
      "existsSync",
      "realpathSync",
      "realpathSync.native",
      "readlinkSync",
      "openSync",
      "readSync",
      "closeSync",
      "readdirSync",
    ]);
    const fsCalls = calls.filter((call) => call.api === "fs");
    expect(fsCalls.length).toBeGreaterThan(0);
    expect(fsCalls.filter((call) => !readOnly.has(call.fn)).map((call) => call.fn)).toEqual([]);

    const writing =
      constants.O_WRONLY |
      constants.O_RDWR |
      constants.O_CREAT |
      constants.O_TRUNC |
      constants.O_APPEND;
    const opens = fsCalls.filter((call) => call.fn === "openSync");
    for (const call of opens) {
      const flags = call.flags;
      expect(
        typeof flags === "number" &&
          (flags & writing) === 0 &&
          (flags & constants.O_NOFOLLOW) !== 0,
        JSON.stringify(call),
      ).toBe(true);
    }

    const tree = `${s.cache}/tree2@`;
    const allowedOpen = [
      `${s.real}/repo/layer.yaml`,
      `${s.real}/repo/layer.lock`,
      `${s.real}/repo/.layer/layer.yaml`,
      `${home}/.config/agent-layerz/accounts.yaml`,
      `${home}/.local/state/agent-layerz/allowed.yaml`,
      `${s.cache}/pathref@`,
      tree,
      `${s.real}/personal/`,
    ];
    const opened = opens.flatMap((call) => call.paths);
    expect(opened.length).toBeGreaterThan(0);
    expect(opened.filter((path) => !allowedOpen.some((prefix) => path.startsWith(prefix)))).toEqual(
      [],
    );

    const listed = fsCalls
      .filter((call) => call.fn === "readdirSync")
      .flatMap((call) => call.paths);
    expect(listed).toContain(s.ws.home);
    expect(
      listed.filter(
        (path) =>
          path !== s.ws.home &&
          path !== home &&
          !path.startsWith(tree) &&
          !path.startsWith(`${s.real}/personal`),
      ),
    ).toEqual([]);

    // $HOME is listed once. The working directory is resolved once on macOS; on Linux the
    // reference's realpath resolves it once per lookup (seen on the ubuntu CI job, PR #14), and
    // src/ is never changed (decision-30). The Rust alz resolves it once on every platform
    // (TASK-55.7 carries that check).
    expect(listed.filter((path) => path === s.ws.home)).toHaveLength(1);
    const cwd = `${named.ws.root}/repo`;
    const resolved = fsCalls
      .filter((call) => call.fn.startsWith("realpathSync"))
      .flatMap((call) => call.paths)
      .filter((path) => path === cwd);
    if (process.platform === "linux") expect(resolved.length).toBeGreaterThan(0);
    else expect(resolved).toHaveLength(1);
    // Every open is also non-blocking, so a FIFO swapped in cannot hang it.
    for (const call of opens) {
      expect(typeof call.flags === "number" && (call.flags & constants.O_NONBLOCK) !== 0).toBe(
        true,
      );
    }

    const planted = [`${s.ws.home}/${CLIENT_A}`, `${home}/${CLIENT_A}`];
    const touched = fsCalls
      .flatMap((call) => call.paths)
      .filter((path) => planted.some((dir) => path === dir || path.startsWith(`${dir}/`)));
    expect(touched).toEqual([]);
  });

  it("opens no stack file for a repository that is not allowed, and runs no git", async () => {
    const s = await prepared();
    writeAllowed(s, [hubEntry(s, { root: `${s.real}/other` })]);
    const { calls } = await tracedAsync(() => which(s));
    expect(calls.filter((call) => call.api === "child_process")).toEqual([]);
    const home = realpathSync.native(s.ws.home);
    const opened = [
      ...new Set(calls.filter((call) => call.fn === "openSync").flatMap((call) => call.paths)),
    ];
    expect(opened.sort()).toEqual(
      [
        `${s.real}/repo/layer.yaml`,
        `${home}/.config/agent-layerz/accounts.yaml`,
        `${home}/.local/state/agent-layerz/allowed.yaml`,
      ].sort(),
    );
  });

  it("proves the import graph sees a git import: the build command has one", () => {
    expect([...valueImports("cli/commands/build.ts")]).toContain("pipeline.ts");
    expect([...valueImports("pipeline.ts")]).toContain("resolve/git.ts");
  });

  it("does not import git, the tree cache, source fetching, the build pipeline or child_process", () => {
    const forbidden = [
      "resolve/git.ts",
      "resolve/tree-cache.ts",
      "resolve/sources.ts",
      "pipeline.ts",
      "node:child_process",
      "child_process",
    ];
    for (const entry of ["resolve/account-resolution.ts", "cli/commands/account.ts"]) {
      const imported = valueImports(entry);
      expect(imported.size).toBeGreaterThan(5);
      expect(
        [...imported].filter((name) => forbidden.includes(name)),
        entry,
      ).toEqual([]);
    }
  });

  it("does not import the writer of allowed.yaml, or the build and check hint (TASK-50.11)", () => {
    for (const entry of ["resolve/account-resolution.ts", "cli/commands/account.ts"]) {
      const imported = [...valueImports(entry)];
      expect(imported, entry).not.toContain("resolve/account-allow-writer.ts");
      expect(imported, entry).not.toContain("resolve/account-allow-hint.ts");
      expect(imported, entry).not.toContain("cli/commands/account-allow.ts");
    }
  });
});

describe("§13.19 review follow-ups: ordering of the checks, folders that are only contained, bad choices", () => {
  /** Does any traced call touch `dir` or something below it, spelled as given or as its real path? */
  function touching(calls: { paths: string[] }[], ...dirs: string[]): string[] {
    return calls
      .flatMap((call) => call.paths)
      .filter((path) => dirs.some((dir) => path === dir || path.startsWith(`${dir}/`)));
  }

  describe("a working directory inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names (mise or direnv can set it)", () => {
    it("opens nothing in the configured folder: only allowed.yaml is read", async () => {
      const s = await built();
      writeAllowed(s, [hubEntry(s, { account: { codex: "client-c" } })]);
      const { result, calls } = await tracedAsync(() =>
        which(s, { CLAUDE_CONFIG_DIR: `${s.real}/repo` }),
      );
      expect(result.code).toBe(1);
      const home = realpathSync.native(s.ws.home);
      const opened = [
        ...new Set(calls.filter((call) => call.fn === "openSync").flatMap((call) => call.paths)),
      ];
      expect(opened).toEqual([`${home}/.local/state/agent-layerz/allowed.yaml`]);
      expect(calls.filter((call) => call.api === "child_process")).toEqual([]);
    });
  });

  it("fails closed for a bad alz run or alz use name, building no path and never falling to a lower step", () => {
    const s = plain();
    accountFolder(s, ".claude-client-b");
    writeAccounts(s, `folders:\n  ${s.real}/repo: client-b\n`);
    const result = resolveAccounts({
      ...input(s),
      run: { target: "claude", account: "../../etc" },
      use: { codex: "A b" },
    });
    for (const [name, source] of [
      ["claude", "run"],
      ["codex", "use"],
    ] as const) {
      expect(target(result, name)).toMatchObject({
        source,
        apply: { action: "keep" },
        failsClosed: { kind: "invalid-name" },
      });
    }
    expect(JSON.stringify(result)).not.toContain("etc");
    expect(JSON.stringify(result)).not.toContain("A b");
  });

  describe("a HOME that is not an absolute path", () => {
    it("reads nothing from the file system in the resolver", () => {
      const s = plain();
      const { result, calls } = traced(() =>
        resolveAccounts({ ...input(s), home: "relative/home", use: { claude: "x" } }),
      );
      expect(calls.filter((call) => call.api === "fs")).toEqual([]);
      expect(target(result, "claude").apply).toEqual({ action: "keep" });
      expect(target(result, "codex").apply).toEqual({ action: "unset" });
    });
  });

  describe("the working directory is resolved once, and never inside an account folder", () => {
    it("makes no file system call under an account folder, and skips the user config step", () => {
      const s = plain();
      const folder = `${s.ws.home}/${CLIENT_A}`;
      mkdirSync(`${folder}/project`, { recursive: true });
      writeFileSync(`${folder}/.credentials.json`, "secret");
      writeAccounts(s, `folders:\n  ${s.real}/repo: client-b\n`);
      const { result, calls } = traced(() => resolveAccounts(input(s, {}, `${folder}/project`)));
      expect(result.repository).toEqual({ status: "none", reason: "account-folder" });
      expect(touching(calls, folder, `${realpathSync.native(s.ws.home)}/${CLIENT_A}`)).toEqual([]);
      expect(target(result, "claude").skipped).toContainEqual({
        step: "user config",
        reason: "cwd-in-account-folder",
      });
    });

    it("treats a link into an account folder as an account folder, and hands the file system only the link's own path", () => {
      const s = plain();
      const folder = `${s.ws.home}/${CLIENT_A}`;
      mkdirSync(`${folder}/project`, { recursive: true });
      symlinkSync(`${folder}/project`, `${s.real}/link-project`);
      const { result, calls } = traced(() =>
        resolveAccounts(input(s, {}, `${s.real}/link-project`)),
      );
      expect(result.repository).toMatchObject({ status: "none", reason: "account-folder" });
      expect(touching(calls, folder, `${realpathSync.native(s.ws.home)}/${CLIENT_A}`)).toEqual([]);
      expect(target(result, "claude").skipped).toContainEqual({
        step: "user config",
        reason: "cwd-in-account-folder",
      });
    });
  });

  describe("text that names an account folder is ignored before any file system call on it", () => {
    function planted(s: Setup): string[] {
      for (const name of [".claude-x", ".claude-y"]) {
        mkdirSync(`${s.ws.home}/${name}/agent-layerz/project`, { recursive: true });
      }
      return [`${s.ws.home}/.claude-x`, `${s.ws.home}/.claude-y`].flatMap((dir) => [
        dir,
        `${realpathSync.native(s.ws.home)}/${dir.slice(s.ws.home.length + 1)}`,
      ]);
    }

    it("covers the two files located by XDG variables", () => {
      const s = plain();
      const dirs = planted(s);
      const { result, calls } = traced(() =>
        resolveAccounts(
          input(s, {
            XDG_CONFIG_HOME: `${s.ws.home}/.claude-x`,
            XDG_STATE_HOME: `${s.ws.home}/.claude-y`,
          }),
        ),
      );
      expect(result.accountsFile.status).toBe("ignored");
      expect(result.allowedFile.status).toBe("ignored");
      expect(touching(calls, ...dirs)).toEqual([]);
    });

    it("covers a folders key, an allowed root and a stack source", async () => {
      const s = await built();
      const dirs = planted(s);
      writeAccounts(
        s,
        `folders:\n  ~/.claude-x/agent-layerz/project: x\n  ${s.real}/repo: client-b\n`,
      );
      writeAllowed(s, [
        hubEntry(s, { root: `${s.ws.home}/.claude-y/agent-layerz/project` }),
        hubEntry(s),
      ]);
      useStack(
        s,
        PERSONAL,
        ["path:../hub/layer", "v1"],
        [`path:${s.ws.home}/.claude-x/agent-layerz/project`],
      );
      removeTreeEntries(s);
      const { result, calls } = traced(() => resolveAccounts(input(s)));
      expect(result.warnings.map((warning) => warning.code)).toEqual(
        expect.arrayContaining(["config-folder", "allowed-entry"]),
      );
      expect(touching(calls, ...dirs)).toEqual([]);
    });

    it("covers a platform where the owner cannot be read: no file of either folder is touched", () => {
      const s = plain();
      writeAccounts(s, "default_name: personal\n");
      const { result, calls } = traced(() => resolveAccounts({ ...input(s), uid: undefined }));
      expect(result.accountsFile).toMatchObject({ status: "ignored" });
      const names = calls
        .flatMap((call) => call.paths)
        .filter((path) => path.includes("agent-layerz"));
      expect(names).toEqual([]);
    });
  });
});
