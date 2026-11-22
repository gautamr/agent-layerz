import { lstatSync, realpathSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
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
function weakDirectory(
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

/** Is the path inside a git work tree? Only `lstat` of `.git`; no git runs. */
function insideWorkTree(real: string): boolean {
  for (let current = dirname(real); ; current = dirname(current)) {
    if (lstatOrUndefined(join(current, ".git")) !== undefined) return true;
    if (dirname(current) === current) return false;
  }
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
  const homes = [...new Set([resolve(home), realOrResolved(home)])];
  if (namesAccountFolder(path, homes, env)) return ignore("it lies inside an account folder");
  const base = xdgBase(env, variable, home, LOCATION[name].fallback);
  // Before the existence check, so a folder that mise or direnv moved the lookup to is reported
  // even when nothing is there yet.
  if (envManagerActive(env) && base.fromVariable) {
    const moved = realOrResolved(base.dir);
    const normal = realOrResolved(join(home, LOCATION[name].fallback));
    if (moved !== normal) {
      return ignore(`${variable} points away from its default and mise or direnv is active here`);
    }
  }
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
  if (insideWorkTree(real)) {
    warnings.add(
      "file-in-work-tree",
      `${name} (${shown}) lies inside a git work tree; keep it outside every repository`,
    );
  }
  return { file: { path, status: "read" }, value };
}
