import {
  chmodSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { insideRoot } from "../core/files";
import { LOCK_FILE } from "../model/lock";
import type { FileTree } from "./file-tree";
import type { WorkspacePlan } from "./workspace";

/** Writes only generated paths, removes stale ones, then writes the lock. */
export function applyPlan(repoRoot: string, tree: FileTree, plan: WorkspacePlan): void {
  for (const change of plan.changes) {
    const abs = insideRoot(repoRoot, change.path);
    if (change.kind === "delete") {
      rmSync(abs, { force: true });
      pruneEmptyDirs(dirname(abs), repoRoot);
      continue;
    }
    const file = tree.get(change.path);
    if (!file) continue;
    mkdirSync(dirname(abs), { recursive: true });
    writeFileSync(abs, file.content);
    chmodSync(abs, file.executable ? 0o755 : 0o644);
  }
  if (plan.lockChanged) writeFileSync(join(repoRoot, LOCK_FILE), plan.lockText);
}

export function pruneEmptyDirs(dir: string, stopAt: string): void {
  let current = dir;
  while (current.startsWith(stopAt) && current !== stopAt && existsSync(current)) {
    if (readdirSync(current).length > 0) return;
    rmdirSync(current);
    current = dirname(current);
  }
}

const BLOCK_START = "# >>> agent-layerz generated (do not edit this block)";
const BLOCK_END = "# <<< agent-layerz generated";
/** The managed block, also as written before the rename to agent-layerz. */
const BLOCK = /# >>> agent-layer[sz] generated[^\n]*\n[\s\S]*?# <<< agent-layer[sz] generated\n?/;

/** §8.3: with `commit_generated: false`, generated paths go into a managed `.gitignore` block. */
export function syncGitignore(repoRoot: string, paths: string[] | undefined): void {
  const path = join(repoRoot, ".gitignore");
  const current = existsSync(path) ? readFileSync(path, "utf8") : "";
  const without = current.replace(BLOCK, "");
  const block = paths
    ? `${BLOCK_START}\n${paths.map((p) => `/${p}`).join("\n")}\n${BLOCK_END}\n`
    : "";
  const separator = without && !without.endsWith("\n") ? "\n" : "";
  const next = block ? `${without}${separator}${block}` : without;
  if (next !== current) writeFileSync(path, next);
}
