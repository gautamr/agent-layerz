---
id: TASK-55.4.2
title: >-
  R3.2: The environment snapshot and the git child environment as a pure
  function
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T08:04:00Z'
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
- [ ] #1 child_environment removes the eight repository variables of decision-24 and GIT_CONFIG_PARAMETERS, GIT_CONFIG_COUNT and every GIT_CONFIG_KEY_n and GIT_CONFIG_VALUE_n for every cache and clone operation, and keeps every other variable byte for byte (proptest laws in alz-core)
- [ ] #2 A remote clone's environment has no GIT_ALLOW_PROTOCOL, every operation has GIT_TERMINAL_PROMPT=0, and tree reads add GIT_NO_REPLACE_OBJECTS=1 and GIT_NO_LAZY_FETCH=1 (unit tests)
- [ ] #3 The process environment is read only by EnvSnapshot in alz-io; clippy's disallowed-methods still bans std::env in alz-core and alz (mise run rust:check)
<!-- AC:END -->
