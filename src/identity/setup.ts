import { lstatSync, realpathSync } from "node:fs";
import { formatDiagnostic } from "../core/diagnostics";
import { insideRoot, isWithin } from "../core/files";
import type { DeclaredIdentity } from "../merge/commit-identity";
import { type Env, git } from "../resolve/git";
import { FIELDS, type Inspection, identityEnv, inspectIdentity, repoState } from "./inspect";
import { declaredBy, identityDiagnostics, repoStateLine, sentenceFor, setCommand } from "./report";

/** Where setup reports go; a subset of the CLI's `Io`. */
export interface Out {
  stdout: (text: string) => void;
  stderr: (text: string) => void;
}

interface Blocker {
  reason: string;
  /** `toplevel`: commands here would write to another repository, so none are printed. */
  kind: "toplevel" | "other";
}

/** Why alz may not write `.git/config` in `repoRoot`; undefined when it may (§9.4). */
function writeBlocker(repoRoot: string, env: Env): Blocker | undefined {
  try {
    const top = git(["rev-parse", "--show-toplevel"], env, repoRoot);
    if (realpathSync.native(top) !== realpathSync.native(repoRoot)) {
      return {
        reason: "the repository root is not the top level of its work tree",
        kind: "toplevel",
      };
    }
    const dotGit = lstatSync(insideRoot(repoRoot, ".git"));
    if (dotGit.isSymbolicLink() || !dotGit.isDirectory()) {
      return {
        reason: ".git is not a plain directory (a linked worktree, submodule or symlink)",
        kind: "other",
      };
    }
    const config = insideRoot(repoRoot, ".git/config");
    let file: ReturnType<typeof lstatSync> | undefined;
    try {
      file = lstatSync(config);
    } catch (error) {
      if ((error as { code?: string }).code !== "ENOENT") throw error;
    }
    if (file) {
      // A link here would let a write land in another file, such as one in the home directory.
      if (!file.isFile() || file.isSymbolicLink()) {
        return { reason: ".git/config is not a regular file", kind: "other" };
      }
      if (!isWithin(realpathSync.native(repoRoot), realpathSync.native(config))) {
        return { reason: ".git/config resolves outside the repository", kind: "other" };
      }
    }
  } catch {
    return { reason: "the repository could not be inspected", kind: "other" };
  }
  return undefined;
}

function printFindings(
  out: Out,
  inspection: Inspection,
  declared: DeclaredIdentity,
  options: { skip?: (key: string, scope: string) => boolean; withFix?: boolean } = {},
): void {
  const findings = inspection.findings.filter(
    (finding) => !options.skip?.(finding.key, finding.scope),
  );
  for (const item of identityDiagnostics("warning", findings, declared, options.withFix)) {
    out.stderr(`${formatDiagnostic(item)}\n`);
  }
}

/** The state line when git cannot be used here; true when it printed one. */
function reportUnusable(out: Out, state: ReturnType<typeof repoState>): boolean {
  const line = repoStateLine(state, "warning");
  if (line) out.stderr(line);
  return line !== undefined;
}

/**
 * §9.4 setup (init and build --adopt): adds `user.name` and `user.email` to the repository's
 * own `.git/config` when they are missing, to the declared identity. Never replaces a value,
 * never touches global or system config, and writes only inside the repository. Afterwards it
 * reads the effective identity again and reports what still differs.
 */
export function ensureLocalIdentity(
  repoRoot: string,
  declared: DeclaredIdentity,
  env: Env,
  out: Out,
): void {
  const gitEnv = identityEnv(env);
  if (reportUnusable(out, repoState(repoRoot, env))) return;

  const blocker = writeBlocker(repoRoot, gitEnv);
  if (blocker) {
    const by = declaredBy(declared);
    const advice =
      blocker.kind === "toplevel"
        ? "Run `git init` here first, or run alz from the repository's top level."
        : `Run by hand:\n  ${FIELDS.map((field) => setCommand(`user.${field}`, declared.identity[field]) ?? sentenceFor(`user.${field}`, declared)).join("\n  ")}`;
    out.stderr(
      `warning [identity] ${blocker.reason}, so alz did not write the git identity declared by ${by}. ${advice}\n`,
    );
    // From another repository's top level a `git config` command would write somewhere else.
    printFindings(out, inspectIdentity(repoRoot, declared.identity, env), declared, {
      withFix: blocker.kind !== "toplevel",
    });
    return;
  }

  const config = insideRoot(repoRoot, ".git/config");
  const kept = new Set<string>();
  for (const field of FIELDS) {
    const key = `user.${field}`;
    const want = declared.identity[field];
    let have: string | undefined;
    try {
      have = git(["config", "--file", config, "--get", key], gitEnv, repoRoot);
    } catch (error) {
      // Exit status 1 means the key is not set in this file; anything else is a real failure.
      if ((error as { status?: number }).status !== 1) {
        out.stderr(`warning [identity] could not read .git/config; set ${key} by hand\n`);
        continue;
      }
    }
    if (have === undefined) {
      try {
        git(["config", "--file", config, key, want], gitEnv, repoRoot);
      } catch {
        out.stderr(`warning [identity] could not write ${key} to .git/config; set it by hand\n`);
        continue;
      }
      out.stdout(`set ${key} in .git/config to '${want}' (declared by ${declaredBy(declared)})\n`);
    } else if (have !== want) {
      kept.add(key);
      const fix = setCommand(key, want) ?? sentenceFor(key, declared);
      out.stderr(
        `warning [identity] ${key} in .git/config differs from the identity declared by ${declaredBy(declared)}; alz never replaces it. Fix: ${fix}\n`,
      );
    }
  }

  // An [include] or an environment variable can still win over what was just written.
  const after = inspectIdentity(repoRoot, declared.identity, env);
  printFindings(out, after, declared, {
    skip: (key, scope) => scope === "local" && kept.has(key),
  });
}

/** Plain `alz build`: report a mismatch as warnings and write nothing. */
export function warnAboutIdentity(
  repoRoot: string,
  declared: DeclaredIdentity,
  env: Env,
  out: Out,
): void {
  const inspection = inspectIdentity(repoRoot, declared.identity, env);
  if (reportUnusable(out, inspection.state)) return;
  printFindings(out, inspection, declared);
}
