import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { scanPublicLayer } from "../../checks/leak-guard";
import { Diagnostics, UNRESOLVED } from "../../core/diagnostics";
import { parseYaml } from "../../core/yaml";
import { mergeAgents } from "../../merge/agents";
import { mergeHooks } from "../../merge/hooks";
import { mergeInstructions } from "../../merge/instructions";
import { mergeMcp } from "../../merge/mcp";
import { mergeModels } from "../../merge/models";
import { claudeExtras, mergePassthrough } from "../../merge/passthrough";
import { mergePermissions } from "../../merge/permissions";
import { mergeRules } from "../../merge/rules";
import { mergeSettings } from "../../merge/settings";
import { mergeSkills } from "../../merge/skills";
import { type Layer, where } from "../../model/layer";
import { readLock } from "../../model/lock";
import { isStackManifest, readManifest } from "../../model/manifest";
import { SKILL_NAME } from "../../model/vocabulary";
import { loadLayer, loadStack } from "../../resolve/stack";
import { buildContext, type Io, plural, printDiagnostics } from "../io";

/** `alz lint [--layer <path>]`: schema, leak guard and slot declarations of a layer (or of every layer in the stack). */
export function lintCommand(io: Io, layerPath: string | undefined): number {
  const diags = new Diagnostics();
  let layers: Layer[];
  if (layerPath) {
    const dir = resolve(io.cwd, layerPath);
    layers = [loadLayer(dir, layerPath, diags)];
  } else if (isRepoManifest(io.cwd)) {
    const ctx = buildContext(io);
    layers = loadStack(
      readManifest(io.cwd),
      { ...ctx, locked: readLock(io.cwd)?.layers ?? [] },
      diags,
    );
  } else {
    layers = [loadLayer(io.cwd, ".", diags)];
  }

  for (const layer of layers) lintLayer(layer, diags);
  printDiagnostics(io, diags.items);
  const names = layers.map((layer) => layer.meta.name).join(", ");
  if (diags.hasErrors) {
    io.stdout(`lint failed: ${plural(diags.errors.length, "error")} in ${names}\n`);
    return 1;
  }
  io.stdout(`lint ok: ${names}\n`);
  return 0;
}

function isRepoManifest(dir: string): boolean {
  const path = join(dir, "layer.yaml");
  if (!existsSync(path)) return false;
  try {
    return isStackManifest(parseYaml(readFileSync(path, "utf8")));
  } catch {
    return false;
  }
}

/** Runs every loader on the layer alone; references to more general layers are not checked. */
function lintLayer(layer: Layer, diags: Diagnostics): void {
  scanPublicLayer(layer, diags);
  const local = new Diagnostics();
  const single = [layer];
  mergeInstructions(single, local);
  mergeAgents(single, local);
  const skills = mergeSkills(single, local);
  mergeRules(single, local);
  mergeHooks(single, local);
  mergePermissions(single, local);
  mergeMcp(single, [], local);
  mergeModels(single, local);
  mergeSettings(single, local);
  claudeExtras(single, local);
  mergePassthrough(single, local);
  diags.items.push(...local.items.filter((item) => item.code !== UNRESOLVED));

  const loc = where(layer, "layer.yaml");
  for (const slot of layer.meta.providesSlots) {
    if (!skills.has(slot))
      diags.error("slot", `provides_slots lists '${slot}', but the layer has no such skill`, loc);
  }
  for (const slot of layer.meta.requiresSlots) {
    if (!SKILL_NAME.test(slot))
      diags.error("slot", `requires_slots entry '${slot}' is not a kebab-case skill name`, loc);
  }
}
