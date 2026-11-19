import { readFileSync } from "node:fs";
import { join } from "node:path";
import { type Diagnostics, InputError } from "../core/diagnostics";
import { type MarkdownDoc, parseMarkdown } from "../core/frontmatter";
import { parseYaml } from "../core/yaml";
import { type Layer, where } from "../model/layer";

export function readLayerText(layer: Layer, path: string): string {
  return readFileSync(join(layer.dir, path), "utf8");
}

/** Parses a Markdown file of a layer; reports and returns undefined on a syntax error. */
export function readLayerMarkdown(
  layer: Layer,
  path: string,
  diags: Diagnostics,
): MarkdownDoc | undefined {
  try {
    return parseMarkdown(readLayerText(layer, path));
  } catch (error) {
    if (!(error instanceof InputError)) throw error;
    diags.error("frontmatter", error.message, where(layer, path));
    return undefined;
  }
}

/** Parses a YAML file of a layer if present; reports and returns undefined on error. */
export function readLayerYaml(layer: Layer, path: string, diags: Diagnostics): unknown {
  if (!layer.files.includes(path)) return undefined;
  try {
    return parseYaml(readLayerText(layer, path));
  } catch (error) {
    if (!(error instanceof InputError)) throw error;
    diags.error("yaml", error.message, where(layer, path));
    return undefined;
  }
}

/** Files directly inside `dir/` (no subdirectories) with the given suffix. */
export function filesIn(layer: Layer, dir: string, suffix: string): string[] {
  return layer.files.filter((file) => {
    if (!file.startsWith(`${dir}/`) || !file.endsWith(suffix)) return false;
    return !file.slice(dir.length + 1).includes("/");
  });
}

export function stem(path: string, suffix: string): string {
  const name = path.slice(path.lastIndexOf("/") + 1);
  return name.slice(0, name.length - suffix.length);
}
