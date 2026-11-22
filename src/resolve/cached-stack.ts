import { lstatSync } from "node:fs";
import { join, resolve } from "node:path";
import { Diagnostics } from "../core/diagnostics";
import type { Env } from "../core/env";
import { isWithin, readRegularFile } from "../core/files";
import { parseYamlLimited } from "../core/yaml";
import type { Declarer } from "../merge/declarers";
import type { Layer } from "../model/layer";
import { type LayerMeta, parseLayerMeta } from "../model/layer-meta";
import { LOCK_FILE, type Lock, type LockedLayer, parseLock } from "../model/lock";
import type { StackEntry } from "../model/manifest";
import { namesAccountFolder, realOrResolved, touchesAccountFolder } from "./account-folders";
import { verifyCachedLayer } from "./cache-integrity";
import { gitEntryName, isObjectId, layerRealPath, treeEntryName } from "./cache-paths";
import { validateStack } from "./layer-load";
import { readPathIndex } from "./path-index";
import { MAX_LAYER_YAML_BYTES, readLayerForResolution } from "./reader-load";
import {
  hasParentSegmentInGitUrl,
  locateSource,
  placeSource,
  sourcePathCandidates,
} from "./source-location";

/**
 * Reads the layers of a stack for account resolution (§7.13 'No fetch on resolve'). It never
 * runs git, fetches or writes: it finds each layer in the cache or in place, checks it against
 * the lock's content hash and reports anything it cannot verify as a miss with a stable code.
 * This module and what it imports never import `git.ts` or `node:child_process`.
 */

/** Stable codes: the caller maps them to its message and never prints repository text. */
export type MissReason =
  | "account-folder"
  | "lock-missing"
  | "lock-unreadable"
  | "source-invalid"
  | "lock-entry-missing"
  | "lock-entry-duplicate"
  | "lock-sha-invalid"
  | "lock-ref-mismatch"
  | "cache-in-repo"
  | "layer-directory-missing"
  | "index-missing"
  | "index-invalid"
  | "cache-entry-missing"
  | "cache-entry-not-directory"
  | "content-hash"
  | "layer-invalid"
  | "stack-invalid"
  | "unreadable";

export interface CachedStackInput {
  /** The real root of the repository. */
  repoRoot: string;
  /** The stack of the manifest the caller already parsed. */
  stack: StackEntry[];
  /** The layer cache, from `cacheRoot`. */
  cacheDir: string;
  home: string;
  /** Only `CLAUDE_CONFIG_DIR` and `CODEX_HOME` are read. */
  env: Env;
}

/**
 * One stack position. A layer that was not read (inside the repository, and unreadable there)
 * declares nothing. `source` is repository text exactly as written: quote it before printing.
 */
export interface StackLayer extends Declarer {
  index: number;
  entry: StackEntry;
  /** The layer is inside the repository (§7.12): it is read only to warn, never to decide. */
  inside: boolean;
  /** The lock's commit SHA; an unpinned `path:` layer has none. */
  sha?: string;
}

export type CachedStack =
  | {
      status: "miss";
      reason: MissReason;
      /** The stack entry that caused the miss; `source` is raw repository text, quote it. */
      entry?: { index: number; source: string };
    }
  | { status: "read"; layers: StackLayer[] };

const MAX_LOCK_BYTES = 8 * 1024 * 1024;

/** Metadata of a stack position that was not read: it declares nothing. */
const UNREAD: LayerMeta = {
  name: "",
  kind: "",
  visibility: "public",
  providesSlots: [],
  requiresSlots: [],
  forbidPatterns: [],
};

function readLockFile(repoRoot: string): Lock | "lock-missing" | "lock-unreadable" {
  const bytes = readRegularFile(join(repoRoot, LOCK_FILE), MAX_LOCK_BYTES);
  if (bytes === "missing") return "lock-missing";
  if (bytes === "invalid") return "lock-unreadable";
  try {
    return parseLock(bytes.toString("utf8"));
  } catch {
    return "lock-unreadable";
  }
}

/**
 * §7.13: a layer inside the repository is read only to warn when it declares an account, which
 * it may not. Only its `layer.yaml`, in place, as a bounded regular file; never checked against
 * the lock. Anything that cannot be read or parsed leaves the position empty.
 */
function readInsideLayer(
  located: ReturnType<typeof locateSource>,
  repoRoot: string,
  home: string,
  env: Env,
): LayerMeta {
  if (located.kind !== "path") return UNREAD;
  try {
    const realDir = layerRealPath(repoRoot, located.pathText);
    if (realDir === undefined || touchesAccountFolder(realDir, home, env)) return UNREAD;
    const bytes = readRegularFile(join(realDir, "layer.yaml"), MAX_LAYER_YAML_BYTES);
    if (!Buffer.isBuffer(bytes)) return UNREAD;
    return parseLayerMeta(parseYamlLimited(bytes.toString("utf8")));
  } catch {
    return UNREAD;
  }
}

type Directory = { dir: string; sha?: string; inPlace: boolean } | { miss: MissReason };

/** Finds the directory an outside layer is read from, with the lock entry it is checked against. */
function locateDirectory(
  located: ReturnType<typeof locateSource>,
  locked: LockedLayer,
  input: CachedStackInput,
): Directory {
  const { repoRoot, cacheDir, home, env } = input;
  if (located.kind === "git") {
    if (locked.sha === undefined || !isObjectId(locked.sha)) return { miss: "lock-sha-invalid" };
    return {
      dir: join(cacheDir, gitEntryName(located.cloneArg, locked.sha)),
      sha: locked.sha,
      inPlace: false,
    };
  }
  if (located.kind !== "path") return { miss: "source-invalid" };
  const realDir = layerRealPath(repoRoot, located.pathText);
  if (realDir === undefined) return { miss: "layer-directory-missing" };
  if (touchesAccountFolder(realDir, home, env)) return { miss: "account-folder" };
  if (!located.ref) {
    if (locked.sha !== undefined) return { miss: "lock-sha-invalid" };
    return { dir: realDir, inPlace: true };
  }
  // The lock records the ref a build resolved. A manifest that names another ref has not been
  // built, so the lock's commit is not the one the manifest asks for.
  if (locked.rev !== located.ref) return { miss: "lock-ref-mismatch" };
  if (locked.sha === undefined || !isObjectId(locked.sha)) return { miss: "lock-sha-invalid" };
  const index = readPathIndex(cacheDir, realDir, locked.sha);
  if ("miss" in index) return { miss: index.miss };
  return { dir: join(cacheDir, treeEntryName(index.tree)), sha: locked.sha, inPlace: false };
}

function isDirectory(path: string): boolean | undefined {
  try {
    return lstatSync(path).isDirectory();
  } catch {
    return undefined;
  }
}

function isRegularFile(path: string): boolean {
  try {
    return lstatSync(path).isFile();
  } catch {
    return false;
  }
}

/**
 * Locates, reads and checks each stack entry in stack order; the first miss is returned. Never
 * throws because of repository or cache content, runs no git and writes nothing.
 *
 * Before any file system call on a path the manifest names, the path is compared with the
 * account folders as written (`namesAccountFolder`); the real path is checked once it is known.
 * Whether a layer is inside the repository is decided only for a path that resolves: one that
 * does not is `layer-directory-missing`, never a layer to skip.
 */
export function readCachedStack(input: CachedStackInput): CachedStack {
  const { repoRoot, stack, cacheDir, home, env } = input;
  const realRepo = realOrResolved(repoRoot);
  const realCache = realOrResolved(cacheDir);
  if (touchesAccountFolder(realRepo, home, env) || touchesAccountFolder(realCache, home, env)) {
    return { status: "miss", reason: "account-folder" };
  }
  if (isWithin(realRepo, realCache)) return { status: "miss", reason: "cache-in-repo" };
  const lock = readLockFile(repoRoot);
  if (typeof lock === "string") return { status: "miss", reason: lock };
  const homes = [...new Set([resolve(home), realOrResolved(home)])];

  const slots: StackLayer[] = [];
  const outside: Layer[] = [];
  for (const [index, entry] of stack.entries()) {
    const miss = (reason: MissReason): CachedStack => ({
      status: "miss",
      reason,
      entry: { index, source: entry.source },
    });
    if (
      sourcePathCandidates(entry.source, repoRoot).some((c) => namesAccountFolder(c, homes, env))
    ) {
      return miss("account-folder");
    }
    if (hasParentSegmentInGitUrl(entry.source)) return miss("source-invalid");
    const located = locateSource(entry, repoRoot);
    if (located.kind === "invalid") return miss("source-invalid");
    if (
      located.kind === "git" &&
      !located.remote &&
      touchesAccountFolder(realOrResolved(located.cloneArg), home, env)
    ) {
      return miss("account-folder");
    }

    const place = placeSource(repoRoot, entry.source);
    if (place === "unresolvable") return miss("layer-directory-missing");
    if (place === "inside") {
      slots.push({
        index,
        entry,
        source: entry.source,
        inside: true,
        meta: readInsideLayer(located, repoRoot, home, env),
      });
      continue;
    }

    const matches = lock.layers.filter((layer) => layer.source === entry.source);
    const [locked] = matches;
    if (!locked) return miss("lock-entry-missing");
    if (matches.length > 1) return miss("lock-entry-duplicate");

    const found = locateDirectory(located, locked, input);
    if ("miss" in found) return miss(found.miss);
    const { dir, inPlace } = found;
    if (inPlace) {
      if (isDirectory(dir) !== true) return miss("layer-directory-missing");
      if (!isRegularFile(join(dir, "layer.yaml"))) return miss("layer-invalid");
    } else {
      const kind = isDirectory(dir);
      if (kind === undefined) return miss("cache-entry-missing");
      if (!kind) return miss("cache-entry-not-directory");
      if (touchesAccountFolder(realOrResolved(dir), home, env)) return miss("account-folder");
    }

    const loaded = readLayerForResolution(
      dir,
      entry.source,
      (contentHash) => verifyCachedLayer({ contentHash }, locked) === "ok",
    );
    if ("miss" in loaded) return miss(loaded.miss);
    const { layer } = loaded;
    outside.push({ ...layer, sha: found.sha });
    slots.push({
      index,
      entry,
      source: entry.source,
      inside: false,
      meta: layer.meta,
      sha: found.sha,
    });
  }

  // Layers inside the repository are not validated: editing `.layer/` never blocks the account.
  const scratch = new Diagnostics();
  validateStack(outside, scratch);
  if (scratch.hasErrors) return { status: "miss", reason: "stack-invalid" };
  return { status: "read", layers: slots };
}
