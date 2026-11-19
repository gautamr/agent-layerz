import { isAbsolute } from "node:path";
import type { AdoptionPlan } from "../../adopt/plan";
import { isMap, type YamlMap } from "../../core/yaml";
import { PROJECT_DIR_VAR, TOOL_EVENTS } from "../../model/vocabulary";
import { modelAlias } from "./adopt-agent";
import { HOOKS_DIR, SETTINGS_FILE } from "./settings";
import { neutralEvent, neutralPermissionMode, neutralRule, neutralTool } from "./tool-names";

/** settings.json keys that the manifest's `mcp.enable` replaces: dropped with a note. */
const MCP_SWITCHES = [
  "enabledMcpjsonServers",
  "disabledMcpjsonServers",
  "enableAllProjectMcpServers",
];
const HOOK_FIELDS = ["type", "command", "timeout", "onFailure"];

/**
 * `.claude/settings.json` → permissions.yaml, hooks/hooks.yaml, settings.yaml, models.yaml and
 * the manifest's `workspace.add_dirs`. Anything that would loosen security if dropped is a problem.
 */
export function adoptSettingsJson(
  text: string,
  plan: AdoptionPlan,
  hookScripts: Map<string, string>,
): void {
  const from = SETTINGS_FILE;
  const json = parseJson(text, from, plan);
  if (!json) return;
  const outputs = new Set<string>();
  const settings: YamlMap = {};

  for (const [key, value] of Object.entries(json)) {
    if (key === "permissions") adoptPermissions(value, plan, outputs);
    else if (key === "hooks") adoptHooks(value, plan, hookScripts, outputs);
    else if (key === "model") {
      plan.mergeYaml(from, "models.yaml", { default: modelAlias(String(value), from, plan) });
      outputs.add("models.yaml");
    } else if (MCP_SWITCHES.includes(key)) {
      plan.notes.push(
        `${from}: dropped '${key}'; the manifest's mcp.enable decides which servers run`,
      );
    } else if (key === "disableAllHooks") {
      plan.problems.push(
        `${from}: 'disableAllHooks' has no layered form; remove it or disable hooks by id`,
      );
    } else settings[key] = value;
  }
  if (Object.keys(settings).length > 0) {
    plan.mergeYaml(from, "settings.yaml", { claude: settings });
    outputs.add("settings.yaml");
  }
  plan.converted(from, [...outputs]);
}

function adoptPermissions(value: unknown, plan: AdoptionPlan, outputs: Set<string>): void {
  const from = `${SETTINGS_FILE} permissions`;
  if (!isMap(value)) {
    plan.problems.push(`${from}: must be an object`);
    return;
  }
  const out: YamlMap = {};
  for (const [key, rules] of Object.entries(value)) {
    if (key === "allow" || key === "ask" || key === "deny") {
      const list = Array.isArray(rules) ? rules.map(String) : [];
      const neutral = list.map((rule) => [rule, neutralRule(rule)] as const);
      for (const [rule] of neutral.filter(([, mapped]) => !mapped)) {
        plan.problems.push(`${from}.${key}: '${rule}' uses a tool agent-layerz does not know`);
      }
      out[key] = neutral.flatMap(([, mapped]) => (mapped ? [mapped] : []));
    } else if (key === "defaultMode") {
      const mode = neutralPermissionMode(String(rules));
      if (mode) out.default_mode = mode;
      else plan.problems.push(`${from}: unknown defaultMode '${String(rules)}'`);
    } else if (key === "additionalDirectories") {
      for (const dir of Array.isArray(rules) ? rules.map(String) : []) {
        if (isAbsolute(dir) || dir.startsWith("~")) {
          plan.problems.push(`${from}: additional directory '${dir}' must be relative to the repo`);
        } else plan.manifest.addDirs.push(dir);
      }
      outputs.add("layer.yaml");
    } else {
      plan.problems.push(`${from}: '${key}' has no layered form yet; move it by hand`);
    }
  }
  if (Object.keys(out).length > 0) {
    plan.mergeYaml(SETTINGS_FILE, "permissions.yaml", out);
    outputs.add("permissions.yaml");
  }
}

function adoptHooks(
  value: unknown,
  plan: AdoptionPlan,
  hookScripts: Map<string, string>,
  outputs: Set<string>,
): void {
  const from = `${SETTINGS_FILE} hooks`;
  if (!isMap(value)) {
    plan.problems.push(`${from}: must be an object`);
    return;
  }
  const hooks: YamlMap[] = [];
  const ids = new Set<string>();
  for (const [event, groups] of Object.entries(value)) {
    for (const group of Array.isArray(groups) ? groups : []) {
      const matcher = isMap(group) && typeof group.matcher === "string" ? group.matcher : undefined;
      const handlers = isMap(group) && Array.isArray(group.hooks) ? group.hooks : [];
      for (const handler of handlers) {
        const extra = isMap(handler)
          ? Object.keys(handler).filter((key) => !HOOK_FIELDS.includes(key))
          : [];
        if (
          !isMap(handler) ||
          handler.type !== "command" ||
          typeof handler.command !== "string" ||
          extra.length > 0
        ) {
          plan.problems.push(
            `${from}.${event}: only command hooks with ${HOOK_FIELDS.join(", ")} can be adopted; move this one by hand`,
          );
          continue;
        }
        const hook = hookEntry(event, matcher, handler.command, plan, hookScripts);
        hook.id = uniqueId(hookId(handler.command, event), ids);
        if (typeof handler.timeout === "number") hook.timeout = handler.timeout;
        hook.on_fail = handler.onFailure === "block" ? "deny" : "warn";
        hooks.push(hook);
      }
    }
  }
  if (hooks.length > 0) {
    plan.mergeYaml(SETTINGS_FILE, "hooks/hooks.yaml", hooks);
    outputs.add("hooks/hooks.yaml");
  }
}

/** A neutral event where one exists; otherwise the native `claude:<Event>` with its matcher. */
function hookEntry(
  event: string,
  matcher: string | undefined,
  command: string,
  plan: AdoptionPlan,
  hookScripts: Map<string, string>,
): YamlMap {
  const entry: YamlMap = { id: "" };
  const neutral = neutralEvent(event);
  const all = matcher === undefined || matcher === "" || matcher === "*";
  const tools = all ? [] : matcher.split("|").map((tool) => neutralTool(tool.trim()));
  if (neutral && (all || (TOOL_EVENTS.includes(neutral) && tools.every(Boolean)))) {
    entry.event = neutral;
    if (tools.length > 0) entry.match = { tools };
  } else {
    entry.event = `claude:${event}`;
    if (!all) entry.matcher = matcher;
  }
  Object.assign(entry, hookAction(command, plan, hookScripts));
  return entry;
}

const CLAUDE_DIR_REF = /\$\{?CLAUDE_PROJECT_DIR\}?/g;

/** A command that only runs a hand-written `.claude/hooks/` script becomes `run:` on its layer copy. */
function hookAction(
  command: string,
  plan: AdoptionPlan,
  hookScripts: Map<string, string>,
): YamlMap {
  const neutral = command.replace(CLAUDE_DIR_REF, PROJECT_DIR_VAR);
  for (const [path, script] of hookScripts) {
    const bare = neutral
      .trim()
      .replace(/^"|"$/g, "")
      .replace(/^"?\$\{PROJECT_DIR\}"?\//, "");
    if (bare === path) return { run: script };
    if (neutral.includes(path)) {
      return { command: neutral.replace(path, `${HOOKS_DIR}/${plan.layerName}/${script}`) };
    }
  }
  return { command: neutral };
}

/** Named after the script the command runs (`session-brief.sh` → `session-brief`), else the event. */
function hookId(command: string, event: string): string {
  const script = /([\w-]+)\.(?:sh|bash|js|mjs|cjs|ts|py|rb)\b/.exec(command)?.[1];
  return (script ?? event)
    .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function uniqueId(base: string, ids: Set<string>): string {
  let id = base || "hook";
  for (let n = 2; ids.has(id); n++) id = `${base}-${n}`;
  ids.add(id);
  return id;
}

/** `.mcp.json` → mcp.yaml plus `mcp.enable`. Env values are never copied: only `${NAME}` references. */
export function adoptMcpJson(text: string, plan: AdoptionPlan): void {
  const from = ".mcp.json";
  const json = parseJson(text, from, plan);
  if (!json) return;
  const servers = isMap(json.mcpServers) ? json.mcpServers : {};
  const out: YamlMap = {};
  for (const [id, server] of Object.entries(servers)) {
    if (!isMap(server)) continue;
    const extra = Object.keys(server).filter(
      (key) => !["type", "command", "args", "env", "url"].includes(key),
    );
    if (extra.length > 0) {
      plan.problems.push(
        `${from}: server '${id}' uses ${extra.join(", ")}, which mcp.yaml does not support yet`,
      );
      continue;
    }
    const envFrom: string[] = [];
    for (const [name, value] of Object.entries(isMap(server.env) ? server.env : {})) {
      if (value === `\${${name}}`) envFrom.push(name);
      else {
        plan.problems.push(
          `${from}: server '${id}' sets ${name} to a value; layers never store values. Use "\${${name}}" and export it`,
        );
      }
    }
    const transport = typeof server.type === "string" ? server.type : "stdio";
    out[id] = {
      transport,
      ...(server.url === undefined ? {} : { url: server.url }),
      ...(server.command === undefined ? {} : { command: server.command }),
      ...(server.args === undefined ? {} : { args: server.args }),
      ...(envFrom.length > 0 ? { env_from: envFrom } : {}),
    };
    plan.manifest.mcpEnable.push(id);
  }
  plan.mergeYaml(from, "mcp.yaml", out);
  plan.converted(from, ["mcp.yaml", "layer.yaml"]);
}

function parseJson(text: string, from: string, plan: AdoptionPlan): YamlMap | undefined {
  try {
    const json = JSON.parse(text);
    if (isMap(json)) return json;
  } catch {
    // reported below
  }
  plan.problems.push(`${from}: not a JSON object`);
  return undefined;
}
