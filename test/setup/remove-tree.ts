import { chmodSync, lstatSync, readdirSync, rmSync } from "node:fs";
import { join } from "node:path";

/** Gives the owner access to every folder under `path`, so a case that left a mode 000 folder can be removed. */
function reopen(path: string): void {
  try {
    if (!lstatSync(path).isDirectory()) return;
    chmodSync(path, 0o700);
    for (const name of readdirSync(path)) reopen(join(path, name));
  } catch {
    // Already gone, or not ours to open: the removal below reports what still stands in the way.
  }
}

/**
 * Removes a folder and everything in it, links included and never followed. Throws when a second
 * try after `reopen` still cannot remove it; a caller that must go on catches per folder.
 */
export function removeTree(path: string): void {
  try {
    rmSync(path, { recursive: true, force: true });
  } catch {
    reopen(path);
    rmSync(path, { recursive: true, force: true });
  }
}
