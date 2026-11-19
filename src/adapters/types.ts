import type { AdoptionPlan } from "../adopt/plan";
import type { Diagnostics } from "../core/diagnostics";
import type { ResolvedConfig } from "../model/config";
import type { FileTree } from "../render/file-tree";

/** A target adapter maps the neutral model onto native files (§8). */
export interface Adapter {
  target: string;
  /** Installed versions verified against: `>= min` and `< below` (docs/target-facts.md). */
  verified: { min: string; below: string };
  /** Directories the adapter owns: any file in them that the tool did not generate is a collision. */
  managedDirs: string[];
  /** Target-owned globs the build never writes or deletes, such as per-agent memory (§4.5). */
  protectedPaths: string[];
  render(config: ResolvedConfig, tree: FileTree, diags: Diagnostics): void;
  /** Hand-written native files `--adopt` takes even when the build does not collide with them. */
  adoptAlways?: string[];
  /** `--adopt` (§4.3): queues repo-layer sources for the native files it knows; returns those paths. */
  adopt?(paths: string[], plan: AdoptionPlan): string[];
  /** Agent and skill names the target will see, for `alz doctor`. */
  builtNames(config: ResolvedConfig): { agents: string[]; skills: string[] };
}
