import { homedir } from "node:os";
import { type Diagnostic, formatDiagnostic } from "../core/diagnostics";
import type { BuildContext } from "../pipeline";
import type { Env } from "../resolve/git";
import type { Terminal } from "./terminal";

/** Everything a command touches from the outside, so tests can run it in-process. */
export interface Io {
  cwd: string;
  env: Env;
  stdout: (text: string) => void;
  stderr: (text: string) => void;
  /** Absent means standard input is not a terminal: person-only commands then refuse. Only `bin.ts` sets it. */
  terminal?: Terminal;
}

export function buildContext(io: Io): BuildContext {
  return { repoRoot: io.cwd, home: io.env.HOME ?? homedir(), env: io.env };
}

export function printDiagnostics(io: Io, items: Diagnostic[]): void {
  for (const item of items) io.stderr(`${formatDiagnostic(item)}\n`);
}

export function plural(count: number, word: string): string {
  return `${count} ${word}${count === 1 ? "" : "s"}`;
}
