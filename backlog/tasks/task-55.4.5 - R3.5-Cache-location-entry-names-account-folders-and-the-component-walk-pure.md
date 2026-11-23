---
id: TASK-55.4.5
title: >-
  R3.5: Cache location, entry names, account folders and the component walk,
  pure
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T07:59:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.4.1
parent_task_id: TASK-55.4
priority: high
type: task
ordinal: 106390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Fixes TASK-53 (AC #3) and the core of TASK-52 (AC #3), as pure code. It adds unicode-normalization =0.1.26 with default features off, per decision-37.
- alz-core `cache/location.rs`: `CacheLocation::from_env(home, xdg_cache_home)`. Unset, empty or relative means `<home>/.cache/agent-layerz`; absolute means `<value>/agent-layerz`. HOME follows the rule of §7.13 "Paths" (R6 shares it). This is the one function that lint, build and account resolution call.
- `cache/entry_name.rs`: `CloneKey` (the sha256 of the clone argument), `EntryName` and its `Display`.
- `account/folders.rs`: `AccountFolders::from_env(home, CLAUDE_CONFIG_DIR, CODEX_HOME)`: `~/.claude`, `~/.claude.json`, `~/.codex`, `~/.claude-*`, `~/.codex-*` and the two named folders. `touches(&AbsolutePath)` says whether a path is one of them, lies inside one, or is `$HOME` or above, comparing each component in NFC and lowercased (TASK-52's note).
- `path/walk.rs`: `RealPathWalk`, a sans-IO state machine that resolves an absolute path one component at a time:
  - its `next()` gives `Request::Lstat(prefix)`, `Request::Readlink(prefix)` or `Done(real path)`;
  - before it requests anything about a prefix, it checks that prefix against `AccountFolders`, and the refusal names the reason, never the path text;
  - links are followed with a hop limit of 40, and an absolute target restarts the walk from `/`.
Law (proptest, TASK-52 AC #3): for every sequence of answers, no request names a path that `touches` reports. Account resolution (R6) drives the same walk.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CacheLocation gives ~/.cache/agent-layerz for an unset, empty or relative XDG_CACHE_HOME and <value>/agent-layerz for an absolute one, and is the only cache-location function in the workspace (unit tests; TASK-53 AC #3)
- [ ] #2 For any answers to its requests, RealPathWalk never requests a path at or below an account folder or at or above HOME, including through planted links and case or Unicode-form variants (proptest; TASK-52 AC #3)
- [ ] #3 EntryName prints git-<hex>@<sha>, tree2@<tree> and pathref@<hex>-<sha>, and is injective over its inputs (unit and property tests)
<!-- AC:END -->
