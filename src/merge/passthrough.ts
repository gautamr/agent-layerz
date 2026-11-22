import type { Diagnostics } from "../core/diagnostics";
import { type Layer, sourceRef, where } from "../model/layer";
import { readLayerMarkdown } from "./layer-files";
import { checkPassthroughFile } from "./protected-variables";

export interface PassthroughFile {
  layer: Layer;
  /** Path inside the layer, e.g. `targets/claude/.claude/output-styles/x.md`. */
  source: string;
}

export interface ClaudeExtra {
  ref: string;
  body: string;
}

const CLAUDE_EXTRA = "targets/claude/CLAUDE.extra.md";

/** §7.10: raw files per target; the most specific layer wins per path. */
export function mergePassthrough(
  layers: Layer[],
  diags: Diagnostics,
): Map<string, Map<string, PassthroughFile>> {
  const byTarget = new Map<string, Map<string, PassthroughFile>>();
  for (const layer of layers) {
    for (const file of layer.files) {
      const match = /^targets\/([^/]+)\/(.+)$/.exec(file);
      if (!match || file === CLAUDE_EXTRA) continue;
      const [, target = "", path = ""] = match;
      checkPassthroughFile(layer, file, path, diags);
      const files = byTarget.get(target) ?? new Map<string, PassthroughFile>();
      files.set(path, { layer, source: file });
      byTarget.set(target, files);
      diags.warn(
        "passthrough",
        `raw passthrough to ${path}; consider turning it into a first-class item`,
        where(layer, file),
      );
    }
  }
  return byTarget;
}

/** `targets/claude/CLAUDE.extra.md` from each layer, in stack order (§7.1). */
export function claudeExtras(layers: Layer[], diags: Diagnostics): ClaudeExtra[] {
  const extras: ClaudeExtra[] = [];
  for (const layer of layers) {
    if (!layer.files.includes(CLAUDE_EXTRA)) continue;
    const doc = readLayerMarkdown(layer, CLAUDE_EXTRA, diags);
    const body = doc?.body.trim();
    if (body) extras.push({ ref: sourceRef(layer, CLAUDE_EXTRA), body });
  }
  return extras;
}
