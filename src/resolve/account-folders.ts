import { readdirSync, realpathSync } from "node:fs";
import { isAbsolute, join, relative, resolve, sep } from "node:path";
import type { Env } from "../core/env";
import { isWithin } from "../core/files";
import {
  ACCOUNT_TARGETS,
  type AccountTarget,
  isAccountName as isValidAccountName,
} from "../model/account";

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

/**
 * How a path relates to the account folders: `inside` an account folder by name (`~/.claude-*`,
 * `~/.codex-*`, `~/.claude`, `~/.codex`, `~/.claude.json`), `configured` (inside the folder
 * `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names, which mise or direnv can point at a repository or a
 * parent of one), the `$HOME` directory or above it, or `contains` a folder a variable names.
 * The order of precedence is `inside`, `configured`, `home`, `contains`.
 */
export type FolderRelation = "inside" | "configured" | "home" | "contains";

function relation(
  path: string,
  home: string,
  folders: string[],
  fold: (text: string) => string,
): FolderRelation | undefined {
  const target = fold(path);
  const base = fold(home);
  const below = firstBelow(base, target);
  const named = folders.map(fold);
  if (below !== undefined && isAccountName(below)) return "inside";
  if (named.some((f) => isWithin(f, target))) return "configured";
  if (isWithin(target, base)) return "home";
  return named.some((f) => isWithin(target, f)) ? "contains" : undefined;
}

function configuredFolders(env: Env): string[] {
  return ["CLAUDE_CONFIG_DIR", "CODEX_HOME"].flatMap((name) => {
    const value = env[name];
    return value === undefined || value === "" ? [] : [value];
  });
}

/**
 * §9.1: how a real path relates to the account folders, or undefined when it does not. Path
 * arithmetic and `realpath` only: nothing is opened, so the answer never reads an account.
 */
export function accountFolderRelation(
  realPath: string,
  home: string,
  env: Env,
): FolderRelation | undefined {
  return relation(
    realPath,
    realOrResolved(home),
    configuredFolders(env).map(realOrResolved),
    (text) => text,
  );
}

/**
 * Does a real path lie inside an account folder, or contain one? True when it is `$HOME` or
 * above it, below `~/.claude`, `~/.codex`, `~/.claude.json`, `~/.claude-*` or `~/.codex-*`, or
 * equal to, below or above the folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names.
 */
export function touchesAccountFolder(realPath: string, home: string, env: Env): boolean {
  return accountFolderRelation(realPath, home, env) !== undefined;
}

/**
 * The same relation on a path as written, before any file system call on it: lexical, with no
 * `realpath`, and case-insensitive, so a case variant on a case-insensitive disk is caught.
 * `homes` are the spellings of `$HOME` to compare with (as given, and its real path). A defined
 * answer other than `contains` means the path must not be touched; undefined means only that the
 * text does not name an account folder, so the real path is checked once it is known.
 */
export function folderRelationAsWritten(
  path: string,
  homes: string[],
  env: Env,
): FolderRelation | undefined {
  const lower = (text: string) => text.toLowerCase();
  const folders = configuredFolders(env).map((value) => resolve(value));
  const found = homes.map((home) => relation(resolve(path), home, folders, lower));
  const order: FolderRelation[] = ["inside", "configured", "home", "contains"];
  return order.find((item) => found.includes(item));
}

/** The same test as `touchesAccountFolder`, on a path as written (see `folderRelationAsWritten`). */
export function namesAccountFolder(path: string, homes: string[], env: Env): boolean {
  return folderRelationAsWritten(path, homes, env) !== undefined;
}

/** §7.13: the name that is never an account; its folders are the ones alz never creates. */
export const BLOCKED_NAME = "alz-blocked";

const FOLDER_PREFIX: Record<AccountTarget, string> = { claude: ".claude-", codex: ".codex-" };

/** `~/.claude-<name>` or `~/.codex-<name>`: built from `$HOME` and a validated account name only. */
export function accountFolder(home: string, target: AccountTarget, name: string): string {
  return join(home, `${FOLDER_PREFIX[target]}${name}`);
}

/** `~/.claude-alz-blocked` or `~/.codex-alz-blocked`. alz never creates it. */
export function blockedFolder(home: string, target: AccountTarget): string {
  return accountFolder(home, target, BLOCKED_NAME);
}

/** The account names that exist per target, by name only, sorted; `alz-blocked` is never listed. */
export type AccountNames = Record<AccountTarget, string[]>;

/**
 * §9.1: lists `$HOME` once and keeps the names that start with `.claude-` or `.codex-`. A
 * directory or a link counts as existing; nothing in the listing is opened or `stat`ed, so
 * no account folder is read. A `$HOME` that cannot be listed has no accounts.
 */
export function listAccountNames(home: string): AccountNames {
  const names: AccountNames = { claude: [], codex: [] };
  let entries: { name: string; isDirectory(): boolean; isSymbolicLink(): boolean }[];
  try {
    entries = readdirSync(home, { withFileTypes: true });
  } catch {
    return names;
  }
  for (const entry of entries) {
    if (!entry.isDirectory() && !entry.isSymbolicLink()) continue;
    for (const target of Object.keys(ACCOUNT_TARGETS) as AccountTarget[]) {
      const prefix = FOLDER_PREFIX[target];
      if (!entry.name.startsWith(prefix)) continue;
      const name = entry.name.slice(prefix.length);
      if (isValidAccountName(name) && name !== BLOCKED_NAME) names[target].push(name);
    }
  }
  for (const target of Object.keys(names) as AccountTarget[]) names[target].sort();
  return names;
}
