import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { sha256Hex } from "./cache";

/**
 * §5.3 "What the content hash covers": the path and content of every regular file of the layer,
 * in code point order (§6 "Name order"), except `.env` files (never read), `.DS_Store`, and
 * anything under `.git` or `node_modules`. Symbolic links and other non-regular entries are not
 * listed; the file mode is not part of the hash.
 *
 * §5.3 "Byte layout": for each file in that order, `<path>`, a NUL, the lowercase sha256 hex of
 * the file's bytes and a line feed, joined and hashed as one UTF-8 text; the lock stores it as
 * `sha256:<hex>`. §5.3 "`.env` files": a file whose own name starts with `.env`.
 *
 * §5.3 "File names": a layer with a name that is not valid UTF-8 is invalid and has no content
 * hash. These helpers read names as strings, so they are for layers whose names are valid; the
 * case of an invalid name is TASK-69's.
 */

const SKIPPED_NAMES: ReadonlySet<string> = new Set([".git", ".DS_Store", "node_modules"]);

const isEnvFile = (path: string): boolean => (path.split("/").pop() ?? "").startsWith(".env");

/** Orders by Unicode code point, not by UTF-16 unit (§5.3 "code point order"). */
const compareCodePoints = (a: string, b: string): number => {
  const left = Array.from(a, (char) => char.codePointAt(0) ?? 0);
  const right = Array.from(b, (char) => char.codePointAt(0) ?? 0);
  const firstDifference = left.findIndex((point, index) => point !== right[index]);
  if (firstDifference === -1) return left.length - right.length;
  return (left[firstDifference] ?? -1) - (right[firstDifference] ?? -1);
};

/** The regular files below `dir` as `/`-separated relative paths, without the skipped names. */
const regularFiles = (dir: string, prefix = ""): string[] =>
  readdirSync(dir, { withFileTypes: true })
    .filter((entry) => !SKIPPED_NAMES.has(entry.name))
    .flatMap((entry) => {
      const rel = prefix === "" ? entry.name : `${prefix}/${entry.name}`;
      if (entry.isDirectory()) return regularFiles(join(dir, entry.name), rel);
      return entry.isFile() ? [rel] : [];
    });

/** The files of a layer the hash covers, in code point order. */
export const hashedFiles = (dir: string): string[] =>
  regularFiles(dir)
    .filter((path) => !isEnvFile(path))
    .sort(compareCodePoints);

/** The layer's content hash, in hex, without the `sha256:` prefix the lock adds. */
export const contentHash = (dir: string): string =>
  sha256Hex(
    hashedFiles(dir)
      .map((path) => `${path}\0${sha256Hex(readFileSync(join(dir, path)))}\n`)
      .join(""),
  );
