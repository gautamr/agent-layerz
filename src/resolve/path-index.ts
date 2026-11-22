import { randomBytes } from "node:crypto";
import { chmodSync, lstatSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { FatalError } from "../core/diagnostics";
import { insideRoot, readRegularFile } from "../core/files";
import { stripControl } from "../core/sanitize";
import { isObjectId, pathIndexName } from "./cache-paths";

/**
 * §7.13 'No fetch on resolve': the cache index of a pinned `path:` layer. Next to each `tree2@`
 * entry, an entry named by the layer directory's real path and the commit holds the tree id,
 * so account resolution finds the entry without git. Both sides of the format are here.
 */

export type PathIndexRead = { tree: string } | { miss: "index-missing" | "index-invalid" };

/**
 * Writes the entry through a temporary file in the same directory and a rename, so a reader
 * sees no file or the whole file. Does nothing when it is already right; replaces a wrong
 * file or a link (the rename never follows the link); refuses a directory with the cache error.
 * Never removes an entry. Any other failure to write (a read-only cache, a full disk) does not
 * stop the build: the index only speeds up account resolution, which then reports a miss and says
 * to run `alz build`. It is returned as a warning text; the only repository text in it is the
 * layer's source, with control characters removed.
 */
export function writePathIndex(
  cacheDir: string,
  realDir: string,
  sha: string,
  tree: string,
  source: string,
): string | undefined {
  const path = insideRoot(cacheDir, pathIndexName(realDir, sha));
  const refuse = (what: string) =>
    new FatalError(
      "cache",
      `the cache index entry of '${stripControl(source)}' ${what}: ${path}; delete it and rebuild`,
      source,
    );
  let kind: "none" | "file" | "directory" | "other" = "none";
  try {
    const found = lstatSync(path);
    kind = found.isFile() ? "file" : found.isDirectory() ? "directory" : "other";
  } catch {
    // no entry yet
  }
  if (kind === "directory") throw refuse("is a directory");
  if (kind === "file") {
    const found = readRegularFile(path, 64);
    if (Buffer.isBuffer(found) && found.toString("utf8") === tree) return undefined;
  }
  const temp = insideRoot(cacheDir, `.tmp-${randomBytes(6).toString("hex")}`);
  try {
    writeFileSync(temp, tree, { flag: "wx", mode: 0o644 });
    chmodSync(temp, 0o644);
    renameSync(temp, path);
    return undefined;
  } catch {
    rmSync(temp, { force: true });
    return `the cache index entry of '${stripControl(source)}' could not be written (${path}); account resolution will report a cache miss until it can be`;
  }
}

/**
 * Reads the entry for a layer directory and commit. It must be a regular file (never a link) of
 * exactly the length of the commit id, whose whole content is a lowercase object id. Anything
 * else is a miss; the reader never writes or repairs it.
 */
export function readPathIndex(cacheDir: string, realDir: string, sha: string): PathIndexRead {
  const path = insideRoot(cacheDir, pathIndexName(realDir, sha));
  const found = readRegularFile(path, 64);
  if (found === "missing") return { miss: "index-missing" };
  if (found === "invalid") return { miss: "index-invalid" };
  const tree = found.toString("utf8");
  if (!isObjectId(tree) || tree.length !== sha.length) return { miss: "index-invalid" };
  return { tree };
}
