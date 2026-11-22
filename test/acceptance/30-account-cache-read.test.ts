import { execFileSync } from "node:child_process";
import {
  chmodSync,
  constants,
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { describe, expect, it, vi } from "vitest";
import { Diagnostics } from "../../src/core/diagnostics";
import { evaluateAccount, mergeAccount } from "../../src/merge/account";
import type { Layer } from "../../src/model/layer";
import type { LayerMeta } from "../../src/model/layer-meta";
import { readManifest } from "../../src/model/manifest";
import {
  type AccountDeclaration,
  type AccountDeclarationInput,
  readAccountDeclaration,
  recordedSource,
} from "../../src/resolve/account-declaration";
import { trace, traced } from "../fs-trace";
import { fixture, layers, readTree, type Workspace } from "../helpers";
import { cacheRoot, gitEntryName, pathIndexName, sha256Hex, treeEntryName } from "../spec/cache";
import { contentHash } from "../spec/content-hash";
import { MAX_LAYER_BYTES, MAX_LAYER_ENTRIES } from "../spec/limits";

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

const FIXTURE = "30-account-cache-read";
const GIT_ENV = {
  PATH: process.env.PATH ?? "",
  GIT_CONFIG_NOSYSTEM: "1",
  GIT_AUTHOR_NAME: "Test",
  GIT_AUTHOR_EMAIL: "test@example.invalid",
  GIT_COMMITTER_NAME: "Test",
  GIT_COMMITTER_EMAIL: "test@example.invalid",
};

function sh(ws: Workspace, cwd: string, args: string[]): string {
  return execFileSync("git", ["-c", "commit.gpgsign=false", ...args], {
    cwd,
    encoding: "utf8",
    env: { ...GIT_ENV, HOME: ws.home },
  }).trim();
}

interface Setup {
  ws: Workspace;
  /** The real path of the workspace: what the cache and the reader see. */
  real: string;
  hub: string;
  v1: string;
  v2: string;
  cache: string;
  /** The real path of the hub's `layer/` directory: the key of its index entries. */
  hubLayer: string;
}

/** The hub is a git repository tagged `v1` and `v2`; the layer's content differs between them. */
function setup(): Setup {
  const ws = fixture(FIXTURE);
  const real = realpathSync.native(ws.root);
  const hub = `${ws.root}/hub`;
  sh(ws, hub, ["init", "--quiet", "--initial-branch=main"]);
  sh(ws, hub, ["add", "."]);
  sh(ws, hub, ["commit", "--quiet", "-m", "hub v1"]);
  sh(ws, hub, ["tag", "v1"]);
  writeFileSync(`${hub}/layer/instructions/10-client.md`, "- Client rule, second edition.\n");
  sh(ws, hub, ["add", "."]);
  sh(ws, hub, ["commit", "--quiet", "-m", "hub v2"]);
  sh(ws, hub, ["tag", "v2"]);
  return {
    ws,
    real,
    hub,
    v1: sh(ws, hub, ["rev-parse", "v1"]),
    v2: sh(ws, hub, ["rev-parse", "v2"]),
    cache: `${real}/cache/agent-layerz`,
    hubLayer: `${real}/hub/layer`,
  };
}

function cacheEnv(s: Setup): Record<string, string> {
  return { XDG_CACHE_HOME: `${s.real}/cache` };
}

function alz(s: Setup, args: string[]) {
  return layers(s.ws, "repo", args, cacheEnv(s));
}

function stackYaml(...entries: [source: string, ref?: string][]): string {
  const items = entries
    .map(([source, ref]) => `  - source: ${source}\n${ref ? `    ref: ${ref}\n` : ""}`)
    .join("");
  return `stack:\n${items}targets: [claude]\n`;
}

const PERSONAL: [string] = ["path:../personal"];
const REPO_LAYER: [string] = ["path:./.layer"];

function useStack(s: Setup, ...entries: [source: string, ref?: string][]): void {
  writeFileSync(`${s.ws.root}/repo/layer.yaml`, stackYaml(...entries));
}

/** The standard stack: personal in place, the hub's layer pinned to a ref, the repo layer. */
function useStandard(s: Setup, ref = "v1", hubDir = "../hub/layer"): void {
  useStack(s, PERSONAL, [`path:${hubDir}`, ref], REPO_LAYER);
}

async function built(ref = "v1"): Promise<Setup> {
  const s = setup();
  useStandard(s, ref);
  const result = await alz(s, ["build"]);
  expect(result.code, result.stderr).toBe(0);
  return s;
}

/** The reader's input, made before a trace starts: the manifest is read by the caller. */
function inputFor(s: Setup, extraEnv: Record<string, string> = {}): AccountDeclarationInput {
  const repoRoot = `${s.real}/repo`;
  const env = { ...cacheEnv(s), ...extraEnv };
  return {
    repoRoot,
    stack: readManifest(repoRoot).stack,
    cacheDir: cacheRoot(s.ws.home, env),
    home: s.ws.home,
    env,
  };
}

function read(s: Setup, extraEnv: Record<string, string> = {}): AccountDeclaration {
  return readAccountDeclaration(inputFor(s, extraEnv));
}

type LockJson = {
  layers: { name: string; source: string; rev?: string; sha?: string; contentHash?: string }[];
  [key: string]: unknown;
};

function lockOf(s: Setup): LockJson {
  return JSON.parse(readFileSync(`${s.ws.root}/repo/layer.lock`, "utf8")) as LockJson;
}

function writeLock(s: Setup, lock: unknown): void {
  writeFileSync(`${s.ws.root}/repo/layer.lock`, JSON.stringify(lock));
}

function editLock(s: Setup, edit: (lock: LockJson) => void): void {
  const lock = lockOf(s);
  edit(lock);
  writeLock(s, lock);
}

function hubEntry(lock: LockJson): LockJson["layers"][number] {
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

function expectMiss(result: AccountDeclaration, reason: string, entry?: { index: number }): void {
  expect(result.status).toBe("miss");
  if (result.status !== "miss") return;
  expect(result.reason).toBe(reason);
  if (entry) expect(result.entry?.index).toBe(entry.index);
}

function expectConflict(result: AccountDeclaration) {
  expect(result.status, JSON.stringify(result)).toBe("conflict");
  if (result.status !== "conflict") throw new Error("not a conflict");
  return result;
}

function expectRead(result: AccountDeclaration) {
  expect(result.status, JSON.stringify(result)).toBe("read");
  if (result.status !== "read") throw new Error("not read");
  return result;
}

/** A remote layer written into the cache from fixture files, with the lock entry that records it. */
const SECOND = "2".repeat(40);
const FIRST = "1".repeat(40);

interface RemoteLayer {
  source: string;
  sha: string;
  files: Record<string, string>;
}

function lockEntry(cache: string, remote: RemoteLayer, url: string) {
  const dir = `${cache}/${gitEntryName(url, remote.sha)}`;
  mkdirSync(dir, { recursive: true });
  for (const [path, content] of Object.entries(remote.files)) {
    mkdirSync(dirname(`${dir}/${path}`), { recursive: true });
    writeFileSync(`${dir}/${path}`, content);
  }
  const name = /^name: ([a-z0-9.-]+)$/m.exec(remote.files["layer.yaml"] ?? "")?.[1] ?? "x";
  return {
    dir,
    entry: {
      name,
      source: remote.source,
      rev: "v1",
      sha: remote.sha,
      contentHash: `sha256:${contentHash(dir)}`,
    },
  };
}

function remoteStack(
  s: Setup,
  remotes: { url: string; ref: string; sha: string; files: Record<string, string> }[],
): string[] {
  const entries = remotes.map((item) => {
    const source = `git:${item.url}@${item.ref}`;
    return {
      ...lockEntry(s.cache, { source, sha: item.sha, files: item.files }, item.url),
      source,
    };
  });
  useStack(s, ...entries.map((entry): [string] => [entry.source]), REPO_LAYER);
  writeLock(s, {
    lockVersion: 1,
    visibility: "private",
    layers: entries.map((entry) => entry.entry),
    files: {},
  });
  return entries.map((entry) => entry.dir);
}

function filesOf(s: Setup, dir: string): Record<string, string> {
  return readTree(`${s.ws.root}/${dir}`);
}

const PERSONAL_URL = "https://example.invalid/personal.git";

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

describe("§13.20 cache index and cache misses: the pathref@ index (AC 1)", () => {
  it("finds a pinned path: layer through the index and reports the client layer's map", async () => {
    const s = await built();
    const result = expectRead(read(s));
    expect(result.declared).toEqual({
      account: { claude: "client-a", codex: "client-a-api" },
      layer: { index: 1, name: "client", source: "path:../hub/layer" },
      ref: "v1",
      sha: s.v1,
      recordedSource: s.hubLayer,
    });
    expect(result.ignored).toEqual([]);
  });

  it("is written by build next to the tree2@ entry, holds the tree id and is not rewritten", async () => {
    const s = await built();
    const tree = treeOf(s, s.v1);
    const file = indexPath(s, s.v1);
    expect(readFileSync(file, "utf8")).toBe(tree);
    expect(statSync(file).mode & 0o777).toBe(0o644);
    expect(existsSync(`${s.cache}/${treeEntryName(tree)}`)).toBe(true);
    expect(readdirSync(s.cache).filter((name) => name.startsWith(".tmp-"))).toEqual([]);

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
    expect(readdirSync(s.cache).filter((name) => name.startsWith(".tmp-"))).toEqual([]);
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
  ])("treats a %s lock sha as an unreadable lock", async (_label, mangle) => {
    const s = await built();
    editLock(s, (lock) => {
      hubEntry(lock).sha = mangle(s.v1);
    });
    expectMiss(read(s), "lock-unreadable");
  });

  it("misses when the lock names a commit that was never built", async () => {
    const s = await built();
    editLock(s, (lock) => {
      hubEntry(lock).sha = s.v2;
    });
    expectMiss(read(s), "index-missing", { index: 1 });
  });

  it("never reads another path's index entry, and leaves it alone", async () => {
    const s = await built();
    const hubLock = lockOf(s);
    cpSync(s.hub, `${s.ws.root}/hub2`, { recursive: true });
    useStandard(s, "v1", "../hub2/layer");
    expect((await alz(s, ["build"])).code).toBe(0);
    const decoy = indexPath(s, s.v1, `${s.real}/hub2/layer`);
    expect(existsSync(decoy)).toBe(true);

    // The stack names the first hub again, whose index entry is gone; the decoy has the same sha.
    useStandard(s);
    writeLock(s, hubLock);
    rmSync(indexPath(s, s.v1));
    const before = listing(s.cache);
    expectMiss(read(s), "index-missing", { index: 1 });
    expect(listing(s.cache)).toEqual(before);
  });

  it.each([
    ["a trailing newline", (tree: string) => `${tree}\n`],
    ["39 characters", (tree: string) => tree.slice(0, 39)],
    ["upper case", (tree: string) => tree.toUpperCase()],
    ["a non-hexadecimal character", (tree: string) => `g${tree.slice(1)}`],
    ["64 characters for a 40-character commit", (tree: string) => `${tree}${tree.slice(0, 24)}`],
    ["no content", () => ""],
  ])("misses on a malformed index entry: %s", async (_label, mangle) => {
    const s = await built();
    writeFileSync(indexPath(s, s.v1), mangle(treeOf(s, s.v1)));
    expectMiss(read(s), "index-invalid", { index: 1 });
  });

  it("misses on a symlinked index entry, even to a good copy", async () => {
    const s = await built();
    const file = indexPath(s, s.v1);
    const copy = `${s.real}/good-index`;
    writeFileSync(copy, treeOf(s, s.v1));
    rmSync(file);
    symlinkSync(copy, file);
    expectMiss(read(s), "index-invalid", { index: 1 });
  });

  it("reads a path: layer without a ref in place, and an edit since the build is a miss", async () => {
    const s = setup();
    useStack(s, PERSONAL, REPO_LAYER);
    expect((await alz(s, ["build"])).code).toBe(0);
    const first = expectRead(read(s));
    expect(first.declared?.account).toEqual({ claude: "personal-a", codex: "personal-a" });
    expect(first.declared?.sha).toBeUndefined();
    expect(first.declared?.recordedSource).toBe(`${s.real}/personal`);
    expect(first.declared?.layer).toEqual({
      index: 0,
      name: "personal",
      source: "path:../personal",
    });

    writeFileSync(`${s.ws.root}/personal/instructions/10-personal.md`, "- Edited.\n");
    expectMiss(read(s), "content-hash", { index: 0 });
  });

  describe("layers inside the repository are read only to warn", () => {
    const REPO_YAML = (s: Setup) => `${s.ws.root}/repo/.layer/layer.yaml`;
    const BASE = "name: repo\nkind: repo\nvisibility: private\n";

    it("does not check them against the lock: editing .layer/ still reads", async () => {
      const s = await built();
      const lock = readFileSync(`${s.ws.root}/repo/layer.lock`, "utf8");
      writeFileSync(`${s.ws.root}/repo/.layer/instructions/10-repo.md`, "- Edited since build.\n");
      writeFileSync(`${s.ws.root}/repo/.layer/instructions/20-new.md`, "- New file.\n");
      const result = expectRead(read(s));
      expect(result.declared?.layer.name).toBe("client");
      expect(readFileSync(`${s.ws.root}/repo/layer.lock`, "utf8")).toBe(lock);
    });

    it.each([
      ["larger than 1 MiB", () => `${BASE}account: other-a\n# ${"x".repeat(1_100_000)}\n`],
      ["not valid YAML", () => `${BASE}account: [other-a\n`],
      ["not a valid layer", () => "name: Not Valid\naccount: other-a\n"],
    ])("skips a layer.yaml that is %s, with no miss", async (_label, text) => {
      const s = await built();
      writeFileSync(REPO_YAML(s), text());
      const result = expectRead(read(s));
      expect(result.declared?.layer.name).toBe("client");
      expect(result.ignored).toEqual([]);
    });

    it("skips a layer.yaml that is a link, with no miss", async () => {
      const s = await built();
      writeFileSync(`${s.real}/outside.yaml`, `${BASE}account: other-a\n`);
      rmSync(REPO_YAML(s));
      symlinkSync(`${s.real}/outside.yaml`, REPO_YAML(s));
      const result = expectRead(read(s));
      expect(result.ignored).toEqual([]);
      expect(result.declared?.layer.name).toBe("client");
    });

    it("reports a declaration by the repo layer as ignored, and the winner does not change", async () => {
      const s = await built();
      writeFileSync(REPO_YAML(s), `${BASE}account: other-a\n`);
      const result = expectRead(read(s));
      expect(result.declared?.layer.name).toBe("client");
      expect(result.ignored).toEqual([
        {
          layer: { index: 2, name: "repo", source: "path:./.layer" },
          reasons: ["it is inside the repository", "its kind is 'repo', not 'client'"],
        },
      ]);
    });
  });
});

describe("§13.20 cache index and cache misses: reads only, checked against the lock (AC 2)", () => {
  it("proves the trace works: a build is seen running git and writing", async () => {
    const s = setup();
    useStandard(s);
    trace.calls = [];
    trace.fds.clear();
    trace.on = true;
    let result: Awaited<ReturnType<typeof alz>>;
    try {
      result = await alz(s, ["build"]);
    } finally {
      trace.on = false;
    }
    expect(result.code, result.stderr).toBe(0);
    const gitCalls = trace.calls.filter((call) => call.api === "child_process");
    expect(gitCalls.length).toBeGreaterThan(0);
    expect(gitCalls.some((call) => call.fn === "execFileSync")).toBe(true);
    const writes = trace.calls.filter((call) => call.api === "fs");
    for (const name of ["mkdirSync", "writeFileSync", "renameSync"]) {
      expect(
        writes.some((call) => call.fn === name),
        name,
      ).toBe(true);
    }
    expect(
      writes.some(
        (call) => call.fn === "renameSync" && call.paths.some((p) => p.includes("pathref@")),
      ),
    ).toBe(true);
    expect(writes.some((call) => call.fn === "openSync" || call.fn === "readFileSync")).toBe(true);
  });

  it("never runs git, fetches or writes, and opens only what it reads", async () => {
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
    const gitDecoy = `${s.cache}/${gitEntryName(PERSONAL_URL, FIRST)}`;
    mkdirSync(`${gitDecoy}/instructions`, { recursive: true });
    writeFileSync(`${gitDecoy}/layer.yaml`, "name: decoy\nkind: group\nvisibility: private\n");
    const decoys = [
      indexPath(s, s.v2, `${s.real}/hub2/layer`),
      indexPath(s, s.v1),
      `${s.cache}/${treeEntryName(treeOf(s, s.v1))}`,
      gitDecoy,
    ];
    for (const decoy of decoys) expect(existsSync(decoy), decoy).toBe(true);

    const tree = treeOf(s, s.v2);
    const reposBefore = listing(`${s.real}/repo`);
    const cacheBefore = listing(s.cache);
    const input = inputFor(s);
    const { result, calls } = traced(() => readAccountDeclaration(input));
    expect(listing(`${s.real}/repo`)).toEqual(reposBefore);
    expect(listing(s.cache)).toEqual(cacheBefore);
    expect(expectRead(result).declared?.sha).toBe(s.v2);

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
      "readFileSync",
      "readdirSync",
    ]);
    const fsCalls = calls.filter((call) => call.api === "fs");
    expect(fsCalls.length).toBeGreaterThan(0);
    expect(fsCalls.filter((call) => !readOnly.has(call.fn)).map((call) => call.fn)).toEqual([]);
    // Every file is opened through readRegularFile: read-only and refusing a link.
    const writing =
      constants.O_WRONLY |
      constants.O_RDWR |
      constants.O_CREAT |
      constants.O_TRUNC |
      constants.O_APPEND;
    expect(fsCalls.filter((call) => call.fn === "readFileSync")).toEqual([]);
    for (const call of fsCalls.filter((item) => item.fn === "openSync")) {
      const flags = call.flags;
      expect(
        typeof flags === "number" &&
          (flags & writing) === 0 &&
          (flags & constants.O_NOFOLLOW) !== 0,
        String(flags),
      ).toBe(true);
    }

    const lock = `${s.real}/repo/layer.lock`;
    const index = indexPath(s, s.v2);
    const allowedPrefixes = [`${s.cache}/${treeEntryName(tree)}/`, `${s.real}/personal/`];
    const allowedFiles = [lock, index, `${s.real}/repo/.layer/layer.yaml`];
    const reads = fsCalls.filter((call) => call.fn === "openSync" || call.fn === "readFileSync");
    for (const path of reads.flatMap((call) => call.paths)) {
      const ok =
        allowedFiles.includes(path) || allowedPrefixes.some((prefix) => path.startsWith(prefix));
      expect(ok, path).toBe(true);
    }
    const readPaths = reads.flatMap((call) => call.paths);
    expect(readPaths).toContain(lock);
    expect(readPaths).toContain(index);
    expect(readPaths).toContain(`${s.cache}/${treeEntryName(tree)}/layer.yaml`);

    for (const path of fsCalls.flatMap((call) => call.paths)) {
      for (const decoy of decoys) {
        expect(path === decoy || path.startsWith(`${decoy}/`), `${path} is a decoy`).toBe(false);
      }
    }
    expect(fsCalls.some((call) => call.fn === "readdirSync" && call.paths[0] === s.cache)).toBe(
      false,
    );
  });

  it("imports neither git nor child_process in the reader modules", () => {
    const root = join(import.meta.dirname, "../../src");
    const seen = new Set<string>();
    const rawImports = new Set<string>();
    const visit = (file: string) => {
      if (seen.has(file)) return;
      seen.add(file);
      const text = readFileSync(file, "utf8");
      for (const match of text.matchAll(/(?:from|import)\s*\(?\s*["']([^"']+)["']/g)) {
        const target = match[1] ?? "";
        rawImports.add(target);
        if (!target.startsWith(".")) continue;
        const base = join(dirname(file), target);
        const found = [`${base}.ts`, join(base, "index.ts")].find((candidate) =>
          existsSync(candidate),
        );
        if (found) visit(found);
      }
    };
    visit(join(root, "resolve/account-declaration.ts"));
    const names = [...seen].map((file) => file.slice(root.length + 1));
    expect(names).toContain("resolve/cached-stack.ts");
    expect(names).toContain("resolve/path-index.ts");
    expect(names).toContain("merge/account.ts");
    for (const banned of ["resolve/git.ts", "resolve/tree-cache.ts", "resolve/sources.ts"]) {
      expect(names, banned).not.toContain(banned);
    }
    expect([...rawImports].filter((name) => /child_process/.test(name))).toEqual([]);
  });

  it.each([
    ["a tree2@ entry", true],
    ["a git entry", false],
  ])("treats an edited %s as a content-hash miss", async (_label, pinned) => {
    if (pinned) {
      const s = await built();
      writeFileSync(
        `${s.cache}/${treeEntryName(treeOf(s, s.v1))}/instructions/10-client.md`,
        "x\n",
      );
      expectMiss(read(s), "content-hash", { index: 1 });
    } else {
      const s = setup();
      const [dir] = remoteStack(s, [
        { url: PERSONAL_URL, ref: "v1", sha: FIRST, files: filesOf(s, "personal") },
      ]);
      expectRead(read(s));
      writeFileSync(`${dir}/instructions/10-personal.md`, "- Edited.\n");
      expectMiss(read(s), "content-hash", { index: 0 });
    }
  });

  it("reports an invalid layer.yaml in the cache as a content-hash miss, so it is parsed after the check", async () => {
    const s = await built();
    writeFileSync(
      `${s.cache}/${treeEntryName(treeOf(s, s.v1))}/layer.yaml`,
      "name: [SECRET-MARKER\n",
    );
    const result = read(s);
    expectMiss(result, "content-hash", { index: 1 });
    expect(JSON.stringify(result)).not.toContain("SECRET-MARKER");
  });

  it("reports an invalid layer.yaml that the lock records as layer-invalid", () => {
    const s = setup();
    remoteStack(s, [
      {
        url: PERSONAL_URL,
        ref: "v1",
        sha: FIRST,
        files: { "layer.yaml": "name: [SECRET-MARKER\n" },
      },
    ]);
    const result = read(s);
    expectMiss(result, "layer-invalid", { index: 0 });
    expect(JSON.stringify(result)).not.toContain("SECRET-MARKER");
  });

  it("misses on a layer with a .env file, without reading it", () => {
    const s = setup();
    remoteStack(s, [
      {
        url: PERSONAL_URL,
        ref: "v1",
        sha: FIRST,
        files: { ...filesOf(s, "personal"), ".env": "TOKEN=SECRET-MARKER\n" },
      },
    ]);
    const input = inputFor(s);
    const { result, calls } = traced(() => readAccountDeclaration(input));
    expectMiss(result, "layer-invalid", { index: 0 });
    expect(calls.flatMap((call) => call.paths).some((path) => path.endsWith("/.env"))).toBe(false);
  });

  it("misses when the lock has no entry for a source", async () => {
    const s = await built();
    editLock(s, (lock) => {
      lock.layers = lock.layers.filter((layer) => layer.source !== "path:../hub/layer");
    });
    expectMiss(read(s), "lock-entry-missing", { index: 1 });
  });

  it("misses when there is no lock", async () => {
    const s = await built();
    rmSync(`${s.ws.root}/repo/layer.lock`);
    expectMiss(read(s), "lock-missing");
  });

  it("misses when the lock is a symlink, even to a good lock", async () => {
    const s = await built();
    const lock = `${s.ws.root}/repo/layer.lock`;
    cpSync(lock, `${s.real}/good.lock`);
    rmSync(lock);
    symlinkSync(`${s.real}/good.lock`, lock);
    expectMiss(read(s), "lock-unreadable");
  });

  it("misses when a lock entry has no content hash", async () => {
    const s = await built();
    editLock(s, (lock) => {
      hubEntry(lock).contentHash = undefined;
    });
    expectMiss(read(s), "lock-unreadable");
  });

  it("misses when the lock has two entries for a source", async () => {
    const s = await built();
    editLock(s, (lock) => {
      lock.layers.push({ ...hubEntry(lock) });
    });
    expectMiss(read(s), "lock-entry-duplicate", { index: 1 });
  });

  it("misses when a pinned layer has no sha, or an unpinned one has one", async () => {
    const s = await built();
    const lock = lockOf(s);
    editLock(s, (value) => {
      hubEntry(value).sha = undefined;
    });
    expectMiss(read(s), "lock-sha-invalid", { index: 1 });
    writeLock(s, lock);
    editLock(s, (value) => {
      const personal = value.layers.find((layer) => layer.source === "path:../personal");
      if (personal) personal.sha = s.v1;
    });
    expectMiss(read(s), "lock-sha-invalid", { index: 0 });
  });

  it("misses when the lock's sha does not belong to the entry it read", async () => {
    const s = await built("v2");
    const v2Lock = lockOf(s);
    useStandard(s, "v1");
    expect((await alz(s, ["build"])).code).toBe(0);
    const v1Hash = hubEntry(lockOf(s)).contentHash;
    // Both commits are cached; the lock names v2's commit with v1's content hash.
    writeLock(s, v2Lock);
    editLock(s, (lock) => {
      hubEntry(lock).contentHash = v1Hash;
    });
    useStandard(s, "v2");
    expectMiss(read(s), "content-hash", { index: 1 });
  });

  it.each([
    ["tree2@", true],
    ["git", false],
  ])("misses on a %s entry that is a link to a good copy", async (_label, pinned) => {
    let dir: string;
    let s: Setup;
    if (pinned) {
      s = await built();
      dir = `${s.cache}/${treeEntryName(treeOf(s, s.v1))}`;
    } else {
      s = setup();
      [dir = ""] = remoteStack(s, [
        { url: PERSONAL_URL, ref: "v1", sha: FIRST, files: filesOf(s, "personal") },
      ]);
    }
    const copy = `${s.real}/good-copy`;
    cpSync(dir, copy, { recursive: true });
    rmSync(dir, { recursive: true });
    symlinkSync(copy, dir);
    expectMiss(read(s), "cache-entry-not-directory");
  });

  it("misses on a git entry that is missing", () => {
    const s = setup();
    const [dir = ""] = remoteStack(s, [
      { url: PERSONAL_URL, ref: "v1", sha: FIRST, files: filesOf(s, "personal") },
    ]);
    rmSync(dir, { recursive: true });
    expectMiss(read(s), "cache-entry-missing", { index: 0 });
  });

  it("misses on a lock sha that is not 40 or 64 hex characters for a git source", () => {
    const s = setup();
    remoteStack(s, [{ url: PERSONAL_URL, ref: "v1", sha: FIRST, files: filesOf(s, "personal") }]);
    editLock(s, (lock) => {
      const [only] = lock.layers;
      if (only) only.sha = undefined;
    });
    expectMiss(read(s), "lock-sha-invalid", { index: 0 });
  });

  it("misses on an invalid source", () => {
    const s = setup();
    useStack(s, ["svn:example.invalid/x"], REPO_LAYER);
    writeLock(s, { lockVersion: 1, visibility: "private", layers: [], files: {} });
    expectMiss(read(s), "source-invalid", { index: 0 });
  });

  it("misses without opening anything when a layer directory is $HOME, and never touches an account folder", async () => {
    const s = setup();
    mkdirSync(`${s.ws.home}/.claude-x`);
    writeFileSync(`${s.ws.home}/.claude-x/.claude.json`, "SECRET-MARKER");
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
    const before = listing(s.ws.home);
    const input = inputFor(s);
    const { result, calls } = traced(() => readAccountDeclaration(input));
    expectMiss(result, "account-folder", { index: 0 });
    expect(listing(s.ws.home)).toEqual(before);
    const home = realpathSync.native(s.ws.home);
    const touched = calls.filter((call) => call.api === "fs").flatMap((call) => call.paths);
    expect(touched.filter((path) => path.startsWith(`${home}/.claude-x`))).toEqual([]);
    const opened = calls
      .filter((call) => call.fn === "openSync" || call.fn === "readFileSync")
      .flatMap((call) => call.paths);
    expect(opened.filter((path) => path.startsWith(home))).toEqual([]);
  });

  it("misses when a layer is the folder CLAUDE_CONFIG_DIR or CODEX_HOME names", async () => {
    const s = setup();
    useStack(s, PERSONAL, REPO_LAYER);
    expect((await alz(s, ["build"])).code).toBe(0);
    expectMiss(read(s, { CLAUDE_CONFIG_DIR: `${s.real}/personal` }), "account-folder", {
      index: 0,
    });
    expectMiss(read(s, { CODEX_HOME: `${s.real}/personal/instructions` }), "account-folder", {
      index: 0,
    });
    expectRead(read(s, { CODEX_HOME: `${s.real}/elsewhere` }));
  });

  it("misses when the repository root lies inside an account folder", async () => {
    const s = setup();
    useStack(s, PERSONAL, REPO_LAYER);
    expect((await alz(s, ["build"])).code).toBe(0);
    expectMiss(read(s, { CLAUDE_CONFIG_DIR: s.real }), "account-folder");
  });
});

describe("§13.20 cache index and cache misses: the declaration (AC 3)", () => {
  it("lets the client layer's map win over the first layer's name", async () => {
    const s = await built();
    const result = expectRead(read(s));
    expect(result.declared?.layer.name).toBe("client");
    expect(result.declared?.account).toEqual({ claude: "client-a", codex: "client-a-api" });
    expect(result.declared?.sha).toBe(s.v1);
    expect(result.declared?.recordedSource).toBe(s.hubLayer);
  });

  it("keeps the recorded source across a ref bump and changes the sha", async () => {
    const s = await built();
    const before = expectRead(read(s)).declared;
    useStandard(s, "v2");
    expect((await alz(s, ["build"])).code).toBe(0);
    const after = expectRead(read(s)).declared;
    expect(after?.recordedSource).toBe(before?.recordedSource);
    expect(before?.sha).toBe(s.v1);
    expect(after?.sha).toBe(s.v2);
  });

  it("keeps the recorded source of a remote across a bump of its ref", () => {
    const s = setup();
    const files = filesOf(s, "personal");
    remoteStack(s, [{ url: PERSONAL_URL, ref: "v1", sha: FIRST, files }]);
    const one = expectRead(read(s)).declared;
    remoteStack(s, [{ url: PERSONAL_URL, ref: "v2", sha: SECOND, files }]);
    const two = expectRead(read(s)).declared;
    expect(one?.recordedSource).toBe(PERSONAL_URL);
    expect(two?.recordedSource).toBe(PERSONAL_URL);
    expect([one?.sha, two?.sha]).toEqual([FIRST, SECOND]);
  });

  it("records a github: source as its clone URL and a local layer as its real path", () => {
    expect(recordedSource({ source: "github:o/r@x" }, "/nonexistent/repo")).toBe(
      "https://github.com/o/r.git",
    );
    expect(recordedSource({ source: `git:${PERSONAL_URL}@v1` }, "/nonexistent/repo")).toBe(
      PERSONAL_URL,
    );
    expect(recordedSource({ source: "svn:x" }, "/nonexistent/repo")).toBeUndefined();
    expect(recordedSource({ source: "path:./missing" }, "/nonexistent/repo")).toBeUndefined();
  });

  it("reports a group layer's declaration as ignored with its reasons", () => {
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
    const result = expectRead(read(s));
    expect(result.declared?.layer.name).toBe("personal");
    expect(result.declared?.account).toEqual({ claude: "personal-a", codex: "personal-a" });
    expect(result.ignored).toEqual([
      {
        layer: { index: 1, name: "group", source: `git:https://example.invalid/group.git@v1` },
        reasons: ["its kind is 'group', not 'client'"],
      },
    ]);
  });

  it("reports no declaration and lists the clients when two client layers declare", () => {
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
    const result = expectConflict(read(s));
    expect(result.clients.map((layer) => [layer.index, layer.name])).toEqual([
      [1, "client"],
      [2, "client-b"],
    ]);
    expect(result.ignored).toEqual([]);
    expect("declared" in result).toBe(false);
  });

  it("only reports a declaration by a repo layer at index 0: it never wins and never blocks", async () => {
    const s = await built();
    useStack(s, REPO_LAYER, ["path:../hub/layer", "v1"]);
    writeFileSync(
      `${s.ws.root}/repo/.layer/layer.yaml`,
      "name: repo\nkind: repo\nvisibility: private\naccount: other-a\n",
    );
    const result = expectRead(read(s));
    expect(result.declared?.layer.name).toBe("client");
    expect(result.ignored).toEqual([
      {
        layer: { index: 0, name: "repo", source: "path:./.layer" },
        reasons: ["it is inside the repository"],
      },
    ]);
  });

  it("applies the client layer when an inside first layer that the build refuses declares", async () => {
    const s = await built();
    cpSync(`${s.ws.root}/personal`, `${s.ws.root}/repo/inside`, { recursive: true });
    useStack(s, ["path:./inside"], ["path:../hub/layer", "v1"], REPO_LAYER);
    const result = expectRead(read(s));
    expect(result.declared?.layer.name).toBe("client");
    expect(result.ignored.map((item) => [item.layer.index, item.layer.name])).toEqual([
      [0, "personal"],
    ]);
  });

  describe("evaluateAccount agrees with mergeAccount", () => {
    const REPO = "/nonexistent/alz-repo";

    function layer(name: string, index: number, meta: Partial<LayerMeta>, source?: string): Layer {
      return {
        meta: {
          name,
          kind: index === 0 ? "personal" : "client",
          visibility: "private",
          providesSlots: [],
          requiresSlots: [],
          forbidPatterns: [],
          ...meta,
        },
        dir: `/nonexistent/${name}`,
        source: source ?? `github:example/${name}`,
        files: [],
        contentHash: "",
      };
    }

    const stacks: [string, Layer[]][] = [
      ["no declaration", [layer("a", 0, {}), layer("b", 1, {})]],
      ["the first layer alone", [layer("a", 0, { account: { claude: "x" } }), layer("b", 1, {})]],
      [
        "a client over the first layer",
        [layer("a", 0, { account: { claude: "x" } }), layer("b", 1, { account: { codex: "y" } })],
      ],
      [
        "two clients",
        [
          layer("a", 0, { account: { claude: "x" } }),
          layer("b", 1, { account: { claude: "y" } }),
          layer("c", 2, { account: { claude: "z" } }),
        ],
      ],
      [
        "a refused first layer",
        [
          layer("a", 0, { account: { claude: "x" } }, "path:./inside"),
          layer("b", 1, { account: { claude: "y" } }),
        ],
      ],
      [
        "a group layer",
        [
          layer("a", 0, { account: { claude: "x" } }),
          layer("g", 1, { kind: "group", account: { claude: "g" } }),
        ],
      ],
      [
        "a public client",
        [
          layer("a", 0, { account: { claude: "x" } }),
          layer("p", 1, { visibility: "public", account: { claude: "p" } }),
        ],
      ],
    ];

    it.each(stacks)("for %s", (_label, stack) => {
      const diags = new Diagnostics();
      const merged = mergeAccount(stack, REPO, diags);
      const evaluation = evaluateAccount(stack, REPO);
      expect(evaluation.winner?.value).toEqual(merged?.account);
      expect(evaluation.winner?.layer.meta.name).toBe(merged?.layer.meta.name);
      const expectedErrors = evaluation.refused.length + (evaluation.clients.length > 1 ? 1 : 0);
      expect(diags.errors).toHaveLength(expectedErrors);
    });
  });
});

describe("§13.20 cache index and cache misses: layers that cannot be found or read", () => {
  it("misses when the directory of a pinned layer is gone, never skipping it as inside", async () => {
    const s = await built();
    rmSync(`${s.ws.root}/hub/layer`, { recursive: true });
    expectMiss(read(s), "layer-directory-missing", { index: 1 });
  });

  it("misses when the directory of an unpinned layer is gone", async () => {
    const s = setup();
    useStack(s, PERSONAL, REPO_LAYER);
    expect((await alz(s, ["build"])).code).toBe(0);
    rmSync(`${s.ws.root}/personal`, { recursive: true });
    expectMiss(read(s), "layer-directory-missing", { index: 0 });
  });

  it("misses when a local git: source names a path that does not exist", () => {
    const s = setup();
    useStack(s, [`git:${s.real}/nowhere@v1`], REPO_LAYER);
    writeLock(s, { lockVersion: 1, visibility: "private", layers: [], files: {} });
    expectMiss(read(s), "layer-directory-missing", { index: 0 });
  });

  it("misses when a pinned layer's tree2@ entry is gone but its index entry is still there", async () => {
    const s = await built();
    rmSync(`${s.cache}/${treeEntryName(treeOf(s, s.v1))}`, { recursive: true });
    expect(existsSync(indexPath(s, s.v1))).toBe(true);
    expectMiss(read(s), "cache-entry-missing", { index: 1 });
  });

  it("misses when the manifest names another ref than the lock recorded", async () => {
    const s = await built();
    useStandard(s, "v2");
    expectMiss(read(s), "lock-ref-mismatch", { index: 1 });
  });

  it("misses when two layers outside the repository have the same name", () => {
    const s = setup();
    const files = filesOf(s, "personal");
    remoteStack(s, [
      { url: PERSONAL_URL, ref: "v1", sha: FIRST, files },
      { url: "https://example.invalid/personal-copy.git", ref: "v1", sha: SECOND, files },
    ]);
    expectMiss(read(s), "stack-invalid");
  });

  it("misses when the cache lies inside the repository", async () => {
    const s = await built();
    expectMiss(read(s, { XDG_CACHE_HOME: `${s.real}/repo/cache` }), "cache-in-repo");
  });

  it("misses on a layer with more files than the limit, and on one with more bytes", async () => {
    const s = setup();
    useStack(s, PERSONAL, REPO_LAYER);
    expect((await alz(s, ["build"])).code).toBe(0);
    expectRead(read(s));
    const extra = `${s.ws.root}/personal/extra`;
    mkdirSync(extra);
    for (let n = 0; n <= MAX_LAYER_ENTRIES; n++) writeFileSync(`${extra}/f${n}.md`, "x");
    expectMiss(read(s), "unreadable", { index: 0 });
    rmSync(extra, { recursive: true });
    writeFileSync(`${s.ws.root}/personal/big.bin`, Buffer.alloc(MAX_LAYER_BYTES + 1));
    expectMiss(read(s), "unreadable", { index: 0 });
  });

  it("does not follow a link inside a cached layer: a replaced file is a content-hash miss", () => {
    const s = setup();
    const [dir = ""] = remoteStack(s, [
      { url: PERSONAL_URL, ref: "v1", sha: FIRST, files: filesOf(s, "personal") },
    ]);
    expectRead(read(s));
    for (const file of ["layer.yaml", "instructions/10-personal.md"]) {
      const copy = `${s.real}/copy-${file.replaceAll("/", "-")}`;
      cpSync(`${dir}/${file}`, copy);
      rmSync(`${dir}/${file}`);
      symlinkSync(copy, `${dir}/${file}`);
      expectMiss(read(s), "content-hash", { index: 0 });
      rmSync(`${dir}/${file}`);
      cpSync(copy, `${dir}/${file}`);
      expectRead(read(s));
    }
  });

  it.each([
    ["an absolute path: source", (home: string) => `path:${home}/.claude-x/foo`],
    ["a case variant of one", (home: string) => `path:${home}/.CLAUDE-X/foo`],
    ["a local git: source", (home: string) => `git:${home}/.claude/projects@v1`],
    ["a file:// git: source", (home: string) => `git:file://${home}/.codex/x@v1`],
    ["the claude.json file", (home: string) => `path:${home}/.claude.json`],
  ])(
    "misses on %s that names an account folder, before any file system call on it",
    (_label, source) => {
      const s = setup();
      useStack(s, [source(s.ws.home)], REPO_LAYER);
      writeLock(s, { lockVersion: 1, visibility: "private", layers: [], files: {} });
      const input = inputFor(s);
      const { result, calls } = traced(() => readAccountDeclaration(input));
      expectMiss(result, "account-folder", { index: 0 });
      const homes = [s.ws.home, realpathSync.native(s.ws.home)].map((home) => home.toLowerCase());
      const touched = calls.flatMap((call) => call.paths).map((path) => path.toLowerCase());
      for (const home of homes) {
        expect(
          touched.filter(
            (path) => path.startsWith(`${home}/.claude`) || path.startsWith(`${home}/.codex`),
          ),
        ).toEqual([]);
      }
    },
  );

  it("misses on a git: source with a '..' segment before probing it", () => {
    const s = setup();
    useStack(s, ["git:link/../x@v1"], REPO_LAYER);
    writeLock(s, { lockVersion: 1, visibility: "private", layers: [], files: {} });
    expectMiss(read(s), "source-invalid", { index: 0 });
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
    expect(readdirSync(s.cache).filter((name) => name.startsWith(".tmp-"))).toEqual([]);
    expectMiss(read(s), "index-missing", { index: 1 });
  });
});
