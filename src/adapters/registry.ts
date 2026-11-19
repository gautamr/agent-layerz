import { claudeAdapter } from "./claude";
import type { Adapter } from "./types";

const ADAPTERS: Adapter[] = [claudeAdapter];

/** Targets named in the spec whose adapter is not part of this build. */
const PLANNED = ["codex"];

export function findAdapter(target: string): Adapter | { missing: string } {
  const adapter = ADAPTERS.find((candidate) => candidate.target === target);
  if (adapter) return adapter;
  const implemented = ADAPTERS.map((candidate) => candidate.target).join(", ");
  return {
    missing: PLANNED.includes(target)
      ? `the '${target}' adapter is not implemented yet (implemented: ${implemented})`
      : `unknown target '${target}' (implemented: ${implemented})`,
  };
}
