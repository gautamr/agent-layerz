import { type CliResult, spawnInteractive } from "./exec";
import { ptyRun } from "./run-root";

/**
 * A command run for a person at a terminal: `pty-run` gives it a pseudo-terminal as standard input
 * and leaves standard output and standard error as pipes, so the tests tell the streams apart. What
 * `type` sends is typed at the terminal (a line, or `CTRL_C`), and `endInput` is the terminal's
 * end of file. The driver kills the command at its deadline, which is shorter than the test's.
 * `pty-run` itself starts through `spawnInteractive` (test/exec.ts): its own process group, the
 * timeout and an allow-listed environment.
 */
export interface PtySession {
  /** Everything the command has written to standard error so far. */
  stderrSoFar(): string;
  /** Resolves once standard error holds `text` at least `times` times; rejects if the command ends first. */
  waitForStderr(text: string, times?: number): Promise<void>;
  /** Types text at the terminal. */
  type(text: string): void;
  /** Ends the terminal's input, as Ctrl-D at the start of a line does. */
  endInput(): void;
  /** Resolves with the exit code and both streams when the command has ended. */
  done(): Promise<CliResult>;
}

/** The interrupt key. */
export const CTRL_C = "\u0003";

export interface PtyOptions {
  cwd: string;
  /** PATH when it holds none, this environment and TMPDIR: never the test process's own. */
  env: Record<string, string>;
  /** How long the command may run; the driver kills it a second earlier. */
  timeoutMs?: number;
}

/** Starts `file args` on a pseudo-terminal in `options.cwd`. */
export function startOnPty(file: string, args: readonly string[], options: PtyOptions): PtySession {
  const timeoutMs = options.timeoutMs ?? 15_000;
  const child = spawnInteractive(
    ptyRun(),
    ["--deadline-ms", String(timeoutMs - 1000), "--", file, ...args],
    { cwd: options.cwd, env: options.env, timeoutMs },
  );
  return {
    stderrSoFar: child.stderrSoFar,
    waitForStderr: child.waitForStderr,
    type: child.write,
    endInput: child.end,
    done: child.done,
  };
}
