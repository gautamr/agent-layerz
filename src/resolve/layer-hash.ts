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

/** Lists and hashes a layer directory without parsing any of it. */
export function hashLayerDir(dir: string): LayerFiles {
  const all = listFiles(dir);
  const files = all.filter((file) => !isEnvFile(file));
  const contentHash = sha256(
    files.map((file) => `${file}\0${sha256(readFileSync(join(dir, file)))}\n`).join(""),
  );
  return { files, envFiles: all.filter(isEnvFile), contentHash };
}
