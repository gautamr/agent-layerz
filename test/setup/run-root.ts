import { mkdirSync, mkdtempSync, realpathSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { TestProject } from "vitest/node";
import { buildReference, checkedAlzBin } from "./alz-bin";
import { removeTree } from "./remove-tree";
import { ROOT_PREFIX, removeStaleRoots } from "./stale-roots";

declare module "vitest" {
  export interface ProvidedContext {
    /** The one folder of this run: `bin/`, `tmp/` and a `pid` file. Removed at the end. */
    runRoot: string;
    /** The binary the acceptance suite spawns. */
    alzBin: string;
    /** True when that binary is the TypeScript reference built for this run (ALZ_BIN unset). */
    alzIsReference: boolean;
  }
}

/**
 * Global setup for both projects. Everything the tests make lives under one run root in the temp
 * folder, which teardown removes whether the tests pass or fail. Stale roots of killed runs go
 * first, and nothing else in the temp folder is touched.
 */
export default function setup(project: TestProject): () => void {
  const given = process.env.ALZ_BIN;
  const checked = given === undefined ? undefined : checkedAlzBin(given);
  // Resolved: a spawned binary sees its working directory by its real path (on macOS /private/var/...,
  // not the link /var/...), and the cases build the paths they expect from the workspace root.
  const temp = realpathSync(tmpdir());
  removeStaleRoots(temp);
  const root = mkdtempSync(join(temp, ROOT_PREFIX));
  try {
    writeFileSync(join(root, "pid"), `${process.pid}\n`);
    mkdirSync(join(root, "bin"));
    mkdirSync(join(root, "tmp"));
    const bin = checked ?? buildReference(join(root, "bin"));
    project.provide("runRoot", root);
    project.provide("alzBin", bin);
    project.provide("alzIsReference", checked === undefined);
  } catch (error) {
    removeTree(root);
    throw error;
  }
  return () => removeTree(root);
}
