import type { Diagnostics } from "../core/diagnostics";
import type { InstructionBlock } from "../merge/instructions";
import { kindTitle, sourceRef } from "../model/layer";
import type { Manifest } from "../model/manifest";
import { htmlHeader } from "./header";

export const AGENTS_MD = "AGENTS.md";

/** §7.1: one `## <Kind>: <name>` section per layer, blocks in stack then filename order. */
export function renderAgentsMd(blocks: InstructionBlock[]): string {
  const sources = blocks.map((block) => sourceRef(block.layer, block.path));
  const parts = [htmlHeader(sources.length > 0 ? sources : ["no instructions"])];
  let current: InstructionBlock["layer"] | undefined;
  for (const block of blocks) {
    if (block.layer !== current) {
      parts.push(`## ${kindTitle(block.layer)}: ${block.layer.meta.name}`);
      current = block.layer;
    }
    parts.push(block.body);
  }
  return `${parts.join("\n\n")}\n`;
}

export function checkInstructionBudgets(
  text: string,
  manifest: Manifest,
  diags: Diagnostics,
): void {
  const lines = text.split("\n").length - 1;
  const bytes = Buffer.byteLength(text);
  const { instructionsMaxLines, instructionsMaxBytes } = manifest.budgets;
  if (instructionsMaxLines !== undefined && lines > instructionsMaxLines) {
    diags.error(
      "budget",
      `AGENTS.md has ${lines} lines; instructions_max_lines is ${instructionsMaxLines}. Move long standards into skills`,
    );
  }
  if (bytes > instructionsMaxBytes) {
    diags.error(
      "budget",
      `AGENTS.md is ${bytes} bytes; instructions_max_bytes is ${instructionsMaxBytes}`,
    );
  }
}
