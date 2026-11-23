import { inject, it } from "vitest";

/**
 * A case the TypeScript reference is known to fail (decision-30): `src/` is never changed, so a
 * bug found there is fixed only in Rust and the case asserts the spec. Against the reference built
 * for this run (ALZ_BIN unset) the case must fail, and vitest reports it as an expected failure;
 * if the reference ever passes it, the run fails and the marker is stale. Against any binary given
 * as ALZ_BIN the case is an ordinary test and must pass. `task` names the task that records the
 * drift.
 */
export function referenceDrift(task: string, name: string, test: () => Promise<void> | void): void {
  const title = `${name} [reference drift: ${task}]`;
  if (inject("alzIsReference")) it.fails(title, test);
  else it(title, test);
}
