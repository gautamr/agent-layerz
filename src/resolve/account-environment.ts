import type { Env } from "../core/env";
import { ACCOUNT_TARGETS, type AccountTarget } from "../model/account";

/** The marker the shell hook exports next to a variable: it holds the exact value the hook set. */
export function markerName(target: AccountTarget): string {
  return `ALZ_SET_${ACCOUNT_TARGETS[target]}`;
}

/**
 * §7.13 'Values set outside alz': the target's variable when it is foreign. A set variable
 * (an empty one included) that differs from its marker, or has none, is foreign. An unset
 * variable never is, and a marker without its variable is stale and ignored. Only the
 * account variable and its marker are read.
 */
export function foreignValue(target: AccountTarget, env: Env): string | undefined {
  const value = env[ACCOUNT_TARGETS[target]];
  if (typeof value !== "string") return undefined;
  return env[markerName(target)] === value ? undefined : value;
}
