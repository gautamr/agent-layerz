import type { AdoptionPlan } from "../../adopt/plan";
import { parseMarkdown } from "../../core/frontmatter";
import { stringifyYaml, type YamlMap } from "../../core/yaml";
import { neutralPermissionMode, neutralTool } from "./tool-names";

/** Native fields that map to a neutral field one-to-one. */
const SAME = ["description", "skills", "isolation", "memory"];

/**
 * A hand-written `.claude/agents/<name>.md` → a neutral agent in the repo layer. Native-only
 * fields go under `claude:`. An agent that a more general layer already defines becomes a
 * `mode: replace` overlay, so the repo's version keeps winning as it did before.
 */
export function adoptClaudeAgent(path: string, text: string, plan: AdoptionPlan): void {
  const file = path.slice(path.lastIndexOf("/") + 1);
  const name = file.slice(0, -".md".length);
  let doc: ReturnType<typeof parseMarkdown>;
  try {
    doc = parseMarkdown(text);
  } catch (error) {
    plan.problems.push(`${path}: ${(error as Error).message}`);
    return;
  }
  const overlay = plan.config.agents.has(name);
  const out: YamlMap = overlay ? { extends: name, mode: "replace" } : { name };
  const native: YamlMap = {};
  let changed = overlay;

  for (const [key, value] of Object.entries(doc.data)) {
    if (key === "name") continue;
    if (SAME.includes(key)) out[key] = value;
    else if (key === "tools") {
      const tools = neutralTools(value);
      if (typeof tools === "string") plan.problems.push(`${path}: ${tools}`);
      else out.tools = tools;
      changed = true;
    } else if (key === "model") {
      if (value !== "inherit") out.model = modelAlias(String(value), path, plan);
      changed = true;
    } else if (key === "permissionMode") {
      const mode = neutralPermissionMode(String(value));
      if (mode) out.permission_mode = mode;
      else plan.problems.push(`${path}: unknown permissionMode '${String(value)}'`);
      changed = true;
    } else {
      native[key] = value;
      changed = true;
    }
  }
  if (Object.keys(native).length > 0) out.claude = native;

  const to = overlay ? `agents/${name}.overlay.md` : `agents/${file}`;
  if (!changed) {
    plan.move(path, to);
    return;
  }
  if (overlay) {
    plan.notes.push(
      `${path}: a more general layer defines '${name}'; adopted as a mode: replace overlay`,
    );
  }
  plan.writeText(path, to, `---\n${stringifyYaml(out)}---\n${doc.body}`);
  plan.converted(path, [to]);
}

/** `Read, Grep` or a list → `[read, grep]`. Scoped entries such as `Agent(a, b)` have no neutral form. */
function neutralTools(value: unknown): string[] | string {
  const entries = Array.isArray(value)
    ? value.map(String)
    : String(value)
        .split(",")
        .map((entry) => entry.trim())
        .filter(Boolean);
  const tools: string[] = [];
  for (const entry of entries) {
    const tool = neutralTool(entry);
    if (!tool) return `tools entry '${entry}' has no neutral form; set it by hand`;
    tools.push(tool);
  }
  return tools;
}

/** Concrete model ids live only in models.yaml (§7.8): reuse an alias, or add one named after the model. */
export function modelAlias(model: string, from: string, plan: AdoptionPlan): string {
  const existing = Object.entries(plan.config.models.aliases).find(
    ([, targets]) => targets.claude === model,
  );
  if (existing) return existing[0];
  // Never redefine an alias a general layer owns: that would change other agents' models.
  const alias = plan.config.models.aliases[model] ? `claude-${model}` : model;
  plan.mergeYaml(from, "models.yaml", { aliases: { [alias]: { claude: model } } });
  return alias;
}
