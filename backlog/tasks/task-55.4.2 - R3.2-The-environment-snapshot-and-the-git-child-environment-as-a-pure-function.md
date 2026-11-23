---
id: TASK-55.4.2
title: >-
  R3.2: The environment snapshot and the git child environment as a pure
  function
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T08:08:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.4.1
parent_task_id: TASK-55.4
priority: high
type: task
ordinal: 103390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Fixes TASK-63's Rust part (AC #2), together with R3.3.
- alz-io `env.rs`: `EnvSnapshot`, taken once in `main` through `std::env::vars_os`. It is the only reader of the process environment. It has typed queries (HOME, XDG_CACHE_HOME, CLAUDE_CONFIG_DIR, CODEX_HOME) and gives `EnvVars`, an ordered map of `OsString`s.
- alz-core `git/environment.rs`: `GitOperation`, with the variants `Clone { transport }`, `CacheRead` (rev-parse and checkout in a temporary clone or a cache entry) and `TreeRead`, and the pure function `child_environment(GitOperation, &EnvVars) -> EnvVars`. For every operation it removes GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE, GIT_OBJECT_DIRECTORY, GIT_ALTERNATE_OBJECT_DIRECTORIES, GIT_COMMON_DIR, GIT_NAMESPACE and GIT_CEILING_DIRECTORIES, and GIT_CONFIG_PARAMETERS, GIT_CONFIG_COUNT and every GIT_CONFIG_KEY_n and GIT_CONFIG_VALUE_n (owner answer 2). A remote clone also loses GIT_ALLOW_PROTOCOL. It adds GIT_TERMINAL_PROMPT=0 everywhere, and GIT_NO_REPLACE_OBJECTS=1 and GIT_NO_LAZY_FETCH=1 for tree reads. Every other variable passes unchanged.
- Calls on the repository being built (identity, `init`) are R6's (decision-24) and get no variant here.
Laws, as proptest:
- no removed name survives;
- GIT_TERMINAL_PROMPT is always 0;
- every other variable is unchanged, byte for byte;
- applying the function twice gives the same result.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 child_environment removes the eight repository variables of decision-24 and GIT_CONFIG_PARAMETERS, GIT_CONFIG_COUNT and every GIT_CONFIG_KEY_n and GIT_CONFIG_VALUE_n for every cache and clone operation, and keeps every other variable byte for byte (proptest laws in alz-core)
- [x] #2 A remote clone's environment has no GIT_ALLOW_PROTOCOL, every operation has GIT_TERMINAL_PROMPT=0, and tree reads add GIT_NO_REPLACE_OBJECTS=1 and GIT_NO_LAZY_FETCH=1 (unit tests)
- [x] #3 The process environment is read only by EnvSnapshot in alz-io; clippy's disallowed-methods still bans std::env in alz-core and alz (mise run rust:check)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (2026-11-23), R3.2 done in the working tree, uncommitted. Files: crates/alz-core/src/env.rs (EnvVars: ordered OsString map, Debug lists names only), crates/alz-core/src/git.rs and git/environment.rs (+ environment/tests.rs: GitOperation { Clone { transport: CloneTransport::{Remote,Local} }, CacheRead, TreeRead }, child_environment), crates/alz-io/src/env.rs (EnvSnapshot: capture() via std::env::vars_os, from_vars, home/xdg_cache_home/claude_config_dir/codex_home returning the raw Option<&OsStr>, vars()), crates/alz-io/clippy.toml (new), one-line mod/pub use edits in alz-core and alz-io lib.rs. EnvVars lives in alz-core because alz-core cannot depend on alz-io. Not wired into main yet: nothing consumes the snapshot until R3.3 and R3.5; capture() is ready for main. The queries return raw values; empty or relative handling is R3.5's (CacheLocation).
Decisions: <n> in GIT_CONFIG_KEY_<n>/VALUE_<n> means one or more ASCII digits (GIT_CONFIG_KEY_, GIT_CONFIG_KEY_x and GIT_CONFIG_GLOBAL pass). Names that are not UTF-8 match no removed name, so they pass. Every tree read (rev-parse, ls-tree, cat-file, including the show-toplevel/show-prefix calls of a pinned path:) uses TreeRead, CacheRead is rev-parse/checkout in a temp clone or cache entry.
AC #1: proptests in alz-core git::environment::tests: no_removed_name_survives, every_other_variable_is_unchanged (both directions, byte for byte), applying_the_function_twice_gives_the_same_result, plus unit tests the_eight_repository_variables_are_removed_for_every_operation and the_config_variables_of_a_parent_git_are_removed_for_every_operation, a non-UTF-8 value test (unix), and a test that ssh/proxy/credential variables pass. cargo test -p alz-core: 287 passed.
AC #2: units a_remote_clone_has_no_allowed_protocol_list, the_other_operations_keep_the_allowed_protocol_list, every_operation_turns_terminal_prompts_off_over_any_inherited_value, only_a_tree_read_ignores_replace_refs_and_lazy_fetch, and proptest terminal_prompts_are_always_off.
AC #3: EnvSnapshot::capture is the only std::env reader; it carries the one #[expect(clippy::disallowed_methods, reason)]. alz-core and alz clippy.toml still ban std::env (unchanged). New crates/alz-io/clippy.toml bans var, var_os, vars, vars_os, set_var and remove_var in alz-io (repeats the four allow-*-in-tests lines, since clippy does not merge files); I checked that a probe std::env::var_os call outside capture warns, then removed it. Note for R3.3 and R3.5: alz-io may not call std::env::var*/vars* anywhere else; current_exe and current_dir are not banned there.
Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, msrv, deny, audit); mise run lint exit 0. Only an Rust/alz-io/alz-core change; no src/ or acceptance change, so test:acceptance not run.

Verified by the lead (2026-11-23): mise run rust:ci exits 0 (361 tests), lint passes, src/ unchanged. Lead: GIT_CONFIG_KEY_<n>/VALUE_<n> with <n> one or more ASCII digits is accepted (what git reads); the GitOperation assignment (TreeRead for pinned-tree reads, CacheRead for rev-parse/checkout in a clone or cache entry) is binding for R3.3.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The git child environment is a pure function in alz-core over an ordered snapshot: it drops the eight repository variables of decision-24 and every inherited -c option (GIT_CONFIG_PARAMETERS, GIT_CONFIG_COUNT, GIT_CONFIG_KEY_n/VALUE_n), always sets GIT_TERMINAL_PROMPT=0, removes GIT_ALLOW_PROTOCOL for remote clones and adds the tree-read guards; property tests cover it. alz-io's EnvSnapshot::capture is the only reader of the process environment (clippy-banned elsewhere).
<!-- SECTION:FINAL_SUMMARY:END -->
