import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { Diagnostics } from "../../core/diagnostics";
import { isExecutable } from "../../core/files";
import type { YamlMap } from "../../core/yaml";
import { type HookSpec, isNativeEvent } from "../../merge/hooks";
import { deepMerge } from "../../merge/settings";
import type { ResolvedConfig } from "../../model/config";
import { PROJECT_DIR_VAR } from "../../model/vocabulary";
import type { FileTree } from "../../render/file-tree";
import { targetsClaude } from "./agents";
import {
  CLAUDE_EVENTS,
  CLAUDE_PERMISSION_MODES,
  CLAUDE_PROJECT_DIR_VAR,
  claudeRule,
  claudeTool,
} from "./tool-names";

export const SETTINGS_FILE = ".claude/settings.json";
export const HOOKS_DIR = ".claude/hooks";

/** §8.1: permissions, workspace dirs, hooks, default model and passthrough settings → `.claude/settings.json`. */
export function buildClaudeSettings(
  config: ResolvedConfig,
  tree: FileTree,
  diags: Diagnostics,
): YamlMap {
  const generated: YamlMap = {};

  const { allow, ask, deny, defaultMode } = config.permissions;
  const permissions: YamlMap = {};
  if (allow.length > 0) permissions.allow = allow.map(claudeRule);
  if (ask.length > 0) permissions.ask = ask.map(claudeRule);
  if (deny.length > 0) permissions.deny = deny.map(claudeRule);
  if (defaultMode) permissions.defaultMode = CLAUDE_PERMISSION_MODES[defaultMode];
  // §4.4, target-facts §9: grants file access only; the hub's .claude/ content is not loaded.
  const { addDirs } = config.manifest.workspace;
  if (addDirs.length > 0) permissions.additionalDirectories = addDirs;
  if (Object.keys(permissions).length > 0) generated.permissions = permissions;

  const hooks = renderHooks(config.hooks, tree, diags);
  if (Object.keys(hooks).length > 0) generated.hooks = hooks;

  const defaultAlias = config.models.default;
  const defaultModel = defaultAlias ? config.models.aliases[defaultAlias]?.claude : undefined;
  if (defaultModel) generated.model = defaultModel;

  return deepMerge(config.settings.claude ?? {}, generated);
}

function renderHooks(hooks: HookSpec[], tree: FileTree, diags: Diagnostics): YamlMap {
  const byEvent: Record<string, unknown[]> = {};
  for (const hook of hooks) {
    if (!targetsClaude(hook.targets)) continue;
    const command = hookCommand(hook, tree);
    const event = isNativeEvent(hook.event) ? hook.event.name : CLAUDE_EVENTS[hook.event];
    const onFailure = blockOnFailure(hook, diags);

    const tools = hook.tools.map(claudeTool);
    // The `if` field takes exactly one permission rule, so each tool × path gets a handler.
    const conditions =
      hook.paths.length > 0
        ? tools.flatMap((tool) => hook.paths.map((path) => `${tool}(${path})`))
        : [undefined];
    const handlers = conditions.map((condition) => ({
      type: "command",
      command,
      if: condition,
      timeout: hook.timeout,
      onFailure,
    }));
    const group: YamlMap = { hooks: handlers };
    const matcher = hook.matcher ?? (tools.length > 0 ? tools.join("|") : undefined);
    if (matcher !== undefined) group.matcher = matcher;
    byEvent[event] = [...(byEvent[event] ?? []), group];
  }
  return byEvent;
}

/** `run:` copies the layer's script into `.claude/hooks/<layer>/`; `command:` is used as written. */
function hookCommand(hook: HookSpec, tree: FileTree): string {
  const { action } = hook;
  if ("command" in action) {
    return action.command.replaceAll(PROJECT_DIR_VAR, CLAUDE_PROJECT_DIR_VAR);
  }
  const scriptPath = `${HOOKS_DIR}/${hook.layer.meta.name}/${action.script.slice("hooks/".length)}`;
  if (!tree.has(scriptPath)) {
    const source = join(hook.layer.dir, action.script);
    tree.add(scriptPath, readFileSync(source), isExecutable(source));
  }
  return `"\${CLAUDE_PROJECT_DIR}/${scriptPath}"`;
}

/** `on_fail: deny` → `onFailure: "block"`: a crashing guard blocks instead of letting the action through. */
function blockOnFailure(hook: HookSpec, diags: Diagnostics): "block" | undefined {
  if (hook.onFail !== "deny") return undefined;
  if (isNativeEvent(hook.event)) return "block";
  if (hook.event === "stop" || hook.event === "subagent_stop") {
    diags.warn(
      "hook-on-fail",
      `hook '${hook.id}': on_fail: deny has no effect on ${hook.event} in Claude; the script must exit 2 itself`,
      hook.where,
    );
    return undefined;
  }
  if (hook.event === "post_tool") {
    diags.warn(
      "hook-on-fail",
      `hook '${hook.id}': post_tool cannot block; a failure is shown to Claude`,
      hook.where,
    );
  }
  return "block";
}
