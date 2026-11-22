import type { Diagnostics } from "../core/diagnostics";
import { isMap, type YamlMap } from "../core/yaml";
import { type Layer, where } from "../model/layer";
import { readLayerYaml } from "./layer-files";
import { checkCodexPolicies, checkEnvKeys } from "./protected-variables";

const FILE = "settings.yaml";

/** Keys that duplicate a first-class item (§7.9), or would switch off the hook floor. */
const RESERVED: Record<string, Record<string, string>> = {
  claude: {
    permissions: "permissions.yaml",
    hooks: "hooks/hooks.yaml",
    model: "models.yaml (default)",
    mcpServers: "mcp.yaml",
    enabledMcpjsonServers: "mcp.enable in the repo manifest",
    disabledMcpjsonServers: "mcp.enable in the repo manifest",
    enableAllProjectMcpServers: "mcp.enable in the repo manifest",
    disableAllHooks: "hooks.yaml `disabled` (locked hooks cannot be switched off)",
  },
  codex: {
    model: "models.yaml",
    mcp_servers: "mcp.yaml",
    hooks: "hooks/hooks.yaml",
    approval_policy: "permissions.yaml",
    sandbox_mode: "permissions.yaml",
  },
};

/** §7.9: deep merge per target. Scalars: most specific wins; arrays: concatenated, de-duplicated. */
export function mergeSettings(layers: Layer[], diags: Diagnostics): Record<string, YamlMap> {
  const merged: Record<string, YamlMap> = {};
  for (const layer of layers) {
    const raw = readLayerYaml(layer, FILE, diags);
    if (raw === undefined || raw === null) continue;
    const loc = where(layer, FILE);
    if (!isMap(raw)) {
      diags.error("settings", "settings.yaml must be a mapping of target → settings", loc);
      continue;
    }
    for (const [target, settings] of Object.entries(raw)) {
      if (!isMap(settings)) {
        diags.error("settings", `'${target}' must be a mapping`, loc);
        continue;
      }
      const reserved = Object.hasOwn(RESERVED, target) ? (RESERVED[target] ?? {}) : {};
      const clean: YamlMap = {};
      for (const [key, value] of Object.entries(settings)) {
        const owner = Object.hasOwn(reserved, key) ? reserved[key] : undefined;
        if (owner) diags.error("settings", `'${target}.${key}' is set through ${owner}`, loc);
        else clean[key] = value;
      }
      if (target === "claude") checkEnvKeys(settings.env, "claude.env", loc, diags);
      if (target === "codex") checkCodexPolicies(settings, loc, diags);
      merged[target] = deepMerge(merged[target] ?? {}, clean);
    }
  }
  return merged;
}

export function deepMerge(base: YamlMap, overlay: YamlMap): YamlMap {
  const result: YamlMap = { ...base };
  for (const [key, value] of Object.entries(overlay)) {
    const current = result[key];
    if (isMap(current) && isMap(value)) result[key] = deepMerge(current, value);
    else if (Array.isArray(current) && Array.isArray(value)) {
      const seen = new Set<string>();
      result[key] = [...current, ...value].filter((item) => {
        const id = JSON.stringify(item);
        if (seen.has(id)) return false;
        seen.add(id);
        return true;
      });
    } else result[key] = value;
  }
  return result;
}
