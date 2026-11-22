import { homedir } from "node:os";
import { isAbsolute } from "node:path";
import { type Diagnostic, formatDiagnostic } from "../../core/diagnostics";
import { agentSessionMarkers } from "../../core/env";
import { displayPath, quoteEscaped } from "../../core/sanitize";
import { ACCOUNT_TARGETS, type AccountMap, type AccountTarget } from "../../model/account";
import {
  type AllowedEntry,
  entryFor,
  loadAllowState,
  sameEntry,
  stateProblem,
} from "../../resolve/account-allow-state";
import { allowedPlacementProblem, updateAllowed } from "../../resolve/account-allow-writer";
import type { AccountDeclaration } from "../../resolve/account-declaration";
import { BLOCKED_NAME, listAccountNames } from "../../resolve/account-folders";
import { missPhrase, repositoryLine } from "../../resolve/account-messages";
import { readRepositoryDeclaration } from "../../resolve/account-resolution";
import { findRepositoryRoot } from "../../resolve/account-root";
import { WarningList } from "../../resolve/account-warning";
import type { Io } from "../io";

const TARGETS = Object.keys(ACCOUNT_TARGETS) as AccountTarget[];
const BASE_COMMAND = "alz account allow";

/** The distinct names of a map, in target order: one confirmation is asked for each. */
function distinctNames(map: AccountMap): string[] {
  const names: string[] = [];
  for (const target of TARGETS) {
    const name = map[target];
    if (name !== undefined && !names.includes(name)) names.push(name);
  }
  return names;
}

function describeMap(map: AccountMap): string {
  return TARGETS.filter((target) => map[target] !== undefined)
    .map((target) => `${target}=${map[target]}`)
    .join(", ");
}

/** The answer without surrounding spaces and tabs: the only thing that is compared. */
function trimBlanks(text: string): string {
  return text.replace(/^[ \t]+|[ \t]+$/g, "");
}

type Declared = Extract<AccountDeclaration, { status: "read" }>["declared"];

/**
 * `alz account allow [--remove]` (LAYERING-SPEC §7.13 'Trust and alz account allow'): the one
 * command that writes `allowed.yaml`. It runs only when a person is at a terminal outside an
 * agent session, and checks that before any file system call. It shows what it would record,
 * asks the person to type each account name, reads the declaration again and only then writes,
 * through `updateAllowed`. Exit 0 done or nothing to do, 1 refused or not confirmed, nothing
 * changed. Repository text is printed only through `quote` and `displayPath`.
 */
export async function accountAllowCommand(io: Io, options: { remove: boolean }): Promise<number> {
  const out = (text: string) => io.stdout(`${text}\n`);
  const fail = (text: string): number => {
    io.stderr(`error [account-allow] ${text}\n`);
    return 1;
  };
  const verb = options.remove ? `${BASE_COMMAND} --remove` : BASE_COMMAND;

  // Before any file system call: nothing below runs without a person at a terminal.
  const markers = agentSessionMarkers(io.env);
  const { terminal } = io;
  if (terminal === undefined || markers.length > 0) {
    const reasons = [
      ...(terminal === undefined ? ["standard input is not a terminal"] : []),
      ...(markers.length > 0
        ? [`an agent session is running here (${markers.join(", ")} set)`]
        : []),
    ];
    io.stderr(
      `error [account-allow] ${verb} must be run by a person at a terminal: ${reasons.join("; ")}\n`,
    );
    if (markers.includes("CLAUDECODE")) {
      io.stderr(
        "Claude Code also sets CLAUDECODE in an IDE's integrated terminal; run alz account allow in a terminal window outside the IDE\n",
      );
    }
    io.stderr("nothing was changed\n");
    return 1;
  }

  const home = io.env.HOME ?? homedir();
  if (!isAbsolute(home)) return fail("HOME is not an absolute path; nothing was changed");
  const uid = process.getuid?.();
  const warnings = new WarningList();
  const printWarnings = () => {
    for (const warning of warnings.items) {
      const item: Diagnostic = {
        level: "warning",
        code: `account-${warning.code}`,
        message: warning.text,
      };
      io.stderr(`${formatDiagnostic(item)}\n`);
    }
    warnings.items.length = 0;
  };

  const repository = findRepositoryRoot(io.cwd, home, io.env, warnings);
  printWarnings();
  if (repository.status === "none") {
    return fail(`${repositoryLine(repository, home)}; run it inside a repository`);
  }
  const root = repository.root;
  const shownRoot = displayPath(root, home);
  if (!options.remove && repository.manifest !== "read") {
    return fail(`repair ${shownRoot}/layer.yaml first; it cannot be used`);
  }

  const files = { home, env: io.env, uid, root };
  const loaded = loadAllowState(files, warnings);
  printWarnings();
  const unusable = stateProblem(loaded);
  if (unusable !== undefined) return fail(`${unusable}; nothing was changed`);
  const current = entryFor(loaded.state, root);

  // The prompt names the root and the source itself: the details above go to standard output,
  // the prompt to standard error, and either may be redirected.
  const confirm = async (names: string[], what: string, source: string): Promise<boolean> => {
    for (const name of names) {
      const answer = await terminal.ask(
        `Type ${name} to ${what} for ${quoteEscaped(root)} (source ${quoteEscaped(source)}): `,
      );
      if (answer === undefined || trimBlanks(answer) !== name) {
        io.stderr("not confirmed; nothing changed\n");
        return false;
      }
    }
    return true;
  };

  if (options.remove) {
    if (current === undefined) {
      out(`nothing to remove: ${shownRoot} has no entry in allowed.yaml`);
      return 0;
    }
    const problem = allowedPlacementProblem({
      change: { kind: "remove" },
      repoRoot: root,
      home,
      env: io.env,
      uid,
    });
    if (problem !== undefined) return fail(`allowed.yaml cannot be changed: ${problem}`);
    out(`repository: ${shownRoot}`);
    out(`allowed now: ${describeMap(current.account)} from ${quoteEscaped(current.source)}`);
    out(
      "removing it sends this repository back to the default account unless accounts.yaml names one",
    );
    if (!(await confirm(distinctNames(current.account), "remove it", current.source))) return 1;
    const result = await updateAllowed({
      change: { kind: "remove" },
      repoRoot: root,
      home,
      env: io.env,
      uid,
    });
    if (result.kind === "refused") return fail(result.reason);
    if (result.kind === "unchanged") {
      out(`nothing to remove: ${shownRoot} has no entry in allowed.yaml`);
      return 0;
    }
    out(`removed the entry for ${shownRoot} from allowed.yaml`);
    return 0;
  }

  if (repository.manifest !== "read") return fail("layer.yaml cannot be used");
  const read = readRepositoryDeclaration(root, repository.stack, home, io.env);
  const { declaration } = read;
  if (read.miss !== undefined) {
    return fail(
      `the layer cache cannot be verified (${missPhrase(read.miss)}); run alz build; nothing was changed`,
    );
  }
  if (declaration.status === "miss") {
    return fail("the layer cache cannot be verified; run alz build; nothing was changed");
  }
  for (const ignored of declaration.ignored) {
    io.stderr(
      `${formatDiagnostic({
        level: "warning",
        code: "account-declaration-ignored",
        message: `the layer ${quoteEscaped(ignored.layer.name)} (${quoteEscaped(ignored.layer.source)}) declares an account but may not, so it is ignored: ${ignored.reasons.join("; ")}`,
      })}\n`,
    );
  }
  if (declaration.status === "conflict") {
    const names = declaration.clients.map((client) => quoteEscaped(client.name)).join(", ");
    return fail(
      `several client layers declare an account (${names}); remove one declaration, then run alz build; nothing was changed`,
    );
  }
  const declared = declaration.declared;
  if (declared === undefined) {
    return fail(
      `the layers name no account here; nothing was changed${current === undefined ? "" : `; to withdraw the earlier approval run: ${BASE_COMMAND} --remove`}`,
    );
  }
  if (TARGETS.some((target) => declared.account[target] === BLOCKED_NAME)) {
    return fail(
      `the name ${BLOCKED_NAME} is reserved for the blocked folders; the layer must name another account; nothing was changed`,
    );
  }

  const entry: AllowedEntry = {
    root,
    account: declared.account,
    source: declared.recordedSource,
  };
  const problem = allowedPlacementProblem({
    change: { kind: "set", entry },
    repoRoot: root,
    home,
    env: io.env,
    uid,
  });
  if (problem !== undefined) return fail(`allowed.yaml cannot be written: ${problem}`);
  if (current !== undefined && sameEntry(current, entry)) {
    out(`already allowed: ${shownRoot}`);
    return 0;
  }

  show(io, { shownRoot, declared, current, home });
  const names = distinctNames(declared.account);
  if (!(await confirm(names, "allow it", entry.source))) return 1;

  // The person took time: the repository, its layer.yaml and the declaration must still be
  // what was shown, so everything is read again, from the working directory.
  const changed = `the declaration changed while you were confirming; run ${BASE_COMMAND} again`;
  const second = findRepositoryRoot(io.cwd, home, io.env, new WarningList());
  if (second.status !== "found" || second.root !== root || second.manifest !== "read") {
    return fail(changed);
  }
  const again = readRepositoryDeclaration(root, second.stack, home, io.env);
  const now = again.declaration.status === "read" ? again.declaration.declared : undefined;
  if (again.miss !== undefined || now === undefined || !sameDeclared(declared, now)) {
    return fail(changed);
  }

  const result = await updateAllowed({
    change: { kind: "set", entry },
    repoRoot: root,
    home,
    env: io.env,
    uid,
  });
  if (result.kind === "refused") return fail(result.reason);
  if (result.kind === "unchanged") {
    out(`already allowed: ${shownRoot}`);
    return 0;
  }
  out(
    `allowed: ${shownRoot} uses ${describeMap(entry.account)} from ${quoteEscaped(entry.source)}`,
  );
  const existing = listAccountNames(home);
  for (const target of TARGETS) {
    const name = entry.account[target];
    if (name === undefined || name === "default" || existing[target].includes(name)) continue;
    out(
      `note: the ${target} account folder for ${name} does not exist yet; create it with: alz account init ${name}`,
    );
  }
  return 0;
}

function sameDeclared(a: NonNullable<Declared>, b: NonNullable<Declared>): boolean {
  return (
    sameEntry(
      { root: "", account: a.account, source: a.recordedSource },
      { root: "", account: b.account, source: b.recordedSource },
    ) &&
    a.sha === b.sha &&
    a.ref === b.ref &&
    a.layer.index === b.layer.index &&
    a.layer.name === b.layer.name &&
    a.layer.source === b.layer.source
  );
}

interface Shown {
  shownRoot: string;
  declared: NonNullable<Declared>;
  current: AllowedEntry | undefined;
  home: string;
}

/** What `allow` is about to record, printed before the person is asked to confirm. */
function show(io: Io, { shownRoot, declared, current, home }: Shown): void {
  const out = (text: string) => io.stdout(`${text}\n`);
  const existing = listAccountNames(home);
  out(`repository: ${shownRoot}`);
  out("accounts the layers name:");
  for (const target of TARGETS) {
    const name = declared.account[target];
    if (name === undefined) {
      out(`  ${target}: not named by the layers (the other sources decide)`);
      continue;
    }
    const folder =
      name === "default"
        ? "the default account"
        : existing[target].includes(name)
          ? "the account folder exists"
          : "the account folder does not exist yet";
    out(`  ${target}: ${name} (${folder})`);
  }
  out(
    `declared by the layer ${quoteEscaped(declared.layer.name)}, stack entry ${declared.layer.index + 1}`,
  );
  out(`  source as written: ${quoteEscaped(declared.layer.source)}`);
  if (declared.ref !== undefined) out(`  ref: ${quoteEscaped(declared.ref)}`);
  out(`  revision: ${declared.sha === undefined ? "none (unpinned path: layer)" : declared.sha}`);
  out(`  recorded source: ${quoteEscaped(declared.recordedSource)}`);
  if (current !== undefined) {
    out(`currently allowed: ${describeMap(current.account)} from ${quoteEscaped(current.source)}`);
  }
  out("Claude Code and Codex started here will use these accounts' logins and MCP connections.");
}
