import { realpathSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import type { Env } from "../core/env";
import { readRegularFile } from "../core/files";
import { displayPath } from "../core/sanitize";
import { parseYamlLimited } from "../core/yaml";
import { isStackManifest, MANIFEST_FILE, type Manifest, parseManifest } from "../model/manifest";
import { accountFolderRelation, folderRelationAsWritten, realOrResolved } from "./account-folders";
import type { WarningList } from "./account-warning";
import { MAX_LAYER_YAML_BYTES } from "./reader-load";

/** Why there is no repository: stable codes for the skip reasons. */
export type NoRoot =
  | "not-found"
  | "account-folder"
  /** Inside a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names: no manifest is read there. */
  | "configured-folder"
  | "cwd-unresolvable";

/** `realCwd` is the real working directory when it was resolved: callers reuse it, never resolve it again. */
export type RepositoryRoot =
  | { status: "found"; root: string; realCwd: string; manifest: "read"; stack: Manifest["stack"] }
  | {
      status: "found";
      root: string;
      realCwd: string;
      manifest: "unreadable" | "invalid";
      stack?: undefined;
    }
  | { status: "none"; reason: NoRoot; realCwd?: string };

/**
 * §7.13 'Paths': the repository root is the nearest directory at or above the working directory
 * whose `layer.yaml` has a `stack` key. A `layer.yaml` without one is skipped. One that cannot
 * be read (a link, a folder, over 1 MiB, unreadable), parsed (invalid YAML, past the alias
 * limit) or validated stops the walk: that directory is the root, with a warning, and the walk
 * never goes on past it (decision-19). The walk never opens a `layer.yaml` at `$HOME`, above
 * it, or in an account folder; a directory that merely contains a folder `CLAUDE_CONFIG_DIR` or
 * `CODEX_HOME` names can still be the root.
 */
export function findRepositoryRoot(
  cwd: string,
  home: string,
  env: Env,
  warnings: WarningList,
): RepositoryRoot {
  const homes = [...new Set([resolve(home), realOrResolved(home)])];
  // As written, before any file system call on the working directory.
  const written = folderRelationAsWritten(cwd, homes, env);
  if (written === "inside") return { status: "none", reason: "account-folder" };
  if (written === "home") return { status: "none", reason: "not-found" };
  let real: string;
  try {
    real = realpathSync.native(resolve(cwd));
  } catch {
    return { status: "none", reason: "cwd-unresolvable" };
  }
  const relation = accountFolderRelation(real, home, env);
  if (relation === "inside") return { status: "none", reason: "account-folder", realCwd: real };
  if (relation === "home") return { status: "none", reason: "not-found", realCwd: real };
  // Inside a folder a variable names (mise or direnv can point it at a repository or a parent):
  // nothing there is opened, so no manifest is read. The caller still checks allowed.yaml
  // against the working directory and fails closed for an allowed repository.
  if (relation === "configured") {
    return { status: "none", reason: "configured-folder", realCwd: real };
  }

  // A directory that only contains a folder a variable names stays a candidate root: step 4
  // then reads no layer and is a cache miss (`account-folder`), which fails closed.
  for (let dir = real; ; dir = dirname(dir)) {
    if (accountFolderRelation(dir, home, env) === "home") {
      return { status: "none", reason: "not-found", realCwd: real };
    }
    const found = readManifestAt(dir);
    if (found.status === "continue") {
      if (dirname(dir) === dir) return { status: "none", reason: "not-found", realCwd: real };
      continue;
    }
    if (found.status === "stack") {
      return { status: "found", root: dir, realCwd: real, manifest: "read", stack: found.stack };
    }
    warnings.add(
      "manifest",
      `${displayPath(join(dir, MANIFEST_FILE), home)} cannot be used (${found.status === "unreadable" ? "it cannot be read, parsed or is too large" : "its stack is not valid"}); the walk for the repository stops here`,
    );
    return { status: "found", root: dir, realCwd: real, manifest: found.status };
  }
}

type Read =
  | { status: "continue" }
  | { status: "stack"; stack: Manifest["stack"] }
  | { status: "unreadable" | "invalid" };

function readManifestAt(dir: string): Read {
  const bytes = readRegularFile(join(dir, MANIFEST_FILE), MAX_LAYER_YAML_BYTES);
  if (bytes === "missing") return { status: "continue" };
  if (bytes === "invalid") return { status: "unreadable" };
  let raw: unknown;
  try {
    raw = parseYamlLimited(bytes.toString("utf8"));
  } catch {
    return { status: "unreadable" };
  }
  if (!isStackManifest(raw)) return { status: "continue" };
  try {
    return { status: "stack", stack: parseManifest(raw).stack };
  } catch {
    return { status: "invalid" };
  }
}
