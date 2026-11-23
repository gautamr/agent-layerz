import { createHash } from "node:crypto";
import { join } from "node:path";

/**
 * The names the layer cache uses, written from docs/LAYERING-SPEC.md and not from src/. A test
 * that builds a cache entry by hand, or looks one up, uses these, so the test agrees with the
 * contract and not with the implementation. `test/reference/spec-formulas.test.ts` checks them
 * against src/ while both exist.
 */

/** The lowercase hexadecimal sha256 of bytes, or of a text's UTF-8 bytes (§6: "sha256 ... in hex"). */
export const sha256Hex = (data: string | Uint8Array): string =>
  createHash("sha256").update(data).digest("hex");

/**
 * §6: the cache is `$XDG_CACHE_HOME/agent-layerz`, default `~/.cache/agent-layerz`. Only a set,
 * absolute `XDG_CACHE_HOME` is a case here: §7.13 says an empty or relative one counts as unset
 * for account resolution, and the same rule for the build is TASK-53's, so that case is not
 * written.
 */
export const cacheRoot = (home: string, env: { XDG_CACHE_HOME?: string }): string =>
  join(env.XDG_CACHE_HOME ?? join(home, ".cache"), "agent-layerz");

/**
 * §6: what `git clone` is given for `github:owner/repo`. A `git:` source is cloned from its URL
 * exactly as written, or from the canonical absolute path of a local repository.
 */
export const githubCloneUrl = (ownerRepo: string): string => `https://github.com/${ownerRepo}.git`;

/** §6: `git-<sha256 of what is cloned, in hex>`; the text is not rewritten before it is hashed. */
export const gitCacheKey = (cloneArg: string): string => `git-${sha256Hex(cloneArg)}`;

/** §6: the cache directory of a `github:` or `git:` layer, `<key>@<commit sha>`. */
export const gitEntryName = (cloneArg: string, sha: string): string =>
  `${gitCacheKey(cloneArg)}@${sha}`;

/** §6: the cache directory of a pinned `path:` layer, `tree2@<git tree id>`. */
export const treeEntryName = (tree: string): string => `tree2@${tree}`;

/**
 * §6 and §7.13: the index entry next to a `tree2@` entry,
 * `pathref@<sha256 of the layer directory's real path>-<commit sha>`.
 */
export const pathIndexName = (realDir: string, sha: string): string =>
  `pathref@${sha256Hex(realDir)}-${sha}`;
