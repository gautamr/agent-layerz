import { randomBytes } from "node:crypto";
import { existsSync, mkdirSync, renameSync, rmSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { FatalError } from "../core/diagnostics";
import type { LockedLayer } from "../model/lock";
import { GIT_REF, type StackEntry } from "../model/manifest";
import { type Env, git } from "./git";

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
}

export function resolveSource(entry: StackEntry, ctx: ResolveContext): ResolvedSource {
  const { source } = entry;
  if (source.startsWith("path:")) return resolvePath(entry, source.slice(5), ctx);
  if (source.startsWith("github:")) {
    const { url, ref } = splitRef(source.slice(7), source);
    return resolveGit(source, `https://github.com/${url}.git`, ref, ctx);
  }
  if (source.startsWith("git:")) {
    const { url, ref } = splitRef(source.slice(4), source);
    return resolveGit(source, url, ref, ctx);
  }
  throw new FatalError("source", `unsupported source '${source}' (use path:, github: or git:)`);
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
  let top: string;
  let sha: string;
  let tree: string;
  try {
    top = git(["rev-parse", "--show-toplevel"], ctx.env, dir);
    const prefix = git(["rev-parse", "--show-prefix"], ctx.env, dir).replace(/\/$/, "");
    sha = git(["rev-parse", "--verify", "--quiet", `${ref}^{commit}`], ctx.env, dir);
    tree = git(["rev-parse", "--verify", "--quiet", `${sha}:${prefix}`], ctx.env, dir);
  } catch {
    throw new FatalError(
      "source",
      `cannot resolve ref ${ref} for '${source}': not a git repo, unknown ref, or the layer did not exist at that ref`,
    );
  }

  // Content-addressed by the layer's tree, so equal content at two refs shares one copy.
  const root = cacheRoot(ctx);
  const cached = join(root, `tree@${tree}`);
  if (!existsSync(cached)) {
    mkdirSync(root, { recursive: true });
    const temp = join(root, `.tmp-${randomBytes(6).toString("hex")}`);
    // A private index keeps the repo's own index and working tree untouched. Run from the top
    // level: inside a subdirectory, checkout-index only writes entries under that subdirectory.
    const env = { ...ctx.env, GIT_INDEX_FILE: `${temp}.index` };
    try {
      git(["read-tree", tree], env, top);
      git(["checkout-index", "--all", `--prefix=${temp}/`], env, top);
      mkdirSync(temp, { recursive: true });
      if (existsSync(cached)) rmSync(temp, { recursive: true, force: true });
      else renameSync(temp, cached);
    } catch (error) {
      rmSync(temp, { recursive: true, force: true });
      const detail = (error as { stderr?: string }).stderr?.trim().split("\n")[0] ?? "";
      throw new FatalError("source", `cannot read '${source}' at ${ref}: ${detail}`);
    } finally {
      rmSync(`${temp}.index`, { force: true });
    }
  }
  return { dir: cached, rev: ref, sha };
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

function resolveGit(source: string, url: string, ref: string, ctx: ResolveContext): ResolvedSource {
  const root = cacheRoot(ctx);
  const key = url
    .replace(/^[a-z]+:\/\//, "")
    .replace(/\.git$/, "")
    .replace(/[^A-Za-z0-9._-]+/g, "-");
  const lockedSha = ctx.locked.find((layer) => layer.source === source)?.sha;
  if (lockedSha) {
    const cached = join(root, `${key}@${lockedSha}`);
    if (existsSync(cached)) return { dir: cached, rev: lockedSha.slice(0, 12), sha: lockedSha };
  }

  mkdirSync(root, { recursive: true });
  const temp = join(root, `.tmp-${randomBytes(6).toString("hex")}`);
  try {
    git(["clone", "--quiet", "--", url, temp], ctx.env);
    const sha = git(["rev-parse", `${lockedSha ?? ref}^{commit}`], ctx.env, temp);
    git(["-c", "advice.detachedHead=false", "checkout", "--quiet", sha], ctx.env, temp);
    const dir = join(root, `${key}@${sha}`);
    if (existsSync(dir)) rmSync(temp, { recursive: true, force: true });
    else renameSync(temp, dir);
    return { dir, rev: sha.slice(0, 12), sha };
  } catch (error) {
    rmSync(temp, { recursive: true, force: true });
    if (error instanceof FatalError) throw error;
    const detail = (error as { stderr?: string }).stderr?.trim().split("\n")[0] ?? "";
    throw new FatalError("source", `cannot fetch '${source}': ${detail}`);
  }
}
