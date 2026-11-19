import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { formatDiagnostic } from "../../core/diagnostics";
import { claudeAccountDirs, scanAccountDir } from "../../doctor/account-dirs";
import { inRange, installedVersion } from "../../doctor/tool-version";
import { computeBuild } from "../../pipeline";
import { buildContext, type Io, plural } from "../io";

/**
 * `alz doctor`: account-dir collisions (§8.4), installed tool version vs. the adapter's
 * verified range (§12), client-root and ancestor files Claude also loads (§4.4), workspace
 * dirs, and the build's own findings.
 */
export function doctorCommand(io: Io): number {
  const ctx = buildContext(io);
  const result = computeBuild(ctx);
  let problems = 0;
  const line = (text: string) => io.stdout(`${text}\n`);

  line(`agent-layerz doctor: ${ctx.repoRoot}`);

  if (result.adapters.some((adapter) => adapter.target === "claude")) {
    const claude = result.adapters.find((adapter) => adapter.target === "claude");
    const built = claude?.builtNames(result.config) ?? { agents: [], skills: [] };
    line("claude account dirs:");
    const dirs = claudeAccountDirs(ctx.home, ctx.env);
    if (dirs.length === 0) line("  none found");
    for (const dir of dirs) {
      const scan = scanAccountDir(dir);
      const collisions = [
        ...scan.skills
          .filter((name) => built.skills.includes(name))
          .map(
            (name) =>
              `collision: skill '${name}' also exists in ${join(dir, "skills")} (Claude: a user skill beats the project one, silently)`,
          ),
        ...scan.agents
          .filter((name) => built.agents.includes(name))
          .map(
            (name) =>
              `collision: agent '${name}' also exists in ${join(dir, "agents")} (move it into a layer)`,
          ),
      ];
      problems += collisions.length;
      line(
        `  ${dir}: ${collisions.length === 0 ? "no collisions" : plural(collisions.length, "collision")}`,
      );
      for (const collision of collisions) line(`    ${collision}`);
      if (scan.hasClaudeMd)
        line(`    note: ${join(dir, "CLAUDE.md")} is loaded into every session as well`);
    }

    const bin = ctx.env.ALZ_CLAUDE_BIN ?? "claude";
    const version = installedVersion(bin, ctx.env);
    const range = claude?.verified ?? { min: "", below: "" };
    if (!version) line(`claude: not found; version check skipped`);
    else if (inRange(version, range))
      line(`claude: ${version} (verified range >=${range.min} <${range.below})`);
    else
      line(
        `warning: claude ${version} is outside the verified range >=${range.min} <${range.below}; re-check docs/target-facts.md`,
      );
  }

  const { hub, addDirs } = result.manifest.workspace;
  const clientRoot = hub ? dirname(resolve(ctx.repoRoot, hub)) : undefined;
  if (clientRoot) {
    // §4.4: tools load ancestor instruction files natively, so these would load twice.
    for (const file of ["CLAUDE.md", "AGENTS.md"]) {
      if (existsSync(join(clientRoot, file)))
        line(
          `warning: client root ${join(clientRoot, file)} exists; remove it (put client content in the hub layer)`,
        );
    }
    if (existsSync(join(clientRoot, ".git")))
      line(
        `warning: client root ${clientRoot} is a git repo; it should not be (its .claude/ and AGENTS.md would load in every repo)`,
      );
  }
  for (const dir of addDirs) {
    const abs = resolve(ctx.repoRoot, dir);
    if (!existsSync(abs)) {
      line(`warning: workspace dir ${dir} does not exist (${abs})`);
      continue;
    }
    // target-facts §9: the generated setting loads nothing, but `claude --add-dir` would.
    const loaded = ["skills", "agents", "commands"].filter((sub) =>
      existsSync(join(abs, ".claude", sub)),
    );
    if (loaded.length > 0)
      line(
        `note: ${dir}/.claude has ${loaded.join(", ")}; they load only if you pass --add-dir ${dir} yourself`,
      );
  }

  // target-facts §3/§5: CLAUDE.md in every ancestor is loaded; a git repo above can inject skills.
  for (let dir = dirname(ctx.repoRoot); dir !== dirname(dir); dir = dirname(dir)) {
    if (dir === clientRoot) continue;
    if (existsSync(join(dir, "CLAUDE.md")))
      line(`warning: ${join(dir, "CLAUDE.md")} is loaded too (ancestor directory)`);
    if (existsSync(join(dir, ".git")) && existsSync(join(dir, ".claude"))) {
      line(
        `warning: ${dir} is a git repo with .claude/; its skills or agents may load in this repo`,
      );
    }
  }

  const { errors, warnings } = result.diags;
  line(`build: ${plural(errors.length, "error")}, ${plural(warnings.length, "warning")}`);
  for (const item of result.diags.items) line(`  ${formatDiagnostic(item)}`);
  problems += errors.length;

  line(problems === 0 ? "doctor: ok" : `doctor: ${plural(problems, "problem")}`);
  return problems === 0 ? 0 : 1;
}
