import { lstatSync, readdirSync, readFileSync, realpathSync } from "node:fs";
import { join } from "node:path";
import { removeTree } from "./remove-tree";

/** Every run root is named `alz-suite-<random>` and lies directly in the temp folder. */
export const ROOT_PREFIX = "alz-suite-";

/**
 * Is a process with this pid running? Only ESRCH says it is gone: EPERM (a process we may not
 * signal) and any other error leave it alive, so a root is never swept on a doubt.
 */
export function processExists(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return (error as NodeJS.ErrnoException).code !== "ESRCH";
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

const reportOnStderr = (message: string): void => {
  process.stderr.write(`${message}\n`);
};

/**
 * Removes the stale roots in `temp`, and nothing else. A root that cannot be removed (another
 * user's mode 000 folder, a busy file system) is reported through `report`, by default on stderr,
 * and the sweep goes on: leftovers of an old run never stop a new one. Returns the names it
 * removed. The loop performs effects, one removal per root, so it is a loop.
 */
export function removeStaleRoots(
  temp: string,
  uid: number = process.getuid?.() ?? -1,
  alive: (pid: number) => boolean = processExists,
  remove: (path: string) => void = removeTree,
  report: (message: string) => void = reportOnStderr,
): string[] {
  const removed: string[] = [];
  for (const name of readdirSync(temp).filter((entry) => isStaleRoot(temp, entry, uid, alive))) {
    try {
      remove(join(temp, name));
      removed.push(name);
    } catch (error) {
      const why = error instanceof Error ? error.message : String(error);
      report(`alz test setup: could not remove the stale run root ${join(temp, name)}: ${why}`);
    }
  }
  return removed;
}
