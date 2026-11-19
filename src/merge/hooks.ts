import { posix } from "node:path";
import { type Diagnostics, UNRESOLVED } from "../core/diagnostics";
import { isMap, unknownKeys, type YamlMap } from "../core/yaml";
import { type Layer, sourceRef, where } from "../model/layer";
import { isKnownTool, NEUTRAL_EVENTS, type NeutralEvent, TOOL_EVENTS } from "../model/vocabulary";
import type { Contribution } from "./agents";
import { readLayerYaml } from "./layer-files";

export type OnFail = "deny" | "warn" | "feedback";

export interface HookSpec {
  id: string;
  /** A neutral event, or a target's own event written `<target>:<Event>` (§7.5). */
  event: NeutralEvent | NativeEvent;
  tools: string[];
  paths: string[];
  /** Native events only: the target's matcher string, passed through. */
  matcher?: string;
  layer: Layer;
  /** Exactly one of `script` (from `run`) and `command` is set. */
  action: { script: string; run: string } | { command: string };
  /** Seconds. */
  timeout?: number;
  onFail: OnFail;
  targets?: string[];
  locked: boolean;
  where: string;
  sources: Contribution[];
}

export interface NativeEvent {
  target: string;
  name: string;
}

export function isNativeEvent(event: HookSpec["event"]): event is NativeEvent {
  return typeof event === "object";
}

const HOOKS_FILE = "hooks/hooks.yaml";
const FIELDS = [
  "id",
  "event",
  "match",
  "matcher",
  "run",
  "command",
  "timeout",
  "on_fail",
  "targets",
  "locked",
  "disabled",
];
const NATIVE_EVENT = /^([a-z]+):([A-Z][A-Za-z]+)$/;

/** §7.5: hooks combine across layers; later layers may disable unlocked hooks by id. */
export function mergeHooks(layers: Layer[], diags: Diagnostics): HookSpec[] {
  const hooks: HookSpec[] = [];
  const disabled = new Map<string, string>();
  for (const layer of layers) {
    const raw = readLayerYaml(layer, HOOKS_FILE, diags);
    if (raw === undefined || raw === null) continue;
    const loc = where(layer, HOOKS_FILE);
    if (!Array.isArray(raw)) {
      diags.error("hooks", "hooks.yaml must be a list", loc);
      continue;
    }
    for (const entry of raw) {
      if (!isMap(entry) || typeof entry.id !== "string") {
        diags.error("hooks", "every hook needs an 'id'", loc);
        continue;
      }
      const extra = unknownKeys(entry, FIELDS);
      if (extra.length > 0) {
        diags.error("hooks", `hook '${entry.id}': unknown field(s): ${extra.join(", ")}`, loc);
        continue;
      }
      if (entry.disabled !== undefined) {
        disableHook(entry, layer, hooks, disabled, diags);
        continue;
      }
      if (hooks.some((hook) => hook.id === entry.id) || disabled.has(entry.id)) {
        diags.error("hooks", `hook id '${entry.id}' is already used by a more general layer`, loc);
        continue;
      }
      const hook = readHook(entry, layer, diags);
      if (hook) hooks.push(hook);
    }
  }
  return hooks;
}

function disableHook(
  entry: YamlMap,
  layer: Layer,
  hooks: HookSpec[],
  disabled: Map<string, string>,
  diags: Diagnostics,
): void {
  const id = entry.id as string;
  const loc = where(layer, HOOKS_FILE);
  if (entry.disabled !== true || Object.keys(entry).length !== 2) {
    diags.error("hooks", `hook '${id}': a disable entry has only 'id' and 'disabled: true'`, loc);
    return;
  }
  const index = hooks.findIndex((hook) => hook.id === id);
  const hook = hooks[index];
  if (!hook) {
    diags.error(UNRESOLVED, `cannot disable hook '${id}': no more general layer defines it`, loc);
    return;
  }
  if (hook.locked) {
    diags.error(
      "hook-locked",
      `hook '${id}' is locked by layer '${hook.layer.meta.name}' and cannot be disabled by '${layer.meta.name}'`,
      loc,
    );
    return;
  }
  hooks.splice(index, 1);
  disabled.set(id, layer.meta.name);
}

function readHook(entry: YamlMap, layer: Layer, diags: Diagnostics): HookSpec | undefined {
  const id = entry.id as string;
  const loc = where(layer, HOOKS_FILE);
  const fail = (message: string) => {
    diags.error("hooks", `hook '${id}': ${message}`, loc);
    return undefined;
  };
  const event = readEvent(entry.event);
  if (!event) {
    return fail(`'event' must be one of ${NEUTRAL_EVENTS.join(", ")}, or <target>:<Event>`);
  }
  const action = readAction(entry, layer);
  if (typeof action === "string") return fail(action);

  const match = entry.match ?? {};
  if (!isMap(match) || unknownKeys(match, ["tools", "paths"]).length > 0) {
    return fail("'match' takes only 'tools' and 'paths'");
  }
  const tools = stringList(match.tools);
  const paths = stringList(match.paths);
  if (!tools || !paths) return fail("'match.tools' and 'match.paths' must be lists of strings");
  if (isNativeEvent(event)) {
    if (tools.length > 0 || paths.length > 0) {
      return fail("a native event takes the target's own 'matcher', not 'match'");
    }
  } else {
    if (entry.matcher !== undefined)
      return fail("'matcher' is only for native events; use 'match'");
    if ((tools.length > 0 || paths.length > 0) && !TOOL_EVENTS.includes(event)) {
      return fail(`'match' only applies to tool events (${TOOL_EVENTS.join(", ")})`);
    }
  }
  if (entry.matcher !== undefined && typeof entry.matcher !== "string") {
    return fail("'matcher' must be a string");
  }
  if (paths.length > 0 && tools.length === 0) return fail("'match.paths' needs 'match.tools'");
  const unknown = tools.filter((tool) => !isKnownTool(tool));
  if (unknown.length > 0) return fail(`unknown tool(s) ${unknown.join(", ")}`);
  if (entry.timeout !== undefined && (typeof entry.timeout !== "number" || !(entry.timeout > 0))) {
    return fail("'timeout' must be a positive number of seconds");
  }

  const onFail = entry.on_fail ?? "warn";
  if (onFail !== "deny" && onFail !== "warn" && onFail !== "feedback") {
    return fail("'on_fail' must be deny, warn or feedback");
  }
  let targets = entry.targets === undefined ? undefined : stringList(entry.targets);
  if (targets === null) return fail("'targets' must be a list of strings");
  if (isNativeEvent(event)) {
    if (targets && (targets.length !== 1 || targets[0] !== event.target)) {
      return fail(`a ${event.target} event only runs on target '${event.target}'`);
    }
    targets = [event.target];
  }
  if (entry.locked !== undefined && typeof entry.locked !== "boolean") {
    return fail("'locked' must be true or false");
  }
  return {
    id,
    event,
    tools,
    paths,
    matcher: entry.matcher as string | undefined,
    layer,
    action,
    timeout: entry.timeout as number | undefined,
    onFail,
    targets,
    locked: entry.locked === true,
    where: loc,
    sources: [{ ref: sourceRef(layer, HOOKS_FILE), action: `hook ${id}` }],
  };
}

function readEvent(value: unknown): HookSpec["event"] | undefined {
  if (NEUTRAL_EVENTS.includes(value as NeutralEvent)) return value as NeutralEvent;
  const native = typeof value === "string" ? NATIVE_EVENT.exec(value) : null;
  return native ? { target: native[1] as string, name: native[2] as string } : undefined;
}

/** `run:` ships a script inside the layer; `command:` runs a command line as written. */
function readAction(entry: YamlMap, layer: Layer): HookSpec["action"] | string {
  if ((entry.run === undefined) === (entry.command === undefined)) {
    return "needs exactly one of 'run' (a script in the layer) or 'command'";
  }
  if (entry.command !== undefined) {
    if (typeof entry.command !== "string" || entry.command.trim() === "") {
      return "'command' must be a non-empty string";
    }
    return { command: entry.command };
  }
  if (typeof entry.run !== "string") return "'run' must be a script path";
  const script = posix.normalize(posix.join("hooks", entry.run));
  if (!script.startsWith("hooks/")) return "'run' must stay inside the layer's hooks/ dir";
  if (!layer.files.includes(script)) return `script '${script}' not found in the layer`;
  return { script, run: entry.run };
}

function stringList(value: unknown): string[] | null {
  if (value === undefined || value === null) return [];
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) return null;
  return value as string[];
}
