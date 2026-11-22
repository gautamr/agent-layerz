import { realpathSync } from "node:fs";
import { isAbsolute, relative, resolve, sep } from "node:path";
import type { Env } from "../core/env";
import { isWithin } from "../core/files";

/** The real path of an existing path, or its absolute spelling when it cannot be resolved. */
export function realOrResolved(path: string): string {
  try {
    return realpathSync.native(path);
  } catch {
    return resolve(path);
  }
}

/** First segment of `path` below `base`, or undefined when `path` is not below it. */
function firstBelow(base: string, path: string): string | undefined {
  const rel = relative(base, path);
  if (rel === "" || rel === ".." || rel.startsWith(`..${sep}`) || isAbsolute(rel)) return undefined;
  return rel.split(sep)[0];
}

function isAccountName(segment: string): boolean {
  const name = segment.toLowerCase();
  return (
    name === ".claude" ||
    name === ".codex" ||
    name === ".claude.json" ||
    name.startsWith(".claude-") ||
    name.startsWith(".codex-")
  );
}

function touches(
  path: string,
  home: string,
  folders: string[],
  fold: (text: string) => string,
): boolean {
  const target = fold(path);
  const base = fold(home);
  if (isWithin(target, base)) return true;
  const below = firstBelow(base, target);
  if (below !== undefined && isAccountName(below)) return true;
  return folders.some((folder) => {
    const named = fold(folder);
    return isWithin(named, target) || isWithin(target, named);
  });
}

function configuredFolders(env: Env): string[] {
  return ["CLAUDE_CONFIG_DIR", "CODEX_HOME"].flatMap((name) => {
    const value = env[name];
    return value === undefined || value === "" ? [] : [value];
  });
}

/**
 * §9.1: does a real path lie inside an account folder, or contain one? True when it is `$HOME`
 * or above it, below `~/.claude`, `~/.codex`, `~/.claude.json`, `~/.claude-*` or `~/.codex-*`,
 * or equal to, below or above the folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names. Path
 * arithmetic and `realpath` only: nothing is opened, so the answer never reads an account.
 */
export function touchesAccountFolder(realPath: string, home: string, env: Env): boolean {
  return touches(
    realPath,
    realOrResolved(home),
    configuredFolders(env).map(realOrResolved),
    (text) => text,
  );
}

/**
 * The same test on a path as written, before any file system call on it: lexical, with no
 * `realpath`, and case-insensitive, so a case variant on a case-insensitive disk is caught.
 * `homes` are the spellings of `$HOME` to compare with (as given, and its real path). A true
 * answer means the path must not be touched; a false one means only that the text does not name
 * an account folder, so the real path is checked with `touchesAccountFolder` once it is known.
 */
export function namesAccountFolder(path: string, homes: string[], env: Env): boolean {
  const lower = (text: string) => text.toLowerCase();
  const folders = configuredFolders(env).map((value) => resolve(value));
  return homes.some((home) => touches(resolve(path), home, folders, lower));
}
