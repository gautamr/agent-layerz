import { stringifyYaml } from "../../core/yaml";
import type { RuleSpec } from "../../merge/rules";
import { headerText } from "../../render/header";

export const RULES_DIR = ".claude/rules";

/**
 * §7.10 → `.claude/rules/<name>.md` with `paths:` frontmatter (target-facts §9). Claude loads a
 * rule whose frontmatter does not parse as if it had no `paths`, so the YAML is generated.
 */
export function renderClaudeRule(rule: RuleSpec): string {
  const header = headerText(
    rule.sources.filter((s) => !s.action.endsWith("(overridden)")).map((s) => s.ref),
  );
  const body = rule.body ? `\n${rule.body}\n` : "";
  return `---\n# ${header}\n${stringifyYaml({ paths: rule.paths })}---\n${body}`;
}
