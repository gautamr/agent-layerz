import { existsSync, readdirSync, statSync } from "node:fs";
import { delimiter, join, resolve } from "node:path";
import type { Env } from "../resolve/git";

export interface AccountScan {
  dir: string;
  agents: string[];
  skills: string[];
  hasClaudeMd: boolean;
}

/**
 * Claude account dirs: `CLAUDE_CONFIG_DIR`, `~/.claude`, every `~/.claude-*`, and
 * `ALZ_ACCOUNT_DIRS` (path-delimited). Only existing directories are returned.
 */
export function claudeAccountDirs(home: string, env: Env): string[] {
  const candidates: string[] = [];
  if (env.CLAUDE_CONFIG_DIR) candidates.push(env.CLAUDE_CONFIG_DIR);
  candidates.push(join(home, ".claude"));
  if (existsSync(home)) {
    for (const entry of readdirSync(home).sort()) {
      if (entry.startsWith(".claude-")) candidates.push(join(home, entry));
    }
  }
  for (const dir of (env.ALZ_ACCOUNT_DIRS ?? "").split(delimiter)) if (dir) candidates.push(dir);
  const unique = [...new Set(candidates.map((dir) => resolve(dir)))];
  return unique.filter((dir) => existsSync(dir) && statSync(dir).isDirectory());
}

/** §9.1: lists names only (directory entries); never opens a file in an account dir. */
export function scanAccountDir(dir: string): AccountScan {
  const list = (sub: string) => {
    const path = join(dir, sub);
    return existsSync(path) ? readdirSync(path, { withFileTypes: true }) : [];
  };
  return {
    dir,
    agents: list("agents")
      .filter((entry) => entry.isFile() && entry.name.endsWith(".md"))
      .map((entry) => entry.name.slice(0, -3))
      .sort(),
    skills: list("skills")
      .filter((entry) => entry.isDirectory() || entry.isSymbolicLink())
      .map((entry) => entry.name)
      .sort(),
    hasClaudeMd: existsSync(join(dir, "CLAUDE.md")),
  };
}
