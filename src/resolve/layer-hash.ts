import { readFileSync } from "node:fs";
import { join } from "node:path";
import { isEnvFile, listFiles, sha256 } from "../core/files";

export interface LayerFiles {
  /** Files a layer provides: `.env` files are left out and never read. */
  files: string[];
  envFiles: string[];
  /** sha256 over each file's path and content, in code point order (§5.3). */
  contentHash: string;
}

/** The content hash from each file's path and the sha256 of its bytes, in code point order (§5.3). */
export function contentHashOf(files: { path: string; digest: string }[]): string {
  return sha256(files.map((file) => `${file.path}\0${file.digest}\n`).join(""));
}

/** Lists and hashes a layer directory without parsing any of it. */
export function hashLayerDir(dir: string): LayerFiles {
  const all = listFiles(dir);
  const files = all.filter((file) => !isEnvFile(file));
  const contentHash = contentHashOf(
    files.map((file) => ({ path: file, digest: sha256(readFileSync(join(dir, file))) })),
  );
  return { files, envFiles: all.filter(isEnvFile), contentHash };
}
