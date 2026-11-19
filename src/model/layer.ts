import type { LayerMeta } from "./layer-meta";

/** A layer resolved to a local directory, with its file list. */
export interface Layer {
  meta: LayerMeta;
  /** Absolute directory of the layer sources. */
  dir: string;
  /** The manifest `source` string, as written. */
  source: string;
  /** Pinned revision: git commit (short) or `ref`; undefined for unpinned path layers. */
  rev?: string;
  /** Full commit SHA for git sources. */
  sha?: string;
  /** Files relative to `dir`, sorted. */
  files: string[];
  contentHash: string;
}

/** Label used in generated-file headers: `<name>@<rev or version>`. */
export function layerLabel(layer: Layer): string {
  const rev = layer.rev ?? layer.meta.version;
  return rev ? `${layer.meta.name}@${rev}` : layer.meta.name;
}

/** `personal@1.0.0:agents/architect.md` */
export function sourceRef(layer: Layer, path: string): string {
  return `${layerLabel(layer)}:${path}`;
}

/** Location used in diagnostics: `personal:agents/architect.md`. */
export function where(layer: Layer, path: string): string {
  return `${layer.meta.name}:${path}`;
}

/** `## Personal: personal` style title part: kind with a capital first letter. */
export function kindTitle(layer: Layer): string {
  const kind = layer.meta.kind;
  return kind.charAt(0).toUpperCase() + kind.slice(1);
}
