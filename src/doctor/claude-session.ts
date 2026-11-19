import { spawnSync } from "node:child_process";
import type { Env } from "../resolve/git";

/** What Claude Code reports it loaded, from the `init` event of a print-mode session. */
export interface ClaudeSession {
  agents: string[];
  skills: string[];
  mcp: { name: string; status: string; source: string }[];
  /** Lines Claude printed to stderr, such as ignored settings. */
  warnings: string[];
}

/**
 * Starts one print-mode session to read the `init` event. Claude emits it only once a turn
 * starts, so this costs one short model call (target-facts §0.2).
 */
export function probeClaude(bin: string, cwd: string, env: Env): ClaudeSession | string {
  const run = spawnSync(
    bin,
    ["-p", "Reply with OK.", "--output-format", "stream-json", "--verbose", "--max-turns", "1"],
    { cwd, env, encoding: "utf8", timeout: 180_000 },
  );
  if (run.error) return `could not run ${bin}: ${run.error.message}`;
  const init = (run.stdout ?? "")
    .split("\n")
    .filter((line) => line.startsWith("{"))
    .map((line) => {
      try {
        return JSON.parse(line) as Record<string, unknown>;
      } catch {
        return undefined;
      }
    })
    .find((event) => event?.type === "system" && event.subtype === "init");
  if (!init)
    return `${bin} printed no init event: ${(run.stderr ?? "").trim().split("\n")[0] ?? ""}`;
  const strings = (value: unknown) => (Array.isArray(value) ? value.map(String) : []);
  return {
    agents: strings(init.agents),
    skills: [...new Set([...strings(init.skills), ...strings(init.slash_commands)])],
    mcp: Array.isArray(init.mcp_servers) ? (init.mcp_servers as ClaudeSession["mcp"]) : [],
    warnings: `${run.stderr ?? ""}\n${run.stdout ?? ""}`
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line !== "" && !line.startsWith("{")),
  };
}
