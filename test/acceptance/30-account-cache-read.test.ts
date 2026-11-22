import {
  chmodSync,
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { describe, expect, it } from "vitest";
import {
  type AllowedFixture,
  accountFolder,
  alz,
  built as builtIn,
  hubEntry,
  PERSONAL,
  REPO_LAYER,
  remoteStack,
  type Setup,
  setup as setupIn,
  sh,
  useStack,
  writeAllowed,
  writeLock,
} from "../account-workspace";
import { gitProxy } from "../git-proxy";
import { type CliResult, readTree, snapshot } from "../helpers";
import { gitEntryName, pathIndexName, treeEntryName } from "../spec/cache";
import { MAX_LAYER_BYTES, MAX_LAYER_ENTRIES } from "../spec/limits";

/**
 * Cache index and cache misses (spec 13.20), seen from outside. What the TypeScript reader
 * returns, the calls it makes and the files it opens are checked in test/reference. Here the
 * binary is driven and every claim is made through language-neutral proxies:
 *
 *  (a) the declaration `alz account which` reports: the account, the source and the warnings;
 *  (b) no writes: HOME (the state folder included), the cache and the repository are equal
 *      before and after, in kind, mode, content and link target;
 *  (c) no git: a `git` stand-in first on PATH records its calls, and none is made. A positive
 *      control shows that `alz build` is seen calling it.
 *
 * Reading inside account folders and the exact set of files opened stay reference-only: they
 * need a trace of the process. alz-io's capability types will carry them (R3, TASK-55.4;
 * R6, TASK-55.7).
 */

const FIXTURE = "30-account-cache-read";
const PERSONAL_URL = "https://example.invalid/personal.git";
const FIRST = "1".repeat(40);
const SECOND = "2".repeat(40);

const setup = () => setupIn(FIXTURE);
const built = (ref = "v1") => builtIn(ref, FIXTURE);

/** The standard stack: personal in place, the hub's layer pinned to a ref, the repo layer. */
function useStandard(s: Setup, ref = "v1", hubDir = "../hub/layer"): void {
  useStack(s, PERSONAL, [`path:${hubDir}`, ref], REPO_LAYER);
}

type LockJson = {
  layers: { name: string; source: string; rev?: string; sha?: string; contentHash?: string }[];
  [key: string]: unknown;
};

function lockOf(s: Setup): LockJson {
  return JSON.parse(readFileSync(`${s.ws.root}/repo/layer.lock`, "utf8")) as LockJson;
}

function editLock(s: Setup, edit: (lock: LockJson) => void): void {
  const lock = lockOf(s);
  edit(lock);
  writeLock(s, lock);
}

function lockHubEntry(lock: LockJson): LockJson["layers"][number] {
  const found = lock.layers.find((layer) => layer.source === "path:../hub/layer");
  if (!found) throw new Error("the lock has no hub entry");
  return found;
}

function indexPath(s: Setup, sha: string, realDir = s.hubLayer): string {
  return `${s.cache}/${pathIndexName(realDir, sha)}`;
}

function treeOf(s: Setup, sha: string): string {
  return sh(s.ws, s.hub, ["rev-parse", `${sha}:layer`]);
}

function filesOf(s: Setup, dir: string): Record<string, string> {
  return readTree(`${s.ws.root}/${dir}`);
}

/** The cache entry of a remote layer that `remoteStack` wrote. */
const remoteDir = (s: Setup, url: string, sha: string): string =>
  `${s.cache}/${gitEntryName(url, sha)}`;

// --- allowed.yaml: `alz account which` reads a layer's declaration once the repository is allowed

/** The hub's layer (a client) declares `client-a` and `client-a-api`; allow it. */
function allowClient(s: Setup): Setup {
  accountFolder(s, ".claude-client-a", ".codex-client-a-api");
  writeAllowed(s, [hubEntry(s)]);
  return s;
}

/** The first layer declares `personal-a`; allow it, recorded under `source`. */
function allowPersonal(s: Setup, source: string): Setup {
  accountFolder(s, ".claude-personal-a", ".codex-personal-a");
  const entry: AllowedFixture = {
    root: `${s.real}/repo`,
    account: { claude: "personal-a", codex: "personal-a" },
    source,
  };
  writeAllowed(s, [entry]);
  return s;
}

// --- the three proxies

/** Everything alz may have written to: the fake HOME and the workspace (cache and repository). */
const world = (s: Setup) => ({ home: snapshot(s.ws.home), root: snapshot(s.ws.root) });

/**
 * Runs `alz account which` the way a shell hook does and checks proxies (b) and (c): it wrote
 * nothing and ran no git.
 */
async function whichReading(s: Setup, extra: Record<string, string> = {}): Promise<CliResult> {
  const proxy = gitProxy();
  const before = world(s);
  const result = await alz(s, ["account", "which"], { ...proxy.env, ...extra });
  expect(world(s), "alz account which wrote").toEqual(before);
  expect(proxy.calls(), "alz account which ran git").toEqual([]);
  return result;
}

const CLIENT_LINES = [
  "claude: client-a (source: layer) CLAUDE_CONFIG_DIR=~/.claude-client-a",
  "codex: client-a-api (source: layer) CODEX_HOME=~/.codex-client-a-api",
];
const PERSONAL_LINES = [
  "claude: personal-a (source: layer) CLAUDE_CONFIG_DIR=~/.claude-personal-a",
  "codex: personal-a (source: layer) CODEX_HOME=~/.codex-personal-a",
];

const warningLines = (text: string): string[] =>
  text.split("\n").filter((line) => line.startsWith("warning ["));

/** The layer's account is applied: the declaration was read, with `warnings` (a) in stderr. */
function expectApplied(result: CliResult, lines: string[], warnings: string[] = []): void {
  expect(result.code, result.stdout + result.stderr).toBe(0);
  for (const line of lines) expect(result.stdout).toContain(line);
  expect(
    warningLines(result.stderr).map((line) => line.replace(/^warning \[[^\]]+\] /, "")),
  ).toEqual(warnings);
}

/** The layer step could not verify the cache: it names the miss and fails closed. */
function expectMissing(result: CliResult, phrase: string): void {
  expect(result.code, result.stdout + result.stderr).toBe(1);
  expect(result.stdout).toContain(`layer step skipped (cache miss: ${phrase}); fails closed`);
}

/** The layer step fails closed for the source the repository was allowed under. */
function expectFailsClosed(result: CliResult): void {
  expect(result.code, result.stdout + result.stderr).toBe(1);
  expect(result.stdout).toContain("fails closed");
}

const MARKER = "SECRET-MARKER";
const expectNoMarker = (result: CliResult): void =>
  expect(result.stdout + result.stderr).not.toContain(MARKER);

const noIndexTmp = (s: Setup): string[] =>
  readdirSync(s.cache).filter((name) => name.startsWith(".tmp-"));

describe("§13.20 cache index and cache misses: the pathref@ index (AC 1)", () => {
  it("finds a pinned path: layer through the index and reports the client layer's map", async () => {
    const s = allowClient(await built());
    expectApplied(await whichReading(s), CLIENT_LINES);
  });

  it("is written by build next to the tree2@ entry, holds the tree id and is not rewritten", async () => {
    const s = await built();
    const tree = treeOf(s, s.v1);
    const file = indexPath(s, s.v1);
    expect(readFileSync(file, "utf8")).toBe(tree);
    expect(statSync(file).mode & 0o777).toBe(0o644);
    expect(existsSync(`${s.cache}/${treeEntryName(tree)}`)).toBe(true);
    expect(noIndexTmp(s)).toEqual([]);

    const before = lstatSync(file);
    expect((await alz(s, ["build"])).code).toBe(0);
    const after = lstatSync(file);
    expect([after.ino, after.mtimeMs]).toEqual([before.ino, before.mtimeMs]);
  });

  it("is written by every command that resolves the stack, not only build", async () => {
    const s = await built();
    rmSync(indexPath(s, s.v1));
    expect((await alz(s, ["check"])).code).toBe(0);
    expect(readFileSync(indexPath(s, s.v1), "utf8")).toBe(treeOf(s, s.v1));
  });

  it.each([
    ["a tampered file", (file: string) => writeFileSync(file, "garbage")],
    [
      "a link",
      (file: string) => {
        const other = `${file}.elsewhere`;
        writeFileSync(other, "0".repeat(40));
        rmSync(file);
        symlinkSync(other, file);
      },
    ],
  ])("is rewritten by the next build when it is %s", async (_label, tamper) => {
    const s = await built();
    const file = indexPath(s, s.v1);
    tamper(file);
    expect((await alz(s, ["build"])).code).toBe(0);
    expect(lstatSync(file).isFile()).toBe(true);
    expect(readFileSync(file, "utf8")).toBe(treeOf(s, s.v1));
  });

  it("fails the build with the cache error when a directory sits at the index name", async () => {
    const s = await built();
    const file = indexPath(s, s.v1);
    rmSync(file);
    mkdirSync(file);
    const result = await alz(s, ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("[cache]");
    expect(result.stderr).toContain("delete it and rebuild");
    expect(lstatSync(file).isDirectory()).toBe(true);
    expect(noIndexTmp(s)).toEqual([]);
  });

  it("changes neither the generated output nor the lock whether the index is kept or rebuilt", async () => {
    const s = await built();
    const kept = readTree(`${s.ws.root}/repo`);
    rmSync(indexPath(s, s.v1));
    expect((await alz(s, ["build"])).code).toBe(0);
    expect(readTree(`${s.ws.root}/repo`)).toEqual(kept);
    expect(existsSync(indexPath(s, s.v1))).toBe(true);
  });

  it.each([
    ["short", (sha: string) => sha.slice(0, 39)],
    ["upper case", (sha: string) => sha.toUpperCase()],
    ["empty", () => ""],
  ])(
    "treats a %s lock sha as an unreadable lock (through alz account which)",
    async (_label, mangle) => {
      const s = allowClient(await built());
      editLock(s, (lock) => {
        lockHubEntry(lock).sha = mangle(s.v1);
      });
      expectMissing(await whichReading(s), "layer.lock cannot be read");
    },
  );

  it("misses when the lock names a commit that was never built (through alz account which)", async () => {
    const s = allowClient(await built());
    editLock(s, (lock) => {
      lockHubEntry(lock).sha = s.v2;
    });
    expectMissing(await whichReading(s), "the cache index entry is missing");
  });

  it("never reads another path's index entry, and leaves it alone (through alz account which)", async () => {
    const s = await built();
    const hubLock = lockOf(s);
    cpSync(s.hub, `${s.ws.root}/hub2`, { recursive: true });
    useStandard(s, "v1", "../hub2/layer");
    expect((await alz(s, ["build"])).code).toBe(0);
    expect(existsSync(indexPath(s, s.v1, `${s.real}/hub2/layer`))).toBe(true);

    // The stack names the first hub again, whose index entry is gone; the decoy has the same sha.
    useStandard(s);
    writeLock(s, hubLock);
    rmSync(indexPath(s, s.v1));
    allowClient(s);
    expectMissing(await whichReading(s), "the cache index entry is missing");
  });

  it.each([
    ["a trailing newline", (tree: string) => `${tree}\n`],
    ["39 characters", (tree: string) => tree.slice(0, 39)],
    ["upper case", (tree: string) => tree.toUpperCase()],
    ["a non-hexadecimal character", (tree: string) => `g${tree.slice(1)}`],
    ["64 characters for a 40-character commit", (tree: string) => `${tree}${tree.slice(0, 24)}`],
    ["no content", () => ""],
  ])(
    "misses on a malformed index entry: %s (through alz account which)",
    async (_label, mangle) => {
      const s = allowClient(await built());
      writeFileSync(indexPath(s, s.v1), mangle(treeOf(s, s.v1)));
      expectMissing(await whichReading(s), "the cache index entry is not valid");
    },
  );

  it("misses on a symlinked index entry, even to a good copy (through alz account which)", async () => {
    const s = allowClient(await built());
    const file = indexPath(s, s.v1);
    const copy = `${s.real}/good-index`;
    writeFileSync(copy, treeOf(s, s.v1));
    rmSync(file);
    symlinkSync(copy, file);
    expectMissing(await whichReading(s), "the cache index entry is not valid");
  });

  it("reads a path: layer without a ref in place, and an edit since the build is a miss (through alz account which)", async () => {
    const s = setup();
    useStack(s, PERSONAL, REPO_LAYER);
    expect((await alz(s, ["build"])).code).toBe(0);
    allowPersonal(s, `${s.real}/personal`);
    expectApplied(await whichReading(s), PERSONAL_LINES);

    writeFileSync(`${s.ws.root}/personal/instructions/10-personal.md`, "- Edited.\n");
    expectMissing(await whichReading(s), "a layer no longer matches layer.lock");
  });

  describe("layers inside the repository are read only to warn", () => {
    const REPO_YAML = (s: Setup) => `${s.ws.root}/repo/.layer/layer.yaml`;
    const BASE = "name: repo\nkind: repo\nvisibility: private\n";

    it("does not check them against the lock: editing .layer/ still reads (through alz account which)", async () => {
      const s = allowClient(await built());
      writeFileSync(`${s.ws.root}/repo/.layer/instructions/10-repo.md`, "- Edited since build.\n");
      writeFileSync(`${s.ws.root}/repo/.layer/instructions/20-new.md`, "- New file.\n");
      expectApplied(await whichReading(s), CLIENT_LINES);
    });

    it.each([
      ["larger than 1 MiB", () => `${BASE}account: other-a\n# ${"x".repeat(1_100_000)}\n`],
      ["not valid YAML", () => `${BASE}account: [other-a\n`],
      ["not a valid layer", () => "name: Not Valid\naccount: other-a\n"],
    ])(
      "skips a layer.yaml that is %s, with no miss (through alz account which)",
      async (_label, text) => {
        const s = allowClient(await built());
        writeFileSync(REPO_YAML(s), text());
        expectApplied(await whichReading(s), CLIENT_LINES);
      },
    );

    it("skips a layer.yaml that is a link, with no miss (through alz account which)", async () => {
      const s = allowClient(await built());
      writeFileSync(`${s.real}/outside.yaml`, `${BASE}account: other-a\n`);
      rmSync(REPO_YAML(s));
      symlinkSync(`${s.real}/outside.yaml`, REPO_YAML(s));
      expectApplied(await whichReading(s), CLIENT_LINES);
    });

    it("reports a declaration by the repo layer as ignored, and the winner does not change (through alz account which)", async () => {
      const s = allowClient(await built());
      writeFileSync(REPO_YAML(s), `${BASE}account: other-a\n`);
      expectApplied(await whichReading(s), CLIENT_LINES, [
        `the layer "repo" ("path:./.layer") declares an account but may not, so it is ignored: it is inside the repository; its kind is 'repo', not 'client'`,
      ]);
    });
  });
});

describe("§13.20 cache index and cache misses: reads only, checked against the lock (AC 2)", () => {
  it("proves the git stand-in and the snapshot work: alz build is seen calling git and writing, alz account which is not", async () => {
    const s = setup();
    useStandard(s);
    const proxy = gitProxy();
    const before = world(s);
    const build = await alz(s, ["build"], proxy.env);
    expect(build.code, build.stderr).toBe(0);
    // The stand-in runs the real git: the pinned layer is in the cache and the lock records it.
    expect(proxy.calls().length).toBeGreaterThan(0);
    // The same snapshot sees a build write: the cache, the lock and the generated files.
    expect(world(s)).not.toEqual(before);
    expect(proxy.calls().every((call) => call.cwd !== "" && call.args.length > 0)).toBe(true);
    expect(existsSync(indexPath(s, s.v1))).toBe(true);
    expect(lockHubEntry(lockOf(s)).sha).toBe(s.v1);

    allowClient(s);
    const reading = await whichReading(s);
    expectApplied(reading, CLIENT_LINES);
  });

  it("never runs git, fetches or writes, with other entries of the cache beside it (through alz account which)", async () => {
    const s = setup();
    // Decoys: another path's index entry with the same sha, another commit's index entry and
    // tree, and another git entry.
    cpSync(s.hub, `${s.ws.root}/hub2`, { recursive: true });
    useStandard(s, "v1");
    expect((await alz(s, ["build"])).code).toBe(0);
    useStandard(s, "v2", "../hub2/layer");
    expect((await alz(s, ["build"])).code).toBe(0);
    useStandard(s, "v2");
    expect((await alz(s, ["build"])).code).toBe(0);
    const gitDecoy = remoteDir(s, PERSONAL_URL, FIRST);
    mkdirSync(`${gitDecoy}/instructions`, { recursive: true });
    writeFileSync(`${gitDecoy}/layer.yaml`, "name: decoy\nkind: group\nvisibility: private\n");
    for (const decoy of [
      indexPath(s, s.v2, `${s.real}/hub2/layer`),
      indexPath(s, s.v1),
      `${s.cache}/${treeEntryName(treeOf(s, s.v1))}`,
      gitDecoy,
    ]) {
      expect(existsSync(decoy), decoy).toBe(true);
    }
    allowClient(s);
    expectApplied(await whichReading(s), CLIENT_LINES);
  });

  it.each([
    ["a tree2@ entry", true],
    ["a git entry", false],
  ])(
    "treats an edited %s as a content-hash miss (through alz account which)",
    async (_label, pinned) => {
      if (pinned) {
        const s = allowClient(await built());
        writeFileSync(
          `${s.cache}/${treeEntryName(treeOf(s, s.v1))}/instructions/10-client.md`,
          "x\n",
        );
        expectMissing(await whichReading(s), "a layer no longer matches layer.lock");
      } else {
        const s = setup();
        remoteStack(s, [
          { url: PERSONAL_URL, ref: "v1", sha: FIRST, files: filesOf(s, "personal") },
        ]);
        allowPersonal(s, PERSONAL_URL);
        expectApplied(await whichReading(s), PERSONAL_LINES);
        writeFileSync(
          `${remoteDir(s, PERSONAL_URL, FIRST)}/instructions/10-personal.md`,
          "- Edited.\n",
        );
        expectMissing(await whichReading(s), "a layer no longer matches layer.lock");
      }
    },
  );

  it("reports an invalid layer.yaml in the cache as a content-hash miss, so it is parsed after the check (through alz account which)", async () => {
    const s = allowClient(await built());
    writeFileSync(`${s.cache}/${treeEntryName(treeOf(s, s.v1))}/layer.yaml`, `name: [${MARKER}\n`);
    const result = await whichReading(s);
    expectMissing(result, "a layer no longer matches layer.lock");
    expectNoMarker(result);
  });

  it("reports an invalid layer.yaml that the lock records as layer-invalid (through alz account which)", async () => {
    const s = setup();
    remoteStack(s, [
      { url: PERSONAL_URL, ref: "v1", sha: FIRST, files: { "layer.yaml": `name: [${MARKER}\n` } },
    ]);
    allowPersonal(s, PERSONAL_URL);
    const result = await whichReading(s);
    expectMissing(result, "a layer is not valid");
    expectNoMarker(result);
  });

  it("misses on a layer with a .env file, without printing it (through alz account which)", async () => {
    const s = setup();
    remoteStack(s, [
      {
        url: PERSONAL_URL,
        ref: "v1",
        sha: FIRST,
        files: { ...filesOf(s, "personal"), ".env": `TOKEN=${MARKER}\n` },
      },
    ]);
    allowPersonal(s, PERSONAL_URL);
    const result = await whichReading(s);
    expectMissing(result, "a layer is not valid");
    expectNoMarker(result);
  });

  it("misses when the lock has no entry for a source (through alz account which)", async () => {
    const s = allowClient(await built());
    editLock(s, (lock) => {
      lock.layers = lock.layers.filter((layer) => layer.source !== "path:../hub/layer");
    });
    expectMissing(await whichReading(s), "layer.lock has no entry for a layer");
  });

  it("misses when there is no lock (through alz account which)", async () => {
    const s = allowClient(await built());
    rmSync(`${s.ws.root}/repo/layer.lock`);
    expectMissing(await whichReading(s), "layer.lock is missing");
  });

  it("misses when the lock is a symlink, even to a good lock (through alz account which)", async () => {
    const s = allowClient(await built());
    const lock = `${s.ws.root}/repo/layer.lock`;
    cpSync(lock, `${s.real}/good.lock`);
    rmSync(lock);
    symlinkSync(`${s.real}/good.lock`, lock);
    expectMissing(await whichReading(s), "layer.lock cannot be read");
  });

  it("misses when a lock entry has no content hash (through alz account which)", async () => {
    const s = allowClient(await built());
    editLock(s, (lock) => {
      lockHubEntry(lock).contentHash = undefined;
    });
    expectMissing(await whichReading(s), "layer.lock cannot be read");
  });

  it("misses when the lock has two entries for a source (through alz account which)", async () => {
    const s = allowClient(await built());
    editLock(s, (lock) => {
      lock.layers.push({ ...lockHubEntry(lock) });
    });
    expectMissing(await whichReading(s), "layer.lock has two entries for a layer");
  });

  it("misses when a pinned layer has no sha, or an unpinned one has one (through alz account which)", async () => {
    const s = allowClient(await built());
    const lock = lockOf(s);
    editLock(s, (value) => {
      lockHubEntry(value).sha = undefined;
    });
    const invalid = "layer.lock holds a commit id that is not valid";
    expectMissing(await whichReading(s), invalid);
    writeLock(s, lock);
    editLock(s, (value) => {
      const personal = value.layers.find((layer) => layer.source === "path:../personal");
      if (personal) personal.sha = s.v1;
    });
    expectMissing(await whichReading(s), invalid);
  });

  it("misses when the lock's sha does not belong to the entry it read (through alz account which)", async () => {
    const s = await built("v2");
    const v2Lock = lockOf(s);
    useStandard(s, "v1");
    expect((await alz(s, ["build"])).code).toBe(0);
    const v1Hash = lockHubEntry(lockOf(s)).contentHash;
    // Both commits are cached; the lock names v2's commit with v1's content hash.
    writeLock(s, v2Lock);
    editLock(s, (lock) => {
      lockHubEntry(lock).contentHash = v1Hash;
    });
    useStandard(s, "v2");
    allowClient(s);
    expectMissing(await whichReading(s), "a layer no longer matches layer.lock");
  });

  it.each([
    ["tree2@", true],
    ["git", false],
  ])(
    "misses on a %s entry that is a link to a good copy (through alz account which)",
    async (_label, pinned) => {
      const s = pinned ? await built() : setup();
      if (!pinned) {
        remoteStack(s, [
          { url: PERSONAL_URL, ref: "v1", sha: FIRST, files: filesOf(s, "personal") },
        ]);
      }
      const dir = pinned
        ? `${s.cache}/${treeEntryName(treeOf(s, s.v1))}`
        : remoteDir(s, PERSONAL_URL, FIRST);
      if (pinned) allowClient(s);
      else allowPersonal(s, PERSONAL_URL);
      const copy = `${s.real}/good-copy`;
      cpSync(dir, copy, { recursive: true });
      rmSync(dir, { recursive: true });
      symlinkSync(copy, dir);
      expectMissing(await whichReading(s), "a cached layer is not a directory");
    },
  );

  it("misses on a git entry that is missing (through alz account which)", async () => {
    const s = setup();
    remoteStack(s, [{ url: PERSONAL_URL, ref: "v1", sha: FIRST, files: filesOf(s, "personal") }]);
    allowPersonal(s, PERSONAL_URL);
    rmSync(remoteDir(s, PERSONAL_URL, FIRST), { recursive: true });
    expectMissing(await whichReading(s), "a cached layer is missing");
  });

  it("misses on a lock sha that is not 40 or 64 hex characters for a git source (through alz account which)", async () => {
    const s = setup();
    remoteStack(s, [{ url: PERSONAL_URL, ref: "v1", sha: FIRST, files: filesOf(s, "personal") }]);
    allowPersonal(s, PERSONAL_URL);
    editLock(s, (lock) => {
      const [only] = lock.layers;
      if (only) only.sha = undefined;
    });
    expectMissing(await whichReading(s), "layer.lock holds a commit id that is not valid");
  });

  it("fails closed on an invalid source (through alz account which)", async () => {
    const s = allowClient(setup());
    useStack(s, ["svn:example.invalid/x"], REPO_LAYER);
    writeLock(s, { lockVersion: 1, visibility: "private", layers: [], files: {} });
    expectFailsClosed(await whichReading(s));
  });

  it("fails closed, and never touches an account folder, when a layer directory is $HOME (through alz account which)", async () => {
    const s = allowClient(setup());
    mkdirSync(`${s.ws.home}/.claude-x`);
    writeFileSync(`${s.ws.home}/.claude-x/.claude.json`, MARKER);
    writeFileSync(
      `${s.ws.home}/layer.yaml`,
      "name: homelayer\nkind: personal\nvisibility: public\naccount: personal-a\n",
    );
    useStack(s, ["path:../../home"], REPO_LAYER);
    writeLock(s, {
      lockVersion: 1,
      visibility: "private",
      layers: [{ name: "homelayer", source: "path:../../home", contentHash: "sha256:0" }],
      files: {},
    });
    const result = await whichReading(s);
    expectFailsClosed(result);
    expectNoMarker(result);
  });
});

describe("§13.20 cache index and cache misses: the declaration (AC 3)", () => {
  it("lets the client layer's map win over the first layer's name (through alz account which)", async () => {
    const s = allowClient(await built());
    expectApplied(await whichReading(s), CLIENT_LINES);
  });

  it("keeps the recorded source across a ref bump (through alz account which)", async () => {
    // The allowed entry names the recorded source; a bump that changed it would be 'changed'.
    const s = allowClient(await built());
    expectApplied(await whichReading(s), CLIENT_LINES);
    useStandard(s, "v2");
    expect((await alz(s, ["build"])).code).toBe(0);
    expectApplied(await whichReading(s), CLIENT_LINES);
  });

  it("keeps the recorded source of a remote across a bump of its ref (through alz account which)", async () => {
    const s = setup();
    const files = filesOf(s, "personal");
    remoteStack(s, [{ url: PERSONAL_URL, ref: "v1", sha: FIRST, files }]);
    allowPersonal(s, PERSONAL_URL);
    expectApplied(await whichReading(s), PERSONAL_LINES);
    remoteStack(s, [{ url: PERSONAL_URL, ref: "v2", sha: SECOND, files }]);
    expectApplied(await whichReading(s), PERSONAL_LINES);
  });

  it("reports a group layer's declaration as ignored with its reasons (through alz account which)", async () => {
    const s = setup();
    remoteStack(s, [
      { url: PERSONAL_URL, ref: "v1", sha: FIRST, files: filesOf(s, "personal") },
      {
        url: "https://example.invalid/group.git",
        ref: "v1",
        sha: SECOND,
        files: filesOf(s, "group"),
      },
    ]);
    allowPersonal(s, PERSONAL_URL);
    expectApplied(await whichReading(s), PERSONAL_LINES, [
      `the layer "group" ("git:https://example.invalid/group.git@v1") declares an account but may not, so it is ignored: its kind is 'group', not 'client'`,
    ]);
  });

  it("blocks when two client layers declare (through alz account which)", async () => {
    const s = setup();
    const client = filesOf(s, "hub/layer");
    remoteStack(s, [
      { url: PERSONAL_URL, ref: "v1", sha: FIRST, files: filesOf(s, "personal") },
      { url: "https://example.invalid/client-a.git", ref: "v1", sha: SECOND, files: client },
      {
        url: "https://example.invalid/client-b.git",
        ref: "v1",
        sha: "3".repeat(40),
        files: {
          ...client,
          "layer.yaml": "name: client-b\nkind: client\nvisibility: private\naccount: client-b-a\n",
        },
      },
    ]);
    allowPersonal(s, PERSONAL_URL);
    const result = await whichReading(s);
    expectFailsClosed(result);
    expect(result.stdout).toContain(
      "layer step skipped (several client layers declare an account); fails closed",
    );
  });

  it("only reports a declaration by a repo layer at index 0: it never wins and never blocks (through alz account which)", async () => {
    const s = allowClient(await built());
    useStack(s, REPO_LAYER, ["path:../hub/layer", "v1"]);
    writeFileSync(
      `${s.ws.root}/repo/.layer/layer.yaml`,
      "name: repo\nkind: repo\nvisibility: private\naccount: other-a\n",
    );
    expectApplied(await whichReading(s), CLIENT_LINES, [
      `the layer "repo" ("path:./.layer") declares an account but may not, so it is ignored: it is inside the repository`,
    ]);
  });

  it("applies the client layer when an inside first layer that the build refuses declares (through alz account which)", async () => {
    const s = allowClient(await built());
    cpSync(`${s.ws.root}/personal`, `${s.ws.root}/repo/inside`, { recursive: true });
    useStack(s, ["path:./inside"], ["path:../hub/layer", "v1"], REPO_LAYER);
    const result = await whichReading(s);
    expectApplied(result, CLIENT_LINES, [
      `the layer "personal" ("path:./inside") declares an account but may not, so it is ignored: it is inside the repository`,
    ]);
  });
});

describe("§13.20 cache index and cache misses: layers that cannot be found or read", () => {
  it("fails closed when the directory of a pinned layer is gone, never skipping it as inside (through alz account which)", async () => {
    const s = allowClient(await built());
    rmSync(`${s.ws.root}/hub/layer`, { recursive: true });
    expectFailsClosed(await whichReading(s));
  });

  it("fails closed when the directory of an unpinned layer is gone (through alz account which)", async () => {
    const s = setup();
    useStack(s, PERSONAL, REPO_LAYER);
    expect((await alz(s, ["build"])).code).toBe(0);
    allowPersonal(s, `${s.real}/personal`);
    rmSync(`${s.ws.root}/personal`, { recursive: true });
    expectFailsClosed(await whichReading(s));
  });

  it("fails closed when a local git: source names a path that does not exist (through alz account which)", async () => {
    const s = allowClient(setup());
    useStack(s, [`git:${s.real}/nowhere@v1`], REPO_LAYER);
    writeLock(s, { lockVersion: 1, visibility: "private", layers: [], files: {} });
    expectFailsClosed(await whichReading(s));
  });

  it("misses when a pinned layer's tree2@ entry is gone but its index entry is still there (through alz account which)", async () => {
    const s = allowClient(await built());
    rmSync(`${s.cache}/${treeEntryName(treeOf(s, s.v1))}`, { recursive: true });
    expect(existsSync(indexPath(s, s.v1))).toBe(true);
    expectMissing(await whichReading(s), "a cached layer is missing");
  });

  it("misses when the manifest names another ref than the lock recorded (through alz account which)", async () => {
    const s = allowClient(await built());
    useStandard(s, "v2");
    expectMissing(await whichReading(s), "layer.lock records another ref than layer.yaml names");
  });

  it("misses when two layers outside the repository have the same name (through alz account which)", async () => {
    const s = setup();
    const files = filesOf(s, "personal");
    remoteStack(s, [
      { url: PERSONAL_URL, ref: "v1", sha: FIRST, files },
      { url: "https://example.invalid/personal-copy.git", ref: "v1", sha: SECOND, files },
    ]);
    allowPersonal(s, PERSONAL_URL);
    expectMissing(await whichReading(s), "the stack is not valid");
  });

  it("misses when the cache lies inside the repository (through alz account which)", async () => {
    const s = allowClient(await built());
    expectMissing(
      await whichReading(s, { XDG_CACHE_HOME: `${s.real}/repo/cache` }),
      "the layer cache is inside the repository",
    );
  });

  it("misses on a layer with more files than the limit, and on one with more bytes (through alz account which)", async () => {
    const s = setup();
    useStack(s, PERSONAL, REPO_LAYER);
    expect((await alz(s, ["build"])).code).toBe(0);
    allowPersonal(s, `${s.real}/personal`);
    expectApplied(await whichReading(s), PERSONAL_LINES);
    const extra = `${s.ws.root}/personal/extra`;
    mkdirSync(extra);
    for (let n = 0; n <= MAX_LAYER_ENTRIES; n++) writeFileSync(`${extra}/f${n}.md`, "x");
    expectMissing(await whichReading(s), "a layer or the lock cannot be read");
    rmSync(extra, { recursive: true });
    writeFileSync(`${s.ws.root}/personal/big.bin`, Buffer.alloc(MAX_LAYER_BYTES + 1));
    expectMissing(await whichReading(s), "a layer or the lock cannot be read");
  });

  it("does not follow a link inside a cached layer: a replaced file is a content-hash miss (through alz account which)", async () => {
    const s = setup();
    remoteStack(s, [{ url: PERSONAL_URL, ref: "v1", sha: FIRST, files: filesOf(s, "personal") }]);
    allowPersonal(s, PERSONAL_URL);
    const dir = remoteDir(s, PERSONAL_URL, FIRST);
    expectApplied(await whichReading(s), PERSONAL_LINES);
    for (const file of ["layer.yaml", "instructions/10-personal.md"]) {
      const copy = `${s.real}/copy-${file.replaceAll("/", "-")}`;
      cpSync(`${dir}/${file}`, copy);
      rmSync(`${dir}/${file}`);
      symlinkSync(copy, `${dir}/${file}`);
      expectMissing(await whichReading(s), "a layer no longer matches layer.lock");
      rmSync(`${dir}/${file}`);
      cpSync(copy, `${dir}/${file}`);
      expectApplied(await whichReading(s), PERSONAL_LINES);
    }
  });

  it.each([
    ["an absolute path: source", (home: string) => `path:${home}/.claude-x/foo`],
    ["a case variant of one", (home: string) => `path:${home}/.CLAUDE-X/foo`],
    ["a local git: source", (home: string) => `git:${home}/.claude/projects@v1`],
    ["a file:// git: source", (home: string) => `git:file://${home}/.codex/x@v1`],
    ["the claude.json file", (home: string) => `path:${home}/.claude.json`],
  ])(
    "fails closed on %s that names an account folder, leaving HOME as it was (through alz account which)",
    async (_label, source) => {
      const s = allowClient(setup());
      useStack(s, [source(s.ws.home)], REPO_LAYER);
      writeLock(s, { lockVersion: 1, visibility: "private", layers: [], files: {} });
      expectFailsClosed(await whichReading(s));
    },
  );

  it("fails closed on a git: source with a '..' segment (through alz account which)", async () => {
    const s = allowClient(setup());
    useStack(s, ["git:link/../x@v1"], REPO_LAYER);
    writeLock(s, { lockVersion: 1, visibility: "private", layers: [], files: {} });
    expectFailsClosed(await whichReading(s));
  });

  it("warns, and does not fail the build, when the index entry cannot be written", async () => {
    if (process.getuid?.() === 0) return; // root ignores the permission this relies on
    const s = await built();
    rmSync(indexPath(s, s.v1));
    chmodSync(s.cache, 0o555);
    try {
      const result = await alz(s, ["build"]);
      expect(result.code, result.stderr).toBe(0);
      expect(result.stdout + result.stderr).toContain("warning [cache-index]");
    } finally {
      chmodSync(s.cache, 0o755);
    }
    expect(existsSync(indexPath(s, s.v1))).toBe(false);
    expect(noIndexTmp(s)).toEqual([]);
    allowClient(s);
    expectMissing(await whichReading(s), "the cache index entry is missing");
  });
});
