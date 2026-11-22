/** A process environment. */
export type Env = Record<string, string | undefined>;

/** The variables that exist while mise or direnv has changed the environment (§7.13). */
const MANAGER_VARIABLES = ["__MISE_DIFF", "DIRENV_DIFF"] as const;

/**
 * Is mise or direnv active in this environment? Only the existence of `__MISE_DIFF` or
 * `DIRENV_DIFF` is checked, with `Object.hasOwn` and nothing else: each holds every variable the
 * manager set, secrets included, so its value is never read, decoded, printed or logged
 * (decision-17). The value is never indexed by name.
 */
export function envManagerActive(env: Env): boolean {
  return MANAGER_VARIABLES.some((name) => Object.hasOwn(env, name));
}

/**
 * The variables that mean a process runs inside an agent session (target-facts §12 item 19,
 * LAYERING-SPEC §7.13 'Agent-session markers').
 */
export const AGENT_SESSION_MARKERS = [
  "CLAUDECODE",
  "CLAUDE_CODE_CHILD_SESSION",
  "AI_AGENT",
  "CODEX_THREAD_ID",
  "CODEX_SESSION_ID",
  "CODEX_CI",
] as const;

/**
 * The agent-session markers that are set, in the order above. A marker counts even when its
 * value is empty. Only the existence is checked, with `Object.hasOwn`: a value is never read,
 * printed or logged, and a marker proves presence only (its absence proves nothing).
 */
export function agentSessionMarkers(env: Env): string[] {
  return AGENT_SESSION_MARKERS.filter((name) => Object.hasOwn(env, name));
}
