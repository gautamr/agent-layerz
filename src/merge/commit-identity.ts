import type { Diagnostics } from "../core/diagnostics";
import type { CommitIdentity } from "../model/commit-identity";
import type { Layer } from "../model/layer";
import { where } from "../model/layer";
import { privacyRank } from "../model/layer-meta";
import { isInsideRepo } from "../resolve/sources";

export interface DeclaredIdentity {
  identity: CommitIdentity;
  /** The layer that declared the winning identity. */
  layer: Layer;
}

/** Why `layer` may not declare a commit identity; empty when it may (§7.12). */
function reasons(layer: Layer, index: number, repoRoot: string): string[] {
  const found: string[] = [];
  if (isInsideRepo(repoRoot, layer.source)) found.push("it is inside the repository");
  if (index > 0 && layer.meta.kind !== "client") {
    found.push(`its kind is '${layer.meta.kind}', not 'client'`);
  }
  if (index > 0 && privacyRank(layer.meta.visibility) < privacyRank("private")) {
    found.push("it is public");
  }
  return found;
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
  const eligible: { layer: Layer; index: number }[] = [];
  layers.forEach((layer, index) => {
    if (!layer.meta.commitIdentity) return;
    const why = reasons(layer, index, repoRoot);
    if (why.length > 0) {
      diags.error(
        "identity",
        `layer '${layer.meta.name}' cannot declare commit_identity: ${why.join("; ")}. Only the first layer, or one private layer with kind: client outside the repository, may`,
        where(layer, "layer.yaml"),
      );
      return;
    }
    eligible.push({ layer, index });
  });

  const clients = eligible.filter((entry) => entry.index > 0);
  const [, second] = clients;
  if (second) {
    const names = clients.map((entry) => `'${entry.layer.meta.name}'`);
    const last = names.pop();
    diags.error(
      "identity",
      `layers ${names.join(", ")} and ${last} ${clients.length === 2 ? "both" : "all"} declare commit_identity; only one client layer may`,
      where(second.layer, "layer.yaml"),
    );
  }
  const winner = clients[0] ?? eligible[0];
  const identity = winner?.layer.meta.commitIdentity;
  return winner && identity ? { identity, layer: winner.layer } : undefined;
}
