import {
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  readlinkSync,
} from "node:fs";
import { join, relative } from "node:path";
import { type CliResult, spawnCaptured } from "./exec";
import { alzBin, tempDir, tmpRoot } from "./run-root";

export type { CliResult } from "./exec";

const fixturesDir = join(import.meta.dirname, "fixtures");

export interface Workspace {
  /** Root of the copied fixture. */
  root: string;
  /** Isolated HOME, so tests never see the real `~/.claude`. */
  home: string;
}

/** Copies `test/fixtures/<name>` into a fresh folder under the run root. */
export function fixture(name: string): Workspace {
  const base = tempDir(`alz-${name}-`);
  const root = join(base, "ws");
  const home = join(base, "home");
  cpSync(join(fixturesDir, name), root, { recursive: true });
  mkdirSync(home);
  return { root, home };
}

/** What `layers` is: the runner of one command in a workspace. */
export type Layers = (
  ws: Workspace,
  dir: string,
  args: string[],
  env?: Record<string, string>,
) => Promise<CliResult>;

export interface LayersOptions {
  /** Standard input for the child; without it the child has none. */
  input?: string;
  /** Raise it for a case that legitimately takes longer than the default. */
  timeoutMs?: number;
}

/**
 * Runs the binary under test (`ALZ_BIN`, or the TypeScript reference built for this run) as
 * `alz <args>` in `ws.root/dir`. The child gets exactly HOME, PATH, ALZ_CLAUDE_BIN and TMPDIR
 * (inside the run root), then `env`.
 */
export function layers(
  ws: Workspace,
  dir: string,
  args: string[],
  env: Record<string, string> = {},
  options: LayersOptions = {},
): Promise<CliResult> {
  return spawnCaptured(alzBin(), args, {
    cwd: join(ws.root, dir),
    env: {
      HOME: ws.home,
      PATH: process.env.PATH ?? "",
      ALZ_CLAUDE_BIN: "/nonexistent",
      TMPDIR: tmpRoot(),
      ...env,
    },
    ...options,
  });
}

/** Every file under `dir` as path → content, sorted, ignoring the given top-level entries. */
export function readTree(dir: string, ignore: string[] = []): Record<string, string> {
  const tree: Record<string, string> = {};
  const walk = (current: string) => {
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const full = join(current, entry.name);
      const rel = relative(dir, full);
      if (ignore.includes(rel)) continue;
      if (entry.isDirectory()) walk(full);
      else tree[rel] = readFileSync(full, "utf8");
    }
  };
  walk(dir);
  return Object.fromEntries(Object.entries(tree).sort(([a], [b]) => a.localeCompare(b)));
}

export function readFile(ws: Workspace, path: string): string {
  return readFileSync(join(ws.root, path), "utf8");
}

export function exists(ws: Workspace, path: string): boolean {
  return existsSync(join(ws.root, path));
}

export function expected(fixtureName: string, path: string): string {
  return readFileSync(join(fixturesDir, fixtureName, "expected", path), "utf8");
}

/** The text of a file, or a marker when nobody can read it (a mode 000 file is still a state). */
function contentOf(full: string): string {
  try {
    return readFileSync(full, "utf8");
  } catch {
    return "<unreadable>";
  }
}

/**
 * Everything under `dir`, without following links: path (relative to `dir`) → kind, mode and
 * content or link target. A folder that is missing or unreadable gives an empty snapshot. Two
 * equal snapshots taken before and after a command prove it wrote nothing there.
 */
export function snapshot(dir: string): Record<string, string> {
  const entry = (full: string): [string, string][] => {
    const stat = lstatSync(full);
    const mode = (stat.mode & 0o7777).toString(8);
    const rel = full.slice(dir.length);
    if (stat.isSymbolicLink()) return [[rel, `link:${readlinkSync(full)}`]];
    if (stat.isDirectory()) return [[rel, `dir:${mode}`], ...walk(full)];
    if (stat.isFile()) return [[rel, `file:${mode}:${contentOf(full)}`]];
    return [[rel, `other:${mode}`]];
  };
  const walk = (current: string): [string, string][] => {
    try {
      return readdirSync(current).flatMap((name) => entry(join(current, name)));
    } catch {
      return [];
    }
  };
  return Object.fromEntries(walk(dir));
}
