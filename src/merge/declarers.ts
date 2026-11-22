import type { Diagnostics } from "../core/diagnostics";
import type { Layer } from "../model/layer";
import { where } from "../model/layer";
import { privacyRank } from "../model/layer-meta";
import { isInsideRepo } from "../resolve/source-location";

/** What the who-may-declare rule looks at: a layer's source and metadata, not its files. */
export type Declarer = Pick<Layer, "source" | "meta">;

/** Why `layer` may not make a first-layer-or-one-client-layer declaration; empty when it may (§7.12, §7.13). */
export function declarerProblems(layer: Declarer, index: number, repoRoot: string): string[] {
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

export interface Evaluation<T, L extends Declarer = Layer> {
  /**
   * The client layer's value if one is eligible, otherwise the first layer's. Undefined when
   * there is no declaration, and also when the declarations conflict (see `conflict`).
   */
  winner?: { layer: L; value: T };
  /** True when several client layers declare, or the first layer's declaration was refused: no winner is chosen. */
  conflict: boolean;
  /** True when the first layer declared and may not: part of why `conflict` is set. */
  firstRefused: boolean;
  /** Layers that declare but may not, with the reasons. */
  refused: { layer: L; reasons: string[] }[];
  /** Eligible declarers after the first layer; more than one is an error. */
  clients: L[];
}

/**
 * The shared rule of `commit_identity` and `account`: only the first layer, or one private
 * `kind: client` layer, both outside the repository, may declare; the client's whole value wins.
 * Several client declarations, or a refused declaration by the first layer, leave no winner
 * (`conflict`), so resolution never picks between layers that disagree about who may declare.
 * Pure: writes no diagnostics and runs no git (`isInsideRepo` only reads the file system).
 */
export function evaluateDeclarations<T, L extends Declarer = Layer>(
  layers: L[],
  repoRoot: string,
  pick: (layer: L) => T | undefined,
): Evaluation<T, L> {
  const refused: Evaluation<T, L>["refused"] = [];
  const eligible: { layer: L; index: number; value: T }[] = [];
  let firstRefused = false;
  layers.forEach((layer, index) => {
    const value = pick(layer);
    if (value === undefined) return;
    const reasons = declarerProblems(layer, index, repoRoot);
    if (reasons.length > 0) {
      refused.push({ layer, reasons });
      if (index === 0) firstRefused = true;
    } else eligible.push({ layer, index, value });
  });
  const clients = eligible.filter((entry) => entry.index > 0);
  const conflict = clients.length > 1 || firstRefused;
  const winner = conflict ? undefined : (clients[0] ?? eligible[0]);
  return {
    winner: winner && { layer: winner.layer, value: winner.value },
    conflict,
    firstRefused,
    refused,
    clients: clients.map((entry) => entry.layer),
  };
}

/** Turns an evaluation into build errors: one per refused layer, one for several clients. */
export function reportDeclarations<T, L extends Declarer>(
  evaluation: Evaluation<T, L>,
  field: string,
  code: string,
  diags: Diagnostics,
): void {
  for (const { layer, reasons } of evaluation.refused) {
    diags.error(
      code,
      `layer '${layer.meta.name}' cannot declare ${field}: ${reasons.join("; ")}. Only the first layer, or one private layer with kind: client outside the repository, may`,
      where(layer, "layer.yaml"),
    );
  }
  const [, second] = evaluation.clients;
  if (second) {
    const names = evaluation.clients.map((layer) => `'${layer.meta.name}'`);
    const last = names.pop();
    diags.error(
      code,
      `layers ${names.join(", ")} and ${last} ${evaluation.clients.length === 2 ? "both" : "all"} declare ${field}; only one client layer may`,
      where(second, "layer.yaml"),
    );
  }
}
