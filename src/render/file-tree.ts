import { compareCodePoints, sha256 } from "../core/files";

export interface GeneratedFile {
  content: string | Buffer;
  executable: boolean;
}

/** The in-memory output of a build: repo-relative POSIX path → file. */
export class FileTree {
  private readonly files = new Map<string, GeneratedFile>();

  add(path: string, content: string | Buffer, executable = false): void {
    if (this.files.has(path)) throw new Error(`two outputs claim ${path}`);
    this.files.set(path, { content, executable });
  }

  remove(path: string): void {
    this.files.delete(path);
  }

  has(path: string): boolean {
    return this.files.has(path);
  }

  get(path: string): GeneratedFile | undefined {
    return this.files.get(path);
  }

  paths(): string[] {
    return [...this.files.keys()].sort(compareCodePoints);
  }

  hashes(): Record<string, string> {
    return Object.fromEntries(
      this.paths().map((path) => [path, hashContent(this.get(path)?.content ?? "")]),
    );
  }
}

export function hashContent(content: string | Uint8Array): string {
  return `sha256:${sha256(content)}`;
}
