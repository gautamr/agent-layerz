import { type Diagnostics, InputError, UNRESOLVED } from "../core/diagnostics";
import { applyListOps, describeOps, type ListOp, parseListOps } from "../core/list-ops";
import { asOptionalString, asStringList, isMap, unknownKeys, type YamlMap } from "../core/yaml";
import { kindTitle, type Layer, sourceRef, where } from "../model/layer";
import { isKnownTool, PERMISSION_MODES, SKILL_NAME } from "../model/vocabulary";
import { filesIn, readLayerMarkdown, stem } from "./layer-files";
import { checkCodexPolicies, checkMcpServerEnvs } from "./protected-variables";
import { deepMerge } from "./settings";
import { parseTool, widenedTools } from "./tools";

export interface Contribution {
  /** `personal@1.0.0:agents/architect.md` */
  ref: string;
  /** What the source did, e.g. `base` or `extend: skills +[x]; body section`. */
  action: string;
}

export interface AgentSpec {
  name: string;
  description: string;
  role?: string;
  model?: string;
  /** Undefined = the agent inherits every tool. */
  tools?: string[];
  skills: string[];
  isolation?: string;
  permissionMode?: string;
  memory?: string;
  targets?: string[];
  /** Target-native frontmatter (`claude:`, `codex:`), passed through as written. */
  native: Record<string, YamlMap>;
  body: string;
  /** Where the agent was defined; used for diagnostics about the merged result. */
  baseWhere: string;
  sources: Contribution[];
}

const SCALARS = ["description", "role", "model", "isolation", "permission_mode", "memory"] as const;
const LISTS = ["tools", "skills", "targets"] as const;
/**
 * §7.2: native fields per target, for features the neutral schema does not model (`effort`,
 * `maxTurns`, per-agent `hooks`, …). Listed keys have a neutral field and are rejected here.
 */
const NATIVE_RESERVED: Record<string, readonly string[]> = {
  claude: [
    "name",
    "description",
    "tools",
    "model",
    "permissionMode",
    "skills",
    "isolation",
    "memory",
  ],
  codex: ["name", "description", "model", "developer_instructions"],
};
const NATIVE_TARGETS = Object.keys(NATIVE_RESERVED);
const BASE_FIELDS = ["name", ...SCALARS, ...LISTS, ...NATIVE_TARGETS];
const OVERLAY_FIELDS = ["extends", "mode", "allow_widen", ...SCALARS, ...LISTS, ...NATIVE_TARGETS];

const ALLOWED_VALUES: Partial<Record<(typeof SCALARS)[number], readonly string[]>> = {
  isolation: ["none", "worktree"],
  memory: ["none", "project", "user", "local"],
  permission_mode: PERMISSION_MODES,
};

/** §7.2: bases, then overlays, layer by layer. */
export function mergeAgents(layers: Layer[], diags: Diagnostics): Map<string, AgentSpec> {
  const agents = new Map<string, AgentSpec>();
  for (const layer of layers) {
    const all = filesIn(layer, "agents", ".md");
    for (const path of all.filter((file) => !file.endsWith(".overlay.md"))) {
      const spec = readBase(layer, path, diags);
      if (!spec) continue;
      const existing = agents.get(spec.name);
      if (existing) {
        diags.error(
          "agent-duplicate",
          `agent '${spec.name}' is already defined by ${existing.sources[0]?.ref}; use ${spec.name}.overlay.md or mode: replace`,
          where(layer, path),
        );
        continue;
      }
      agents.set(spec.name, spec);
    }
    for (const path of all.filter((file) => file.endsWith(".overlay.md"))) {
      applyOverlay(layer, path, agents, diags);
    }
  }
  return agents;
}

function readBase(layer: Layer, path: string, diags: Diagnostics): AgentSpec | undefined {
  const doc = readLayerMarkdown(layer, path, diags);
  if (!doc) return undefined;
  const loc = where(layer, path);
  const data = doc.data;
  if (!checkFields(data, BASE_FIELDS, loc, diags)) return undefined;
  const name = data.name;
  if (typeof name !== "string" || !SKILL_NAME.test(name)) {
    diags.error("agent", "'name' is required and must be kebab-case", loc);
    return undefined;
  }
  if (name !== stem(path, ".md")) {
    diags.error("agent", `'name: ${name}' must match the file name`, loc);
    return undefined;
  }
  try {
    const description = asOptionalString(data.description, "description");
    if (!description) throw new InputError("'description' is required");
    const spec: AgentSpec = {
      name,
      description,
      skills: data.skills === undefined ? [] : asStringList(data.skills, "skills"),
      tools: data.tools === undefined ? undefined : asStringList(data.tools, "tools"),
      targets: data.targets === undefined ? undefined : asStringList(data.targets, "targets"),
      native: {},
      body: doc.body.trim(),
      baseWhere: loc,
      sources: [{ ref: sourceRef(layer, path), action: "base" }],
    };
    applyScalars(spec, data);
    applyNative(spec, data, loc, diags);
    validateSpec(spec, loc, diags);
    return spec;
  } catch (error) {
    if (!(error instanceof InputError)) throw error;
    diags.error("agent", error.message, loc);
    return undefined;
  }
}

function applyOverlay(
  layer: Layer,
  path: string,
  agents: Map<string, AgentSpec>,
  diags: Diagnostics,
): void {
  const doc = readLayerMarkdown(layer, path, diags);
  if (!doc) return;
  const loc = where(layer, path);
  const data = doc.data;
  if (!checkFields(data, OVERLAY_FIELDS, loc, diags)) return;
  const name = stem(path, ".overlay.md");
  if (data.extends !== name) {
    diags.error("agent-overlay", `'extends' must be '${name}' to match the file name`, loc);
    return;
  }
  const mode = data.mode ?? "extend";
  if (mode !== "extend" && mode !== "replace") {
    diags.error("agent-overlay", "'mode' must be extend or replace", loc);
    return;
  }
  const allowWiden = data.allow_widen === true;
  if (allowWiden && layer.meta.visibility === "public") {
    diags.error("tool-widen", "allow_widen is not allowed in a public layer", loc);
    return;
  }

  try {
    let spec = agents.get(name);
    const ref = sourceRef(layer, path);
    if (!spec) {
      if (mode !== "replace") {
        diags.error(UNRESOLVED, `extends '${name}', but no more general layer defines it`, loc);
        return;
      }
      const description = asOptionalString(data.description, "description");
      if (!description) throw new InputError("defining a new agent needs 'description'");
      spec = { name, description, skills: [], native: {}, body: "", baseWhere: loc, sources: [] };
      agents.set(name, spec);
    }

    const changes: string[] = [];
    for (const field of SCALARS) {
      if (data[field] !== undefined && data[field] !== null) changes.push(field);
    }
    applyScalars(spec, data);
    for (const target of applyNative(spec, data, loc, diags)) changes.push(`${target} fields`);

    const skillOps = parseListOps(data.skills, "skills");
    if (skillOps) {
      spec.skills = applyListOps(spec.skills, skillOps);
      changes.push(`skills ${describeOps(skillOps)}`);
    }
    const targetOps = parseListOps(data.targets, "targets");
    if (targetOps) {
      spec.targets = applyListOps(spec.targets ?? [], targetOps);
      changes.push(`targets ${describeOps(targetOps)}`);
    }
    const toolOps = parseListOps(data.tools, "tools");
    if (toolOps) {
      applyToolOps(spec, toolOps, allowWiden, loc, diags);
      changes.push(`tools ${describeOps(toolOps)}`);
    }

    const body = doc.body.trim();
    if (mode === "replace") {
      spec.body = body;
      changes.push("body replaced");
      if (spec.sources.length > 0) {
        diags.warn("agent-replace", `replaces the whole body of agent '${name}'`, loc);
      }
    } else if (body) {
      spec.body = `${spec.body}\n\n## ${kindTitle(layer)} context: ${layer.meta.name}\n\n${body}`;
      changes.push("body section");
    }
    spec.sources.push({ ref, action: `${mode}: ${changes.join("; ") || "no changes"}` });
    validateSpec(spec, loc, diags);
  } catch (error) {
    if (!(error instanceof InputError)) throw error;
    diags.error("agent-overlay", error.message, loc);
  }
}

/** §7.2 narrowing rule: an overlay may only remove tools or narrow their scope. */
function applyToolOps(
  spec: AgentSpec,
  ops: ListOp[],
  allowWiden: boolean,
  loc: string,
  diags: Diagnostics,
): void {
  if (spec.tools === undefined && ops.some((op) => op.kind === "remove")) {
    throw new InputError(
      `agent '${spec.name}' inherits all tools; give it an explicit tools list before removing one`,
    );
  }
  const result = applyListOps(spec.tools ?? [], ops);
  const widened = widenedTools(spec.tools, result);
  if (widened.length > 0 && !allowWiden) {
    for (const entry of widened) {
      diags.error(
        "tool-widen",
        `overlay on agent '${spec.name}' adds tool '${entry}', which the base does not grant; overlays may only remove or narrow tools (allow_widen: true is accepted in non-public layers)`,
        loc,
      );
    }
    return;
  }
  spec.tools = result;
}

function applyScalars(spec: AgentSpec, data: YamlMap): void {
  for (const field of SCALARS) {
    const value = data[field];
    if (value === undefined || value === null) continue;
    if (typeof value !== "string") throw new InputError(`'${field}' must be a string`);
    const allowed = ALLOWED_VALUES[field];
    if (allowed && !allowed.includes(value)) {
      throw new InputError(`'${field}' must be one of ${allowed.join(", ")}`);
    }
    if (field === "description") spec.description = value;
    else if (field === "role") spec.role = value;
    else if (field === "model") spec.model = value;
    else if (field === "isolation") spec.isolation = value;
    else if (field === "permission_mode") spec.permissionMode = value;
    else spec.memory = value;
  }
}

/** Deep-merges each `<target>:` map into the spec; returns the targets it touched. */
function applyNative(spec: AgentSpec, data: YamlMap, loc: string, diags: Diagnostics): string[] {
  const touched: string[] = [];
  for (const target of NATIVE_TARGETS) {
    const value = data[target];
    if (value === undefined || value === null) continue;
    if (!isMap(value)) throw new InputError(`'${target}' must be a mapping of native fields`);
    const reserved = Object.keys(value).filter((key) => NATIVE_RESERVED[target]?.includes(key));
    if (reserved.length > 0) {
      throw new InputError(
        `'${target}.${reserved[0]}' has a neutral field; set it outside '${target}:' so every target gets it`,
      );
    }
    checkNativeEnvironment(target, value, loc, diags);
    spec.native[target] = deepMerge(spec.native[target] ?? {}, value);
    touched.push(target);
  }
  return touched;
}

/** §7.9: a native `mcpServers` / `mcp_servers` or Codex policy in agent frontmatter must not set the protected variables. */
function checkNativeEnvironment(
  target: string,
  native: YamlMap,
  loc: string,
  diags: Diagnostics,
): void {
  if (target === "claude") checkMcpServerEnvs(native.mcpServers, "claude.mcpServers", loc, diags);
  if (target === "codex") {
    checkMcpServerEnvs(native.mcp_servers, "codex.mcp_servers", loc, diags);
    checkCodexPolicies(native, loc, diags);
  }
}

function validateSpec(spec: AgentSpec, loc: string, diags: Diagnostics): void {
  for (const entry of spec.tools ?? []) {
    if (!isKnownTool(parseTool(entry).tool)) {
      diags.error("agent", `agent '${spec.name}': unknown tool '${entry}'`, loc);
    }
  }
  for (const skill of spec.skills) {
    if (!SKILL_NAME.test(skill))
      diags.error("agent", `agent '${spec.name}': bad skill name '${skill}'`, loc);
  }
}

function checkFields(data: YamlMap, allowed: string[], loc: string, diags: Diagnostics): boolean {
  const extra = unknownKeys(data, allowed);
  if (extra.length === 0) return true;
  diags.error(
    "agent",
    `unknown field(s): ${extra.join(", ")} (allowed: ${allowed.join(", ")})`,
    loc,
  );
  return false;
}
