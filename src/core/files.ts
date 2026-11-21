import { createHash } from "node:crypto";
import { readdirSync, statSync } from "node:fs";
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

/** Every file under `dir` as a POSIX relative path, sorted by code point for determinism. */
export function listFiles(dir: string): string[] {
  const files: string[] = [];
  const walk = (current: string, prefix: string) => {
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      if (SKIPPED.has(entry.name)) continue;
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
