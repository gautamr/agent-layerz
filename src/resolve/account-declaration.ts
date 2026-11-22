import { stripControl } from "../core/sanitize";
import { evaluateAccount } from "../merge/account";
import { declarerProblems } from "../merge/declarers";
import type { AccountMap } from "../model/account";
import {
  type CachedStackInput,
  type MissReason,
  readCachedStack,
  type StackLayer,
} from "./cached-stack";
import { recordedSource } from "./source-location";

export type { CachedStackInput as AccountDeclarationInput, MissReason };

/**
 * A layer of the stack. `source` is raw repository text exactly as written (a manifest value):
 * a caller that prints it passes it through `quote` or `stripControl`. `name` was validated as
 * a layer name by the layer schema, but a caller prints it the same way.
 */
export interface LayerRef {
  index: number;
  name: string;
  source: string;
}

/** A declaration that is reported but not applied. `reasons` are fixed phrases, control characters removed. */
export interface IgnoredDeclaration {
  layer: LayerRef;
  reasons: string[];
}

/**
 * The result of reading a stack's account declaration. Every `source` and `recordedSource` in it
 * is raw repository text, or a real path: quote it (`quote`, `stripControl`) before printing.
 * Nothing in it is file content other than names and sources.
 */
export type AccountDeclaration =
  /** Something could not be verified: no account applies from the layers; `reason` is a stable code. */
  | {
      status: "miss";
      reason: MissReason;
      /** The entry that caused it; `source` is raw repository text. */
      entry?: { index: number; source: string };
    }
  /** The stack was read. `declared` is the winning declaration, if there is one. */
  | {
      status: "read";
      declared?: {
        /** The per-target map as declared. */
        account: AccountMap;
        layer: LayerRef;
        /** The stack entry's `ref`, raw text exactly as written, for display: quote it. */
        ref?: string;
        /** The lock's commit SHA, the revision of §5.3; an unpinned `path:` layer has none. */
        sha?: string;
        /**
         * The source in the form `allowed.yaml` records: a local layer's real path, a remote
         * URL without its ref, or a local repository's canonical path. A ref bump never changes
         * it. Raw: quote it before printing.
         */
        recordedSource: string;
      };
      /** Declarations that are not applied, in stack order, including those inside the repository. */
      ignored: IgnoredDeclaration[];
    }
  /**
   * Several client layers declare: no declaration applies. A caller that only looks at
   * `declared` cannot mistake this for "no declaration".
   */
  | { status: "conflict"; clients: LayerRef[]; ignored: IgnoredDeclaration[] };

function ref(layer: StackLayer): LayerRef {
  return { index: layer.index, name: layer.meta.name, source: layer.source };
}

/**
 * §7.13: the account declaration of a stack, read from the layer cache without git. Runs no git,
 * writes nothing, prints nothing and never throws because of repository or cache content: what
 * it cannot verify is a miss with a stable `reason`.
 *
 * A layer inside the repository is reported as `ignored` when it declares, with its reasons, and
 * never changes the outcome: it is not part of the evaluation, so it can neither win nor create a
 * conflict, and editing `.layer/` never blocks the account (decision-18).
 */
export function readAccountDeclaration(input: CachedStackInput): AccountDeclaration {
  try {
    const stack = readCachedStack(input);
    if (stack.status === "miss") return stack;
    // Inside layers take no part in the decision: their declarations are only reported.
    const deciding = stack.layers.map((layer) =>
      layer.inside ? { ...layer, meta: { ...layer.meta, account: undefined } } : layer,
    );
    const evaluation = evaluateAccount(deciding, input.repoRoot);
    const ignored: IgnoredDeclaration[] = [
      ...evaluation.refused.map(({ layer, reasons }) => ({
        index: layer.index,
        item: { layer: ref(layer), reasons },
      })),
      ...stack.layers
        .filter((layer) => layer.inside && layer.meta.account !== undefined)
        .map((layer) => ({
          index: layer.index,
          item: {
            layer: ref(layer),
            reasons: declarerProblems(layer, layer.index, input.repoRoot),
          },
        })),
    ]
      .sort((a, b) => a.index - b.index)
      .map(({ item }) => ({ ...item, reasons: item.reasons.map(stripControl) }));

    if (evaluation.conflict)
      return { status: "conflict", clients: evaluation.clients.map(ref), ignored };
    const { winner } = evaluation;
    if (!winner) return { status: "read", ignored };
    const recorded = recordedSource(winner.layer.entry, input.repoRoot);
    if (recorded === undefined) {
      return {
        status: "miss",
        reason: "unreadable",
        entry: { index: winner.layer.index, source: winner.layer.source },
      };
    }
    return {
      status: "read",
      declared: {
        account: winner.value,
        layer: ref(winner.layer),
        ref: winner.layer.entry.ref,
        sha: winner.layer.sha,
        recordedSource: recorded,
      },
      ignored,
    };
  } catch {
    return { status: "miss", reason: "unreadable" };
  }
}

export { cacheRoot } from "./cache-paths";
export { recordedSource } from "./source-location";
