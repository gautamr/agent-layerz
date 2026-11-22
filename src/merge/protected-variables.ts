import type { Diagnostics } from "../core/diagnostics";
import { InputError } from "../core/diagnostics";
import { parseMarkdown } from "../core/frontmatter";
import { isMap } from "../core/yaml";
import {
  asciiLower,
  asciiUpper,
  isProtectedName,
  matchedProtectedNames,
  PROTECTED_VARIABLES,
} from "../model/account";
import { type Layer, where } from "../model/layer";
import { readLayerText } from "./layer-files";

/** Build-error code for a layer that sets an account or location variable (§7.9). */
export const PROTECTED_CODE = "protected-variable";

const CHOOSES =
  "no layer may set the variables that choose an account folder or locate alz's files";

/**
 * One error per problem. `path` is a key path chosen by alz, never a key from the layer; the
 * variable named is one of the six fixed names, never the text the layer wrote (§7.9).
 */
export function refuse(diags: Diagnostics, loc: string, path: string, variable: string): void {
  diags.error(PROTECTED_CODE, `${path} names ${variable}; ${CHOOSES} (§7.9)`, loc);
}

export function refuseUnchecked(diags: Diagnostics, loc: string, path: string, why: string): void {
  diags.error(PROTECTED_CODE, `${path} ${why}, so it cannot be checked (§7.9)`, loc);
}

/** A variable name is a plain identifier; anything else (`NAME=x`, NUL, spaces) is refused (§7.9). */
const ENV_NAME = /^[A-Za-z_][A-Za-z0-9_]*$/;

export function isEnvName(name: string): boolean {
  return ENV_NAME.test(name);
}

export function refuseName(diags: Diagnostics, loc: string, path: string, what: string): void {
  diags.error(
    PROTECTED_CODE,
    `${path} has ${what} that is not a plain variable name (letters, digits and '_', not starting with a digit) (§7.9)`,
    loc,
  );
}

/** The fixed name a variable name stands for, ignoring ASCII case. */
export function protectedFor(name: string): string | undefined {
  return isProtectedName(name)
    ? PROTECTED_VARIABLES.find((fixed) => fixed === asciiUpper(name))
    : undefined;
}

/** One variable name from an untrusted list or key: refused when not a plain name or a protected one. */
export function checkEnvName(
  name: string,
  path: string,
  what: string,
  loc: string,
  diags: Diagnostics,
): void {
  if (!isEnvName(name)) {
    refuseName(diags, loc, path, what);
    return;
  }
  const variable = protectedFor(name);
  if (variable) refuse(diags, loc, path, variable);
}

/** Claude `env`, Codex `set`, an MCP `env` (§7.9): keys are variable names; a non-mapping cannot be checked. */
export function checkEnvKeys(env: unknown, path: string, loc: string, diags: Diagnostics): void {
  if (env === undefined || env === null) return;
  if (!isMap(env)) {
    refuseUnchecked(diags, loc, path, "is not a mapping");
    return;
  }
  for (const key of Object.keys(env)) checkEnvName(key, path, "a key", loc, diags);
}

/** `env` of each server in either shape: a map of name to definition, or a list of names and one-entry maps. */
export function checkMcpServerEnvs(
  servers: unknown,
  path: string,
  loc: string,
  diags: Diagnostics,
): void {
  let definitions: unknown[] = [];
  if (Array.isArray(servers)) {
    definitions = servers.flatMap((item) => (isMap(item) ? Object.values(item) : []));
  } else if (isMap(servers)) {
    definitions = Object.values(servers);
  }
  for (const definition of definitions) {
    if (isMap(definition)) checkEnvKeys(definition.env, `${path}.env`, loc, diags);
  }
}

/**
 * Codex `shell_environment_policy` anywhere in the tree (profiles included): `set` keys,
 * `exclude` patterns, and `filters` patterns not set to `include` (target-facts §12 item 15).
 */
export function checkCodexPolicies(tree: unknown, loc: string, diags: Diagnostics): void {
  const walk = (node: unknown): void => {
    if (Array.isArray(node)) {
      for (const item of node) walk(item);
      return;
    }
    if (!isMap(node)) return;
    for (const [key, value] of Object.entries(node)) {
      if (key === "shell_environment_policy") checkPolicy(value, loc, diags);
      else walk(value);
    }
  };
  walk(tree);
}

const POLICY = "codex.shell_environment_policy";

function checkPattern(pattern: unknown, path: string, loc: string, diags: Diagnostics): void {
  if (typeof pattern !== "string") {
    refuseUnchecked(diags, loc, path, "holds a pattern that is not a string");
    return;
  }
  for (const variable of matchedProtectedNames(pattern)) refuse(diags, loc, path, variable);
}

function checkPolicy(policy: unknown, loc: string, diags: Diagnostics): void {
  if (!isMap(policy)) return;
  checkEnvKeys(policy.set, `${POLICY}.set`, loc, diags);
  const { exclude, filters } = policy;
  if (Array.isArray(exclude)) {
    for (const pattern of exclude) checkPattern(pattern, `${POLICY}.exclude`, loc, diags);
  } else if (exclude !== undefined && exclude !== null) {
    refuseUnchecked(diags, loc, `${POLICY}.exclude`, "is not a list");
  }
  if (isMap(filters)) {
    for (const [pattern, action] of Object.entries(filters)) {
      if (action !== "include") checkPattern(pattern, `${POLICY}.filters`, loc, diags);
    }
  } else if (filters !== undefined && filters !== null) {
    refuseUnchecked(diags, loc, `${POLICY}.filters`, "is not a mapping");
  }
}

const MAX_DEPTH = 32;

/**
 * Every `env` mapping below the root of a parsed JSON file. Returns false when the file is
 * nested deeper than `MAX_DEPTH`, so the caller can refuse it instead of stopping silently.
 */
function checkNestedEnvs(node: unknown, loc: string, diags: Diagnostics, depth = 0): boolean {
  if (depth > MAX_DEPTH) return false;
  let complete = true;
  if (Array.isArray(node)) {
    for (const item of node) complete = checkNestedEnvs(item, loc, diags, depth + 1) && complete;
    return complete;
  }
  if (!isMap(node)) return true;
  for (const [key, value] of Object.entries(node)) {
    if (key === "env" && isMap(value)) checkEnvKeys(value, "a nested env", loc, diags);
    else complete = checkNestedEnvs(value, loc, diags, depth + 1) && complete;
  }
  return complete;
}

/**
 * Claude settings, `.mcp.json` and plugin manifests: the root `env`, the root `mcpServers` and
 * any nested `env`. The root `mcpServers` is checked once, by `checkMcpServerEnvs`.
 */
function checkClaudeJson(json: unknown, loc: string, diags: Diagnostics): void {
  if (!isMap(json)) return;
  checkEnvKeys(json.env, "env", loc, diags);
  checkMcpServerEnvs(json.mcpServers, "mcpServers", loc, diags);
  let complete = true;
  for (const [key, value] of Object.entries(json)) {
    if (key !== "env" && key !== "mcpServers") {
      complete = checkNestedEnvs(value, loc, diags) && complete;
    }
  }
  if (!complete) {
    refuseUnchecked(diags, loc, "this file", `is nested deeper than ${MAX_DEPTH} levels`);
  }
}

const JSON_SUFFIXES = [".claude/settings.json", ".claude/settings.local.json", ".mcp.json"];
const PLUGIN_JSON = /(^|\/)\.claude-plugin\/[^/]+\.json$/;
const CLAUDE_AGENT = /(^|\/)\.claude\/agents\/.+\.md$/;
function isClaudeJson(key: string): boolean {
  return JSON_SUFFIXES.some((suffix) => key.endsWith(suffix)) || PLUGIN_JSON.test(key);
}

const POLICY_KEY = "shell_environment_policy";
/** A bare TOML key is letters, digits, `_` and `-`; a name inside a longer word is a different name. */
const WORD_CHAR = "[A-Za-z0-9_-]";
const NAMES = new RegExp(
  `(?<!${WORD_CHAR})(${PROTECTED_VARIABLES.join("|")})(?!${WORD_CHAR})`,
  "i",
);
/** A TOML escape or a line-ending backslash can spell a name in a way text search cannot see. */
const HIDING_ESCAPE = /\\(?:[uUx]|[ \t]*\r?\n)/;
const TOML_FILE = ".toml file";

function checkTomlText(text: string, loc: string, diags: Diagnostics): void {
  if (asciiLower(text).includes(POLICY_KEY)) refuse(diags, loc, TOML_FILE, POLICY_KEY);
  const named = NAMES.exec(text)?.[1];
  if (named) refuse(diags, loc, TOML_FILE, asciiUpper(named));
  if (HIDING_ESCAPE.test(text)) {
    refuseUnchecked(
      diags,
      loc,
      TOML_FILE,
      "has a \\u, \\U or \\x escape or a line-ending backslash",
    );
  }
}

function checkAgentFile(text: string, loc: string, diags: Diagnostics): void {
  try {
    const { data } = parseMarkdown(text);
    checkMcpServerEnvs(data.mcpServers, "mcpServers", loc, diags);
  } catch (error) {
    if (!(error instanceof InputError)) throw error;
    refuseUnchecked(diags, loc, "this agent file", "has frontmatter that does not parse");
  }
}

/**
 * Passthrough files that can set an account or location variable (§7.9, §7.11). `path` is the
 * path after `targets/<target>/`, compared with ASCII case folding. JSON files are parsed and
 * nothing is executed; alz has no TOML parser, so a Codex `.toml` file is refused on any
 * mention (text rule, owner-accepted). Non-ASCII text in a path that a file system might fold
 * into a checked name is refused.
 */
export function checkPassthroughFile(
  layer: Layer,
  file: string,
  path: string,
  diags: Diagnostics,
): void {
  const loc = where(layer, file);
  const key = asciiLower(path);
  // A file system may fold a non-ASCII character into an ASCII one (APFS folds a long s into
  // `s`), so no checked name can be recognised in a path that has one (§7.9, target-facts §12.2).
  if ([...file].some((char) => (char.codePointAt(0) ?? 0) > 0x7f)) {
    refuseUnchecked(diags, loc, "this path", "has a non-ASCII character");
  }
  if (isClaudeJson(key)) {
    let json: unknown;
    try {
      json = JSON.parse(readLayerText(layer, file));
    } catch {
      refuseUnchecked(diags, loc, "this file", "is not valid JSON");
      return;
    }
    checkClaudeJson(json, loc, diags);
    return;
  }
  if (CLAUDE_AGENT.test(key)) {
    checkAgentFile(readLayerText(layer, file), loc, diags);
    return;
  }
  if (key.endsWith(".toml")) {
    checkTomlText(readLayerText(layer, file), loc, diags);
  }
}
