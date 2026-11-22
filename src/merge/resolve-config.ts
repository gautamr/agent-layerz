import type { Diagnostics } from "../core/diagnostics";
import type { ResolvedConfig } from "../model/config";
import type { Layer } from "../model/layer";
import type { Manifest } from "../model/manifest";
import { mergeAccount } from "./account";
import { mergeAgents } from "./agents";
import { mergeCommitIdentity } from "./commit-identity";
import { mergeHooks } from "./hooks";
import { mergeInstructions } from "./instructions";
import { mergeMcp } from "./mcp";
import { mergeModels } from "./models";
import { claudeExtras, mergePassthrough } from "./passthrough";
import { mergePermissions } from "./permissions";
import { mergeRules } from "./rules";
import { mergeSettings } from "./settings";
import { fillSlots, mergeSkills } from "./skills";

/** §6 steps 3–5: collect every item type in stack order and apply its merge rule. */
export function resolveConfig(
  manifest: Manifest,
  layers: Layer[],
  repoRoot: string,
  diags: Diagnostics,
): ResolvedConfig {
  const agents = mergeAgents(layers, diags);
  const skills = mergeSkills(layers, diags);
  fillSlots(layers, agents, skills, manifest.options.strictSlots, diags);
  return {
    manifest,
    layers,
    instructions: mergeInstructions(layers, diags),
    agents,
    skills,
    rules: mergeRules(layers, diags),
    hooks: mergeHooks(layers, diags),
    permissions: mergePermissions(layers, diags),
    mcp: mergeMcp(layers, manifest.mcpEnable, diags),
    models: mergeModels(layers, diags),
    settings: mergeSettings(layers, diags),
    passthrough: mergePassthrough(layers, diags),
    claudeExtras: claudeExtras(layers, diags),
    commitIdentity: mergeCommitIdentity(layers, repoRoot, diags),
    account: mergeAccount(layers, repoRoot, diags),
  };
}
