import type { Diagnostic, Level } from "../core/diagnostics";
import { stripControl } from "../core/sanitize";
import type { DeclaredIdentity } from "../merge/commit-identity";
import { FIELDS, type Finding, type RepoState, ROLES } from "./inspect";

/**
 * Values a pasted `git config` command may carry in any common shell (POSIX, fish, PowerShell):
 * letters, marks, digits, space and a few punctuation marks. A value with anything else, an
 * apostrophe included, gets no command; the report names the layer instead.
 */
const SHELL_SAFE = /^[\p{L}\p{M}\p{N} ._@+-]+$/u;

export function shellQuote(value: string): string {
  return `'${value.replaceAll("'", "'\\''")}'`;
}

/** `git config --local user.email 'owner@example.com'`, or undefined when the value is not shell-safe. */
export function setCommand(
  key: string,
  value: string,
  scope: "--local" | "--worktree" = "--local",
): string | undefined {
  return SHELL_SAFE.test(value) ? `git config ${scope} ${key} ${shellQuote(value)}` : undefined;
}

/** What to do about one wrong value: a command, or a sentence when no safe command exists. */
export function fixText(finding: Finding, declared: DeclaredIdentity): string {
  if (finding.scope === "command") return "unset GIT_CONFIG_COUNT GIT_CONFIG_PARAMETERS";
  if (finding.scope === "environment" && finding.key !== "EMAIL") return `unset ${finding.key}`;
  const key = finding.key === "EMAIL" ? "user.email" : finding.key;
  const command = setCommand(
    key,
    declared.identity[finding.field],
    finding.scope === "worktree" ? "--worktree" : "--local",
  );
  return command ?? sentenceFor(key, declared);
}

/** The fallback for a value no shell command may carry. */
export function sentenceFor(key: string, declared: DeclaredIdentity): string {
  return `set ${key} to the value declared in ${declaredBy(declared)} (\`alz build --adopt\` adds it when unset)`;
}

function sourceText(finding: Finding): string {
  switch (finding.scope) {
    case "environment":
      return `the environment (${finding.key})`;
    case "unset":
      return "no setting (git guessed it from the host)";
    case "unknown":
      return "a source git could not report";
    case "command":
      return `command-line config (${finding.origin})`;
    default:
      return `${finding.scope} config (${finding.origin})`;
  }
}

/** The layer that declared the identity, and where it came from (§9.4). */
export function declaredBy(declared: DeclaredIdentity): string {
  return `layer '${declared.layer.meta.name}' (${stripControl(declared.layer.source)})`;
}

/** One finding per distinct key, in a fixed order, so the output of a setup never varies. */
export function distinctFindings(findings: Finding[]): Finding[] {
  const sorted = [...findings].sort(
    (a, b) =>
      FIELDS.indexOf(a.field) - FIELDS.indexOf(b.field) ||
      ROLES.indexOf(a.role) - ROLES.indexOf(b.role),
  );
  const seen = new Set<string>();
  return sorted.filter((finding) => {
    if (seen.has(finding.key)) return false;
    seen.add(finding.key);
    return true;
  });
}

/** Report lines for findings: where each wrong value comes from, never the value itself. */
export function identityDiagnostics(
  level: Level,
  findings: Finding[],
  declared: DeclaredIdentity,
  withFix = true,
): Diagnostic[] {
  return distinctFindings(findings).map((finding) => ({
    level,
    code: "identity",
    message: `git takes the ${finding.field} (${finding.key}) from ${sourceText(finding)}, which differs from the commit identity declared by ${declaredBy(declared)}${withFix ? `; fix: ${fixText(finding, declared)}` : ""}`,
  }));
}

/**
 * The line for a directory that is not a git repository (a note) or where git cannot run (the
 * given level, so `check` can fail on it); undefined inside a repository.
 */
export function repoStateLine(state: RepoState, level: Level): string | undefined {
  if (state === "no-repo") {
    return "note [identity] this is not a git repository, so the commit identity was not checked\n";
  }
  if (state === "no-git") {
    return `${level} [identity] git is not available or failed, so the commit identity could not be checked\n`;
  }
  return undefined;
}
