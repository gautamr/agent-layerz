import type { Diagnostics } from "../core/diagnostics";
import { matchesAny } from "../core/glob";
import type { FileTree } from "./file-tree";

/** §4.5: repo-owned content — task boards, docs, specs and local files. Adapters add memory dirs. */
export const DEFAULT_PROTECTED = ["backlog/**", "docs/**", "specs/**", "*.local.*"];

/** Drops every protected path from the build output, with a warning naming what was skipped. */
export function dropProtected(
  tree: FileTree,
  patterns: readonly string[],
  diags: Diagnostics,
): void {
  for (const path of tree.paths()) {
    if (!matchesAny(path, patterns)) continue;
    tree.remove(path);
    diags.warn(
      "protected",
      "protected path; a layer ships it, but the build leaves it alone",
      path,
    );
  }
}
