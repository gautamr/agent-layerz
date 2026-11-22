import { lstatSync } from "node:fs";
import { join } from "node:path";
import { type Diagnostics, FatalError, InputError } from "../core/diagnostics";
import { stripControl } from "../core/sanitize";
import { readYamlFile } from "../core/yaml";
import type { Layer } from "../model/layer";
import { type LayerMeta, parseLayerMeta, privacyRank, type Visibility } from "../model/layer-meta";
import { LOCK_FILE } from "../model/lock";
import type { Manifest } from "../model/manifest";
import { lockedEntryFor, verifyCachedLayer } from "./cache-integrity";
import { hashLayerDir } from "./layer-hash";
import { type ResolveContext, resolveSource } from "./sources";

/**
 * Loads one layer directory: metadata, file list and content hash. `verify` runs on the content
 * hash before anything is parsed, so an edited cache entry is refused without its `layer.yaml`
 * being read. `layer.yaml` must be a regular file, never a link to somewhere else.
 */
export function loadLayer(
  dir: string,
  source: string,
  diags: Diagnostics,
  verify?: (contentHash: string) => void,
): Layer {
  const metaPath = join(dir, "layer.yaml");
  let kind: "missing" | "file" | "other";
  try {
    kind = lstatSync(metaPath).isFile() ? "file" : "other";
  } catch {
    kind = "missing";
  }
  const missing = () =>
    new FatalError("layer", `missing layer.yaml in layer '${stripControl(source)}'`);
  // Without a cache check there is nothing to hash first (the directory may not exist).
  if (kind === "missing" && !verify) throw missing();
  const { files, envFiles, contentHash } = hashLayerDir(dir);
  // Before anything about layer.yaml is judged: a tampered cache entry gets the cache error
  // whether its layer.yaml was edited, replaced or deleted.
  verify?.(contentHash);
  if (kind === "missing") throw missing();
  if (kind === "other" || !files.includes("layer.yaml")) {
    throw new FatalError(
      "layer",
      `layer.yaml in layer '${stripControl(source)}' must be a regular file, not a link or a folder`,
    );
  }
  let meta: LayerMeta;
  try {
    meta = parseLayerMeta(readYamlFile(metaPath, `${source}:layer.yaml`));
  } catch (error) {
    if (error instanceof InputError)
      throw new FatalError("layer", error.message, `${source}:layer.yaml`);
    throw error;
  }
  for (const file of envFiles) {
    // Never read: secrets do not belong in a layer.
    diags.error(
      "env-file",
      "layers must not contain .env files; remove it",
      `${meta.name}:${file}`,
    );
  }
  return { meta, dir, source, files, contentHash };
}

/**
 * §5.3: what to run on the content hash of a layer read from the cache. It compares with the
 * lock's entry for that source and commit, and does nothing when there is none (build and the
 * other commands that rebuild treat "no entry" as "new content": see `lockedEntryFor`).
 */
function cacheCheck(
  source: string,
  dir: string,
  sha: string | undefined,
  ctx: ResolveContext,
): (contentHash: string) => void {
  return (contentHash) => {
    const entry = lockedEntryFor(ctx.locked, source, sha);
    if (!entry || verifyCachedLayer({ contentHash }, entry) === "ok") return;
    // Named by the lock and the source: the layer's own files are not trusted, or read, yet.
    throw new FatalError(
      "cache",
      `cached layer '${stripControl(entry.name)}' (${stripControl(source)}) no longer matches ${LOCK_FILE}: its files in ${dir} changed after they were cached; delete that directory and rebuild`,
      source,
    );
  };
}

/** Resolves every stack entry and validates the stack (§6 steps 1–2). */
export function loadStack(manifest: Manifest, ctx: ResolveContext, diags: Diagnostics): Layer[] {
  const layers = manifest.stack.map((entry) => {
    const resolved = resolveSource(entry, ctx);
    const verify = resolved.cached
      ? cacheCheck(entry.source, resolved.dir, resolved.sha, ctx)
      : undefined;
    const layer = loadLayer(resolved.dir, entry.source, diags, verify);
    return { ...layer, rev: resolved.rev, sha: resolved.sha };
  });
  validateStack(layers, diags);
  return layers;
}

export function validateStack(layers: Layer[], diags: Diagnostics): void {
  const seen = new Map<string, Layer>();
  for (const layer of layers) {
    const previous = seen.get(layer.meta.name);
    if (previous) {
      diags.error(
        "duplicate-layer",
        `layer name '${layer.meta.name}' is used by both '${previous.source}' and '${layer.source}'`,
      );
    }
    seen.set(layer.meta.name, layer);
  }
  // A layer extends every layer before it, so privacy may only grow along the stack.
  layers.forEach((layer, index) => {
    for (const general of layers.slice(0, index)) {
      if (privacyRank(layer.meta.visibility) < privacyRank(general.meta.visibility)) {
        diags.error(
          "visibility",
          `${layer.meta.visibility} layer '${layer.meta.name}' cannot extend ${general.meta.visibility} layer '${general.meta.name}'; order the stack from most public to most private`,
          layer.source,
        );
      }
    }
  });
}

/** The output inherits the most private visibility in the stack. */
export function stackVisibility(layers: Layer[]): Visibility {
  return layers.reduce<Visibility>(
    (most, layer) =>
      privacyRank(layer.meta.visibility) > privacyRank(most) ? layer.meta.visibility : most,
    "public",
  );
}
