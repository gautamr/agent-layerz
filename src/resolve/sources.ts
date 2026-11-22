import { randomBytes } from "node:crypto";
import { existsSync, lstatSync, mkdirSync, renameSync, rmSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { FatalError } from "../core/diagnostics";
import { stripControl } from "../core/sanitize";
import type { LockedLayer } from "../model/lock";
import type { StackEntry } from "../model/manifest";
import { cacheRoot, gitEntryName, isObjectId, layerRealPath, treeEntryName } from "./cache-paths";
import { type Env, git } from "./git";
import { hashLayerDir } from "./layer-hash";
import { writePathIndex } from "./path-index";
import { locateSource } from "./source-location";
import {
  listTree,
  requireBlobs,
  TreeError,
  treeAt,
  treeEnv,
  verifyTreeEntry,
  writeTree,
} from "./tree-cache";

export { gitCacheKey } from "./cache-paths";
export { type GitTarget, githubCloneUrl, gitTarget, isInsideRepo } from "./source-location";

export interface ResolveContext {
  repoRoot: string;
  home: string;
  env: Env;
  /** Layers recorded in the previous lock; git sources reuse their pinned commit. */
  locked: LockedLayer[];
}

export interface ResolvedSource {
  dir: string;
  rev?: string;
  sha?: string;
  /** The directory was already in the cache, so its content is not new (§5.3). */
  cached?: boolean;
  /** The cache index entry could not be written: a warning, not an error (§6). */
  indexWarning?: string;
}

export function resolveSource(entry: StackEntry, ctx: ResolveContext): ResolvedSource {
  const located = locateSource(entry, ctx.repoRoot);
  if (located.kind === "invalid") throw new FatalError("source", located.reason);
  if (located.kind === "path") return resolvePath(entry, located.pathText, ctx);
  return resolveGit(entry.source, located.ref, ctx, {
    arg: located.cloneArg,
    remote: located.remote,
  });
}

/**
 * Arguments and environment for cloning a source alz classed as remote: without the file
 * transport git cannot fall back to a local path. `GIT_ALLOW_PROTOCOL` would override that
 * setting, so it is dropped (target-facts §11).
 */
export function guardedClone(env: Env): { args: string[]; env: Env } {
  const clean = { ...env };
  delete clean.GIT_ALLOW_PROTOCOL;
  return { args: ["-c", "protocol.file.allow=never"], env: clean };
}

function resolvePath(entry: StackEntry, path: string, ctx: ResolveContext): ResolvedSource {
  const dir = resolve(ctx.repoRoot, path);
  const realDir = layerRealPath(ctx.repoRoot, path);
  if (realDir === undefined || !existsSync(dir) || !statSync(dir).isDirectory()) {
    throw new FatalError("source", `layer directory not found for '${entry.source}'`);
  }
  if (!entry.ref) return { dir };
  return materializeAtRef(entry.source, dir, realDir, entry.ref, ctx);
}

/**
 * §4.4/§5.2: a pinned path layer is read from the git tree at `ref`, not from the working
 * tree, so later commits in the same repo (e.g. hub docs) and local edits do not change the
 * build. The layer may be the repo root or a subdirectory of it (`hub/layer`).
 */
function materializeAtRef(
  source: string,
  dir: string,
  realDir: string,
  ref: string,
  ctx: ResolveContext,
): ResolvedSource {
  // No replace refs and no lazy fetch: the tree and its blobs are what the repository holds.
  const env = treeEnv(ctx.env);
  let top: string;
  let sha: string;
  let tree: string;
  try {
    top = git(["rev-parse", "--show-toplevel"], env, dir);
    const prefix = git(["rev-parse", "--show-prefix"], env, dir).replace(/\/$/, "");
    sha = git(["rev-parse", "--verify", "--quiet", `${ref}^{commit}`], env, dir);
    // A real tree, not a link or a submodule entry whose recorded id could be any tree id.
    tree = treeAt(sha, prefix, env, top);
  } catch {
    throw new FatalError(
      "source",
      `cannot resolve ref ${ref} for '${stripControl(source)}': not a git repo, unknown ref, the layer did not exist at that ref, or its path is not a plain directory there`,
    );
  }

  // Content-addressed by the layer's tree, so equal content at two refs shares one copy. The
  // name is versioned (`tree2@`): entries of the older format were written by `checkout-index`,
  // with attribute conversions, and are ignored.
  const root = cacheRoot(ctx.home, ctx.env);
  const cached = join(root, treeEntryName(tree));
  let files: ReturnType<typeof listTree>;
  try {
    files = listTree(tree, env, top);
    requireBlobs(files, env, top);
  } catch (error) {
    const detail = error instanceof Error ? error.message : "unreadable tree";
    throw new FatalError("source", `cannot read '${stripControl(source)}' at ${ref}: ${detail}`);
  }
  const wasCached = existsSync(cached) || isLink(cached);
  if (wasCached) {
    // Checked against the tree itself, not the lock: the commit may have moved while the tree
    // did not, which is the normal case for a hub whose docs change.
    const problem = verifyTreeEntry(cached, files);
    if (problem)
      throw cacheError(source, cached, `no longer matches git tree ${tree} (${problem})`);
  } else {
    mkdirSync(root, { recursive: true });
    const temp = join(root, `.tmp-${randomBytes(6).toString("hex")}`);
    try {
      writeTree(temp, files, env, top);
      // The first build is checked too, before the entry gets its name.
      const problem = verifyTreeEntry(temp, files);
      if (problem) throw new TreeError(`the written entry does not match its tree (${problem})`);
      if (existsSync(cached)) {
        // Another process made the entry first: it is checked like any entry that was already there.
        rmSync(temp, { recursive: true, force: true });
        const other = verifyTreeEntry(cached, files);
        if (other) {
          throw cacheError(source, cached, `no longer matches git tree ${tree} (${other})`);
        }
      } else renameSync(temp, cached);
    } catch (error) {
      rmSync(temp, { recursive: true, force: true });
      if (error instanceof FatalError) throw error;
      const detail =
        error instanceof TreeError
          ? error.message
          : ((error as { stderr?: Buffer }).stderr?.toString().trim().split("\n")[0] ?? "");
      throw new FatalError(
        "source",
        `cannot read '${stripControl(source)}' at ${ref}: ${stripControl(detail)}`,
      );
    }
  }
  // The index lets account resolution find this entry without git (§7.13 'No fetch on resolve').
  const indexWarning =
    isObjectId(sha) && isObjectId(tree)
      ? writePathIndex(root, realDir, sha, tree, source)
      : undefined;
  return { dir: cached, rev: ref, sha, cached: wasCached, indexWarning };
}

function isLink(path: string): boolean {
  try {
    return lstatSync(path).isSymbolicLink();
  } catch {
    return false;
  }
}

/** §5.3: a cache entry that is not what it should be; never quotes file content. */
function cacheError(source: string, dir: string, what: string): FatalError {
  return new FatalError(
    "cache",
    `cached layer '${stripControl(source)}' ${what}: its files in ${dir} changed after they were cached; delete that directory and rebuild`,
    source,
  );
}

/** A cache entry is a real directory: a link could point at another source's entry. */
function requireRealDirectory(source: string, dir: string): void {
  if (!lstatSync(dir).isDirectory()) throw cacheError(source, dir, "is not a plain directory");
}

function resolveGit(
  source: string,
  ref: string,
  ctx: ResolveContext,
  clone: { arg: string; remote: boolean },
): ResolvedSource {
  const root = cacheRoot(ctx.home, ctx.env);
  // `readLock` already refuses a sha that is not 40 or 64 hexadecimal characters.
  const lockedSha = ctx.locked.find((layer) => layer.source === source)?.sha;
  if (lockedSha) {
    const cached = join(root, gitEntryName(clone.arg, lockedSha));
    if (existsSync(cached) || isLink(cached)) {
      requireRealDirectory(source, cached);
      return { dir: cached, rev: lockedSha.slice(0, 12), sha: lockedSha, cached: true };
    }
  }

  mkdirSync(root, { recursive: true });
  const temp = join(root, `.tmp-${randomBytes(6).toString("hex")}`);
  try {
    // Run in the repo, so a relative path means the same place here as in `isInsideRepo`.
    const guard = clone.remote ? guardedClone(ctx.env) : { args: [], env: ctx.env };
    git([...guard.args, "clone", "--quiet", "--", clone.arg, temp], guard.env, ctx.repoRoot);
    const sha = git(["rev-parse", `${lockedSha ?? ref}^{commit}`], ctx.env, temp);
    git(["-c", "advice.detachedHead=false", "checkout", "--quiet", sha], ctx.env, temp);
    const dir = join(root, gitEntryName(clone.arg, sha));
    const cached = existsSync(dir) || isLink(dir);
    if (cached) {
      // An entry already there is not trusted over a fresh clone of the same commit: with no
      // lock entry to check it against, compare the two, and keep the clone out of the cache.
      requireRealDirectory(source, dir);
      const same = hashLayerDir(dir).contentHash === hashLayerDir(temp).contentHash;
      rmSync(temp, { recursive: true, force: true });
      if (!same) throw cacheError(source, dir, "differs from a fresh clone of the same commit");
    } else renameSync(temp, dir);
    return { dir, rev: sha.slice(0, 12), sha, cached };
  } catch (error) {
    rmSync(temp, { recursive: true, force: true });
    if (error instanceof FatalError) throw error;
    const detail = (error as { stderr?: string }).stderr?.trim().split("\n")[0] ?? "";
    throw new FatalError("source", `cannot fetch '${source}': ${detail}`);
  }
}
