import { execFileSync } from "node:child_process";
import { chmodSync, mkdirSync, realpathSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import type { Terminal } from "../src/cli/terminal";
import { stringifyYaml } from "../src/core/yaml";
import { gitEntryName } from "../src/resolve/cache-paths";
import { hashLayerDir } from "../src/resolve/layer-hash";
import { fixture, layers, type Workspace } from "./helpers";

/**
 * Shared set-up of the account tests (31 and the account cases of 24): the fixture
 * `31-account-which` with the hub as a git repository tagged `v1` and `v2`, a fake HOME, and
 * helpers that write `accounts.yaml` and `allowed.yaml` the way a user would (modes set
 * explicitly, so a CI umask never makes them group-writable).
 */

export const FIXTURE = "31-account-which";

const GIT_ENV = {
  PATH: process.env.PATH ?? "",
  GIT_CONFIG_NOSYSTEM: "1",
  GIT_AUTHOR_NAME: "Test",
  GIT_AUTHOR_EMAIL: "test@example.invalid",
  GIT_COMMITTER_NAME: "Test",
  GIT_COMMITTER_EMAIL: "test@example.invalid",
};

export function sh(ws: Workspace, cwd: string, args: string[]): string {
  return execFileSync("git", ["-c", "commit.gpgsign=false", ...args], {
    cwd,
    encoding: "utf8",
    env: { ...GIT_ENV, HOME: ws.home },
  }).trim();
}

export interface Setup {
  ws: Workspace;
  /** The real path of the workspace: what the cache and the resolver see. */
  real: string;
  hub: string;
  v1: string;
  v2: string;
  cache: string;
  /** The real path of the hub's `layer/` directory: what `allowed.yaml` records for it. */
  hubLayer: string;
}

/** A copy of the fixture without git: enough for every case that never builds. */
export function plain(name = FIXTURE): Setup {
  const ws = fixture(name);
  const real = realpathSync.native(ws.root);
  return {
    ws,
    real,
    hub: `${ws.root}/hub`,
    v1: "",
    v2: "",
    cache: `${real}/cache/agent-layerz`,
    hubLayer: `${real}/hub/layer`,
  };
}

/** The hub is a git repository tagged `v1` and `v2`; the layer's content differs between them. */
export function setup(name = FIXTURE): Setup {
  const s = plain(name);
  const { ws, hub } = s;
  sh(ws, hub, ["init", "--quiet", "--initial-branch=main"]);
  sh(ws, hub, ["add", "."]);
  sh(ws, hub, ["commit", "--quiet", "-m", "hub v1"]);
  sh(ws, hub, ["tag", "v1"]);
  writeFileSync(`${hub}/layer/instructions/10-client.md`, "- Client rule, second edition.\n");
  sh(ws, hub, ["add", "."]);
  sh(ws, hub, ["commit", "--quiet", "-m", "hub v2"]);
  sh(ws, hub, ["tag", "v2"]);
  return { ...s, v1: sh(ws, hub, ["rev-parse", "v1"]), v2: sh(ws, hub, ["rev-parse", "v2"]) };
}

/** The environment every case runs with: the three XDG locations, all inside the fake HOME or cache. */
export function envFor(s: Setup, extra: Record<string, string> = {}): Record<string, string> {
  return {
    XDG_CONFIG_HOME: `${s.ws.home}/.config`,
    XDG_STATE_HOME: `${s.ws.home}/.local/state`,
    XDG_CACHE_HOME: `${s.real}/cache`,
    ...extra,
  };
}

export function alz(
  s: Setup,
  args: string[],
  extra: Record<string, string> = {},
  dir = "repo",
  terminal?: Terminal,
) {
  return layers(s.ws, dir, args, envFor(s, extra), terminal);
}

function stackYaml(...entries: [source: string, ref?: string][]): string {
  const items = entries
    .map(([source, ref]) => `  - source: ${source}\n${ref ? `    ref: ${ref}\n` : ""}`)
    .join("");
  return `stack:\n${items}targets: [claude]\n`;
}

export const PERSONAL: [string] = ["path:../personal"];
export const REPO_LAYER: [string] = ["path:./.layer"];

export function useStack(s: Setup, ...entries: [source: string, ref?: string][]): void {
  writeFileSync(`${s.ws.root}/repo/layer.yaml`, stackYaml(...entries));
}

/** Builds the standard stack: personal in place, the hub's layer pinned to `ref`, the repo layer. */
export async function built(ref = "v1", name = FIXTURE): Promise<Setup> {
  const s = setup(name);
  useStack(s, PERSONAL, ["path:../hub/layer", ref], REPO_LAYER);
  const result = await alz(s, ["build"]);
  if (result.code !== 0) throw new Error(`build failed: ${result.stderr}`);
  return s;
}

export function configDir(s: Setup): string {
  return `${s.ws.home}/.config/agent-layerz`;
}

export function stateDir(s: Setup): string {
  return `${s.ws.home}/.local/state/agent-layerz`;
}

function writeSafe(folder: string, file: string, text: string, root: string): void {
  mkdirSync(folder, { recursive: true });
  for (
    let current = folder;
    current !== root && current.length > root.length;
    current = dirname(current)
  ) {
    chmodSync(current, current === folder ? 0o700 : 0o755);
  }
  writeFileSync(`${folder}/${file}`, text);
  chmodSync(`${folder}/${file}`, 0o600);
}

/** Writes the hand-written user config, owned by the test user, mode 0600 in a 0700 folder. */
export function writeAccounts(s: Setup, text: string): string {
  writeSafe(configDir(s), "accounts.yaml", text, s.ws.home);
  return `${configDir(s)}/accounts.yaml`;
}

export interface AllowedFixture {
  root: string;
  account: Record<string, string>;
  source: string;
}

/** Writes allowed.yaml the way `alz account allow` would. */
export function writeAllowed(s: Setup, entries: AllowedFixture[], raw?: string): string {
  const text = raw ?? `# written by alz account allow\n${stringifyYaml(entries)}`;
  writeSafe(stateDir(s), "allowed.yaml", text, s.ws.home);
  return `${stateDir(s)}/allowed.yaml`;
}

/** The entry `alz account allow` records for the fixture's repository: the hub's declaration. */
export function hubEntry(s: Setup, patch: Partial<AllowedFixture> = {}): AllowedFixture {
  return {
    root: `${s.real}/repo`,
    account: { claude: "client-a", codex: "client-a-api" },
    source: s.hubLayer,
    ...patch,
  };
}

/** An account folder in the fake HOME, made empty. */
export function accountFolder(s: Setup, ...names: string[]): void {
  for (const name of names) mkdirSync(`${s.ws.home}/${name}`, { recursive: true });
}

/** A remote layer written into the cache from fixture files, with the lock entry that records it. */
export interface RemoteLayer {
  source: string;
  sha: string;
  files: Record<string, string>;
}

export function lockEntry(cache: string, remote: RemoteLayer, url: string) {
  const folder = `${cache}/${gitEntryName(url, remote.sha)}`;
  mkdirSync(folder, { recursive: true });
  for (const [path, content] of Object.entries(remote.files)) {
    mkdirSync(dirname(`${folder}/${path}`), { recursive: true });
    writeFileSync(`${folder}/${path}`, content);
  }
  const name = /^name: ([a-z0-9.-]+)$/m.exec(remote.files["layer.yaml"] ?? "")?.[1] ?? "x";
  return {
    dir: folder,
    entry: {
      name,
      source: remote.source,
      rev: "v1",
      sha: remote.sha,
      contentHash: `sha256:${hashLayerDir(folder).contentHash}`,
    },
  };
}

export function writeLock(s: Setup, lock: unknown): void {
  writeFileSync(`${s.ws.root}/repo/layer.lock`, JSON.stringify(lock));
}

/**
 * A stack of remote layers written into the cache by the test, with the lock that records
 * them: git layers need no git here, because the cache entry is all resolution reads.
 */
export function remoteStack(
  s: Setup,
  remotes: { url: string; ref: string; sha: string; files: Record<string, string> }[],
): void {
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
}
