import { type Diagnostics, InputError } from "../core/diagnostics";
import { applyListOps, parseListOps } from "../core/list-ops";
import { isMap, unknownKeys } from "../core/yaml";
import { type Layer, where } from "../model/layer";
import {
  isKnownTool,
  PERMISSION_MODES,
  type PermissionMode,
  strictness,
} from "../model/vocabulary";
import { readLayerYaml } from "./layer-files";
import { parseTool } from "./tools";

export interface PermissionsSpec {
  allow: string[];
  ask: string[];
  deny: string[];
  defaultMode?: PermissionMode;
}

const FILE = "permissions.yaml";
const LISTS = ["allow", "ask", "deny"] as const;

/** §7.6: lists combine; deny rules form a floor that more specific layers cannot lower. */
export function mergePermissions(layers: Layer[], diags: Diagnostics): PermissionsSpec {
  const spec: PermissionsSpec = { allow: [], ask: [], deny: [] };
  const denyOwner = new Map<string, string>();
  let floor: { mode: PermissionMode; layer: string } | undefined;

  for (const layer of layers) {
    const raw = readLayerYaml(layer, FILE, diags);
    if (raw === undefined || raw === null) continue;
    const loc = where(layer, FILE);
    if (!isMap(raw)) {
      diags.error("permissions", "permissions.yaml must be a mapping", loc);
      continue;
    }
    const extra = unknownKeys(raw, [...LISTS, "default_mode", "floor"]);
    if (extra.length > 0) diags.error("permissions", `unknown field(s): ${extra.join(", ")}`, loc);

    for (const list of LISTS) {
      try {
        const ops = parseListOps(raw[list], list);
        if (!ops) continue;
        // A plain YAML list adds to the inherited rules; only "[…]" strings replace.
        const effective = Array.isArray(raw[list])
          ? ops.map((op) => ({ ...op, kind: "add" as const }))
          : ops;
        const next = applyListOps(spec[list], effective);
        for (const rule of next) validateRule(rule);
        if (list === "deny") {
          for (const removed of spec.deny.filter((rule) => !next.includes(rule))) {
            diags.error(
              "permission-floor",
              `layer '${layer.meta.name}' removes deny rule '${removed}' set by '${denyOwner.get(removed)}'; deny rules can only be added`,
              loc,
            );
          }
          for (const rule of next) if (!denyOwner.has(rule)) denyOwner.set(rule, layer.meta.name);
          spec.deny = [...new Set([...spec.deny, ...next])];
        } else {
          spec[list] = next;
        }
      } catch (error) {
        if (!(error instanceof InputError)) throw error;
        diags.error("permissions", error.message, loc);
      }
    }

    if (raw.default_mode !== undefined) {
      const mode = raw.default_mode as PermissionMode;
      if (!PERMISSION_MODES.includes(mode)) {
        diags.error(
          "permissions",
          `'default_mode' must be one of ${PERMISSION_MODES.join(", ")}`,
          loc,
        );
        continue;
      }
      if (floor && strictness(mode) < strictness(floor.mode)) {
        diags.error(
          "permission-floor",
          `default_mode '${mode}' is looser than the floor '${floor.mode}' set by '${floor.layer}'`,
          loc,
        );
        continue;
      }
      spec.defaultMode = mode;
      if (raw.floor === true && (!floor || strictness(mode) > strictness(floor.mode))) {
        floor = { mode, layer: layer.meta.name };
      }
    } else if (raw.floor !== undefined) {
      diags.error("permissions", "'floor' needs a 'default_mode'", loc);
    }
  }

  for (const rule of spec.allow.filter((allowed) => spec.deny.includes(allowed))) {
    diags.warn("permission-shadowed", `allow rule '${rule}' is also denied; deny wins`);
  }
  return spec;
}

/** Neutral rule: `tool` or `tool:pattern`. */
function validateRule(rule: string): void {
  const { tool, scope } = parseTool(rule);
  if (!isKnownTool(tool)) throw new InputError(`rule '${rule}': unknown tool '${tool}'`);
  if (scope === "") throw new InputError(`rule '${rule}': empty pattern`);
}
