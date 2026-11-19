import { type Diagnostics, UNRESOLVED } from "../core/diagnostics";
import { isMap, unknownKeys } from "../core/yaml";
import { type Layer, where } from "../model/layer";
import type { AgentSpec } from "./agents";
import { readLayerYaml } from "./layer-files";

export interface ModelsSpec {
  /** Logical alias → target → concrete model id. */
  aliases: Record<string, Record<string, string>>;
  /** Role → alias. */
  roles: Record<string, string>;
  /** Alias used as the session default. */
  default?: string;
}

const FILE = "models.yaml";

/** §7.8: aliases and roles deep-merge; the most specific value wins. */
export function mergeModels(layers: Layer[], diags: Diagnostics): ModelsSpec {
  const spec: ModelsSpec = { aliases: {}, roles: {} };
  for (const layer of layers) {
    const raw = readLayerYaml(layer, FILE, diags);
    if (raw === undefined || raw === null) continue;
    const loc = where(layer, FILE);
    if (!isMap(raw) || unknownKeys(raw, ["aliases", "roles", "default"]).length > 0) {
      diags.error("models", "models.yaml takes 'aliases', 'roles' and 'default'", loc);
      continue;
    }
    if (raw.aliases !== undefined) {
      if (!isMap(raw.aliases)) diags.error("models", "'aliases' must be a mapping", loc);
      else {
        for (const [alias, targets] of Object.entries(raw.aliases)) {
          if (!isMap(targets) || Object.values(targets).some((id) => typeof id !== "string")) {
            diags.error("models", `alias '${alias}' must map target → model id`, loc);
            continue;
          }
          spec.aliases[alias] = { ...spec.aliases[alias], ...(targets as Record<string, string>) };
        }
      }
    }
    if (raw.roles !== undefined) {
      if (
        !isMap(raw.roles) ||
        Object.values(raw.roles).some((alias) => typeof alias !== "string")
      ) {
        diags.error("models", "'roles' must map role → alias", loc);
      } else Object.assign(spec.roles, raw.roles);
    }
    if (raw.default !== undefined) {
      if (typeof raw.default !== "string") diags.error("models", "'default' must be an alias", loc);
      else spec.default = raw.default;
    }
  }
  for (const [role, alias] of Object.entries(spec.roles)) {
    if (!spec.aliases[alias])
      diags.error(UNRESOLVED, `role '${role}' routes to unknown alias '${alias}'`);
  }
  if (spec.default && !spec.aliases[spec.default]) {
    diags.error(UNRESOLVED, `default '${spec.default}' is not an alias`);
  }
  return spec;
}

/**
 * Resolves an agent's model for a target. An explicit alias beats the role; `inherit`
 * (or no model) routes through the role. Undefined means "use the session model".
 */
export function resolveAgentModel(
  agent: AgentSpec,
  models: ModelsSpec,
  target: string,
  diags: Diagnostics,
): string | undefined {
  let alias: string | undefined;
  if (agent.model && agent.model !== "inherit") {
    alias = agent.model;
    if (!models.aliases[alias]) {
      diags.error(
        "models",
        `agent '${agent.name}': model '${alias}' is not an alias in models.yaml; concrete ids live only there`,
        agent.baseWhere,
      );
      return undefined;
    }
  } else if (agent.role) {
    alias = models.roles[agent.role];
  }
  if (!alias) return undefined;
  const id = models.aliases[alias]?.[target];
  if (!id) {
    diags.warn(
      "models",
      `alias '${alias}' has no ${target} model; agent '${agent.name}' inherits`,
      agent.baseWhere,
    );
  }
  return id;
}
