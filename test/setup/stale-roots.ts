import { lstatSync, readdirSync, readFileSync, realpathSync } from "node:fs";
import { join } from "node:path";
import { removeTree } from "./remove-tree";

/** Every run root is named `alz-suite-<random>` and lies directly in the temp folder. */
export const ROOT_PREFIX = "alz-suite-";

/** Is a process with this pid running? A process we may not signal still exists (EPERM). */
export function processExists(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return (error as NodeJS.ErrnoException).code === "EPERM";
  }
}

/** The pid a root's `pid` file names, or undefined when the file is missing or is not a pid. */
function recordedPid(root: string): number | undefined {
  try {
    const text = readFileSync(join(root, "pid"), "utf8").trim();
    return /^[1-9][0-9]*$/.test(text) ? Number(text) : undefined;
  } catch {
    return undefined;
  }
}

/**
 * A leftover of a run that was killed before its teardown. All of these hold: the entry is a real
 * directory (not a link), the current user owns it, its real path lies directly inside the temp
 * folder, and its `pid` file names a process that no longer exists. Anything else is left alone.
 */
export function isStaleRoot(
  temp: string,
  name: string,
  uid: number,
  alive: (pid: number) => boolean,
): boolean {
  if (!name.startsWith(ROOT_PREFIX)) return false;
  const path = join(temp, name);
  try {
    const stat = lstatSync(path);
    if (!stat.isDirectory() || stat.isSymbolicLink() || stat.uid !== uid) return false;
    if (realpathSync(path) !== join(realpathSync(temp), name)) return false;
  } catch {
    return false;
  }
  const pid = recordedPid(path);
  return pid !== undefined && !alive(pid);
}

/** Removes the stale roots in `temp`, and nothing else. Returns the names it removed. */
export function removeStaleRoots(
  temp: string,
  uid: number = process.getuid?.() ?? -1,
  alive: (pid: number) => boolean = processExists,
): string[] {
  const stale = readdirSync(temp).filter((name) => isStaleRoot(temp, name, uid, alive));
  for (const name of stale) removeTree(join(temp, name));
  return stale;
}
