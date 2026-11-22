import { realpathSync } from "node:fs";
import { isAbsolute, join, resolve } from "node:path";
import type { Env } from "../core/env";
import { isWithin } from "../core/files";
import { quote } from "../core/sanitize";
import { isMap, unknownKeys } from "../core/yaml";
import { type AccountMap, isAccountName, parseAccount } from "../model/account";
import {
  type AccountNames,
  accountFolderRelation,
  folderRelationAsWritten,
  realOrResolved,
} from "./account-folders";
import type { WarningList } from "./account-warning";

/** One `folders` entry whose key resolved: the real path and the account it names per target. */
export interface FolderEntry {
  real: string;
  account: AccountMap;
}

export interface UserConfig {
  /** What the default account is called on this machine. */
  defaultName?: string;
  folders: FolderEntry[];
}

export const EMPTY_USER_CONFIG: UserConfig = { folders: [] };

function sameAccount(a: AccountMap, b: AccountMap): boolean {
  return a.claude === b.claude && a.codex === b.codex;
}

/** `~/x` is expanded from `home`; an absolute path is kept; anything else is not a path. */
function expand(key: string, home: string): string | undefined {
  if (key === "~") return home;
  if (key.startsWith("~/")) return join(home, key.slice(2));
  return isAbsolute(key) ? resolve(key) : undefined;
}

function realOf(path: string): string | undefined {
  try {
    return realpathSync.native(path);
  } catch {
    return undefined;
  }
}

/**
 * §7.13 'User config': parses the value of `accounts.yaml` and says why anything is ignored.
 * Never throws. A key text reaches a warning only through `quote`; an account name never does.
 */
export function parseUserConfig(
  value: unknown,
  ctx: { home: string; env: Env; names: AccountNames },
  warnings: WarningList,
): UserConfig {
  const { home, env } = ctx;
  if (value === null || value === undefined) return EMPTY_USER_CONFIG;
  if (!isMap(value)) {
    warnings.add("config-shape", "accounts.yaml is ignored: its top level must be a mapping");
    return EMPTY_USER_CONFIG;
  }
  for (const key of unknownKeys(value, ["default_name", "folders"])) {
    warnings.add("config-key", `accounts.yaml: the key ${quote(key)} is not known and is ignored`);
  }

  let defaultName: string | undefined;
  if (Object.hasOwn(value, "default_name")) {
    const name = value.default_name;
    if (typeof name === "string" && isAccountName(name)) {
      defaultName = name;
      const { names } = ctx;
      if (names.claude.includes(name) || names.codex.includes(name)) {
        warnings.add(
          "default-name-folder",
          `accounts.yaml: default_name '${name}' is also the name of an account folder (~/.claude-${name} or ~/.codex-${name}); that folder can never be chosen`,
        );
      }
    } else {
      warnings.add(
        "config-default-name",
        "accounts.yaml: default_name must be an account name; it is ignored",
      );
    }
  }

  const folders: FolderEntry[] = [];
  const raw = value.folders;
  if (raw !== undefined && raw !== null && !isMap(raw)) {
    warnings.add("config-shape", "accounts.yaml: 'folders' must be a mapping; it is ignored");
  } else if (isMap(raw)) {
    const homes = [...new Set([resolve(home), realOrResolved(home)])];
    for (const [key, entry] of Object.entries(raw)) {
      const path = expand(key, home);
      if (path === undefined) {
        warnings.add(
          "config-folder",
          `accounts.yaml: the folders key ${quote(key)} is not an absolute path or ~/..., so it is ignored`,
        );
        continue;
      }
      let account: AccountMap;
      try {
        account = parseAccount(entry);
      } catch {
        warnings.add(
          "config-folder",
          `accounts.yaml: the folders entry ${quote(key)} must be an account name or a mapping of target to name, so it is ignored`,
        );
        continue;
      }
      const written = folderRelationAsWritten(path, homes, env);
      if (written === "inside" || written === "configured" || written === "home") {
        warnings.add(
          "config-folder",
          `accounts.yaml: the folders key ${quote(key)} lies inside an account folder, or is $HOME or above it, so it is ignored`,
        );
        continue;
      }
      const real = realOf(path);
      if (real === undefined) continue;
      const relation = accountFolderRelation(real, home, env);
      if (relation === "inside" || relation === "configured" || relation === "home") {
        warnings.add(
          "config-folder",
          `accounts.yaml: the folders key ${quote(key)} lies inside an account folder, or is $HOME or above it, so it is ignored`,
        );
        continue;
      }
      folders.push({ real, account });
    }
  }

  // Two keys with the same real path and different values: neither is used.
  const clashing = new Set(
    folders
      .filter((a) =>
        folders.some((b) => b !== a && b.real === a.real && !sameAccount(a.account, b.account)),
      )
      .map((entry) => entry.real),
  );
  if (clashing.size > 0) {
    warnings.add(
      "config-duplicate",
      "accounts.yaml: two folders keys resolve to the same directory with different accounts; both are ignored",
    );
  }
  return {
    defaultName,
    folders: folders.filter((entry) => !clashing.has(entry.real)),
  };
}

/** The `folders` entry for a working directory: the longest real path that is it or a parent. */
export function matchFolder(config: UserConfig, realCwd: string): FolderEntry | undefined {
  let best: FolderEntry | undefined;
  for (const entry of config.folders) {
    if (!isWithin(entry.real, realCwd)) continue;
    if (best === undefined || entry.real.length > best.real.length) best = entry;
  }
  return best;
}
