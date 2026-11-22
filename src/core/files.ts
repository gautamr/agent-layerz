import { createHash } from "node:crypto";
import {
  closeSync,
  constants,
  fstatSync,
  lstatSync,
  openSync,
  readdirSync,
  readSync,
  statSync,
} from "node:fs";
import { basename, isAbsolute, join, relative, resolve, sep } from "node:path";
import { FatalError } from "./diagnostics";

/** A POSIX path that stays inside its base: relative, no `..` or `.` segments, no backslashes. */
export function isSafeRelativePath(path: string): boolean {
  if (path === "" || isAbsolute(path) || /^[A-Za-z]:/.test(path) || path.includes("\\"))
    return false;
  return path.split("/").every((segment) => segment !== "" && segment !== "." && segment !== "..");
}

/** `root/path`, refusing any path that would resolve outside `root`. Guards every write and delete. */
export function insideRoot(root: string, path: string): string {
  const abs = resolve(root, path);
  const rel = relative(resolve(root), abs);
  if (rel === "" || rel.startsWith("..") || isAbsolute(rel)) {
    throw new FatalError("unsafe-path", `refusing to touch '${path}': it is outside ${root}`);
  }
  return abs;
}

/** True when `path` is `root` or below it, compared lexically on already-resolved paths. */
export function isWithin(root: string, path: string): boolean {
  const rel = relative(root, path);
  return rel === "" || (rel !== ".." && !rel.startsWith(`..${sep}`) && !isAbsolute(rel));
}

const SKIPPED = new Set([".git", ".DS_Store", "node_modules"]);

/** Names `listFiles` never lists, so a layer never provides them. */
export function isSkippedName(name: string): boolean {
  return SKIPPED.has(name);
}

/**
 * Every file under `dir` as a POSIX relative path, sorted by code point for determinism. With
 * `maxEntries`, it throws once more than that many files and folders have been seen, so a
 * caller that reads an untrusted layer is not made to walk an unbounded tree.
 */
export function listFiles(dir: string, maxEntries?: number): string[] {
  const files: string[] = [];
  let seen = 0;
  const walk = (current: string, prefix: string) => {
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      if (SKIPPED.has(entry.name)) continue;
      seen += 1;
      if (maxEntries !== undefined && seen > maxEntries) throw new Error("too many entries");
      const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (entry.isDirectory()) walk(join(current, entry.name), rel);
      else if (entry.isFile()) files.push(rel);
    }
  };
  walk(dir, "");
  return files.sort(compareCodePoints);
}

export function compareCodePoints(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

export function sha256(data: string | Uint8Array): string {
  return createHash("sha256").update(data).digest("hex");
}

export function isExecutable(path: string): boolean {
  return (statSync(path).mode & 0o111) !== 0;
}

/** `.env`, `.env.dev`, … — never read, only reported. */
export function isEnvFile(path: string): boolean {
  return basename(path).startsWith(".env");
}

/** True when the buffer looks binary (contains a NUL byte in the first 8 KiB). */
export function isBinary(data: Uint8Array): boolean {
  return data.subarray(0, 8192).includes(0);
}

/** A regular file read in full, with the identity of the file that was opened. */
export interface RegularFile {
  bytes: Buffer;
  dev: number;
  ino: number;
  /** The owner and permission bits of the open file, from the same `fstat` as the checks. */
  uid: number;
  mode: number;
}

/**
 * Reads a regular file of at most `maxBytes` that is not a link. `lstat` must show a file, the
 * open refuses a link (`O_NOFOLLOW`), and the open file must be the same regular file, so a link
 * swapped in between is not followed. `missing` is no such file; anything else wrong, such as a
 * link, a folder, a larger file or an unreadable one, is `invalid`. Never throws.
 */
export function readRegularFileWithId(
  path: string,
  maxBytes: number,
): RegularFile | "missing" | "invalid" {
  let before: ReturnType<typeof lstatSync>;
  try {
    before = lstatSync(path);
  } catch (error) {
    const code = (error as { code?: string }).code;
    return code === "ENOENT" || code === "ENOTDIR" ? "missing" : "invalid";
  }
  if (!before.isFile() || before.size > maxBytes) return "invalid";
  let fd: number;
  try {
    // O_NONBLOCK: a FIFO swapped in after the `lstat` must not make the open wait for a writer;
    // the `isFile` check on the open file then rejects it.
    fd = openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
  } catch {
    return "invalid";
  }
  try {
    const open = fstatSync(fd);
    if (!open.isFile() || open.size > maxBytes) return "invalid";
    if (open.ino !== before.ino || open.dev !== before.dev) return "invalid";
    const bytes = Buffer.alloc(open.size);
    let at = 0;
    while (at < bytes.length) {
      const count = readSync(fd, bytes, at, bytes.length - at, at);
      if (count === 0) break;
      at += count;
    }
    return at === bytes.length
      ? { bytes, dev: open.dev, ino: open.ino, uid: open.uid, mode: open.mode }
      : "invalid";
  } catch {
    return "invalid";
  } finally {
    closeSync(fd);
  }
}

/** Like `readRegularFileWithId`, for callers that only need the bytes. */
export function readRegularFile(path: string, maxBytes: number): Buffer | "missing" | "invalid" {
  const found = readRegularFileWithId(path, maxBytes);
  return typeof found === "string" ? found : found.bytes;
}
