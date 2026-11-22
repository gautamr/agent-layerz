import { execFileSync } from "node:child_process";
import type { Env } from "../core/env";

export type { Env };

export function git(args: string[], env: Env, cwd?: string): string {
  return execFileSync("git", args, {
    cwd,
    env: { ...env, GIT_TERMINAL_PROMPT: "0" },
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

const MAX_OUTPUT = 1024 * 1024 * 1024;

/** Like `git`, but returns the raw bytes (no trimming or decoding) and can feed standard input. */
export function gitBytes(args: string[], env: Env, cwd?: string, input?: string): Buffer {
  return execFileSync("git", args, {
    cwd,
    env: { ...env, GIT_TERMINAL_PROMPT: "0" },
    input,
    maxBuffer: MAX_OUTPUT,
    stdio: ["pipe", "pipe", "pipe"],
  });
}
