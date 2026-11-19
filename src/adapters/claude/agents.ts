import type { Diagnostics } from "../../core/diagnostics";
import { stringifyYaml } from "../../core/yaml";
import type { AgentSpec } from "../../merge/agents";
import { type ModelsSpec, resolveAgentModel } from "../../merge/models";
import { parseTool } from "../../merge/tools";
import type { PermissionMode } from "../../model/vocabulary";
import { headerText } from "../../render/header";
import { CLAUDE_PERMISSION_MODES, claudeTool, isClaudeModel } from "./tool-names";

export function targetsClaude(targets: string[] | undefined): boolean {
  return targets === undefined || targets.includes("claude");
}

/** §8.1 + target-facts §8.1: neutral agent → `.claude/agents/<name>.md` with camelCase fields. */
export function renderClaudeAgent(
  agent: AgentSpec,
  models: ModelsSpec,
  diags: Diagnostics,
): string | undefined {
  const frontmatter: Record<string, unknown> = { name: agent.name, description: agent.description };

  if (agent.tools !== undefined) {
    if (agent.tools.length === 0) {
      diags.error(
        "claude-agent",
        `agent '${agent.name}' has an empty tools list, which Claude reads as "all tools"`,
        agent.baseWhere,
      );
      return undefined;
    }
    const names: string[] = [];
    for (const entry of agent.tools) {
      const { tool, scope } = parseTool(entry);
      const native = claudeTool(tool);
      if (scope !== undefined) {
        diags.warn(
          "tool-scope",
          `agent '${agent.name}': Claude cannot scope a tool per agent, so '${entry}' becomes '${native}'; put the scope in permissions.yaml`,
          agent.baseWhere,
        );
      }
      if (!names.includes(native)) names.push(native);
    }
    frontmatter.tools = names.join(", ");
  }

  const model = resolveAgentModel(agent, models, "claude", diags);
  if (model !== undefined) {
    if (!isClaudeModel(model)) {
      diags.error(
        "claude-agent",
        `agent '${agent.name}': '${model}' is not a Claude model value`,
        agent.baseWhere,
      );
    }
    frontmatter.model = model;
  }
  if (agent.permissionMode) {
    frontmatter.permissionMode = CLAUDE_PERMISSION_MODES[agent.permissionMode as PermissionMode];
  }
  if (agent.skills.length > 0) frontmatter.skills = agent.skills;
  // Claude has no `none` for these: omit the field (target-facts §1).
  if (agent.isolation === "worktree") frontmatter.isolation = "worktree";
  if (agent.memory && agent.memory !== "none") frontmatter.memory = agent.memory;
  Object.assign(frontmatter, agent.native.claude);

  const header = headerText(agent.sources.map((source) => source.ref));
  const body = agent.body ? `\n${agent.body}\n` : "";
  return `---\n# ${header}\n${stringifyYaml(frontmatter)}---\n${body}`;
}
