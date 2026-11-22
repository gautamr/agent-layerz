import { createHash } from "node:crypto";
import { chmodSync, lstatSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { isSafeRelativePath, isSkippedName } from "../core/files";
import { quote } from "../core/sanitize";
import { type Env, gitBytes } from "./git";

/** A regular file of a git tree, as `git ls-tree -r` lists it. */
export interface TreeFile {
  path: string;
  oid: string;
  executable: boolean;
}

/** A problem with a tree or its entry. Messages quote every path, which is repository content. */
export class TreeError extends Error {}

/**
 * The environment of every git call that reads a layer's tree: replace refs would swap one
 * object for another under the same id, and a lazy fetch would read from a remote the repository
 * names.
 */
export function treeEnv(env: Env): Env {
  return { ...env, GIT_NO_REPLACE_OBJECTS: "1", GIT_NO_LAZY_FETCH: "1" };
}

function record(entry: string): { mode: string; type: string; oid: string; path: string } {
  const match = /^([0-7]{6}) ([a-z]+) ([0-9a-f]{40}|[0-9a-f]{64})\t([^\0]+)$/.exec(entry);
  if (!match) throw new TreeError("unreadable tree listing");
  return { mode: match[1] ?? "", type: match[2] ?? "", oid: match[3] ?? "", path: match[4] ?? "" };
}

/**
 * The id of the tree at `prefix` (the repository root when empty) of a commit. It must be a
 * real tree: `<commit>:<path>` also names a symbolic link's blob or a submodule's commit, whose
 * recorded id can be any other tree id, so the entry is listed and its mode and type checked.
 */
export function treeAt(commit: string, prefix: string, env: Env, top: string): string {
  if (prefix === "") {
    const found = gitBytes(["rev-parse", "--verify", "--quiet", `${commit}^{tree}`], env, top);
    return found.toString("utf8").trim();
  }
  const listing = gitBytes(
    ["--literal-pathspecs", "ls-tree", "-z", commit, "--", prefix],
    env,
    top,
  ).toString("utf8");
  const entries = listing.split("\0").filter((entry) => entry !== "");
  const [only] = entries.map(record);
  if (entries.length !== 1 || !only) throw new TreeError("not found");
  if (only.mode !== "040000" || only.type !== "tree" || only.path !== prefix) {
    throw new TreeError("not a directory");
  }
  return only.oid;
}

/** Names that differ only by case or Unicode form are one name on some file systems. */
function fold(name: string): string {
  return name.normalize("NFC").toLowerCase().normalize("NFC");
}

/** Refuses two paths, or a file and a folder, that a case-folding or normalising disk would merge. */
function refuseCollisions(paths: string[]): void {
  const seen = new Map<string, { spelling: string; folder: boolean }>();
  const note = (spelling: string, folder: boolean) => {
    const key = fold(spelling);
    const before = seen.get(key);
    if (before && (before.spelling !== spelling || before.folder !== folder)) {
      throw new TreeError(
        `paths ${quote(before.spelling)} and ${quote(spelling)} differ only by case or Unicode form`,
      );
    }
    seen.set(key, { spelling, folder });
  };
  for (const path of paths) {
    const parts = path.split("/");
    parts.forEach((_part, index) => {
      note(parts.slice(0, index + 1).join("/"), index < parts.length - 1);
    });
  }
}

/**
 * Every regular file of a tree. Links and submodule entries are skipped, as the loader skips
 * them in a git or unpinned layer, so a hub with `CLAUDE.md -> AGENTS.md` keeps working; they
 * are not written, and a link found in an entry is a change. A path with a backslash, a `.`,
 * `..` or `.git` segment is refused, and so is a tree with two paths one disk would merge.
 */
export function listTree(treeOid: string, env: Env, top: string): TreeFile[] {
  const listing = gitBytes(["ls-tree", "-r", "-z", treeOid], env, top).toString("utf8");
  const files: TreeFile[] = [];
  for (const entry of listing.split("\0").filter((item) => item !== "")) {
    const { mode, type, oid, path } = record(entry);
    if (
      !isSafeRelativePath(path) ||
      path.split("/").some((part) => part.toLowerCase() === ".git")
    ) {
      throw new TreeError(`unsafe path ${quote(path)}`);
    }
    if (type === "commit" || mode === "120000") continue;
    if (type !== "blob") throw new TreeError(`${quote(path)} is not a file`);
    files.push({ path, oid, executable: (Number.parseInt(mode, 8) & 0o111) !== 0 });
  }
  refuseCollisions(files.map((file) => file.path));
  return files.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
}

/**
 * The repository being built must hold every blob its tree lists, as a blob. Another
 * repository's tree id can name a cache entry this repository never had the content of.
 */
export function requireBlobs(files: TreeFile[], env: Env, top: string): void {
  const unique = [...new Set(files.map((file) => file.oid))];
  if (unique.length === 0) return;
  const out = gitBytes(["cat-file", "--batch-check"], env, top, `${unique.join("\n")}\n`)
    .toString("utf8")
    .split("\n");
  unique.forEach((oid, index) => {
    const type = (out[index] ?? "").split(" ")[1];
    if (type !== "blob") {
      const path = files.find((file) => file.oid === oid)?.path ?? "";
      throw new TreeError(`the repository does not hold the file ${quote(path)}`);
    }
  });
}

/** The object id git gives a blob, computed here with no conversion. */
export function blobId(bytes: Buffer, algorithm: "sha1" | "sha256"): string {
  return createHash(algorithm).update(`blob ${bytes.length}\0`).update(bytes).digest("hex");
}

function algorithmOf(oid: string): "sha1" | "sha256" {
  return oid.length === 64 ? "sha256" : "sha1";
}

/** Blob contents by id, read with `git cat-file --batch`: raw bytes, no filter or conversion. */
function readBlobs(oids: string[], env: Env, top: string): Map<string, Buffer> {
  const unique = [...new Set(oids)];
  const blobs = new Map<string, Buffer>();
  if (unique.length === 0) return blobs;
  const out = gitBytes(["cat-file", "--batch"], env, top, `${unique.join("\n")}\n`);
  let at = 0;
  for (const oid of unique) {
    const end = out.indexOf(0x0a, at);
    const [id, type, size] = out.subarray(at, end).toString("utf8").split(" ");
    if (id !== oid || type !== "blob" || size === undefined) throw new TreeError("missing object");
    const start = end + 1;
    const bytes = out.subarray(start, start + Number(size));
    if (blobId(bytes, algorithmOf(oid)) !== oid)
      throw new TreeError("an object differs from its id");
    blobs.set(oid, bytes);
    at = start + Number(size) + 1;
  }
  return blobs;
}

/**
 * Writes the files of a tree into `dest` from the tree's own blobs, each checked against its
 * id. Not `git checkout-index`: that applies the working repository's attribute files,
 * `core.autocrlf` and filters, so the bytes would depend on a repository that may be crafted.
 * This is a write into the layer cache, which is outside the repo and the only place alz writes
 * outside it.
 */
export function writeTree(dest: string, files: TreeFile[], env: Env, top: string): void {
  const blobs = readBlobs(
    files.map((file) => file.oid),
    env,
    top,
  );
  mkdirSync(dest, { recursive: true });
  for (const file of files) {
    const target = join(dest, file.path);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, blobs.get(file.oid) ?? Buffer.alloc(0));
    // Explicit, so the mode does not depend on the umask.
    chmodSync(target, file.executable ? 0o755 : 0o644);
  }
}

/** The key of a path on a disk that may store or return another Unicode form of a name. */
function nfc(path: string): string {
  return path.normalize("NFC");
}

/**
 * Compares a cache entry with the tree it is named after, whatever the lock says. Returns the
 * first problem as a quoted path and a reason (never file content), or undefined when every file
 * of the tree is there with the same bytes and executable bit and nothing else is. Links are
 * never in an entry. Names the loader skips (`.DS_Store`, `.git`, `node_modules`) are ignored
 * unless the tree has them. Names are compared in Unicode NFC, because HFS+ returns NFD and
 * APFS keeps the form it was given; `listTree` has already refused names that would collide.
 */
export function verifyTreeEntry(dir: string, files: TreeFile[]): string | undefined {
  try {
    if (!lstatSync(dir).isDirectory()) return "the entry is not a directory";
  } catch {
    return "the entry cannot be read";
  }
  const expected = new Map(files.map((file) => [nfc(file.path), file]));
  const inTree = (rel: string) => [...expected.keys()].some((path) => path.startsWith(`${rel}/`));
  const seen = new Set<string>();
  const walk = (current: string, prefix: string): string | undefined => {
    for (const item of readdirSync(current, { withFileTypes: true })) {
      const rel = nfc(prefix ? `${prefix}/${item.name}` : item.name);
      const full = join(current, item.name);
      if (isSkippedName(item.name) && !expected.has(rel) && !inTree(rel)) continue;
      if (item.isSymbolicLink()) return `${quote(rel)} is a link`;
      if (item.isDirectory()) {
        const problem = walk(full, rel);
        if (problem) return problem;
        continue;
      }
      if (!item.isFile()) return `${quote(rel)} is not a regular file`;
      const file = expected.get(rel);
      if (!file) return `${quote(rel)} is not in the tree`;
      const bytes = readFileSync(full);
      if (blobId(bytes, algorithmOf(file.oid)) !== file.oid) {
        return `${quote(rel)} has different content`;
      }
      if (((lstatSync(full).mode & 0o111) !== 0) !== file.executable) {
        return `${quote(rel)} has a different executable bit`;
      }
      seen.add(rel);
    }
    return undefined;
  };
  const problem = walk(dir, "");
  if (problem) return problem;
  const missing = files.find((file) => !seen.has(nfc(file.path)));
  return missing ? `${quote(missing.path)} is missing` : undefined;
}
