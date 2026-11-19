import { type Diagnostics, UNRESOLVED } from "../core/diagnostics";
import { unknownKeys } from "../core/yaml";
import { type Layer, where } from "../model/layer";
import { filesIn, readLayerMarkdown } from "./layer-files";

export interface InstructionBlock {
  layer: Layer;
  path: string;
  /** Filename, the identity used by `mode: replace | remove`. */
  file: string;
  body: string;
}

/** §7.1: concatenate in stack order, then filename; later layers may replace or remove a block. */
export function mergeInstructions(layers: Layer[], diags: Diagnostics): InstructionBlock[] {
  let blocks: InstructionBlock[] = [];
  for (const layer of layers) {
    for (const path of filesIn(layer, "instructions", ".md")) {
      const doc = readLayerMarkdown(layer, path, diags);
      if (!doc) continue;
      const loc = where(layer, path);
      const extra = unknownKeys(doc.data, ["mode"]);
      if (extra.length > 0)
        diags.error("instructions", `unknown field(s): ${extra.join(", ")}`, loc);
      const file = path.slice("instructions/".length);
      const mode = doc.data.mode ?? "append";
      if (mode !== "append" && mode !== "replace" && mode !== "remove") {
        diags.error("instructions", "'mode' must be replace or remove", loc);
        continue;
      }
      if (mode !== "append") {
        const before = blocks.length;
        blocks = blocks.filter((block) => block.file !== file);
        if (blocks.length === before) {
          diags.error(
            UNRESOLVED,
            `mode: ${mode}, but no more general layer has instructions/${file}`,
            loc,
          );
        }
        if (mode === "remove") continue;
      }
      blocks.push({ layer, path, file, body: doc.body.trim() });
    }
  }
  return blocks.filter((block) => block.body.length > 0);
}
