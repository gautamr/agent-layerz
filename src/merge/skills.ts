import type { Diagnostics } from "../core/diagnostics";
import { asOptionalString, unknownKeys } from "../core/yaml";
import { type Layer, sourceRef, where } from "../model/layer";
import { SKILL_NAME } from "../model/vocabulary";
import type { AgentSpec, Contribution } from "./agents";
import { filesIn, readLayerMarkdown, stem } from "./layer-files";

export type SkillSpec =
  | {
      kind: "skill";
      name: string;
      layer: Layer;
      /** Files relative to the skill directory, e.g. `SKILL.md`, `references/x.md`. */
      files: string[];
      sources: Contribution[];
    }
  | {
      kind: "command";
      name: string;
      layer: Layer;
      path: string;
      description: string;
      argumentHint?: string;
      body: string;
      sources: Contribution[];
    }
  | { kind: "stub"; name: string; sources: Contribution[] };

const COMMAND_FIELDS = ["description", "argument_hint"];

/** §7.3/§7.4: the most specific layer wins a name collision and replaces the whole skill. */
export function mergeSkills(layers: Layer[], diags: Diagnostics): Map<string, SkillSpec> {
  const skills = new Map<string, SkillSpec>();
  for (const layer of layers) {
    const local = new Map<string, SkillSpec>();
    for (const spec of [...readSkills(layer, diags), ...readCommands(layer, diags)]) {
      if (local.has(spec.name)) {
        diags.error(
          "skill-command-clash",
          `'${spec.name}' is both a skill and a command in this layer`,
          layer.meta.name,
        );
        continue;
      }
      local.set(spec.name, spec);
    }
    for (const spec of local.values()) {
      const previous = skills.get(spec.name);
      if (previous) {
        spec.sources = [
          ...previous.sources.map((source) => ({
            ...source,
            action: `${source.action} (overridden)`,
          })),
          ...spec.sources,
        ];
      }
      skills.set(spec.name, spec);
    }
  }
  return skills;
}

function readSkills(layer: Layer, diags: Diagnostics): SkillSpec[] {
  const names = new Set(
    layer.files
      .filter((file) => /^skills\/[^/]+\//.test(file))
      .map((file) => file.split("/")[1] as string),
  );
  const specs: SkillSpec[] = [];
  for (const name of [...names].sort()) {
    const dir = `skills/${name}`;
    const loc = where(layer, `${dir}/SKILL.md`);
    if (!layer.files.includes(`${dir}/SKILL.md`)) {
      diags.error("skill", `skill directory '${name}' has no SKILL.md`, where(layer, dir));
      continue;
    }
    const doc = readLayerMarkdown(layer, `${dir}/SKILL.md`, diags);
    if (!doc) continue;
    if (!SKILL_NAME.test(name))
      diags.error("skill", `skill name '${name}' must be kebab-case`, loc);
    if (doc.data.name !== name) {
      diags.error("skill", `'name' in SKILL.md must be '${name}' (the directory name)`, loc);
      continue;
    }
    if (typeof doc.data.description !== "string" || doc.data.description.trim() === "") {
      diags.error("skill", "'description' is required", loc);
      continue;
    }
    specs.push({
      kind: "skill",
      name,
      layer,
      files: layer.files
        .filter((file) => file.startsWith(`${dir}/`))
        .map((file) => file.slice(dir.length + 1)),
      sources: [{ ref: sourceRef(layer, dir), action: "skill" }],
    });
  }
  return specs;
}

function readCommands(layer: Layer, diags: Diagnostics): SkillSpec[] {
  const specs: SkillSpec[] = [];
  for (const path of filesIn(layer, "commands", ".md")) {
    const doc = readLayerMarkdown(layer, path, diags);
    if (!doc) continue;
    const loc = where(layer, path);
    const name = stem(path, ".md");
    const extra = unknownKeys(doc.data, COMMAND_FIELDS);
    if (extra.length > 0) {
      diags.error("command", `unknown field(s): ${extra.join(", ")}`, loc);
      continue;
    }
    if (!SKILL_NAME.test(name)) {
      diags.error("command", `command name '${name}' must be kebab-case`, loc);
      continue;
    }
    const description = typeof doc.data.description === "string" ? doc.data.description : "";
    if (!description) {
      diags.error("command", "'description' is required", loc);
      continue;
    }
    specs.push({
      kind: "command",
      name,
      layer,
      path,
      description,
      argumentHint: asOptionalString(doc.data.argument_hint, "argument_hint"),
      body: doc.body.trim(),
      sources: [{ ref: sourceRef(layer, path), action: "command" }],
    });
  }
  return specs;
}

/** Claude preloads at most 32 skills per agent (target-facts §1). */
const MAX_PRELOAD = 32;

/** §7.3 slots: every referenced skill must exist; a stub fills it when `strict_slots` is false. */
export function fillSlots(
  layers: Layer[],
  agents: Map<string, AgentSpec>,
  skills: Map<string, SkillSpec>,
  strict: boolean,
  diags: Diagnostics,
): void {
  const required = new Map<string, string>();
  for (const layer of layers) {
    for (const slot of layer.meta.requiresSlots) {
      if (!required.has(slot)) required.set(slot, `${layer.meta.name} (requires_slots)`);
    }
  }
  for (const agent of agents.values()) {
    if (agent.skills.length > MAX_PRELOAD) {
      diags.error(
        "agent",
        `agent '${agent.name}' preloads ${agent.skills.length} skills; Claude allows ${MAX_PRELOAD}`,
        agent.baseWhere,
      );
    }
    for (const skill of agent.skills) {
      if (!required.has(skill)) required.set(skill, `agent '${agent.name}'`);
      if (skills.get(skill)?.kind === "command") {
        diags.error(
          "slot-command",
          `agent '${agent.name}' preloads '${skill}', which is a command; commands cannot be preloaded`,
          agent.baseWhere,
        );
      }
    }
  }
  for (const [slot, by] of required) {
    if (skills.has(slot)) continue;
    if (strict) {
      diags.error("slot-missing", `slot '${slot}' required by ${by} is not filled by any layer`);
      continue;
    }
    diags.warn(
      "slot-missing",
      `slot '${slot}' required by ${by} is not filled by any layer; emitting a stub skill`,
    );
    skills.set(slot, { kind: "stub", name: slot, sources: [{ ref: "slot stub", action: "stub" }] });
  }
}
