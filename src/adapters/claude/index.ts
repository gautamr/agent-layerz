import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { Diagnostics } from "../../core/diagnostics";
import { isExecutable } from "../../core/files";
import { stableJson } from "../../core/stable-json";
import type { ResolvedConfig } from "../../model/config";
import { AGENTS_MD } from "../../render/agents-md";
import type { FileTree } from "../../render/file-tree";
import { htmlHeader } from "../../render/header";
import type { Adapter } from "../types";
import { adoptClaude, CLAUDE_ADOPT_ALWAYS } from "./adopt";
import { renderClaudeAgent, targetsClaude } from "./agents";
import { MCP_FILE, renderMcpServers } from "./mcp";
import { RULES_DIR, renderClaudeRule } from "./rules";
import { buildClaudeSettings, HOOKS_DIR, SETTINGS_FILE } from "./settings";
import { renderClaudeSkill, SKILLS_DIR } from "./skills";

const AGENTS_DIR = ".claude/agents";

export const claudeAdapter: Adapter = {
  target: "claude",
  verified: { min: "2.1.311", below: "2.2.0" },
  managedDirs: [AGENTS_DIR, SKILLS_DIR, RULES_DIR, HOOKS_DIR],
  // target-facts §9: per-agent memory for `memory: project` and `memory: local`.
  protectedPaths: [".claude/agent-memory/**", ".claude/agent-memory-local/**"],

  adoptAlways: CLAUDE_ADOPT_ALWAYS,
  adopt: adoptClaude,

  render(config: ResolvedConfig, tree: FileTree, diags: Diagnostics): void {
    tree.add("CLAUDE.md", renderClaudeMd(config));
    for (const agent of config.agents.values()) {
      if (!targetsClaude(agent.targets)) continue;
      const text = renderClaudeAgent(agent, config.models, diags);
      if (text) tree.add(`${AGENTS_DIR}/${agent.name}.md`, text);
    }
    for (const skill of config.skills.values()) renderClaudeSkill(skill, tree);
    for (const rule of config.rules.values()) {
      tree.add(`${RULES_DIR}/${rule.name}.md`, renderClaudeRule(rule));
    }
    const settings = buildClaudeSettings(config, tree, diags);
    if (Object.keys(settings).length > 0) tree.add(SETTINGS_FILE, stableJson(settings));
    if (config.mcp.enabled.length > 0)
      tree.add(MCP_FILE, stableJson(renderMcpServers(config.mcp.enabled)));
    renderPassthrough(config, tree, diags);
  },

  builtNames(config: ResolvedConfig) {
    return {
      agents: [...config.agents.values()]
        .filter((agent) => targetsClaude(agent.targets))
        .map((agent) => agent.name),
      skills: [...config.skills.keys()],
    };
  },
};

/** §7.1: a thin CLAUDE.md that imports AGENTS.md (loaded once, target-facts §3). */
function renderClaudeMd(config: ResolvedConfig): string {
  const header = htmlHeader([AGENTS_MD, ...config.claudeExtras.map((extra) => extra.ref)]);
  const parts = [header, `@${AGENTS_MD}`];
  if (config.claudeExtras.length > 0) {
    parts.push("## Claude-specific", ...config.claudeExtras.map((extra) => extra.body));
  }
  return `${parts.join("\n\n")}\n`;
}

function renderPassthrough(config: ResolvedConfig, tree: FileTree, diags: Diagnostics): void {
  for (const [path, file] of config.passthrough.get("claude") ?? []) {
    if (tree.has(path)) {
      diags.error(
        "passthrough",
        `passthrough ${path} collides with a generated file`,
        `${file.layer.meta.name}:${file.source}`,
      );
      continue;
    }
    const source = join(file.layer.dir, file.source);
    tree.add(path, readFileSync(source), isExecutable(source));
  }
}
