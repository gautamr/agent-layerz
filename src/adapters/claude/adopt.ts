import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import type { AdoptionPlan } from "../../adopt/plan";
import { adoptClaudeAgent } from "./adopt-agent";
import { adoptMcpJson, adoptSettingsJson } from "./adopt-settings";
import { MCP_FILE } from "./mcp";
import { RULES_DIR } from "./rules";
import { HOOKS_DIR, SETTINGS_FILE } from "./settings";
import { SKILLS_DIR } from "./skills";

export const CLAUDE_ADOPT_ALWAYS = [SETTINGS_FILE, MCP_FILE];
const CLAUDE_EXTRA = "targets/claude/CLAUDE.extra.md";

/** `--adopt` for Claude Code: hand-written `.claude/`, `CLAUDE.md` and `.mcp.json` → repo layer. */
export function adoptClaude(paths: string[], plan: AdoptionPlan): string[] {
  const handled: string[] = [];
  // Hook scripts move first, so settings hooks that call them can point at the layer copy.
  const hookScripts = new Map<string, string>();
  for (const path of paths) {
    const hook = under(path, HOOKS_DIR);
    if (hook === undefined) continue;
    plan.move(path, `hooks/${hook}`);
    hookScripts.set(path, hook);
    handled.push(path);
  }

  for (const path of paths) {
    const text = () => readFileSync(join(plan.repoRoot, path), "utf8");
    const skill = under(path, SKILLS_DIR);
    const rule = under(path, RULES_DIR);
    if (path === "CLAUDE.md") adoptClaudeMd(text(), plan);
    else if (/^\.claude\/agents\/[^/]+\.md$/.test(path)) adoptClaudeAgent(path, text(), plan);
    else if (skill !== undefined) plan.move(path, `skills/${skill}`);
    else if (rule !== undefined && !rule.includes("/")) plan.move(path, `rules/${rule}`);
    else if (path === SETTINGS_FILE) adoptSettingsJson(text(), plan, hookScripts);
    else if (path === MCP_FILE) adoptMcpJson(text(), plan);
    else continue;
    handled.push(path);
  }
  return handled;
}

/**
 * The generated CLAUDE.md imports AGENTS.md itself, so the adopted copy drops that import and
 * every paragraph AGENTS.md already holds (tools such as Backlog.md write the same block to both).
 */
function adoptClaudeMd(text: string, plan: AdoptionPlan): void {
  const agentsMd = join(plan.repoRoot, "AGENTS.md");
  const known = new Set(existsSync(agentsMd) ? paragraphs(readFileSync(agentsMd, "utf8")) : []);
  const extra = paragraphs(text)
    .map((paragraph) =>
      paragraph
        .split("\n")
        .filter((line) => line.trim() !== "@AGENTS.md")
        .join("\n")
        .trim(),
    )
    .filter((paragraph) => paragraph !== "" && !known.has(paragraph))
    .join("\n\n");
  if (extra !== "") plan.writeText("CLAUDE.md", CLAUDE_EXTRA, `${extra}\n`);
  plan.converted("CLAUDE.md", extra === "" ? [] : [CLAUDE_EXTRA]);
}

function paragraphs(text: string): string[] {
  return text
    .replace(/\r\n/g, "\n")
    .split(/\n\s*\n/)
    .map((paragraph) => paragraph.trim());
}

function under(path: string, dir: string): string | undefined {
  return path.startsWith(`${dir}/`) ? path.slice(dir.length + 1) : undefined;
}
