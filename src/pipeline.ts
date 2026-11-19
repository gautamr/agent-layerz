import { findAdapter } from "./adapters/registry";
import type { Adapter } from "./adapters/types";
import { scanPublicLayer } from "./checks/leak-guard";
import { Diagnostics } from "./core/diagnostics";
import { listFiles } from "./core/files";
import { resolveConfig } from "./merge/resolve-config";
import type { ResolvedConfig } from "./model/config";
import type { Layer } from "./model/layer";
import { type Lock, readLock } from "./model/lock";
import { type Manifest, readManifest } from "./model/manifest";
import { AGENTS_MD, checkInstructionBudgets, renderAgentsMd } from "./render/agents-md";
import { FileTree } from "./render/file-tree";
import { DEFAULT_PROTECTED, dropProtected } from "./render/protected";
import type { Env } from "./resolve/git";
import { loadStack, stackVisibility } from "./resolve/stack";

export interface BuildContext {
  repoRoot: string;
  home: string;
  env: Env;
}

export interface BuildResult {
  manifest: Manifest;
  layers: Layer[];
  config: ResolvedConfig;
  adapters: Adapter[];
  tree: FileTree;
  /** Directories the adapters own (collision checks). */
  managedDirs: string[];
  /** Globs the build never writes or deletes (§4.5). */
  protectedPaths: string[];
  lock: Lock;
  previousLock?: Lock;
  diags: Diagnostics;
}

/** §6: resolve, merge, check and render into memory. Writes nothing. */
export function computeBuild(ctx: BuildContext): BuildResult {
  const diags = new Diagnostics();
  const manifest = readManifest(ctx.repoRoot);
  const previousLock = readLock(ctx.repoRoot);
  const layers = loadStack(
    manifest,
    { repoRoot: ctx.repoRoot, home: ctx.home, env: ctx.env, locked: previousLock?.layers ?? [] },
    diags,
  );

  const adapters: Adapter[] = [];
  for (const target of manifest.targets) {
    const found = findAdapter(target);
    if ("missing" in found) diags.error("target", found.missing, "layer.yaml");
    else adapters.push(found);
  }
  for (const layer of layers) scanPublicLayer(layer, diags);

  const config = resolveConfig(manifest, layers, diags);
  const tree = new FileTree();
  const agentsMd = renderAgentsMd(config.instructions);
  checkInstructionBudgets(agentsMd, manifest, diags);
  tree.add(AGENTS_MD, agentsMd);
  for (const adapter of adapters) adapter.render(config, tree, diags);
  const protectedPaths = [
    ...DEFAULT_PROTECTED,
    ...adapters.flatMap((adapter) => adapter.protectedPaths),
    ...manifest.protectedPaths,
  ];
  dropProtected(tree, protectedPaths, diags);
  checkShadowingFiles(ctx.repoRoot, diags);

  const lock: Lock = {
    lockVersion: 1,
    visibility: stackVisibility(layers),
    layers: layers.map((layer) => ({
      name: layer.meta.name,
      source: layer.source,
      rev: layer.rev,
      sha: layer.sha,
      contentHash: `sha256:${layer.contentHash}`,
    })),
    files: tree.hashes(),
  };
  const managedDirs = adapters.flatMap((adapter) => adapter.managedDirs);
  return {
    manifest,
    layers,
    config,
    adapters,
    tree,
    managedDirs,
    protectedPaths,
    lock,
    previousLock,
    diags,
  };
}

/** target-facts §8.4: Codex reads AGENTS.override.md instead of AGENTS.md in its directory. */
function checkShadowingFiles(repoRoot: string, diags: Diagnostics): void {
  for (const file of listFiles(repoRoot)) {
    if (file === "AGENTS.override.md" || file.endsWith("/AGENTS.override.md")) {
      diags.error(
        "collision",
        "AGENTS.override.md silently shadows the generated AGENTS.md; remove it",
        file,
      );
    }
  }
}
