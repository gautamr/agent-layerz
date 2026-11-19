import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { type Diagnostics, FatalError, InputError } from "../core/diagnostics";
import { isEnvFile, listFiles, sha256 } from "../core/files";
import { readYamlFile } from "../core/yaml";
import type { Layer } from "../model/layer";
import { type LayerMeta, parseLayerMeta, privacyRank, type Visibility } from "../model/layer-meta";
import type { Manifest } from "../model/manifest";
import { type ResolveContext, resolveSource } from "./sources";

/** Loads one layer directory: metadata, file list and content hash. */
export function loadLayer(dir: string, source: string, diags: Diagnostics): Layer {
  const metaPath = join(dir, "layer.yaml");
  if (!existsSync(metaPath)) {
    throw new FatalError("layer", `missing layer.yaml in layer '${source}'`);
  }
  let meta: LayerMeta;
  try {
    meta = parseLayerMeta(readYamlFile(metaPath, `${source}:layer.yaml`));
  } catch (error) {
    if (error instanceof InputError)
      throw new FatalError("layer", error.message, `${source}:layer.yaml`);
    throw error;
  }
  const files = listFiles(dir);
  for (const file of files.filter(isEnvFile)) {
    // Never read: secrets do not belong in a layer.
    diags.error(
      "env-file",
      "layers must not contain .env files; remove it",
      `${meta.name}:${file}`,
    );
  }
  const contentHash = sha256(
    files
      .filter((file) => !isEnvFile(file))
      .map((file) => `${file}\0${sha256(readFileSync(join(dir, file)))}\n`)
      .join(""),
  );
  return { meta, dir, source, files: files.filter((file) => !isEnvFile(file)), contentHash };
}

/** Resolves every stack entry and validates the stack (§6 steps 1–2). */
export function loadStack(manifest: Manifest, ctx: ResolveContext, diags: Diagnostics): Layer[] {
  const layers = manifest.stack.map((entry) => {
    const resolved = resolveSource(entry, ctx);
    const layer = loadLayer(resolved.dir, entry.source, diags);
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
