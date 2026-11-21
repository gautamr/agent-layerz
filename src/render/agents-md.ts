import type { Diagnostics } from "../core/diagnostics";
import type { DeclaredIdentity } from "../merge/commit-identity";
import type { InstructionBlock } from "../merge/instructions";
import { kindTitle, sourceRef } from "../model/layer";
import type { Manifest } from "../model/manifest";
import { htmlHeader } from "./header";

export const AGENTS_MD = "AGENTS.md";

/** §7.12: the generated Commits section, last in AGENTS.md. Changing it changes every build. */
export function commitsSection({ identity, layer }: DeclaredIdentity): string {
  return [
    "## Commits",
    "",
    `- Author and committer of every commit made in a local clone: \`${identity.name} <${identity.email}>\` (declared by layer \`${layer.meta.name}\`). This repository's local git config sets it; never change \`user.name\` or \`user.email\`, and never commit locally under a CI, bot or shared identity.`,
    "- A commit made by an agent ends with a `Co-Authored-By:` trailer that names the agent. Use the trailer your tool adds; if it adds none, write one.",
    "- If `git var GIT_AUTHOR_IDENT` shows another identity, do not commit; report it.",
    "- Bots that commit on the hosting service, such as release or CI bots, are exempt from this rule.",
  ].join("\n");
}

/**
 * §7.1: one `## <Kind>: <name>` section per layer, blocks in stack then filename order, then
 * the §7.12 Commits section when a layer declares a commit identity.
 */
export function renderAgentsMd(blocks: InstructionBlock[], commits?: DeclaredIdentity): string {
  const sources = blocks.map((block) => sourceRef(block.layer, block.path));
  if (commits) sources.push(sourceRef(commits.layer, "layer.yaml"));
  const parts = [htmlHeader(sources.length > 0 ? sources : ["no instructions"])];
  let current: InstructionBlock["layer"] | undefined;
  for (const block of blocks) {
    if (block.layer !== current) {
      parts.push(`## ${kindTitle(block.layer)}: ${block.layer.meta.name}`);
      current = block.layer;
    }
    parts.push(block.body);
  }
  if (commits) parts.push(commitsSection(commits));
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
