/** A neutral tool entry: `write` or `write:docs/**`. */
export interface ToolRef {
  tool: string;
  scope?: string;
}

export function parseTool(entry: string): ToolRef {
  const colon = entry.indexOf(":");
  if (colon < 0) return { tool: entry };
  return { tool: entry.slice(0, colon), scope: entry.slice(colon + 1) };
}

/**
 * True when `candidate` grants nothing beyond `base`: same tool, and either the base is
 * unscoped, the scopes are equal, or the base scope ends in a wildcard and the candidate
 * scope starts with the base's literal prefix (`docs/**` covers `docs/adr/**`).
 */
export function covers(base: ToolRef, candidate: ToolRef): boolean {
  if (base.tool !== candidate.tool) return false;
  if (base.scope === undefined) return true;
  if (candidate.scope === undefined) return false;
  if (candidate.scope === base.scope) return true;
  if (!/\*$/.test(base.scope) || candidate.scope.includes("..")) return false;
  return candidate.scope.startsWith(base.scope.replace(/\*+$/, ""));
}

/** Entries of `result` that no entry of `base` covers. An unrestricted base covers everything. */
export function widenedTools(
  base: readonly string[] | undefined,
  result: readonly string[],
): string[] {
  if (base === undefined) return [];
  const baseRefs = base.map(parseTool);
  return result.filter((entry) => !baseRefs.some((ref) => covers(ref, parseTool(entry))));
}
