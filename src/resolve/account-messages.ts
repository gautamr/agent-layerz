import { displayPath } from "../core/sanitize";
import type { AccountTarget } from "../model/account";
import { blockedFolder } from "./account-folders";
import type { AccountResolution, TargetAccount } from "./account-resolution";
import type { MissReason } from "./cached-stack";

/**
 * Fixed phrases for account resolution, shared by `alz account which`, `alz run`, the shell hook
 * and `alz doctor`. Nothing here repeats repository text: callers print sources and layer names
 * through `quote`, and paths through `displayPath`.
 */

/** Why a step named no account for a target, or why the layer step could not apply. */
export type SkipReason =
  | "no-use-choice"
  | "no-user-config"
  | "user-config-ignored"
  | "no-folders-match"
  | "entry-names-no-target"
  | "no-repository"
  | "cwd-in-account-folder"
  | "cwd-unresolvable"
  | "cwd-in-configured-folder"
  | "configured-folder"
  | "allowed-missing"
  | "allowed-ignored"
  | "not-allowed"
  | "layers-name-no-target"
  | "cache-miss"
  | "declaration-changed"
  | "declaration-removed"
  | "source-gone"
  | "declarations-conflict"
  | "manifest-unreadable"
  | "manifest-invalid";

const MISS: Record<MissReason | "cache-location", string> = {
  "account-folder": "a layer or the repository lies inside an account folder",
  "lock-missing": "layer.lock is missing",
  "lock-unreadable": "layer.lock cannot be read",
  "source-invalid": "a stack source is not valid",
  "lock-entry-missing": "layer.lock has no entry for a layer",
  "lock-entry-duplicate": "layer.lock has two entries for a layer",
  "lock-sha-invalid": "layer.lock holds a commit id that is not valid",
  "lock-ref-mismatch": "layer.lock records another ref than layer.yaml names",
  "cache-in-repo": "the layer cache is inside the repository",
  "layer-directory-missing": "a layer directory does not exist",
  "index-missing": "the cache index entry is missing",
  "index-invalid": "the cache index entry is not valid",
  "cache-entry-missing": "a cached layer is missing",
  "cache-entry-not-directory": "a cached layer is not a directory",
  "content-hash": "a layer no longer matches layer.lock",
  "layer-invalid": "a layer is not valid",
  "stack-invalid": "the stack is not valid",
  unreadable: "a layer or the lock cannot be read",
  "cache-location": "XDG_CACHE_HOME points away from ~/.cache and mise or direnv is active here",
};

export function missPhrase(miss: MissReason | "cache-location"): string {
  return MISS[miss];
}

const SKIP: Record<SkipReason, (target: AccountTarget) => string> = {
  "no-use-choice": () => "no alz use choice",
  "no-user-config": () => "no accounts.yaml",
  "user-config-ignored": () => "accounts.yaml is ignored",
  "no-folders-match": () => "no folders entry for this directory",
  "entry-names-no-target": (target) => `the folders entry names no ${target} account`,
  "no-repository": () => "no repository here",
  "cwd-in-account-folder": () => "the working directory is inside an account folder",
  "cwd-unresolvable": () => "the working directory cannot be resolved",
  "cwd-in-configured-folder": () =>
    "the working directory is inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names",
  "configured-folder": () =>
    "an allowed repository lies in a folder CLAUDE_CONFIG_DIR or CODEX_HOME names, so its layers are not read",
  "allowed-missing": () => "no allowed.yaml",
  "allowed-ignored": () => "allowed.yaml is ignored",
  "not-allowed": () => "this repository is not allowed",
  "layers-name-no-target": (target) => `the layers name no ${target} account`,
  "cache-miss": () => "the layer cache cannot be verified",
  "declaration-changed": () => "the layers' account declaration changed since it was allowed",
  "declaration-removed": () => "the layers no longer declare an account",
  "source-gone": () => "the allowed source is no longer in the stack",
  "declarations-conflict": () => "several client layers declare an account",
  "manifest-unreadable": () => "layer.yaml cannot be read",
  "manifest-invalid": () => "layer.yaml is not valid",
};

export function skipPhrase(
  reason: SkipReason,
  target: AccountTarget,
  miss?: MissReason | "cache-location",
): string {
  const phrase = SKIP[reason](target);
  return reason === "cache-miss" && miss !== undefined ? `cache miss: ${missPhrase(miss)}` : phrase;
}

export const BLOCKED_PHRASE = "the blocked folder";

export function foldersHere(existing: string[]): string {
  return existing.length === 0
    ? "no account folders here"
    : `account folders here: ${existing.join(", ")}`;
}

const NONE_REASON = {
  "not-found": "no repository here",
  "account-folder": "the working directory is inside an account folder",
  "configured-folder":
    "the working directory is inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names",
  "cwd-unresolvable": "the working directory cannot be resolved",
} as const;

/** The `repository:` line of `alz account which`. */
export function repositoryLine(repository: AccountResolution["repository"], home: string): string {
  if (repository.status === "none") return `repository: none (${NONE_REASON[repository.reason]})`;
  const problem =
    repository.manifest === "read"
      ? ""
      : ` (layer.yaml ${repository.manifest === "unreadable" ? "cannot be read" : "is not valid"})`;
  return `repository: ${displayPath(repository.root, home)}${problem}`;
}

/** The `accounts.yaml:` or `allowed.yaml:` line of `alz account which`. */
export function fileLine(
  name: string,
  file: AccountResolution["accountsFile"],
  home: string,
  allowed?: boolean,
): string {
  const path = displayPath(file.path, home);
  if (file.status === "missing") return `${name}: ${path} (not found)`;
  if (file.status === "ignored") return `${name}: ${path} (ignored: ${file.reason})`;
  return `${name}: ${path} (read${allowed ? "; this repository is allowed" : ""})`;
}

function applied(account: TargetAccount, home: string): string {
  const { apply, variable } = account;
  if (apply.action === "unset") return `${variable} unset`;
  if (apply.action === "keep") return `${variable}="${account.foreign ?? ""}" kept`;
  return `${variable}=${displayPath(apply.folder, home)}`;
}

/**
 * The lines `alz account which` prints for one target; `alz run`, the hook and `alz doctor`
 * reuse the pieces. Paths go through `displayPath`; repository text is never repeated here.
 */
export function describeTarget(
  account: TargetAccount,
  result: AccountResolution,
  home: string,
): string[] {
  const { target, source, failsClosed, variable } = account;
  // The layer step of a target that fails closed is reported with what the hook applies.
  const skippedLines = account.skipped
    .filter((skip) => !(failsClosed && skip.step === "layer"))
    .map((skip) => `  ${skip.step} step skipped (${skipPhrase(skip.reason, target)})`);

  if (failsClosed === undefined) {
    const head =
      source === "environment"
        ? "set outside alz"
        : account.account === "default" && result.defaultName !== undefined
          ? `default, called ${result.defaultName} here`
          : (account.account ?? "default");
    return [`${target}: ${head} (source: ${source}) ${applied(account, home)}`, ...skippedLines];
  }

  const lines = [`${target}: fails closed (source: ${source})`, ...skippedLines];
  const blocked = `${variable}=${displayPath(blockedFolder(home, target), home)} (${BLOCKED_PHRASE})`;
  switch (failsClosed.kind) {
    case "invalid-name":
      lines.push(
        `  ${source} step names something that is not an account name; fails closed`,
        "  the hook sets no folder",
        "  fix: use a name of lower-case letters, digits and '-' (or default)",
      );
      return lines;
    case "home":
      lines.push(
        `  ${source} step names an account, but HOME is not an absolute path; fails closed`,
        "  the hook sets no folder",
        "  fix: set HOME to an absolute path",
      );
      return lines;
    case "missing-folder": {
      const folder = account.apply.action === "set" ? displayPath(account.apply.folder, home) : "";
      lines.push(
        `  ${source} step names ${failsClosed.account}, but ${folder} does not exist; fails closed`,
        `  the hook applies ${variable}=${folder} (the missing folder)`,
        failsClosed.account === "alz-blocked"
          ? "  fix: the name alz-blocked is reserved for the blocked folders; choose another account name"
          : `  fix: alz account init ${failsClosed.account} (${foldersHere(failsClosed.existing)})`,
      );
      return lines;
    }
    case "cache-miss": {
      const recorded =
        failsClosed.recorded === "default"
          ? `${variable} unset (the recorded account default)`
          : `${applied(account, home)} (the recorded account ${failsClosed.recorded}${failsClosed.folderMissing ? "; that folder does not exist" : ""})`;
      lines.push(
        `  layer step skipped (cache miss: ${missPhrase(failsClosed.miss)}); fails closed`,
        `  the hook applies ${recorded}`,
        "  fix: alz build",
      );
      return lines;
    }
    default: {
      const reason = account.skipped.find((skip) => skip.step === "layer")?.reason;
      lines.push(
        `  layer step skipped (${reason === undefined ? "" : skipPhrase(reason, target)}); fails closed`,
        `  the hook applies ${blocked}`,
      );
      if (failsClosed.kind === "changed") {
        lines.push(`  fix: alz account allow${failsClosed.now ? "" : " --remove"}`);
      } else if (failsClosed.kind === "source-gone") {
        lines.push("  fix: alz account allow");
      } else if (failsClosed.kind === "configured-folder") {
        lines.push(
          "  fix: unset CLAUDE_CONFIG_DIR and CODEX_HOME here, or point them outside the repository",
        );
      } else if (failsClosed.kind === "conflict") {
        lines.push("  fix: remove one of the declarations, then run alz build");
      } else {
        const root = result.repository.status === "found" ? result.repository.root : "";
        lines.push(`  fix: repair ${displayPath(`${root}/layer.yaml`, home)}`);
      }
      return lines;
    }
  }
}
