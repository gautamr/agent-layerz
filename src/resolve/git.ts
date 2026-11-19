import { execFileSync } from "node:child_process";

export type Env = Record<string, string | undefined>;

export function git(args: string[], env: Env, cwd?: string): string {
  return execFileSync("git", args, {
    cwd,
    env: { ...env, GIT_TERMINAL_PROMPT: "0" },
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}
