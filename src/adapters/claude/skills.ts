import { readFileSync } from "node:fs";
import { join } from "node:path";
import { isExecutable } from "../../core/files";
import { insertFrontmatterComment } from "../../core/frontmatter";
import { stringifyYaml } from "../../core/yaml";
import type { SkillSpec } from "../../merge/skills";
import type { FileTree } from "../../render/file-tree";
import { headerText } from "../../render/header";

export const SKILLS_DIR = ".claude/skills";

/** §8.1: skills, commands (explicit-only skills) and slot stubs → `.claude/skills/<name>/`. */
export function renderClaudeSkill(spec: SkillSpec, tree: FileTree): void {
  const out = `${SKILLS_DIR}/${spec.name}`;
  const header = headerText(
    spec.sources.filter((s) => !s.action.endsWith("(overridden)")).map((s) => s.ref),
  );

  if (spec.kind === "skill") {
    for (const file of spec.files) {
      const source = join(spec.layer.dir, "skills", spec.name, file);
      if (file === "SKILL.md") {
        tree.add(`${out}/SKILL.md`, insertFrontmatterComment(readFileSync(source, "utf8"), header));
      } else {
        tree.add(`${out}/${file}`, readFileSync(source), isExecutable(source));
      }
    }
    return;
  }

  if (spec.kind === "command") {
    const frontmatter: Record<string, unknown> = { name: spec.name, description: spec.description };
    if (spec.argumentHint) frontmatter["argument-hint"] = spec.argumentHint;
    // target-facts §5: commands are skills that only the user can invoke.
    frontmatter["disable-model-invocation"] = true;
    tree.add(`${out}/SKILL.md`, markdown(header, frontmatter, spec.body));
    return;
  }

  tree.add(
    `${out}/SKILL.md`,
    markdown(
      header,
      { name: spec.name, description: `Slot stub: ${spec.name} is not defined for this repo` },
      `The \`${spec.name}\` skill is not defined for this repo. Do not assume its conventions; ask the user when they matter.`,
    ),
  );
}

function markdown(header: string, frontmatter: Record<string, unknown>, body: string): string {
  return `---\n# ${header}\n${stringifyYaml(frontmatter)}---\n${body ? `\n${body}\n` : ""}`;
}
