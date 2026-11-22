import { createInterface } from "node:readline";
import { isatty } from "node:tty";
import { agentSessionMarkers } from "../core/env";
import type { Io } from "./io";

/**
 * A person at a terminal. This is the only file that reads standard input (a source scan in
 * test 32 proves it), so a command can ask a question only through `Io.terminal`, and an
 * in-process test runs the code the real CLI runs.
 */
export interface Terminal {
  /**
   * Writes the prompt to standard error and returns one line typed in answer, or undefined at
   * the end of input or on Ctrl-C. The answer is never echoed by alz.
   */
  ask(prompt: string): Promise<string | undefined>;
  /** Releases standard input so the process can exit. Optional: a fake terminal has nothing to release. */
  close?(): void;
}

/**
 * A terminal over one input and one output. One line reader serves every question, and lines
 * that arrive together (a paste, or two answers in one chunk) are queued, so each is read by the
 * question it answers. The reader is made at the first question; a command that never asks
 * never touches the input.
 */
export function createTerminal(streams: {
  input: NodeJS.ReadableStream;
  output: NodeJS.WritableStream;
  /** Line editing and echo: true for a real terminal. */
  tty: boolean;
}): Terminal {
  const queued: string[] = [];
  const waiting: ((line: string | undefined) => void)[] = [];
  let lines: ReturnType<typeof createInterface> | undefined;
  let ended = false;

  const start = () => {
    lines = createInterface({
      input: streams.input,
      output: streams.output,
      terminal: streams.tty,
    });
    lines.on("line", (line) => {
      const next = waiting.shift();
      if (next) next(line);
      else queued.push(line);
    });
    lines.on("close", () => {
      ended = true;
      for (const next of waiting.splice(0)) next(undefined);
    });
    lines.on("SIGINT", () => lines?.close());
  };

  return {
    ask: (prompt) => {
      if (lines === undefined) start();
      // The prompt goes through the line reader, so a terminal that redraws the line keeps it.
      lines?.setPrompt(prompt);
      lines?.prompt();
      const early = queued.shift();
      if (early !== undefined) return Promise.resolve(early);
      if (ended) return Promise.resolve(undefined);
      return new Promise((resolve) => waiting.push(resolve));
    },
    close: () => lines?.close(),
  };
}

/**
 * The process's terminal when standard input is one, else undefined. Only `bin.ts` calls it, so
 * a command reaches standard input only through `Io.terminal`.
 */
export function processTerminal(): Terminal | undefined {
  if (!isatty(0)) return undefined;
  return createTerminal({ input: process.stdin, output: process.stderr, tty: true });
}

/**
 * Is a person at the terminal? Standard input must be a terminal (`Io.terminal` is set only by
 * `bin.ts`, and only when it is) and no agent-session marker may be set. A missing terminal
 * means no.
 */
export function personAtTerminal(io: Io): boolean {
  return io.terminal !== undefined && agentSessionMarkers(io.env).length === 0;
}
