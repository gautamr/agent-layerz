import { formatDiagnostic } from "../../core/diagnostics";
import { inspectIdentity } from "../../identity/inspect";
import { identityDiagnostics, repoStateLine } from "../../identity/report";
import type { DeclaredIdentity } from "../../merge/commit-identity";
import { LOCK_FILE } from "../../model/lock";
import { computeBuild } from "../../pipeline";
import { planWorkspace } from "../../render/workspace";
import { allowHint } from "../../resolve/account-allow-hint";
import { buildContext, type Io, printDiagnostics } from "../io";
import { personAtTerminal } from "../terminal";
import { collisionMessage } from "./build";

/** `alz check [--skip-identity]`: rebuild in memory and fail if the repo, the lock or the git identity differs (CI, Stop hook). */
export function checkCommand(io: Io, options: { skipIdentity: boolean }): number {
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
  // §7.13 'Trust': only when this comparison passed, because `alz account allow` needs the
  // lock a build writes; whatever the identity result.
  if (problems.length === 0 && personAtTerminal(io)) {
    const lines = allowHint({
      repoRoot: ctx.repoRoot,
      stack: result.manifest.stack,
      home: ctx.home,
      env: io.env,
      uid: process.getuid?.(),
      declares: result.config.account !== undefined,
    });
    for (const line of lines) io.stderr(`note [account-allow] ${line}\n`);
  }
  const identityFailed =
    !options.skipIdentity && reportIdentity(io, ctx.repoRoot, result.config.commitIdentity);
  if (problems.length > 0) {
    io.stderr("check failed: run `alz build` and commit the result\n");
    return 1;
  }
  if (identityFailed) {
    io.stderr("check failed: the git identity differs from the declared commit identity\n");
    return 1;
  }
  io.stdout(
    `check passed: ${result.tree.paths().length} generated files match the layers and the lock\n`,
  );
  return 0;
}

/** §9.4: prints where a wrong effective identity comes from; true when there is one. */
function reportIdentity(io: Io, repoRoot: string, declared: DeclaredIdentity | undefined): boolean {
  if (!declared) return false;
  const inspection = inspectIdentity(repoRoot, declared.identity, io.env);
  const line = repoStateLine(inspection.state, "error");
  if (line) {
    io.stderr(line);
    return inspection.state === "no-git";
  }
  const items = identityDiagnostics("error", inspection.findings, declared);
  for (const item of items) io.stderr(`${formatDiagnostic(item)}\n`);
  return items.length > 0;
}
