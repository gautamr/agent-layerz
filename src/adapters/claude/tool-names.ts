import type { NeutralEvent, NeutralTool, PermissionMode } from "../../model/vocabulary";

const TOOLS: Record<NeutralTool, string> = {
  read: "Read",
  write: "Write",
  edit: "Edit",
  grep: "Grep",
  glob: "Glob",
  bash: "Bash",
  web_fetch: "WebFetch",
  web_search: "WebSearch",
  agent: "Agent",
  notebook_edit: "NotebookEdit",
  todo_write: "TodoWrite",
  skill: "Skill",
};

export function claudeTool(neutral: string): string {
  return TOOLS[neutral as NeutralTool] ?? neutral;
}

/** target-facts §4: Claude expands this in hook commands and exports it to the hook. */
// biome-ignore lint/suspicious/noTemplateCurlyInString: Claude expands this literal itself.
export const CLAUDE_PROJECT_DIR_VAR = "${CLAUDE_PROJECT_DIR}";

/** target-facts §8.5: all six neutral events exist in Claude Code. */
export const CLAUDE_EVENTS: Record<NeutralEvent, string> = {
  session_start: "SessionStart",
  pre_tool: "PreToolUse",
  post_tool: "PostToolUse",
  stop: "Stop",
  subagent_stop: "SubagentStop",
  prompt_submit: "UserPromptSubmit",
};

/** target-facts §1: native `permissionMode` values. */
export const CLAUDE_PERMISSION_MODES: Record<PermissionMode, string> = {
  bypass_permissions: "bypassPermissions",
  auto: "auto",
  accept_edits: "acceptEdits",
  default: "default",
  dont_ask: "dontAsk",
  plan: "plan",
};

/** target-facts §8.9: the values Claude accepts for `model`. */
export function isClaudeModel(value: string): boolean {
  return (
    ["sonnet", "opus", "haiku", "fable", "inherit"].includes(value) ||
    /^claude-[a-z0-9.-]+$/.test(value)
  );
}

/** Neutral rule `tool:pattern` → Claude `Tool(pattern)` (target-facts §7b). */
export function claudeRule(rule: string): string {
  const colon = rule.indexOf(":");
  if (colon < 0) return claudeTool(rule);
  return `${claudeTool(rule.slice(0, colon))}(${rule.slice(colon + 1)})`;
}

function invert(map: Record<string, string>): Record<string, string> {
  return Object.fromEntries(Object.entries(map).map(([neutral, native]) => [native, neutral]));
}
const NEUTRAL_TOOLS = invert(TOOLS);
const NEUTRAL_EVENTS = invert(CLAUDE_EVENTS);
const NEUTRAL_MODES: Record<string, string> = {
  ...invert(CLAUDE_PERMISSION_MODES),
  manual: "default",
};

/** Native tool name → neutral (`Read` → `read`); MCP tools keep their name; undefined if unknown. */
export function neutralTool(native: string): string | undefined {
  return native.startsWith("mcp__") ? native : NEUTRAL_TOOLS[native];
}

export function neutralEvent(native: string): NeutralEvent | undefined {
  return NEUTRAL_EVENTS[native] as NeutralEvent | undefined;
}

export function neutralPermissionMode(native: string): PermissionMode | undefined {
  return NEUTRAL_MODES[native] as PermissionMode | undefined;
}

/** Claude `Tool(pattern)` or `Tool` → neutral `tool:pattern`; undefined if the tool is unknown. */
export function neutralRule(rule: string): string | undefined {
  const match = /^([A-Za-z_][\w-]*)(?:\((.*)\))?$/s.exec(rule.trim());
  const tool = match?.[1] ? neutralTool(match[1]) : undefined;
  if (!tool) return undefined;
  return match?.[2] === undefined ? tool : `${tool}:${match[2]}`;
}
