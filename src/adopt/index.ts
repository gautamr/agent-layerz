import { existsSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import type { BuildResult } from "../pipeline";
import { AGENTS_MD } from "../render/agents-md";
import { hasGeneratedMark } from "../render/workspace";
import { AdoptionPlan } from "./plan";

export { applyAdoption } from "./apply";
export { AdoptionPlan } from "./plan";

/**
 * §4.3 `--adopt`: plans how the repo's hand-written agent config becomes repo-layer sources.
 * Takes every collision with the build, plus each adapter's always-adopted files (settings,
 * MCP config) that the tool did not generate.
 */
export function planAdoption(
  repoRoot: string,
  build: BuildResult,
  collisions: string[],
): AdoptionPlan | string {
  const layerDir = findRepoLayer(repoRoot, build);
  const repoLayer = build.layers.at(-1);
  if (!layerDir || !repoLayer) {
    return "--adopt needs a repo layer inside the repo as the last stack entry (e.g. path:./.layer)";
  }
  const plan = new AdoptionPlan(repoRoot, layerDir, repoLayer.meta.name, build.config);

  const owned = new Set(Object.keys(build.previousLock?.files ?? {}));
  const candidates = new Set(collisions);
  for (const path of build.adapters.flatMap((adapter) => adapter.adoptAlways ?? [])) {
    const abs = join(repoRoot, path);
    if (existsSync(abs) && !owned.has(path) && !hasGeneratedMark(abs)) candidates.add(path);
  }

  let remaining = [...candidates].sort();
  if (remaining.includes(AGENTS_MD)) {
    plan.move(AGENTS_MD, "instructions/00-adopted.md");
    remaining = remaining.filter((path) => path !== AGENTS_MD);
  }
  for (const adapter of build.adapters) {
    if (!adapter.adopt) continue;
    const handled = new Set(adapter.adopt(remaining, plan));
    remaining = remaining.filter((path) => !handled.has(path));
  }
  for (const path of remaining) {
    plan.problems.push(`${path}: cannot be adopted automatically; move it into a layer by hand`);
  }
  return plan;
}

function findRepoLayer(repoRoot: string, build: BuildResult): string | undefined {
  const last = build.manifest.stack.at(-1)?.source;
  if (!last?.startsWith("path:")) return undefined;
  const rel = relative(repoRoot, resolve(repoRoot, last.slice(5)));
  if (rel === "" || rel.startsWith("..")) return undefined;
  return rel.split("\\").join("/");
}
