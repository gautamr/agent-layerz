import { findSecrets } from "../checks/secrets";
import type { Diagnostics } from "../core/diagnostics";
import { isMap, unknownKeys, type YamlMap } from "../core/yaml";
import { type Layer, sourceRef, where } from "../model/layer";
import type { Contribution } from "./agents";
import { readLayerYaml } from "./layer-files";

export interface McpServer {
  id: string;
  transport: "stdio" | "http" | "sse";
  url?: string;
  command?: string;
  args?: string[];
  /** Names of env vars passed through; values are never stored. */
  envFrom: string[];
  sources: Contribution[];
}

const FILE = "mcp.yaml";
const FIELDS = ["transport", "url", "command", "args", "env_from"];

/** §7.7: same id → most specific wins; only ids in the manifest's `mcp.enable` are emitted. */
export function mergeMcp(
  layers: Layer[],
  enable: string[],
  diags: Diagnostics,
): { all: Map<string, McpServer>; enabled: McpServer[] } {
  const all = new Map<string, McpServer>();
  for (const layer of layers) {
    const raw = readLayerYaml(layer, FILE, diags);
    if (raw === undefined || raw === null) continue;
    const loc = where(layer, FILE);
    if (!isMap(raw)) {
      diags.error("mcp", "mcp.yaml must be a mapping of server id → definition", loc);
      continue;
    }
    for (const [id, definition] of Object.entries(raw)) {
      const server = readServer(id, definition, layer, loc, diags);
      if (!server) continue;
      const previous = all.get(id);
      if (previous) {
        server.sources = [
          ...previous.sources.map((source) => ({
            ...source,
            action: `${source.action} (overridden)`,
          })),
          ...server.sources,
        ];
      }
      all.set(id, server);
    }
  }
  const enabled: McpServer[] = [];
  for (const id of enable) {
    const server = all.get(id);
    if (server) enabled.push(server);
    else diags.error("mcp", `mcp.enable lists '${id}', but no layer defines it`, "layer.yaml");
  }
  return { all, enabled };
}

function readServer(
  id: string,
  definition: unknown,
  layer: Layer,
  loc: string,
  diags: Diagnostics,
): McpServer | undefined {
  const fail = (message: string) => {
    diags.error("mcp", `server '${id}': ${message}`, loc);
    return undefined;
  };
  if (!isMap(definition)) return fail("definition must be a mapping");
  if ("env" in definition || "headers" in definition) {
    return fail("values are never stored here; list variable names in 'env_from'");
  }
  const extra = unknownKeys(definition, FIELDS);
  if (extra.length > 0) return fail(`unknown field(s): ${extra.join(", ")}`);
  if (containsSecret(definition)) {
    diags.error("leak-secret", `server '${id}': a value looks like a secret token`, loc);
    return undefined;
  }
  const { transport, url, command, args, env_from: envFrom } = definition;
  if (transport !== "stdio" && transport !== "http" && transport !== "sse") {
    return fail("'transport' must be stdio, http or sse");
  }
  if (transport === "stdio" && typeof command !== "string") return fail("stdio needs 'command'");
  if (transport !== "stdio" && typeof url !== "string") return fail(`${transport} needs 'url'`);
  if (args !== undefined && !isStrings(args)) return fail("'args' must be a list of strings");
  if (envFrom !== undefined && !isStrings(envFrom))
    return fail("'env_from' must be a list of names");
  if (transport !== "stdio" && isStrings(envFrom) && envFrom.length > 0) {
    return fail("'env_from' only applies to stdio servers");
  }
  return {
    id,
    transport,
    url: typeof url === "string" ? url : undefined,
    command: typeof command === "string" ? command : undefined,
    args: isStrings(args) ? args : undefined,
    envFrom: isStrings(envFrom) ? envFrom : [],
    sources: [{ ref: sourceRef(layer, FILE), action: `server ${id}` }],
  };
}

function isStrings(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

function containsSecret(definition: YamlMap): boolean {
  const values = Object.values(definition).flatMap((value) =>
    Array.isArray(value) ? value : [value],
  );
  return values.some((value) => typeof value === "string" && findSecrets(value).length > 0);
}
