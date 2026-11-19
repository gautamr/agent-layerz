/** Neutral tool vocabulary used in agent `tools`, hook `match.tools` and permission rules. */
export const NEUTRAL_TOOLS = [
  "read",
  "write",
  "edit",
  "grep",
  "glob",
  "bash",
  "web_fetch",
  "web_search",
  "agent",
  "notebook_edit",
  "todo_write",
  "skill",
] as const;
export type NeutralTool = (typeof NEUTRAL_TOOLS)[number];

export function isKnownTool(name: string): boolean {
  return (NEUTRAL_TOOLS as readonly string[]).includes(name) || name.startsWith("mcp__");
}

export const NEUTRAL_EVENTS = [
  "session_start",
  "pre_tool",
  "post_tool",
  "stop",
  "subagent_stop",
  "prompt_submit",
] as const;
export type NeutralEvent = (typeof NEUTRAL_EVENTS)[number];

/** §7.5: the repo root in a hook command; each adapter replaces it with its own variable. */
// biome-ignore lint/suspicious/noTemplateCurlyInString: a literal placeholder, expanded by adapters.
export const PROJECT_DIR_VAR = "${PROJECT_DIR}";

export const TOOL_EVENTS: readonly NeutralEvent[] = ["pre_tool", "post_tool"];

/** Neutral permission modes, ordered from loosest to strictest. */
export const PERMISSION_MODES = [
  "bypass_permissions",
  "auto",
  "accept_edits",
  "default",
  "dont_ask",
  "plan",
] as const;
export type PermissionMode = (typeof PERMISSION_MODES)[number];

export function strictness(mode: PermissionMode): number {
  return PERMISSION_MODES.indexOf(mode);
}

export const SKILL_NAME = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
