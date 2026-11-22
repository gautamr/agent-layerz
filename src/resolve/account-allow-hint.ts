import { realpathSync } from "node:fs";
import type { Env } from "../core/env";
import { quoteEscaped } from "../core/sanitize";
import { ACCOUNT_TARGETS, type AccountMap, type AccountTarget } from "../model/account";
import type { StackEntry } from "../model/manifest";
import { entryFor, loadAllowState, sameEntry, stateProblem } from "./account-allow-state";
import { accountFolderRelation } from "./account-folders";
import { missPhrase } from "./account-messages";
import { readRepositoryDeclaration } from "./account-resolution";
import { WarningList } from "./account-warning";

const TARGETS = Object.keys(ACCOUNT_TARGETS) as AccountTarget[];

export interface AllowHintInput {
  /** The directory `alz build` or `alz check` ran in: the repository root. */
  repoRoot: string;
  /** The manifest's stack. */
  stack: StackEntry[];
  home: string;
  env: Env;
  uid: number | undefined;
  /**
   * The merged build configuration names an account. Without it the account feature is off here
   * and nothing is printed for a cache miss: it is off by default and prints nothing.
   */
  declares: boolean;
}

function describeMap(map: AccountMap): string {
  return TARGETS.filter((target) => map[target] !== undefined)
    .map((target) => `${target}=${map[target]}`)
    .join(", ");
}

/**
 * §7.13 'Trust': the note `alz build` and `alz check` print on a terminal when a layer names an
 * account that is not allowed yet, or an allowed entry has changed. Read-only: it reads the
 * stack the way resolution does and `allowed.yaml` with its reader, and never writes, runs git
 * or prints to a stream itself. The caller prints the lines, only when a person is at a
 * terminal. No lines for a root that resolution never applies an account to (at or above
 * `$HOME`, inside or under an account folder, or in a folder a variable names). Repository
 * text is quoted.
 */
export function allowHint(input: AllowHintInput): string[] {
  const { home, env, uid } = input;
  let root: string;
  try {
    root = realpathSync.native(input.repoRoot);
  } catch {
    return [];
  }
  const relation = accountFolderRelation(root, home, env);
  if (relation === "inside" || relation === "configured" || relation === "home") return [];

  const { declaration, miss } = readRepositoryDeclaration(root, input.stack, home, env);
  if (miss !== undefined) {
    return input.declares ? [`the account declaration was not checked: ${missPhrase(miss)}`] : [];
  }
  if (declaration.status !== "read") return [];
  const warnings = new WarningList();
  const loaded = loadAllowState({ home, env, uid, root }, warnings);
  const declared = declaration.declared;

  const unusable = stateProblem(loaded);
  if (unusable !== undefined) {
    return declared === undefined
      ? []
      : [`the layers name an account, but ${unusable}; then run: alz account allow`];
  }
  const entry = entryFor(loaded.state, root);
  if (declared === undefined) {
    return entry === undefined
      ? []
      : [
          "this repository is allowed an account that its layers no longer name; run: alz account allow --remove",
        ];
  }
  const now = { root, account: declared.account, source: declared.recordedSource };
  if (entry === undefined) {
    return [
      `the layer ${quoteEscaped(declared.layer.name)} names ${describeMap(declared.account)}; it applies only after you run: alz account allow`,
    ];
  }
  if (sameEntry(entry, now)) return [];
  return [
    `the account declaration changed since it was allowed: was ${describeMap(entry.account)} from ${quoteEscaped(entry.source)}, now ${describeMap(declared.account)} from ${quoteEscaped(declared.recordedSource)}; run: alz account allow`,
  ];
}
