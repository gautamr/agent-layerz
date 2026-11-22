import { lstatSync, realpathSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { type Env, envManagerActive } from "../core/env";
import { isWithin, readRegularFileWithId } from "../core/files";
import { displayPath } from "../core/sanitize";
import { xdgBase } from "../core/xdg";
import { parseYamlLimited } from "../core/yaml";
import { namesAccountFolder, realOrResolved, touchesAccountFolder } from "./account-folders";
import type { WarningList } from "./account-warning";

/** §7.13 'Both files': at most 1 MiB, read with the YAML alias limit. */
const MAX_FILE_BYTES = 1024 * 1024;

export type AccountFileName = "accounts.yaml" | "allowed.yaml";

const LOCATION: Record<AccountFileName, { variable: string; fallback: string }> = {
  "accounts.yaml": { variable: "XDG_CONFIG_HOME", fallback: ".config" },
  "allowed.yaml": { variable: "XDG_STATE_HOME", fallback: ".local/state" },
};

/** One of the two files, as `alz account which` reports it. `path` is the path as located. */
export interface AccountFile {
  path: string;
  status: "read" | "missing" | "ignored";
  /** A fixed phrase; set when the status is `ignored`. */
  reason?: string;
}

export interface AccountFileContext {
  home: string;
  env: Env;
  /** `process.getuid?.()`; undefined means the owner cannot be checked, so both files are ignored. */
  uid: number | undefined;
  /** The real root of the repository, when one was found. */
  root?: string;
}

export interface AccountFileRead {
  file: AccountFile;
  /** The parsed YAML, only when `file.status` is `read`. */
  value?: unknown;
}

/** The path `name` is looked for at, before any file system call. */
export function locateAccountFile(
  name: AccountFileName,
  home: string,
  env: Env,
): { path: string; relativeIgnored: boolean; variable: string } {
  const { variable, fallback } = LOCATION[name];
  const base = xdgBase(env, variable, home, fallback);
  return {
    path: join(base.dir, "agent-layerz", name),
    relativeIgnored: base.relativeIgnored,
    variable,
  };
}

function lstatOrUndefined(path: string): { mode: number; uid: number } | undefined {
  try {
    const { mode, uid } = lstatSync(path);
    return { mode, uid };
  } catch {
    return undefined;
  }
}

/** True when `mode` lets group or others write. */
function othersCanWrite(mode: number): boolean {
  return (mode & 0o022) !== 0;
}

/**
 * The directories whose owner and mode decide whether another user could swap the file:
 * `dir` and each parent strictly below `$HOME`, or only `dir` when it lies outside `$HOME`.
 * Returns the first that breaks the rule, with the reason.
 */
export function weakDirectory(
  dir: string,
  realHome: string,
  uid: number,
  home: string,
): string | undefined {
  const below = dir !== realHome && isWithin(realHome, dir);
  const chain: string[] = [];
  for (let current = dir; ; current = dirname(current)) {
    if (below && current === realHome) break;
    chain.push(current);
    if (!below || dirname(current) === current) break;
  }
  for (const current of chain) {
    const stat = lstatOrUndefined(current);
    const shown = displayPath(current, home);
    if (stat === undefined) return `the directory ${shown} cannot be checked`;
    if (othersCanWrite(stat.mode))
      return `the directory ${shown} can be written by group or others`;
    const trusted = stat.uid === uid || (!below && stat.uid === 0);
    if (!trusted) return `the directory ${shown} is not owned by you`;
  }
  return undefined;
}

/**
 * The `.git` (a folder or a file) that makes `path` part of a git work tree, or undefined. Only
 * `lstat` of `.git` in each directory above the path, as written; no git runs. Only a git
 * checkout is detected: an archive, a jj repository without git, Mercurial, Sapling and a bare
 * repository used for dotfiles leave no `.git` and are not.
 */
function workTreeMarker(path: string): string | undefined {
  for (let current = dirname(path); ; current = dirname(current)) {
    const marker = join(current, ".git");
    if (lstatOrUndefined(marker) !== undefined) return marker;
    if (dirname(current) === current) return undefined;
  }
}

/** `$HOME` as written, or as its real path when the marker was found by real path. */
function shownHome(path: string, home: string): string {
  const real = realOrResolved(home);
  return isWithin(real, path) ? real : home;
}

/**
 * The refusal for an approval file inside a git work tree, checked for the real path and for the
 * path as located, so a link placed inside a work tree is refused even when its target is not.
 */
function workTreeProblem(realFile: string, locatedFile: string, home: string): string | undefined {
  const marker = workTreeMarker(realFile) ?? workTreeMarker(locatedFile);
  return marker === undefined
    ? undefined
    : `it lies inside a git work tree (${displayPath(marker, shownHome(marker, home))})`;
}

/**
 * The checks of §7.13 'Both files' that depend only on the text of the path and the
 * environment, before the file is looked for: the path names an account folder, or an `XDG_*`
 * variable that mise or direnv may have set points away from its default. A fixed phrase, or
 * undefined. The reader and the writer of `allowed.yaml` both call it.
 */
export function unplacedReason(name: AccountFileName, home: string, env: Env): string | undefined {
  const { path, variable } = locateAccountFile(name, home, env);
  const homes = [...new Set([resolve(home), realOrResolved(home)])];
  if (namesAccountFolder(path, homes, env)) return "it lies inside an account folder";
  const base = xdgBase(env, variable, home, LOCATION[name].fallback);
  // Before the existence check, so a folder that mise or direnv moved the lookup to is reported
  // even when nothing is there yet.
  if (envManagerActive(env) && base.fromVariable) {
    const moved = realOrResolved(base.dir);
    const normal = realOrResolved(join(home, LOCATION[name].fallback));
    if (moved !== normal) {
      return `${variable} points away from its default and mise or direnv is active here`;
    }
  }
  return undefined;
}

/** The nearest directory at or above `dir` that exists, as written. */
function nearestExisting(dir: string): string {
  for (let current = dir; ; current = dirname(current)) {
    if (lstatOrUndefined(current) !== undefined || dirname(current) === current) return current;
  }
}

/**
 * The checks of §7.13 'Both files' on the real path of `allowed.yaml`, as the reader makes them before
 * it opens it: inside an account folder, inside the current repository, inside a git work tree, and a directory that
 * group or others can write or that the user does not own, for the directory of the real path
 * and for `locatedDir`, the directory of the path as located. The writer uses it for a file
 * that is not there yet, so a directory that does not exist yet is checked through its nearest
 * existing parent (never `$HOME` itself, which the reader does not check either). A fixed
 * phrase, or undefined.
 */
export function placementProblem(
  realFile: string,
  locatedDir: string,
  ctx: AccountFileContext & { uid: number },
  /** The path as located, for the work tree check; default: the real file name in `locatedDir`. */
  locatedFile = join(locatedDir, basename(realFile)),
): string | undefined {
  const { home, env, uid } = ctx;
  if (touchesAccountFolder(realFile, home, env)) return "it lies inside an account folder";
  if (ctx.root !== undefined && isWithin(ctx.root, realFile)) {
    return "it lies inside the current repository";
  }
  const inTree = workTreeProblem(realFile, locatedFile, home);
  if (inTree !== undefined) return inTree;
  const realHome = realOrResolved(home);
  for (const dir of new Set([dirname(realFile), locatedDir])) {
    const existing = nearestExisting(dir);
    if (existing !== dir && existing === realHome) continue;
    const weak = weakDirectory(existing, realHome, uid, home);
    if (weak !== undefined) return weak;
  }
  return undefined;
}

/**
 * §7.13 'Both files': locates, checks and reads `accounts.yaml` or `allowed.yaml`. The first
 * check that fails ignores the file with one warning and nothing further is opened. Reads at
 * most 1 MiB, never writes, and never throws because of the file's content.
 */
export function readAccountFile(
  name: AccountFileName,
  ctx: AccountFileContext,
  warnings: WarningList,
): AccountFileRead {
  const { home, env, uid } = ctx;
  const located = locateAccountFile(name, home, env);
  const { path, variable } = located;
  const shown = displayPath(path, home);
  if (located.relativeIgnored) {
    warnings.add(
      "xdg-relative",
      `${variable} is not an absolute path; ${name} is looked for in the default place`,
    );
  }
  const ignore = (reason: string): AccountFileRead => {
    warnings.add("file-ignored", `${name} (${shown}) is ignored: ${reason}`);
    return { file: { path, status: "ignored", reason } };
  };

  // No file system call before these three: they depend only on the text of the path.
  if (uid === undefined) return ignore("the owner of the file cannot be checked on this platform");
  const unplaced = unplacedReason(name, home, env);
  if (unplaced !== undefined) return ignore(unplaced);
  let real: string;
  try {
    real = realpathSync.native(path);
  } catch (error) {
    const code = (error as { code?: string }).code;
    if (code === "ENOENT" || code === "ENOTDIR") return { file: { path, status: "missing" } };
    return ignore("its path cannot be resolved");
  }
  const realHome = realOrResolved(home);
  if (touchesAccountFolder(real, home, env)) return ignore("it lies inside an account folder");
  if (ctx.root !== undefined && isWithin(ctx.root, real)) {
    return ignore("it lies inside the current repository");
  }
  // An approval file inside any git work tree could be planted by a checkout: refuse it.
  const inTree = name === "allowed.yaml" ? workTreeProblem(real, path, home) : undefined;
  if (inTree !== undefined) return ignore(inTree);
  const read = readRegularFileWithId(real, MAX_FILE_BYTES);
  if (read === "missing") return { file: { path, status: "missing" } };
  if (read === "invalid")
    return ignore("it is not a regular file of at most 1 MiB that can be read");
  if (read.uid !== uid) return ignore("it is not owned by you");
  if (othersCanWrite(read.mode)) return ignore("group or others can write it");
  for (const dir of new Set([dirname(real), realOrResolved(dirname(path))])) {
    const weak = weakDirectory(dir, realHome, uid, home);
    if (weak !== undefined) return ignore(weak);
  }
  let value: unknown;
  try {
    value = parseYamlLimited(read.bytes.toString("utf8"));
  } catch {
    return ignore("it is not valid YAML");
  }
  if (name === "accounts.yaml" && workTreeMarker(real) !== undefined) {
    warnings.add(
      "file-in-work-tree",
      `${name} (${shown}) lies inside a git work tree; keep it outside every repository`,
    );
  }
  return { file: { path, status: "read" }, value };
}
