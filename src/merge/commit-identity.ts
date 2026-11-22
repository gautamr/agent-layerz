import type { Diagnostics } from "../core/diagnostics";
import type { CommitIdentity } from "../model/commit-identity";
import type { Layer } from "../model/layer";
import { evaluateDeclarations, reportDeclarations } from "./declarers";

export interface DeclaredIdentity {
  identity: CommitIdentity;
  /** The layer that declared the winning identity. */
  layer: Layer;
}

/**
 * §7.12: only the first layer, or one private `kind: client` layer, both outside the repo, may
 * declare `commit_identity`. The client layer's whole identity wins; identities never mix.
 */
export function mergeCommitIdentity(
  layers: Layer[],
  repoRoot: string,
  diags: Diagnostics,
): DeclaredIdentity | undefined {
  const evaluation = evaluateDeclarations(layers, repoRoot, (layer) => layer.meta.commitIdentity);
  reportDeclarations(evaluation, "commit_identity", "identity", diags);
  const { winner } = evaluation;
  return winner && { identity: winner.value, layer: winner.layer };
}
