/**
 * Limits and names the spec states, written out here and not imported from src/, so a change in
 * src/ fails a test instead of moving the test with it (the convention of `AGENT_SESSION_MARKERS`
 * in test 32).
 */

/**
 * §7.13 "No fetch on resolve": a layer read by account resolution with more than 2000 files and
 * folders, or more than 16 MiB in all, is a cache miss (`unreadable`).
 */
export const MAX_LAYER_ENTRIES = 2000;
export const MAX_LAYER_BYTES = 16 * 1024 * 1024;

/** §7.13 "Both files" and "No fetch on resolve": a file read by resolution is at most 1 MiB. */
export const MAX_READ_FILE_BYTES = 1024 * 1024;

/**
 * §7.13 "Paths": the YAML alias limit for text read without trust. The spec names the limit but
 * not its number; 100 is the number src/ uses (spec gap, reported in TASK-55.2.2).
 */
export const YAML_ALIAS_LIMIT = 100;

/**
 * §7.13: the environment variables account resolution reads by name. The spec names each one
 * in its text (the two account variables of §7.13's table, their `ALZ_SET_*` markers, and the
 * three XDG locations) but never lists "the variables resolution reads"; the list is the union
 * of those mentions (spec gap, reported in TASK-55.2.2). `__MISE_DIFF` and `DIRENV_DIFF` are
 * checked for existence only and are not read.
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
