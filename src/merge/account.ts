import type { Diagnostics } from "../core/diagnostics";
import type { AccountMap } from "../model/account";
import type { Layer } from "../model/layer";
import { evaluateDeclarations, reportDeclarations } from "./declarers";

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
  const evaluation = evaluateDeclarations(layers, repoRoot, (layer) => layer.meta.account);
  reportDeclarations(evaluation, "account", "account", diags);
  const { winner } = evaluation;
  return winner && { account: winner.value, layer: winner.layer };
}
