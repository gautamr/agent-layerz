import type { Layer } from "../model/layer";
import { type LockedLayer, lockedContentHash } from "../model/lock";

/**
 * The lock entry a cache read is checked against: the one for the same source whose commit is
 * the one just resolved. A different `sha` means the ref moved, so new content is expected and
 * there is nothing to compare; a source with no entry has not been locked yet.
 *
 * `build` and the commands that rebuild treat "no entry" as "skip", because new content is
 * expected. Account resolution (§7.13, TASK-50.2) must not: it has to treat anything it cannot
 * verify (no entry, a different sha, a missing field) as a miss that fails closed.
 */
export function lockedEntryFor(
  locked: LockedLayer[],
  source: string,
  sha: string | undefined,
): LockedLayer | undefined {
  if (sha === undefined) return undefined;
  const entry = locked.find((layer) => layer.source === source);
  return entry?.sha === sha ? entry : undefined;
}

/**
 * Is a layer read from the cache what the lock recorded? `miss` means the content hash differs,
 * so the entry was edited or replaced after it was written. Pure: the caller picks the entry
 * (see `lockedEntryFor`) and decides what a miss means: `alz build` stops with an error, account
 * resolution (§7.13) fails closed. A caller that cannot select an entry has nothing verified:
 * account resolution must count that as a miss too, not as "ok".
 */
export function verifyCachedLayer(
  layer: Pick<Layer, "contentHash">,
  lockedEntry: Pick<LockedLayer, "contentHash">,
): "ok" | "miss" {
  return lockedContentHash(layer) === lockedEntry.contentHash ? "ok" : "miss";
}
