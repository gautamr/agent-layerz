import { existsSync } from "node:fs";
import { isAbsolute, join, posix } from "node:path";
import { FatalError, InputError } from "../core/diagnostics";
import { asStringList, isMap, readYamlFile, unknownKeys, type YamlMap } from "../core/yaml";

export const MANIFEST_FILE = "layer.yaml";

/** A git tag, branch or commit; never starts with `-`, so git cannot read it as an option. */
export const GIT_REF = /^(?!-)(?!.*\.\.)[A-Za-z0-9._/+-]+$/;

export interface StackEntry {
  source: string;
  ref?: string;
}

export interface Manifest {
  stack: StackEntry[];
  targets: string[];
  mcpEnable: string[];
  workspace: {
    /** The client hub (§4.4), relative to the repo. */
    hub?: string;
    /** Additional directories each target may read and write: the hub first, then `add_dirs`. */
    addDirs: string[];
  };
  /** Extra globs the build never writes or deletes, on top of the defaults (§4.5). */
  protectedPaths: string[];
  budgets: {
    instructionsMaxLines?: number;
    instructionsMaxBytes: number;
    alwaysLoadedSkillsMax?: number;
  };
  options: {
    commitGenerated: boolean;
    strictSlots: boolean;
  };
}

/** Codex stops reading AGENTS.md at 32 KiB (`project_doc_max_bytes`). */
const DEFAULT_MAX_BYTES = 32 * 1024;

export function readManifest(repoRoot: string): Manifest {
  const path = join(repoRoot, MANIFEST_FILE);
  if (!existsSync(path)) {
    throw new FatalError(
      "no-manifest",
      `no ${MANIFEST_FILE} here; run \`alz init\` first`,
      repoRoot,
    );
  }
  const raw = readYamlFile(path, MANIFEST_FILE);
  try {
    return parseManifest(raw);
  } catch (error) {
    if (error instanceof InputError) throw new FatalError("manifest", error.message, MANIFEST_FILE);
    throw error;
  }
}

export function isStackManifest(raw: unknown): boolean {
  return isMap(raw) && "stack" in raw;
}

export function parseManifest(raw: unknown): Manifest {
  if (!isMap(raw) || !Array.isArray(raw.stack)) {
    throw new InputError("a repo manifest needs a 'stack' list");
  }
  const extra = unknownKeys(raw, [
    "stack",
    "targets",
    "mcp",
    "workspace",
    "protected_paths",
    "budgets",
    "options",
  ]);
  if (extra.length > 0) throw new InputError(`unknown field(s): ${extra.join(", ")}`);

  const stack = raw.stack.map((entry, index): StackEntry => {
    if (!isMap(entry) || typeof entry.source !== "string") {
      throw new InputError(`stack[${index}] needs a 'source' string`);
    }
    const bad = unknownKeys(entry, ["source", "ref"]);
    if (bad.length > 0)
      throw new InputError(`stack[${index}]: unknown field(s): ${bad.join(", ")}`);
    const ref = entry.ref === undefined ? undefined : String(entry.ref);
    if (ref !== undefined && !GIT_REF.test(ref)) {
      throw new InputError(`stack[${index}]: '${ref}' is not a valid git ref`);
    }
    return { source: entry.source, ref };
  });
  if (stack.length === 0) throw new InputError("'stack' is empty");

  const mcp = section(raw.mcp, "mcp", ["enable"]);
  const workspace = section(raw.workspace, "workspace", ["hub", "add_dirs"]);
  const budgets = section(raw.budgets, "budgets", [
    "instructions_max_lines",
    "instructions_max_bytes",
    "always_loaded_skills_max",
  ]);
  const options = section(raw.options, "options", ["commit_generated", "strict_slots"]);

  return {
    stack,
    targets: raw.targets === undefined ? ["claude"] : asStringList(raw.targets, "targets"),
    mcpEnable: mcp.enable === undefined ? [] : asStringList(mcp.enable, "mcp.enable"),
    workspace: parseWorkspace(workspace),
    protectedPaths:
      raw.protected_paths === undefined || raw.protected_paths === null
        ? []
        : asStringList(raw.protected_paths, "protected_paths"),
    budgets: {
      instructionsMaxLines: optionalNumber(
        budgets.instructions_max_lines,
        "instructions_max_lines",
      ),
      instructionsMaxBytes:
        optionalNumber(budgets.instructions_max_bytes, "instructions_max_bytes") ??
        DEFAULT_MAX_BYTES,
      alwaysLoadedSkillsMax: optionalNumber(
        budgets.always_loaded_skills_max,
        "always_loaded_skills_max",
      ),
    },
    options: {
      commitGenerated: optionalBoolean(options.commit_generated, "commit_generated") ?? true,
      strictSlots: optionalBoolean(options.strict_slots, "strict_slots") ?? true,
    },
  };
}

function parseWorkspace(raw: YamlMap): Manifest["workspace"] {
  const hub = raw.hub === undefined || raw.hub === null ? undefined : relativeDir(raw.hub, "hub");
  const listed =
    raw.add_dirs === undefined || raw.add_dirs === null
      ? []
      : asStringList(raw.add_dirs, "workspace.add_dirs").map((dir) => relativeDir(dir, "add_dirs"));
  return { hub, addDirs: [...new Set([...(hub ? [hub] : []), ...listed])] };
}

/** Workspace dirs stay relative so the generated output is the same on every machine. */
function relativeDir(value: unknown, field: string): string {
  if (typeof value !== "string" || value.trim() === "") {
    throw new InputError(`workspace.${field} entries must be non-empty paths`);
  }
  if (isAbsolute(value) || value.startsWith("~")) {
    throw new InputError(`workspace.${field} '${value}' must be relative to the repo`);
  }
  return posix.normalize(value.replaceAll("\\", "/")).replace(/\/+$/, "");
}

function section(value: unknown, name: string, allowed: string[]): YamlMap {
  if (value === undefined || value === null) return {};
  if (!isMap(value)) throw new InputError(`'${name}' must be a mapping`);
  const extra = unknownKeys(value, allowed);
  if (extra.length > 0) throw new InputError(`${name}: unknown field(s): ${extra.join(", ")}`);
  return value;
}

function optionalNumber(value: unknown, field: string): number | undefined {
  if (value === undefined || value === null) return undefined;
  if (typeof value !== "number" || !Number.isInteger(value) || value < 0) {
    throw new InputError(`'${field}' must be a non-negative integer`);
  }
  return value;
}

function optionalBoolean(value: unknown, field: string): boolean | undefined {
  if (value === undefined || value === null) return undefined;
  if (typeof value !== "boolean") throw new InputError(`'${field}' must be true or false`);
  return value;
}
