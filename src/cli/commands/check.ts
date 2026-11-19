import { LOCK_FILE } from "../../model/lock";
import { computeBuild } from "../../pipeline";
import { planWorkspace } from "../../render/workspace";
import { buildContext, type Io, printDiagnostics } from "../io";
import { collisionMessage } from "./build";

/** `alz check`: rebuild in memory and fail if the repo or the lock differs (CI, Stop hook). */
export function checkCommand(io: Io): number {
  const ctx = buildContext(io);
  const result = computeBuild(ctx);
  printDiagnostics(io, result.diags.items);
  if (result.diags.hasErrors) return 1;

  const plan = planWorkspace(ctx.repoRoot, result);
  const problems = [
    ...plan.changes.map((change) => {
      const what = { create: "missing", update: "differs from a fresh build", delete: "is stale" }[
        change.kind
      ];
      return `error [check] ${change.path}: ${what}\n`;
    }),
    ...plan.collisions.map(collisionMessage),
    ...(plan.lockChanged ? [`error [check] ${LOCK_FILE}: out of date\n`] : []),
  ];
  for (const problem of problems) io.stderr(problem);
  if (problems.length > 0) {
    io.stderr("check failed: run `alz build` and commit the result\n");
    return 1;
  }
  io.stdout(
    `check passed: ${result.tree.paths().length} generated files match the layers and the lock\n`,
  );
  return 0;
}
