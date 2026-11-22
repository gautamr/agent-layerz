import type { Diagnostics } from "../core/diagnostics";
import type { AccountMap } from "../model/account";
import type { Layer } from "../model/layer";
import {
  type Declarer,
  type Evaluation,
  evaluateDeclarations,
  reportDeclarations,
} from "./declarers";

export interface DeclaredAccount {
  account: AccountMap;
  /** The layer that declared the winning value. */
  layer: Layer;
}

/**
 * §7.13 'Who may declare it': the same rule as `commit_identity`. The client layer's whole
 * value replaces the first layer's, a per-target map included; values never mix.
 */
export function mergeAccount(
  layers: Layer[],
  repoRoot: string,
  diags: Diagnostics,
): DeclaredAccount | undefined {
  const evaluation = evaluateAccount(layers, repoRoot);
  reportDeclarations(evaluation, "account", "account", diags);
  const { winner } = evaluation;
  return winner && { account: winner.value, layer: winner.layer };
}

/**
 * The rule of `mergeAccount` without its diagnostics: who may declare, which declaration wins and
 * which are refused. Pure: it runs no git and writes nothing, so account resolution (§7.13) can
 * apply it to layers it read from the cache.
 */
export function evaluateAccount<L extends Declarer>(
  layers: L[],
  repoRoot: string,
): Evaluation<AccountMap, L> {
  return evaluateDeclarations(layers, repoRoot, (layer) => layer.meta.account);
}
