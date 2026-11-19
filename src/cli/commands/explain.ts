import type { Contribution } from "../../merge/agents";
import { sourceRef } from "../../model/layer";
import { computeBuild } from "../../pipeline";
import { buildContext, type Io, printDiagnostics } from "../io";

const TYPES = ["agent", "skill", "command", "rule", "hook", "mcp", "instruction"] as const;

/** `alz explain <type> <name>`: contributing layers in order, then the merged result. */
export function explainCommand(io: Io, type: string | undefined, name: string | undefined): number {
  if (!type || !name || !TYPES.includes(type as (typeof TYPES)[number])) {
    io.stderr(`usage: alz explain <${TYPES.join("|")}> <name>\n`);
    return 2;
  }
  const result = computeBuild(buildContext(io));
  printDiagnostics(io, result.diags.items);
  const { config, tree } = result;

  let sources: Contribution[] | undefined;
  let outputs: string[] = [];
  if (type === "agent") {
    sources = config.agents.get(name)?.sources;
    outputs = [`.claude/agents/${name}.md`];
  } else if (type === "skill" || type === "command") {
    const spec = config.skills.get(name);
    if (spec && (type === "skill") === (spec.kind !== "command")) sources = spec.sources;
    outputs = [`.claude/skills/${name}/SKILL.md`];
  } else if (type === "rule") {
    sources = config.rules.get(name)?.sources;
    outputs = [`.claude/rules/${name}.md`];
  } else if (type === "hook") {
    sources = config.hooks.find((hook) => hook.id === name)?.sources;
    outputs = [".claude/settings.json"];
  } else if (type === "mcp") {
    const server = config.mcp.all.get(name);
    sources = server?.sources;
    if (server && !config.mcp.enabled.includes(server)) {
      io.stdout(`mcp ${name} is defined but not enabled (add it to mcp.enable in layer.yaml)\n`);
    }
    outputs = [".mcp.json"];
  } else {
    const blocks = config.instructions.filter((block) => block.file === name);
    if (blocks.length > 0) {
      sources = blocks.map((block) => ({
        ref: sourceRef(block.layer, block.path),
        action: "block",
      }));
    }
    outputs = ["AGENTS.md"];
  }

  if (!sources) {
    io.stderr(`no ${type} named '${name}' in the merged result\n`);
    return 1;
  }
  io.stdout(`${type} ${name}\nsources, general → specific:\n`);
  sources.forEach((source, index) => {
    io.stdout(`  ${index + 1}. ${source.ref} — ${source.action}\n`);
  });
  for (const path of outputs) {
    const file = tree.get(path);
    if (!file) continue;
    io.stdout(`\nmerged result (${path}):\n${file.content.toString()}`);
  }
  return 0;
}
