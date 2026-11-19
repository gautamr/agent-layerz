import { probeClaude } from "../../doctor/claude-session";
import type { Contribution } from "../../merge/agents";
import { computeBuild } from "../../pipeline";
import { buildContext, type Io, printDiagnostics } from "../io";

/** `alz ls [--live]`: what the stack builds and which layer each item comes from. */
export function lsCommand(io: Io, options: { live: boolean }): number {
  const ctx = buildContext(io);
  const result = computeBuild(ctx);
  printDiagnostics(io, result.diags.items);
  if (result.diags.hasErrors) return 1;
  const { config } = result;

  const section = (title: string, items: [string, Contribution[]][]) => {
    io.stdout(`${title} (${items.length})\n`);
    for (const [name, sources] of items) io.stdout(`  ${name.padEnd(28)} ${owner(sources)}\n`);
  };
  const skills = [...config.skills.values()];
  section(
    "agents",
    [...config.agents.values()].map((agent) => [agent.name, agent.sources]),
  );
  section(
    "skills",
    skills.filter((s) => s.kind !== "command").map((s) => [s.name, s.sources]),
  );
  section(
    "commands",
    skills.filter((s) => s.kind === "command").map((s) => [s.name, s.sources]),
  );
  section(
    "rules",
    [...config.rules.values()].map((rule) => [rule.name, rule.sources]),
  );
  section(
    "hooks",
    config.hooks.map((hook) => [hook.id, hook.sources]),
  );
  section(
    "mcp",
    config.mcp.enabled.map((server) => [server.id, server.sources]),
  );

  if (!options.live) return 0;
  if (!result.manifest.targets.includes("claude")) {
    io.stderr("error [ls] --live needs the claude target\n");
    return 1;
  }
  return compareWithClaude(io, result);
}

/** The layer that decides the item: its most specific contribution. */
function owner(sources: Contribution[]): string {
  const ref = sources.at(-1)?.ref ?? "";
  return ref.slice(0, ref.search(/[@:]/) < 0 ? undefined : ref.search(/[@:]/));
}

function compareWithClaude(io: Io, result: ReturnType<typeof computeBuild>): number {
  const ctx = buildContext(io);
  const bin = ctx.env.ALZ_CLAUDE_BIN ?? "claude";
  io.stdout(`\nasking ${bin} what it loads here (one short model call)…\n`);
  const session = probeClaude(bin, ctx.repoRoot, ctx.env);
  if (typeof session === "string") {
    io.stderr(`error [ls] ${session}\n`);
    return 1;
  }
  const adapter = result.adapters.find((candidate) => candidate.target === "claude");
  const built = adapter?.builtNames(result.config) ?? { agents: [], skills: [] };
  let failed = false;
  const missing = (kind: string, names: string[], loaded: string[]) => {
    for (const name of names.filter((n) => !loaded.includes(n))) {
      io.stderr(`error [ls] ${kind} '${name}' was built but Claude did not load it\n`);
      failed = true;
    }
  };
  missing("agent", built.agents, session.agents);
  missing("skill", built.skills, session.skills);
  for (const server of result.config.mcp.enabled) {
    const live = session.mcp.find(
      (entry) => entry.name === server.id && entry.source === "project",
    );
    if (!live) {
      io.stderr(`error [ls] mcp '${server.id}' is enabled but Claude did not start it\n`);
      failed = true;
    } else if (live.status !== "connected") {
      io.stderr(`warning [ls] mcp '${server.id}' is ${live.status}\n`);
    }
  }
  for (const line of session.warnings) io.stderr(`warning [claude] ${line}\n`);

  // Items from outside the layers: user dirs, plugins and Claude's built-ins. Shown, not judged.
  const extraAgents = session.agents.filter((name) => !built.agents.includes(name));
  const extraSkills = session.skills.filter(
    (name) => !built.skills.includes(name) && !name.includes(":"),
  );
  io.stdout(
    `claude loaded ${session.agents.length} agents, ${session.skills.length} skills and commands\n`,
  );
  if (extraAgents.length > 0) io.stdout(`  not from layers, agents: ${extraAgents.join(", ")}\n`);
  if (extraSkills.length > 0) io.stdout(`  not from layers, skills: ${extraSkills.join(", ")}\n`);
  io.stdout(
    failed
      ? "ls --live: Claude does not match the build\n"
      : "ls --live: Claude matches the build\n",
  );
  return failed ? 1 : 0;
}
