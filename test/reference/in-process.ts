import { join } from "node:path";
import { run } from "../../src/cli/main";
import type { Terminal } from "../../src/cli/terminal";
import type { CliResult, Workspace } from "../helpers";
import { tmpRoot } from "../run-root";

/**
 * Runs `alz <args>` in `ws.root/dir` inside this process, with an optional fake `Terminal`.
 *
 * This is the TypeScript-only driver, for reference cases that need what a spawned binary cannot
 * give (the fake terminal, a `vi.mock` trace). The acceptance suite must not use it: it imports
 * src/, and the guard in test/guards keeps acceptance files away from it. It is deleted with
 * src/ at the cutover.
 */
export async function layersInProcess(
  ws: Workspace,
  dir: string,
  args: string[],
  env: Record<string, string> = {},
  terminal?: Terminal,
): Promise<CliResult> {
  let stdout = "";
  let stderr = "";
  const code = await run(args, {
    cwd: join(ws.root, dir),
    // TMPDIR is the run root's tmp folder, so a git or sh that alz starts keeps its temp files there.
    env: {
      HOME: ws.home,
      PATH: process.env.PATH ?? "",
      ALZ_CLAUDE_BIN: "/nonexistent",
      TMPDIR: tmpRoot(),
      ...env,
    },
    stdout: (text) => {
      stdout += text;
    },
    stderr: (text) => {
      stderr += text;
    },
    terminal,
  });
  return { code, stdout, stderr };
}
