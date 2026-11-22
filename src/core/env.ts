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
