import { isAbsolute, join, resolve } from "node:path";
import { type Env, envManagerActive } from "../core/env";
import { compareCodePoints } from "../core/files";
import { displayPath, quote } from "../core/sanitize";
import { xdgBase } from "../core/xdg";
import {
  ACCOUNT_TARGETS,
  type AccountMap,
  type AccountTarget,
  isAccountName,
} from "../model/account";
import type { StackEntry } from "../model/manifest";
import {
  type AllowedEntry,
  type AllowState,
  EMPTY_ALLOW_STATE,
  entriesCovering,
  entryFor,
  parseAllowState,
  sameMap,
} from "./account-allow-state";
import {
  type AccountDeclaration,
  type LayerRef,
  type MissReason,
  readAccountDeclaration,
} from "./account-declaration";
import { foreignValue } from "./account-environment";
import { type AccountFile, locateAccountFile, readAccountFile } from "./account-files";
import {
  type AccountNames,
  accountFolder,
  blockedFolder,
  listAccountNames,
  namesAccountFolder,
  realOrResolved,
} from "./account-folders";
import type { SkipReason } from "./account-messages";
import { findRepositoryRoot, type NoRoot, type RepositoryRoot } from "./account-root";
import {
  EMPTY_USER_CONFIG,
  matchFolder,
  parseUserConfig,
  type UserConfig,
} from "./account-user-config";
import { type AccountWarning, WarningList } from "./account-warning";
import { hasParentSegmentInGitUrl, recordedSource, sourcePathCandidates } from "./source-location";

export type { AccountFile, AllowedEntry, LayerRef, MissReason, SkipReason };

export type Source = "run" | "use" | "environment" | "user config" | "layer" | "default";

export interface AccountInput {
  /** The working directory, as given. */
  cwd: string;
  /** `$HOME`; folders are built from it as given. */
  home: string;
  /** Read by name only, and only the variables listed in `ENV_NAMES`. Never spread or copied. */
  env: Env;
  /** `process.getuid?.()`; undefined means both files are ignored. */
  uid: number | undefined;
  /** Default: every target. */
  targets?: AccountTarget[];
  /** Step 1: `alz run --account`. */
  run?: { target: AccountTarget; account: string };
  /** Step 2: the choice the `alz` shell function passes. */
  use?: AccountMap;
}

/** The environment variables resolution reads the value of; the presence of two more is checked. */
export const ENV_NAMES = [
  "CLAUDE_CONFIG_DIR",
  "CODEX_HOME",
  "ALZ_SET_CLAUDE_CONFIG_DIR",
  "ALZ_SET_CODEX_HOME",
  "XDG_CONFIG_HOME",
  "XDG_STATE_HOME",
  "XDG_CACHE_HOME",
] as const;

export type FailClosed =
  | { kind: "missing-folder"; account: string; existing: string[] }
  | {
      kind: "cache-miss";
      miss: MissReason | "cache-location";
      entry?: { index: number; source: string };
      recorded: string;
      folderMissing?: true;
    }
  | { kind: "changed"; allowed: AllowedEntry; now?: { account: AccountMap; source: string } }
  | { kind: "source-gone"; allowed: AllowedEntry }
  | { kind: "conflict"; clients: LayerRef[] }
  | { kind: "manifest"; problem: "unreadable" | "invalid" }
  /** `$HOME` is not an absolute path: no folder can be named, so a named account cannot apply. */
  | { kind: "home" }
  /** An explicit `alz run` or `alz use` name that is not an account name: no folder is built. */
  | { kind: "invalid-name" }
  /**
   * The working directory is inside a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names, and an
   * allowed repository covers it: its layers are not read there, so the allowed entry's
   * targets get the blocked folder.
   */
  | { kind: "configured-folder"; allowed: AllowedEntry };

export type Apply = { action: "set"; folder: string } | { action: "unset" } | { action: "keep" };

export interface TargetAccount {
  target: AccountTarget;
  variable: (typeof ACCOUNT_TARGETS)[AccountTarget];
  source: Source;
  /** A validated name or "default"; undefined for a foreign value or a blocked target. */
  account?: string;
  /** For a foreign value: the value, through `displayPath`. */
  foreign?: string;
  apply: Apply;
  /** Steps that were consulted and named nothing. */
  skipped: { step: "use" | "user config" | "layer"; reason: SkipReason }[];
  /** `alz run` and `alz use` stop; the hook warns and applies `apply`. */
  failsClosed?: FailClosed;
}

export interface AccountResolution {
  repository:
    | { status: "found"; root: string; manifest: "read" | "unreadable" | "invalid" }
    | { status: "none"; reason: NoRoot };
  accountsFile: AccountFile;
  allowedFile: AccountFile & { allowed: boolean };
  /**
   * Roots of valid `allowed.yaml` entries that do not resolve on this machine: approvals that
   * match nothing and that `alz account allow` keeps. Raw text; print through `displayPath` and `quote`.
   */
  staleApprovals: string[];
  /** What the default account is called on this machine, when accounts.yaml says so. */
  defaultName?: string;
  targets: TargetAccount[];
  warnings: AccountWarning[];
}

const ALL_TARGETS = Object.keys(ACCOUNT_TARGETS) as AccountTarget[];

type LayerOutcome =
  | { kind: "skipped"; reason: SkipReason }
  | { kind: "declared"; account: AccountMap }
  | {
      kind: "fails";
      reason: SkipReason;
      /** Targets that fail closed, with what the hook applies. */
      targets: Partial<Record<AccountTarget, { apply: Apply; failsClosed: FailClosed }>>;
    };

function isDefault(name: string, defaultName: string | undefined): boolean {
  return name === "default" || name === defaultName;
}

function describeMap(map: AccountMap): string {
  return ALL_TARGETS.filter((target) => map[target] !== undefined)
    .map((target) => `${target}=${map[target]}`)
    .join(", ");
}

/**
 * §7.13 'Resolution': the account of each target. Writes nothing, runs no git and fetches
 * nothing. It reads the `layer.yaml` files on the walk to the repository root, `accounts.yaml`,
 * `allowed.yaml`, the names in `$HOME` and, only for an allowed repository, what
 * `readAccountDeclaration` reads. The result holds no file content and no environment value
 * other than the account variables; print repository text only through `quote`.
 */
export function resolveAccounts(input: AccountInput): AccountResolution {
  const { cwd, home, env, uid } = input;
  const targets = input.targets ?? ALL_TARGETS;
  const warnings = new WarningList();

  // A relative `$HOME` names no folder: both files are ignored, nothing is read from the file
  // system, and no account folder is ever set.
  const homeValid = isAbsolute(home);
  if (!homeValid) {
    warnings.add(
      "home",
      "HOME is not an absolute path, so accounts.yaml and allowed.yaml are ignored and no account folder is set",
    );
  }
  const repository: RepositoryRoot = homeValid
    ? findRepositoryRoot(cwd, home, env, warnings)
    : { status: "none", reason: "not-found" };
  const root = repository.status === "found" ? repository.root : undefined;
  const files = { home, env, uid, root };
  const unusable = (name: "accounts.yaml" | "allowed.yaml") => ({
    file: {
      path: locateAccountFile(name, home, env).path,
      status: "ignored" as const,
      reason: "HOME is not an absolute path",
    },
    value: undefined,
  });
  const accounts = homeValid
    ? readAccountFile("accounts.yaml", files, warnings)
    : unusable("accounts.yaml");
  const allowed = homeValid
    ? readAccountFile("allowed.yaml", files, warnings)
    : unusable("allowed.yaml");
  const names: AccountNames = homeValid ? listAccountNames(home) : { claude: [], codex: [] };
  const config: UserConfig =
    accounts.file.status === "read"
      ? parseUserConfig(accounts.value, { home, env, names }, warnings)
      : EMPTY_USER_CONFIG;
  const state: AllowState =
    allowed.file.status === "read"
      ? parseAllowState(allowed.value, { home, env }, warnings)
      : EMPTY_ALLOW_STATE;
  const entry = root === undefined ? undefined : entryFor(state, root);
  // The walk resolved the working directory once; nothing resolves it again.
  const realCwd = repository.realCwd;

  const named = (
    target: AccountTarget,
    name: string,
    source: Source,
    skipped: TargetAccount["skipped"],
  ): TargetAccount => {
    const base = { target, variable: ACCOUNT_TARGETS[target], source, account: name, skipped };
    if (isDefault(name, config.defaultName)) {
      return { ...base, account: "default", apply: { action: "unset" } };
    }
    if (!homeValid) {
      return { ...base, apply: { action: "keep" }, failsClosed: { kind: "home" } };
    }
    const folder = accountFolder(home, target, name);
    const apply: Apply = { action: "set", folder };
    if (names[target].includes(name)) return { ...base, apply };
    return {
      ...base,
      apply,
      failsClosed: { kind: "missing-folder", account: name, existing: names[target] },
    };
  };

  let outcome: LayerOutcome | undefined;
  const layerOutcome = (): LayerOutcome => {
    outcome ??= resolveLayerStep({
      home,
      env,
      names,
      repository,
      accountsFile: accounts.file,
      allowedFile: allowed.file,
      state,
      entry,
      defaultName: config.defaultName,
      warnings,
    });
    return outcome;
  };

  const resolved = targets.map((target): TargetAccount => {
    const variable = ACCOUNT_TARGETS[target];
    const skipped: TargetAccount["skipped"] = [];
    // A name that is not an account name is a content problem, never a path. An explicit choice
    // that cannot apply fails closed: `alz run` and `alz use` stop, and it never falls to a
    // lower step.
    const invalid = (source: "run" | "use"): TargetAccount => ({
      target,
      variable,
      source,
      apply: { action: "keep" },
      skipped,
      failsClosed: { kind: "invalid-name" },
    });
    if (input.run?.target === target) {
      return isAccountName(input.run.account)
        ? named(target, input.run.account, "run", skipped)
        : invalid("run");
    }
    const used = input.use?.[target];
    if (used !== undefined) {
      return isAccountName(used) ? named(target, used, "use", skipped) : invalid("use");
    }
    skipped.push({ step: "use", reason: "no-use-choice" });

    const foreign = foreignValue(target, env);
    if (foreign !== undefined) {
      warnings.add(
        "foreign",
        `${variable} was set outside alz${envManagerActive(env) ? "; mise or direnv is active here" : ""}; alz keeps it`,
      );
      return {
        target,
        variable,
        source: "environment",
        foreign: displayPath(foreign, home),
        apply: { action: "keep" },
        skipped,
      };
    }

    // Step 3: the user config.
    // No folders entry can match a working directory that is in an account folder or that could
    // not be resolved, and nothing is resolved again.
    const cwdReason =
      repository.status === "none" && repository.reason === "account-folder"
        ? "cwd-in-account-folder"
        : repository.status === "none" && repository.reason === "cwd-unresolvable"
          ? "cwd-unresolvable"
          : undefined;
    const match =
      cwdReason !== undefined || realCwd === undefined ? undefined : matchFolder(config, realCwd);
    const configName = match?.account[target];
    if (configName !== undefined) return named(target, configName, "user config", skipped);
    skipped.push({
      step: "user config",
      reason:
        cwdReason ??
        (accounts.file.status === "ignored"
          ? "user-config-ignored"
          : accounts.file.status === "missing"
            ? "no-user-config"
            : match === undefined
              ? "no-folders-match"
              : "entry-names-no-target"),
    });

    // Step 4: the layer declaration.
    const layer = layerOutcome();
    if (layer.kind === "declared") {
      const name = layer.account[target];
      if (name !== undefined) return named(target, name, "layer", skipped);
      skipped.push({ step: "layer", reason: "layers-name-no-target" });
    } else if (layer.kind === "skipped") {
      skipped.push({ step: "layer", reason: layer.reason });
    } else {
      const failing = layer.targets[target];
      if (failing !== undefined) {
        skipped.push({ step: "layer", reason: layer.reason });
        return {
          target,
          variable,
          source: "layer",
          account:
            failing.failsClosed.kind === "cache-miss" ? failing.failsClosed.recorded : undefined,
          apply: failing.apply,
          skipped,
          failsClosed: failing.failsClosed,
        };
      }
      skipped.push({ step: "layer", reason: "layers-name-no-target" });
    }

    // Step 5: the default account. It never fails.
    return {
      target,
      variable,
      source: "default",
      account: "default",
      apply: { action: "unset" },
      skipped,
    };
  });

  return {
    repository:
      repository.status === "found"
        ? { status: "found", root: repository.root, manifest: repository.manifest }
        : repository,
    accountsFile: accounts.file,
    allowedFile: { ...allowed.file, allowed: entry !== undefined },
    staleApprovals: state.unresolved.map((item) => item.root).sort(compareCodePoints),
    defaultName: config.defaultName,
    targets: resolved,
    warnings: warnings.items,
  };
}

interface LayerStepInput {
  home: string;
  env: Env;
  names: AccountNames;
  repository: RepositoryRoot;
  accountsFile: AccountFile;
  allowedFile: AccountFile;
  state: AllowState;
  entry: AllowedEntry | undefined;
  defaultName: string | undefined;
  warnings: WarningList;
}

/** Step 4, evaluated once: it reads the stack at most one time, and only for an allowed root. */
function resolveLayerStep(input: LayerStepInput): LayerOutcome {
  const { home, env, repository, entry, warnings } = input;
  const blocked = (
    failsClosed: FailClosed,
    reason: SkipReason,
    affected: AccountTarget[],
  ): LayerOutcome => ({
    kind: "fails",
    reason,
    targets: Object.fromEntries(
      affected.map((target) => [
        target,
        { apply: { action: "set", folder: blockedFolder(home, target) } as Apply, failsClosed },
      ]),
    ),
  });
  if (repository.status === "none") {
    if (repository.reason === "configured-folder" && repository.realCwd !== undefined) {
      if (input.allowedFile.status === "missing") {
        return { kind: "skipped", reason: "allowed-missing" };
      }
      if (input.allowedFile.status === "ignored") {
        return { kind: "skipped", reason: "allowed-ignored" };
      }
      // Nothing in the configured folder is opened, so the repository cannot be identified: an
      // allowed repository that is the working directory or one of its parents fails closed.
      const covering = entriesCovering(input.state, repository.realCwd);
      const [nearest] = covering;
      if (nearest === undefined) return { kind: "skipped", reason: "cwd-in-configured-folder" };
      const affected = ALL_TARGETS.filter((target) =>
        covering.some((item) => item.account[target] !== undefined),
      );
      return blocked(
        { kind: "configured-folder", allowed: nearest },
        "configured-folder",
        affected,
      );
    }
    return {
      kind: "skipped",
      reason:
        repository.reason === "account-folder"
          ? "cwd-in-account-folder"
          : repository.reason === "cwd-unresolvable"
            ? "cwd-unresolvable"
            : repository.reason === "configured-folder"
              ? "cwd-in-configured-folder"
              : "no-repository",
    };
  }
  if (input.allowedFile.status === "missing") return { kind: "skipped", reason: "allowed-missing" };
  if (input.allowedFile.status === "ignored") return { kind: "skipped", reason: "allowed-ignored" };
  if (entry === undefined) return { kind: "skipped", reason: "not-allowed" };

  const allowedTargets = ALL_TARGETS.filter((target) => entry.account[target] !== undefined);
  const blockedAllowed = (failsClosed: FailClosed, reason: SkipReason, affected = allowedTargets) =>
    blocked(failsClosed, reason, affected);

  if (repository.manifest !== "read") {
    return blockedAllowed(
      { kind: "manifest", problem: repository.manifest },
      repository.manifest === "unreadable" ? "manifest-unreadable" : "manifest-invalid",
    );
  }

  const { declaration, miss } = readRepositoryDeclaration(
    repository.root,
    repository.stack,
    home,
    env,
  );

  if (declaration.status !== "miss" && miss === undefined) {
    for (const ignored of declaration.ignored) {
      warnings.add(
        "declaration-ignored",
        `the layer ${quote(ignored.layer.name)} (${quote(ignored.layer.source)}) declares an account but may not, so it is ignored: ${ignored.reasons.join("; ")}`,
      );
    }
  }

  if (miss !== undefined) {
    if (!sourceStillInStack(repository.root, repository.stack, entry.source, home, env)) {
      return blockedAllowed({ kind: "source-gone", allowed: entry }, "source-gone");
    }
    const targets: LayerOutcome & { kind: "fails" } = {
      kind: "fails",
      reason: "cache-miss",
      targets: {},
    };
    for (const target of allowedTargets) {
      const recorded = entry.account[target] ?? "default";
      const defaulted = isDefault(recorded, input.defaultName);
      const folder = accountFolder(home, target, recorded);
      targets.targets[target] = {
        apply: defaulted ? { action: "unset" } : { action: "set", folder },
        failsClosed: {
          kind: "cache-miss",
          miss,
          entry: declaration.status === "miss" ? declaration.entry : undefined,
          recorded: defaulted ? "default" : recorded,
          ...(!defaulted && !input.names[target].includes(recorded)
            ? { folderMissing: true as const }
            : {}),
        },
      };
    }
    return targets;
  }

  if (declaration.status === "conflict") {
    return blockedAllowed(
      { kind: "conflict", clients: declaration.clients },
      "declarations-conflict",
    );
  }
  if (declaration.status !== "read")
    return blockedAllowed({ kind: "manifest", problem: "invalid" }, "manifest-invalid");
  const declared = declaration.declared;
  if (declared === undefined) {
    return blockedAllowed({ kind: "changed", allowed: entry }, "declaration-removed");
  }
  if (sameMap(declared.account, entry.account) && declared.recordedSource === entry.source) {
    return { kind: "declared", account: declared.account };
  }
  const affected = ALL_TARGETS.filter(
    (target) => entry.account[target] !== undefined || declared.account[target] !== undefined,
  );
  warnings.add(
    "changed",
    `the account declaration changed since it was allowed: was ${describeMap(entry.account)} from ${quote(entry.source)}, now ${describeMap(declared.account)} from ${quote(declared.recordedSource)}`,
  );
  return blockedAllowed(
    {
      kind: "changed",
      allowed: entry,
      now: { account: declared.account, source: declared.recordedSource },
    },
    "declaration-changed",
    affected,
  );
}

/**
 * The stack of a repository read the way resolution reads it (§7.13 'No fetch on resolve'): the
 * layer cache location, then `readAccountDeclaration`. Resolution, `alz account allow` and the
 * build and check hints all call this one function, so `allow` accepts exactly what resolution
 * applies. `miss` is set when something could not be verified, with `"cache-location"` for a
 * cache that `XDG_CACHE_HOME` moved while mise or direnv is active. Reads only; no git.
 */
export function readRepositoryDeclaration(
  root: string,
  stack: StackEntry[],
  home: string,
  env: Env,
): { declaration: AccountDeclaration; miss: MissReason | "cache-location" | undefined } {
  const cacheDir = cacheLocation(home, env);
  const declaration: AccountDeclaration =
    cacheDir === undefined
      ? { status: "miss", reason: "unreadable" }
      : readAccountDeclaration({ repoRoot: root, stack, cacheDir, home, env });
  const miss: MissReason | "cache-location" | undefined =
    cacheDir === undefined
      ? "cache-location"
      : declaration.status === "miss"
        ? declaration.reason
        : undefined;
  return { declaration, miss };
}

/**
 * §7.13 'No fetch on resolve': where the layer cache is read from. A cache that `XDG_CACHE_HOME`
 * moves away from `~/.cache` while mise or direnv is active here is not trusted: undefined.
 */
function cacheLocation(home: string, env: Env): string | undefined {
  const base = xdgBase(env, "XDG_CACHE_HOME", home, ".cache");
  if (envManagerActive(env) && base.fromVariable) {
    if (realOrResolved(base.dir) !== realOrResolved(join(home, ".cache"))) return undefined;
  }
  return join(base.dir, "agent-layerz");
}

/**
 * Is the source recorded in `allowed.yaml` still in the stack? An entry whose path names an
 * account folder as written, or whose git URL has a `..` segment, is skipped before anything is
 * probed on disk, as the cache reader does.
 */
function sourceStillInStack(
  root: string,
  stack: StackEntry[],
  recorded: string,
  home: string,
  env: Env,
): boolean {
  const homes = [...new Set([resolve(home), realOrResolved(home)])];
  return stack.some((entry) => {
    if (
      sourcePathCandidates(entry.source, root).some((path) => namesAccountFolder(path, homes, env))
    ) {
      return false;
    }
    if (hasParentSegmentInGitUrl(entry.source)) return false;
    return recordedSource(entry, root) === recorded;
  });
}
