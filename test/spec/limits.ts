/**
 * Limits and names the spec states, written out here and not imported from src/, so a change in
 * src/ fails a test instead of moving the test with it (the convention of `AGENT_SESSION_MARKERS`
 * in test 32).
 */

/**
 * §7.13 "No fetch on resolve": a layer read by account resolution with more than 2000 entries, or
 * more than 16 MiB in all, is a cache miss (`unreadable`). Files, folders, links, other entries
 * and `.env` files count as entries; the skipped names `.git`, `.DS_Store` and `node_modules`,
 * everything below them and the layer directory do not. The bytes are those of the files read,
 * so `.env` files and links add none.
 */
export const MAX_LAYER_ENTRIES = 2000;
export const MAX_LAYER_BYTES = 16 * 1024 * 1024;

/** §7.13 "Both files" and "No fetch on resolve": a file read by resolution is at most 1 MiB. */
export const MAX_READ_FILE_BYTES = 1024 * 1024;

/**
 * §7.13 "YAML limits": at most 100 aliases in a document, counted as written, with
 * decision-28's depth, node and replay budgets. The TypeScript reference counts uses per anchor instead (reference drift,
 * TASK-62), so a case that holds for both binaries stays well below or well above 100 aliases
 * of one anchor.
 */
export const YAML_ALIAS_LIMIT = 100;

/**
 * §7.13 "Environment": the environment variables whose values account resolution reads by name,
 * besides `HOME`, which the spec lists too and which is passed as the home directory, not in
 * this list. `__MISE_DIFF` and `DIRENV_DIFF` are checked for existence only and are not read.
 */
export const ENV_NAMES: readonly string[] = [
  "CLAUDE_CONFIG_DIR",
  "CODEX_HOME",
  "ALZ_SET_CLAUDE_CONFIG_DIR",
  "ALZ_SET_CODEX_HOME",
  "XDG_CONFIG_HOME",
  "XDG_STATE_HOME",
  "XDG_CACHE_HOME",
];
