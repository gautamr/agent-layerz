import { InputError } from "../core/diagnostics";
import { asOptionalString, asStringList, isMap, unknownKeys } from "../core/yaml";

export const VISIBILITIES = ["public", "private", "confidential"] as const;
export type Visibility = (typeof VISIBILITIES)[number];

/** Higher is more private. */
export function privacyRank(visibility: Visibility): number {
  return VISIBILITIES.indexOf(visibility);
}

export interface LayerMeta {
  name: string;
  kind: string;
  visibility: Visibility;
  version?: string;
  description?: string;
  providesSlots: string[];
  requiresSlots: string[];
  forbidPatterns: string[];
}

const FIELDS = [
  "name",
  "kind",
  "visibility",
  "version",
  "description",
  "provides_slots",
  "requires_slots",
  "forbid_patterns",
] as const;

export const NAME_PATTERN = /^[a-z0-9]+(?:[-.][a-z0-9]+)*$/;

export function parseLayerMeta(raw: unknown): LayerMeta {
  if (!isMap(raw)) throw new InputError("layer.yaml must be a mapping");
  if ("stack" in raw) {
    throw new InputError("this is a repo stack manifest, not a layer (it has 'stack')");
  }
  const extra = unknownKeys(raw, FIELDS);
  if (extra.length > 0) throw new InputError(`unknown field(s): ${extra.join(", ")}`);

  const name = asOptionalString(raw.name, "name");
  if (!name || !NAME_PATTERN.test(name)) {
    throw new InputError("'name' is required and must be lowercase kebab-case");
  }
  const kind = asOptionalString(raw.kind, "kind");
  if (!kind) throw new InputError("'kind' is required (personal, client, group, repo, …)");
  const visibility = raw.visibility;
  if (!VISIBILITIES.includes(visibility as Visibility)) {
    throw new InputError(`'visibility' must be one of ${VISIBILITIES.join(", ")}`);
  }
  const version = raw.version === undefined ? undefined : String(raw.version);
  return {
    name,
    kind,
    visibility: visibility as Visibility,
    version,
    description: asOptionalString(raw.description, "description"),
    providesSlots: raw.provides_slots ? asStringList(raw.provides_slots, "provides_slots") : [],
    requiresSlots: raw.requires_slots ? asStringList(raw.requires_slots, "requires_slots") : [],
    forbidPatterns: raw.forbid_patterns ? asStringList(raw.forbid_patterns, "forbid_patterns") : [],
  };
}
