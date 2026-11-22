import { lstatSync } from "node:fs";
import { join } from "node:path";
import { InputError } from "../core/diagnostics";
import {
  isEnvFile,
  listFiles,
  type RegularFile,
  readRegularFileWithId,
  sha256,
} from "../core/files";
import { parseYamlLimited } from "../core/yaml";
import type { Layer } from "../model/layer";
import { parseLayerMeta } from "../model/layer-meta";
import { contentHashOf } from "./layer-hash";

/**
 * Account resolution runs at every prompt after a `cd`, on layers it does not trust, so a layer
 * has a size: more files or folders than this, or more bytes in all, is a cache miss
 * (`unreadable`). A layer that large is not an account layer; `alz build` has no such limit.
 */
export const MAX_LAYER_ENTRIES = 2000;
export const MAX_LAYER_BYTES = 16 * 1024 * 1024;
/** The most `layer.yaml` may hold, here and for a layer inside the repository (§7.13). */
export const MAX_LAYER_YAML_BYTES = 1024 * 1024;

export type LoadMiss = "unreadable" | "content-hash" | "layer-invalid";

/**
 * Reads a layer directory for account resolution, in one pass: each file is read once, through
 * `readRegularFileWithId` (no link followed, the open file checked against its `lstat`), and
 * those bytes are the ones hashed, so `layer.yaml` is parsed from exactly the bytes the content
 * check covered. The hash is compared with the lock first (`content-hash`), and `layer.yaml`
 * is judged and parsed only after it. A `.env` file is never read and makes the layer invalid.
 * Never throws because of the directory's content.
 */
export function readLayerForResolution(
  dir: string,
  source: string,
  matchesLock: (contentHash: string) => boolean,
): { layer: Layer } | { miss: LoadMiss } {
  try {
    // The identity `layer.yaml` has before anything is hashed; the read must be the same file.
    let identity: { dev: number; ino: number } | undefined;
    try {
      const found = lstatSync(join(dir, "layer.yaml"));
      if (found.isFile()) identity = { dev: found.dev, ino: found.ino };
    } catch {
      // judged after the content check
    }
    const all = listFiles(dir, MAX_LAYER_ENTRIES);
    const files = all.filter((file) => !isEnvFile(file));
    const digests: { path: string; digest: string }[] = [];
    let total = 0;
    let yaml: RegularFile | undefined;
    for (const file of files) {
      const read = readRegularFileWithId(join(dir, file), MAX_LAYER_BYTES - total);
      if (typeof read === "string") return { miss: "unreadable" };
      total += read.bytes.length;
      digests.push({ path: file, digest: sha256(read.bytes) });
      if (file === "layer.yaml") yaml = read;
    }
    const contentHash = contentHashOf(digests);
    if (!matchesLock(contentHash)) return { miss: "content-hash" };
    if (all.some(isEnvFile)) return { miss: "layer-invalid" };
    if (!yaml || !identity || yaml.dev !== identity.dev || yaml.ino !== identity.ino) {
      return { miss: "layer-invalid" };
    }
    if (yaml.bytes.length > MAX_LAYER_YAML_BYTES) return { miss: "layer-invalid" };
    const meta = parseLayerMeta(parseYamlLimited(yaml.bytes.toString("utf8")));
    return { layer: { meta, dir, source, files, contentHash } };
  } catch (error) {
    // The walk gave up (too many entries) or the directory could not be read, or `layer.yaml` is
    // not a valid layer.
    return { miss: error instanceof InputError ? "layer-invalid" : "unreadable" };
  }
}
