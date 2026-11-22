import { displayPath, stripControl } from "../core/sanitize";
import type { CommitIdentity } from "../model/commit-identity";
import { type Env, git } from "../resolve/git";

export const FIELDS = ["name", "email"] as const;
export const ROLES = ["author", "committer"] as const;
export type IdentityField = (typeof FIELDS)[number];
export type IdentityRole = (typeof ROLES)[number];

const GIT_SCOPES = ["system", "global", "local", "worktree", "command"] as const;
/** Git's config scopes, plus `environment` (variables), `unset` (guessed) and `unknown` (git failed). */
export type IdentityScope = (typeof GIT_SCOPES)[number] | "environment" | "unset" | "unknown";

/**
 * Where one part of the effective identity comes from. It deliberately has no value field: the
 * wrong value is compared in memory and dropped, so no report can echo it (§9.4).
 */
export interface Finding {
  field: IdentityField;
  role: IdentityRole;
  /** A config key (`user.email`) or an environment variable (`GIT_AUTHOR_EMAIL`). */
  key: string;
  scope: IdentityScope;
  /** Source file as git reports it, sanitised; HOME shown as `~`. */
  origin: string;
}

/** `repo`, `no-repo` (git says: not a repository) or `no-git` (git is missing or failed otherwise). */
export type RepoState = "repo" | "no-repo" | "no-git";

export interface Inspection {
  state: RepoState;
  /** Empty when the effective identity matches the declared one. */
  findings: Finding[];
}

/** Variables that could point a git subprocess at another repository or config file. */
const REDIRECTING = ["GIT_DIR", "GIT_WORK_TREE", "GIT_COMMON_DIR", "GIT_CONFIG", "GIT_INDEX_FILE"];
const IDENT = /^(.*) <([^<>]*)> \d+ [+-]\d{4}$/s;

/** The environment identity subprocesses run in: the inherited one, minus redirecting variables. */
export function identityEnv(env: Env): Env {
  const clean = { ...env };
  for (const key of REDIRECTING) delete clean[key];
  return clean;
}

/** The origin git prints, as a report may show it: no control characters, HOME as `~`. */
export function sanitizeOrigin(origin: string, home: string | undefined): string {
  return displayPath(stripControl(origin).replace(/^file:/, ""), home);
}

function exitStatus(error: unknown): number | null | undefined {
  return (error as { status?: number | null }).status;
}

/**
 * Is `repoRoot` inside a git repository? Only git's own "not a git repository" answer means no.
 * A missing git, or a failure such as a broken config file, is `no-git`, never a quiet pass.
 */
export function repoState(repoRoot: string, env: Env): RepoState {
  try {
    git(["rev-parse", "--git-dir"], { ...identityEnv(env), LC_ALL: "C" }, repoRoot);
    return "repo";
  } catch (error) {
    const stderr = String((error as { stderr?: unknown }).stderr ?? "");
    const refused = exitStatus(error) === 128 && stderr.includes("not a git repository");
    return refused ? "no-repo" : "no-git";
  }
}

function envVar(role: IdentityRole, field: IdentityField): string {
  return `GIT_${role.toUpperCase()}_${field.toUpperCase()}`;
}

/** `git var GIT_<ROLE>_IDENT`, parsed; undefined when git has no identity or fails. */
function effective(role: IdentityRole, cwd: string, env: Env): CommitIdentity | undefined {
  try {
    const match = IDENT.exec(git(["var", `GIT_${role.toUpperCase()}_IDENT`], env, cwd));
    return match ? { name: match[1] ?? "", email: match[2] ?? "" } : undefined;
  } catch {
    return undefined;
  }
}

function toScope(scope: string): IdentityScope {
  return (GIT_SCOPES as readonly string[]).includes(scope) ? (scope as IdentityScope) : "unknown";
}

/** Git's own lookup order for one value: environment, `<role>.*`, `user.*`, `EMAIL`, guessed. */
function locate(
  role: IdentityRole,
  field: IdentityField,
  cwd: string,
  env: Env,
  gitEnv: Env,
): Finding {
  const variable = envVar(role, field);
  if (env[variable] !== undefined) {
    return { field, role, key: variable, scope: "environment", origin: "environment variable" };
  }
  for (const key of [`${role}.${field}`, `user.${field}`]) {
    try {
      const line = git(["config", "--show-scope", "--show-origin", "--get", key], gitEnv, cwd);
      // `<scope>\t<origin>\t<value>`: only the first two fields are kept, the value is dropped.
      const [scope = "", origin = ""] = line.split("\n")[0]?.split("\t") ?? [];
      return { field, role, key, scope: toScope(scope), origin: sanitizeOrigin(origin, env.HOME) };
    } catch (error) {
      // Only exit status 1 means "not set"; any other failure leaves the source unknown.
      if (exitStatus(error) !== 1) {
        return { field, role, key, scope: "unknown", origin: "git could not report it" };
      }
    }
  }
  if (field === "email" && env.EMAIL !== undefined) {
    return { field, role, key: "EMAIL", scope: "environment", origin: "environment variable" };
  }
  return { field, role, key: `user.${field}`, scope: "unset", origin: "guessed from the host" };
}

/**
 * §9.4: compares what git would use for a commit in `repoRoot` with the declared identity and,
 * for every part that differs, finds where git takes it from. Runs only git subprocesses, with
 * the redirecting variables removed, and writes nothing.
 */
export function inspectIdentity(repoRoot: string, declared: CommitIdentity, env: Env): Inspection {
  const state = repoState(repoRoot, env);
  if (state !== "repo") return { state, findings: [] };
  const gitEnv = identityEnv(env);
  const findings: Finding[] = [];
  for (const role of ROLES) {
    const found = effective(role, repoRoot, gitEnv);
    for (const field of FIELDS) {
      // No identity at all (git failed) counts as a mismatch of both parts.
      if (found && found[field] === declared[field]) continue;
      findings.push(locate(role, field, repoRoot, env, gitEnv));
    }
  }
  return { state, findings };
}
