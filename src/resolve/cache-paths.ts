import { realpathSync } from "node:fs";
import { join, resolve } from "node:path";
import type { Env } from "../core/env";
import { sha256 } from "../core/files";

/** A git commit or tree id: 40 (SHA-1) or 64 (SHA-256) lowercase hexadecimal characters (§5.3). */
export function isObjectId(text: string): boolean {
  return /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/.test(text);
}

/** §6: the layer cache, `$XDG_CACHE_HOME/agent-layerz` or `~/.cache/agent-layerz`. */
export function cacheRoot(home: string, env: Env): string {
  return join(env.XDG_CACHE_HOME ?? join(home, ".cache"), "agent-layerz");
}

/**
 * §6: the cache directory name of a git source, before `@<sha>`. A hash of exactly what is
 * cloned (the remote URL, or the canonical path of a local repository), so no two different
 * URLs share an entry; the URL is not rewritten, so the same text always gives the same key.
 */
export function gitCacheKey(cloneArg: string): string {
  return `git-${sha256(cloneArg)}`;
}

/** The cache entry of a `github:` or `git:` layer at a commit. */
export function gitEntryName(cloneArg: string, sha: string): string {
  return `${gitCacheKey(cloneArg)}@${sha}`;
}

/** The cache entry of a pinned `path:` layer: its directory in git at a commit, by tree id. */
export function treeEntryName(tree: string): string {
  return `tree2@${tree}`;
}

/** §7.13: the index entry that names the tree of a pinned `path:` layer at a commit. */
export function pathIndexName(realDir: string, sha: string): string {
  return `pathref@${sha256(realDir)}-${sha}`;
}

/**
 * The real path of the directory a `path:` source names, or undefined when it does not exist.
 * `build` and account resolution both key the cache index with this one function.
 */
export function layerRealPath(repoRoot: string, pathText: string): string | undefined {
  try {
    return realpathSync.native(resolve(repoRoot, pathText));
  } catch {
    return undefined;
  }
}
