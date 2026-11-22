import { realpathSync } from "node:fs";
import { isAbsolute, resolve } from "node:path";
import type { Env } from "../core/env";
import { isWithin } from "../core/files";
import { isMap, unknownKeys } from "../core/yaml";
import {
  ACCOUNT_TARGETS,
  type AccountMap,
  type AccountTarget,
  parseAccount,
} from "../model/account";
import { accountFolderRelation, folderRelationAsWritten, realOrResolved } from "./account-folders";
import type { WarningList } from "./account-warning";

/** One approval in `allowed.yaml` (§7.13 'Allow state'). `source` is raw text: quote it to print. */
export interface AllowedEntry {
  root: string;
  account: AccountMap;
  source: string;
}

export interface AllowState {
  /** Entries whose root resolved, each with its real root. */
  entries: { entry: AllowedEntry; real: string }[];
}

export const EMPTY_ALLOW_STATE: AllowState = { entries: [] };

const TARGETS = Object.keys(ACCOUNT_TARGETS) as AccountTarget[];

/** The same set of targets and the same names. */
export function sameMap(a: AccountMap, b: AccountMap): boolean {
  return TARGETS.every((target) => a[target] === b[target]);
}

function realOf(path: string): string | undefined {
  try {
    return realpathSync.native(path);
  } catch {
    return undefined;
  }
}

/**
 * §7.13 'Allow state': parses the value of `allowed.yaml`. The top level is a list (null is an
 * empty file). An entry that is not a map with exactly `root`, `account` and `source`, whose
 * root is not absolute, or whose account is not a map of valid names, is ignored with a
 * warning that holds its position only. Two entries for one real root are both ignored, so
 * that root is not allowed.
 */
export function parseAllowState(
  value: unknown,
  ctx: { home: string; env: Env },
  warnings: WarningList,
): AllowState {
  if (value === null || value === undefined) return EMPTY_ALLOW_STATE;
  if (!Array.isArray(value)) {
    warnings.add("allowed-shape", "allowed.yaml is ignored: its top level must be a list");
    return EMPTY_ALLOW_STATE;
  }
  const homes = [...new Set([resolve(ctx.home), realOrResolved(ctx.home)])];
  const found: { entry: AllowedEntry; real: string; position: number }[] = [];
  for (const [index, raw] of value.entries()) {
    const position = index + 1;
    const ignore = (reason: string) =>
      warnings.add("allowed-entry", `allowed.yaml: entry ${position} is ignored (${reason})`);
    if (!isMap(raw) || unknownKeys(raw, ["root", "account", "source"]).length > 0) {
      ignore("it needs exactly root, account and source");
      continue;
    }
    const { root, account, source } = raw;
    if (typeof root !== "string" || !isAbsolute(root)) {
      ignore("its root is not an absolute path");
      continue;
    }
    if (typeof source !== "string" || source === "") {
      ignore("its source is not a non-empty string");
      continue;
    }
    let map: AccountMap;
    try {
      if (!isMap(account)) throw new Error("not a map");
      map = parseAccount(account);
    } catch {
      ignore("its account is not a mapping of target to a valid account name");
      continue;
    }
    // A root that only contains a folder a variable names, or lies inside one, stays allowed:
    // step 4 then fails closed (a root in a configured folder is matched by working directory).
    const written = folderRelationAsWritten(root, homes, ctx.env);
    if (written === "inside" || written === "home") {
      ignore("its root lies inside an account folder, or is $HOME or above it");
      continue;
    }
    const real = realOf(root);
    if (real === undefined) continue;
    if (real !== root) {
      ignore("its root is not a real path (alz account allow records the real path)");
      continue;
    }
    const relation = accountFolderRelation(real, ctx.home, ctx.env);
    if (relation === "inside" || relation === "home") {
      ignore("its root lies inside an account folder, or is $HOME or above it");
      continue;
    }
    found.push({ entry: { root, account: map, source }, real, position });
  }
  const roots = new Map<string, number[]>();
  for (const item of found) roots.set(item.real, [...(roots.get(item.real) ?? []), item.position]);
  const entries = found.filter((item) => (roots.get(item.real)?.length ?? 0) === 1);
  for (const positions of roots.values()) {
    if (positions.length > 1) {
      warnings.add(
        "allowed-entry",
        `allowed.yaml: entries ${positions.join(", ")} are ignored (they have the same root)`,
      );
    }
  }
  return { entries: entries.map(({ entry, real }) => ({ entry, real })) };
}

/** §7.13 'Paths': an entry matches only the exact real root, so a nested repository needs its own. */
export function entryFor(state: AllowState, realRoot: string): AllowedEntry | undefined {
  return state.entries.find((item) => item.real === realRoot)?.entry;
}

/**
 * Entries whose real root is `realCwd` or one of its parents, nearest first. Used when the
 * working directory is inside a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names: no manifest
 * is read there, so the repository cannot be identified, and an allowed repository fails closed.
 */
export function entriesCovering(state: AllowState, realCwd: string): AllowedEntry[] {
  return state.entries
    .filter((item) => isWithin(item.real, realCwd))
    .sort((a, b) => b.real.length - a.real.length)
    .map((item) => item.entry);
}
