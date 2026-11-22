import { type Diagnostics, FatalError } from "../core/diagnostics";
import { stripControl } from "../core/sanitize";
import type { Layer } from "../model/layer";
import { privacyRank, type Visibility } from "../model/layer-meta";
import { LOCK_FILE } from "../model/lock";
import type { Manifest } from "../model/manifest";
import { lockedEntryFor, verifyCachedLayer } from "./cache-integrity";
import { loadLayer, validateStack } from "./layer-load";
import { type ResolveContext, resolveSource } from "./sources";

export { loadLayer, validateStack };

/**
 * §5.3: what to run on the content hash of a layer read from the cache. It compares with the
 * lock's entry for that source and commit, and does nothing when there is none (build and the
 * other commands that rebuild treat "no entry" as "new content": see `lockedEntryFor`).
 */
function cacheCheck(
  source: string,
  dir: string,
  sha: string | undefined,
  ctx: ResolveContext,
): (contentHash: string) => void {
  return (contentHash) => {
    const entry = lockedEntryFor(ctx.locked, source, sha);
    if (!entry || verifyCachedLayer({ contentHash }, entry) === "ok") return;
    // Named by the lock and the source: the layer's own files are not trusted, or read, yet.
    throw new FatalError(
      "cache",
      `cached layer '${stripControl(entry.name)}' (${stripControl(source)}) no longer matches ${LOCK_FILE}: its files in ${dir} changed after they were cached; delete that directory and rebuild`,
      source,
    );
  };
}

/** Resolves every stack entry and validates the stack (§6 steps 1–2). */
export function loadStack(manifest: Manifest, ctx: ResolveContext, diags: Diagnostics): Layer[] {
  const layers = manifest.stack.map((entry) => {
    const resolved = resolveSource(entry, ctx);
    const verify = resolved.cached
      ? cacheCheck(entry.source, resolved.dir, resolved.sha, ctx)
      : undefined;
    if (resolved.indexWarning) diags.warn("cache-index", resolved.indexWarning, entry.source);
    const layer = loadLayer(resolved.dir, entry.source, diags, verify);
    return { ...layer, rev: resolved.rev, sha: resolved.sha };
  });
  validateStack(layers, diags);
  return layers;
}

/** The output inherits the most private visibility in the stack. */
export function stackVisibility(layers: Layer[]): Visibility {
  return layers.reduce<Visibility>(
    (most, layer) =>
      privacyRank(layer.meta.visibility) > privacyRank(most) ? layer.meta.visibility : most,
    "public",
  );
}
