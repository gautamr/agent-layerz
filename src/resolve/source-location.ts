import { existsSync, realpathSync } from "node:fs";
import { resolve } from "node:path";
import { FatalError } from "../core/diagnostics";
import { isWithin } from "../core/files";
import { GIT_REF, type StackEntry } from "../model/manifest";
import { layerRealPath } from "./cache-paths";

/** What `git clone` is given for `github:<owner>/<repo>`. */
export function githubCloneUrl(ownerRepo: string): string {
  return `https://github.com/${ownerRepo}.git`;
}

/** Where a stack entry's layer comes from, found without touching git or the network. */
export type LocatedSource =
  | { kind: "path"; pathText: string; dir: string; ref?: string }
  | {
      kind: "git";
      /** What `git clone` is given, and what the cache key is made of. */
      cloneArg: string;
      remote: boolean;
      ref: string;
      /** The source in the form `allowed.yaml` records: no ref, and a canonical path if local. */
      recorded: string;
    }
  | { kind: "invalid"; reason: string };

/**
 * Parses a stack entry into the place it is read from. It only parses and looks at file
 * metadata (`gitTarget`), so `build` and account resolution key a source the same way. The
 * `reason` of an invalid source is the message `build` stops with.
 */
export function locateSource(entry: StackEntry, repoRoot: string): LocatedSource {
  const { source } = entry;
  try {
    if (source.startsWith("path:")) {
      const pathText = source.slice(5);
      return { kind: "path", pathText, dir: resolve(repoRoot, pathText), ref: entry.ref };
    }
    if (source.startsWith("github:")) {
      const { url, ref } = splitRef(source.slice(7), source);
      const cloneArg = githubCloneUrl(url);
      return { kind: "git", cloneArg, remote: true, ref, recorded: cloneArg };
    }
    if (source.startsWith("git:")) {
      const { url, ref } = splitRef(source.slice(4), source);
      const target = gitTarget(repoRoot, url);
      if (target.kind === "invalid")
        return { kind: "invalid", reason: `'${source}': ${target.reason}` };
      if (target.kind === "local") {
        return { kind: "git", cloneArg: target.clone, remote: false, ref, recorded: target.clone };
      }
      return { kind: "git", cloneArg: url, remote: true, ref, recorded: url };
    }
  } catch (error) {
    if (error instanceof FatalError) return { kind: "invalid", reason: error.message };
    throw error;
  }
  return {
    kind: "invalid",
    reason: `unsupported source '${source}' (use path:, github: or git:)`,
  };
}

/**
 * §7.13: the source of a layer as `allowed.yaml` records it, so a ref bump never changes it: a
 * local layer's real path, a remote URL without its ref, or the canonical path of a local git
 * repository. Undefined when the source is invalid or a `path:` layer does not exist.
 */
export function recordedSource(entry: StackEntry, repoRoot: string): string | undefined {
  const located = locateSource(entry, repoRoot);
  if (located.kind === "path") return layerRealPath(repoRoot, located.pathText);
  return located.kind === "git" ? located.recorded : undefined;
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

/** Where a layer is relative to the repository; `unresolvable` when that cannot be told. */
export type Placement = "inside" | "outside" | "unresolvable";

function placeDir(repoRoot: string, dir: string): Placement {
  if (isWithin(resolve(repoRoot), dir)) return "inside";
  try {
    return isWithin(realpathSync.native(repoRoot), realpathSync.native(dir)) ? "inside" : "outside";
  } catch {
    return "unresolvable";
  }
}

/** The directories a `path:` or local `git:` source names, or undefined for other sources. */
function sourceDirs(repoRoot: string, source: string): string[] | "invalid" | undefined {
  if (source.startsWith("path:")) return [resolve(repoRoot, source.slice(5))];
  if (source.startsWith("git:")) {
    const target = gitTarget(repoRoot, gitUrl(source.slice(4)));
    if (target.kind === "invalid") return "invalid";
    if (target.kind === "local") return target.paths;
  }
  return undefined;
}

/**
 * Like `isInsideRepo`, but tells a directory that cannot be resolved from one that is outside.
 * Account resolution needs the difference: a layer that cannot be found is a cache miss, not a
 * layer to skip. `inside` wins over `unresolvable` when a source names several directories.
 */
export function placeSource(repoRoot: string, source: string): Placement {
  const dirs = sourceDirs(repoRoot, source);
  if (dirs === "invalid") return "inside";
  if (dirs === undefined) return "outside";
  const places = dirs.map((dir) => placeDir(repoRoot, dir));
  if (places.includes("inside")) return "inside";
  return places.includes("unresolvable") ? "unresolvable" : "outside";
}

/**
 * §7.12: is the layer inside the repository? Never for `github:` sources or `git:` sources that
 * name a remote. A `path:` source, and a `git:` source that is a local path or a `file://` URL
 * (see `gitTarget`), is inside when it resolves under the repo root as written, or after
 * following symlinks (with the file system's own spelling, so a case variant on a
 * case-insensitive disk is the same place). A link outside the repo that points into it is
 * inside as well. A `git:` source that cannot be classified safely, or whose directory cannot
 * be resolved, counts as inside: the stricter answer for who may declare.
 */
export function isInsideRepo(repoRoot: string, source: string): boolean {
  return placeSource(repoRoot, source) !== "outside";
}

/**
 * The paths a `path:` or `git:` source names as written, made absolute without touching the file
 * system, for a check that must come before any call on them (`namesAccountFolder`).
 */
export function sourcePathCandidates(source: string, repoRoot: string): string[] {
  if (source.startsWith("path:")) return [resolve(repoRoot, source.slice(5))];
  if (!source.startsWith("git:")) return [];
  const url = gitUrl(source.slice(4));
  const found = [resolve(repoRoot, url)];
  if (url.startsWith("file://")) {
    const path = fileUrlPath(url);
    if (path !== undefined) found.push(resolve(path));
  }
  return found;
}

/**
 * A `git:` source with a `..` segment in its URL. `gitTarget` probes the path as written, which
 * the operating system may resolve through a link to somewhere a lexical check did not see, so
 * account resolution refuses such a source before probing it.
 */
export function hasParentSegmentInGitUrl(source: string): boolean {
  return source.startsWith("git:") && hasDotDot(gitUrl(source.slice(4)));
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
