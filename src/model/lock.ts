import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { FatalError } from "../core/diagnostics";
import { isSafeRelativePath } from "../core/files";
import { stableJson } from "../core/stable-json";
import { NAME_PATTERN, type Visibility } from "./layer-meta";

export const LOCK_FILE = "layer.lock";
const SHA = /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/;

export interface LockedLayer {
  name: string;
  source: string;
  rev?: string;
  sha?: string;
  contentHash: string;
}

/** How a layer's content hash is written in the lock. */
export function lockedContentHash(layer: { contentHash: string }): string {
  return `sha256:${layer.contentHash}`;
}

export interface Lock {
  lockVersion: 1;
  /** The most private visibility in the stack; the output inherits it. */
  visibility: Visibility;
  layers: LockedLayer[];
  /** Generated path → sha256 of its content. */
  files: Record<string, string>;
}

function isLockedLayer(layer: unknown): boolean {
  if (typeof layer !== "object" || layer === null) return false;
  const item = layer as Record<string, unknown>;
  return (
    typeof item.name === "string" &&
    NAME_PATTERN.test(item.name) &&
    typeof item.source === "string" &&
    typeof item.contentHash === "string" &&
    (item.sha === undefined || typeof item.sha === "string") &&
    (item.rev === undefined || typeof item.rev === "string")
  );
}

export function readLock(repoRoot: string): Lock | undefined {
  const path = join(repoRoot, LOCK_FILE);
  if (!existsSync(path)) return undefined;
  let unsafe: string | undefined;
  try {
    const lock = JSON.parse(readFileSync(path, "utf8")) as Lock;
    if (lock.lockVersion !== 1 || typeof lock.files !== "object") throw new Error("bad shape");
    if (!Array.isArray(lock.layers)) throw new Error("bad shape");
    unsafe = Object.keys(lock.files).find((file) => !isSafeRelativePath(file));
    // Every field that reaches a cache path or a comparison has the type the code assumes.
    if (lock.layers.some((layer) => !isLockedLayer(layer))) throw new Error("bad shape");
    unsafe ??= lock.layers
      .map((layer) => layer.sha)
      .find((sha) => sha !== undefined && !SHA.test(sha));
    if (unsafe === undefined) return lock;
  } catch {
    // reported below
  }
  // A lock is repo content: a cloned repo must not be able to steer deletes or cache reads.
  throw new FatalError(
    "lock",
    unsafe === undefined
      ? `${LOCK_FILE} is not a valid lock file; delete it and rebuild`
      : `${LOCK_FILE} lists an unsafe entry '${unsafe}'; delete the lock and rebuild`,
    LOCK_FILE,
  );
}

export function serializeLock(lock: Lock): string {
  return stableJson(lock);
}
