import { randomBytes } from "node:crypto";
import {
  existsSync,
  lstatSync,
  mkdirSync,
  realpathSync,
  renameSync,
  rmSync,
  statSync,
} from "node:fs";
import { join, resolve } from "node:path";
import { FatalError } from "../core/diagnostics";
import { isWithin, sha256 } from "../core/files";
import { stripControl } from "../core/sanitize";
import type { LockedLayer } from "../model/lock";
import { GIT_REF, type StackEntry } from "../model/manifest";
import { type Env, git } from "./git";
import { hashLayerDir } from "./layer-hash";
import {
  listTree,
  requireBlobs,
  TreeError,
  treeAt,
  treeEnv,
  verifyTreeEntry,
  writeTree,
} from "./tree-cache";

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
}

/** What `git clone` is given for `github:<owner>/<repo>`. */
export function githubCloneUrl(ownerRepo: string): string {
  return `https://github.com/${ownerRepo}.git`;
}

export function resolveSource(entry: StackEntry, ctx: ResolveContext): ResolvedSource {
  const { source } = entry;
  if (source.startsWith("path:")) return resolvePath(entry, source.slice(5), ctx);
  if (source.startsWith("github:")) {
    const { url, ref } = splitRef(source.slice(7), source);
    const clone = githubCloneUrl(url);
    return resolveGit(source, ref, ctx, { arg: clone, remote: true });
  }
  if (source.startsWith("git:")) {
    const { url, ref } = splitRef(source.slice(4), source);
    const target = gitTarget(ctx.repoRoot, url);
    if (target.kind === "invalid") throw new FatalError("source", `'${source}': ${target.reason}`);
    const arg = target.kind === "local" ? target.clone : url;
    return resolveGit(source, ref, ctx, { arg, remote: target.kind === "remote" });
  }
  throw new FatalError("source", `unsupported source '${source}' (use path:, github: or git:)`);
}

/** What `git clone` tries for a local repository argument (clone.c `get_repo_path`), in its order. */
const CLONE_SUFFIXES = ["/.git", "", ".git/.git", ".git", ".bundle"];

/** How alz reads the URL of a `git:` source. */
export type GitTarget =
  | { kind: "remote" }
  | {
      kind: "local";
      /** Canonical absolute paths git may read, so what is checked is what is cloned. */
      paths: string[];
      /** The one path handed to `git clone`, instead of the URL as written. */
      clone: string;
    }
  | { kind: "invalid"; reason: string };

function hasDotDot(path: string): boolean {
  return path.split("/").includes("..");
}

/** The path of a `file://` URL as written: no normalisation, so `..` stays visible. */
function fileUrlPath(url: string): string | undefined {
  const rest = url.slice("file://".length);
  const path = rest.startsWith("/") ? rest : rest.slice(rest.indexOf("/"));
  if (!path.startsWith("/")) return undefined;
  try {
    return decodeURIComponent(path);
  } catch {
    return undefined;
  }
}

function canonical(path: string): string {
  try {
    return realpathSync.native(path);
  } catch {
    return path;
  }
}

/**
 * Classifies the URL of a `git:` source. Git looks for the argument on disk, relative to the
 * working directory (the repo root), before it parses it as a URL, so `x:y`, or `file:///dir`
 * with a `file:/dir` folder, can be repository content: those are local whenever such a path
 * exists, with git's own suffixes. Only then is a `file://` URL its absolute path, and a
 * `scheme://` or `host:path` argument a remote.
 *
 * The path is looked up from the string as written, so the operating system resolves any `..`
 * through symlinks exactly as git will; it is then canonicalised, and that canonical path is
 * what is checked and cloned. A local source with a `..` segment is invalid: text and disk can
 * disagree about where it leads.
 */
export function gitTarget(repoRoot: string, url: string): GitTarget {
  const invalid = (reason: string): GitTarget => ({ kind: "invalid", reason });
  const dotdot = "a local git: path must not contain '..' segments; use a path without them";
  const raw = url.startsWith("/") ? url : `${repoRoot}/${url}`;
  const onDisk = (base: string) =>
    CLONE_SUFFIXES.map((suffix) => `${base}${suffix}`).filter((path) => existsSync(path));
  const local = (found: string[]): GitTarget => {
    const paths = found.map(canonical);
    return { kind: "local", paths, clone: paths[0] ?? "" };
  };

  const found = onDisk(raw);
  if (found.length > 0) return hasDotDot(url) ? invalid(dotdot) : local(found);
  if (url.startsWith("file://")) {
    const path = fileUrlPath(url);
    if (path === undefined) return invalid("not a valid file:// URL");
    if (hasDotDot(path)) return invalid(dotdot);
    const there = onDisk(path);
    return there.length > 0 ? local(there) : { kind: "local", paths: [path], clone: path };
  }
  if (/^[^/]*:/.test(url)) return { kind: "remote" };
  if (hasDotDot(url)) return invalid(dotdot);
  const path = resolve(repoRoot, url);
  return { kind: "local", paths: [path], clone: path };
}

/** The URL of a `git:<url>@<ref>` source, split the way `splitRef` does. */
function gitUrl(spec: string): string {
  const at = spec.lastIndexOf("@");
  return at > 0 ? spec.slice(0, at) : spec;
}

function isInside(repoRoot: string, dir: string): boolean {
  if (isWithin(resolve(repoRoot), dir)) return true;
  try {
    return isWithin(realpathSync.native(repoRoot), realpathSync.native(dir));
  } catch {
    // Unreadable means unknown: treat it as inside, the stricter answer.
    return true;
  }
}

/**
 * §7.12: is the layer inside the repository? Never for `github:` sources or `git:` sources that
 * name a remote. A `path:` source, and a `git:` source that is a local path or a `file://` URL
 * (see `gitTarget`), is inside when it resolves under the repo root as written, or after
 * following symlinks (with the file system's own spelling, so a case variant on a
 * case-insensitive disk is the same place). A link outside the repo that points into it is
 * inside as well. A `git:` source that cannot be classified safely counts as inside.
 */
export function isInsideRepo(repoRoot: string, source: string): boolean {
  let dirs: string[] = [];
  if (source.startsWith("path:")) dirs = [resolve(repoRoot, source.slice(5))];
  else if (source.startsWith("git:")) {
    const target = gitTarget(repoRoot, gitUrl(source.slice(4)));
    if (target.kind === "invalid") return true;
    if (target.kind === "local") dirs = target.paths;
  }
  return dirs.some((dir) => isInside(repoRoot, dir));
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
  if (!existsSync(dir) || !statSync(dir).isDirectory()) {
    throw new FatalError("source", `layer directory not found for '${entry.source}'`);
  }
  if (!entry.ref) return { dir };
  return materializeAtRef(entry.source, dir, entry.ref, ctx);
}

/**
 * §4.4/§5.2: a pinned path layer is read from the git tree at `ref`, not from the working
 * tree, so later commits in the same repo (e.g. hub docs) and local edits do not change the
 * build. The layer may be the repo root or a subdirectory of it (`hub/layer`).
 */
function materializeAtRef(
  source: string,
  dir: string,
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
  const root = cacheRoot(ctx);
  const cached = join(root, `tree2@${tree}`);
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
      if (existsSync(cached)) rmSync(temp, { recursive: true, force: true });
      else renameSync(temp, cached);
    } catch (error) {
      rmSync(temp, { recursive: true, force: true });
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
  return { dir: cached, rev: ref, sha, cached: wasCached };
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

function cacheRoot(ctx: ResolveContext): string {
  return join(ctx.env.XDG_CACHE_HOME ?? join(ctx.home, ".cache"), "agent-layerz");
}

function splitRef(spec: string, source: string): { url: string; ref: string } {
  const at = spec.lastIndexOf("@");
  const ref = at < 0 ? "" : spec.slice(at + 1);
  if (at <= 0 || ref === "" || ref.includes(":")) {
    throw new FatalError("source", `'${source}' needs a ref: <url>@<tag or commit>`);
  }
  if (!GIT_REF.test(ref))
    throw new FatalError("source", `'${source}': '${ref}' is not a valid git ref`);
  if (spec.startsWith("-"))
    throw new FatalError("source", `'${source}': a URL cannot start with '-'`);
  return { url: spec.slice(0, at), ref };
}

/**
 * §6: the cache directory name of a git source, before `@<sha>`. A hash of exactly what is
 * cloned (the remote URL, or the canonical path of a local repository), so no two different
 * URLs share an entry; the URL is not rewritten, so the same text always gives the same key.
 */
export function gitCacheKey(cloneArg: string): string {
  return `git-${sha256(cloneArg)}`;
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
  const root = cacheRoot(ctx);
  const key = gitCacheKey(clone.arg);
  // `readLock` already refuses a sha that is not 40 or 64 hexadecimal characters.
  const lockedSha = ctx.locked.find((layer) => layer.source === source)?.sha;
  if (lockedSha) {
    const cached = join(root, `${key}@${lockedSha}`);
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
    const dir = join(root, `${key}@${sha}`);
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
