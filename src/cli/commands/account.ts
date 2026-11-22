import { type Diagnostic, formatDiagnostic } from "../../core/diagnostics";
import { displayPath, quote } from "../../core/sanitize";
import type { AccountMap } from "../../model/account";
import { describeTarget, fileLine, repositoryLine } from "../../resolve/account-messages";
import { resolveAccounts } from "../../resolve/account-resolution";
import { buildContext, type Io } from "../io";

export interface AccountWhichOptions {
  /** The `alz use` choice the shell function passes; nothing in the environment is read as one. */
  use: AccountMap;
}

/**
 * `alz account which`: what the shell hook would apply per target, and why. Read-only: it never
 * writes, runs git or fetches. Exit 0 when every target resolves (warnings included), 1 when
 * any target fails closed.
 */
export function accountWhichCommand(io: Io, options: AccountWhichOptions): number {
  const { home } = buildContext(io);
  const result = resolveAccounts({
    cwd: io.cwd,
    home,
    env: io.env,
    uid: process.getuid?.(),
    use: options.use,
  });
  const line = (text: string) => io.stdout(`${text}\n`);
  line(repositoryLine(result.repository, home));
  line(fileLine("accounts.yaml", result.accountsFile, home));
  line(fileLine("allowed.yaml", result.allowedFile, home, result.allowedFile.allowed));
  // Approvals for a root that does not exist here match nothing, but stay in the file.
  for (const root of result.staleApprovals) {
    line(`  stale approval: ${quote(displayPath(root, home))} does not exist here`);
  }
  for (const account of result.targets) {
    for (const text of describeTarget(account, result, home)) line(text);
  }
  for (const warning of result.warnings) {
    const item: Diagnostic = {
      level: "warning",
      code: `account-${warning.code}`,
      message: warning.text,
    };
    io.stderr(`${formatDiagnostic(item)}\n`);
  }
  return result.targets.some((account) => account.failsClosed) ? 1 : 0;
}
