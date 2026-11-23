---
id: TASK-55.4.5
title: >-
  R3.5: Cache location, entry names, account folders and the component walk,
  pure
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T08:16:00Z'
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
- [x] #1 CacheLocation gives ~/.cache/agent-layerz for an unset, empty or relative XDG_CACHE_HOME and <value>/agent-layerz for an absolute one, and is the only cache-location function in the workspace (unit tests; TASK-53 AC #3)
- [x] #2 Property test: no answers make the path walk ask about a name inside an account folder (or a configured CLAUDE_CONFIG_DIR or CODEX_HOME folder), and a result at or above HOME or holding a configured folder is refused (TASK-52 AC #3); names above HOME, such as / and /Users, are looked at, since every walk passes through them
- [x] #3 EntryName prints git-<hex>@<sha>, tree2@<tree> and pathref@<hex>-<sha>, and is injective over its inputs (unit and property tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented in an isolated worktree at fa4aaec, merged by the lead (2026-11-23); Cargo.toml line added by hand beside R3.3's rustix change. Implementer's rust:ci and lint exit 0 there; after the merge cargo test -p alz-core gives 359 passed. Lead reworded AC #2 to what a walk can do (names above HOME are looked at; lookups inside account folders are never made; results at or above HOME are refused), matching spec §6 and §9.1. Mutation check: making AccountFolder not count as inside failed the walk proptest and the planted-link test. Open for R3.6 and R6: resolve HOME and CLAUDE_CONFIG_DIR/CODEX_HOME to real paths once and pass them to with_real_home / with_real_configured (macOS /var vs /private/var). Layer-level refusal message constructors are not built here.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
One CacheLocation function computes the layer cache root (an empty or relative XDG_CACHE_HOME counts as unset); EntryName prints and parses git-, tree2@ and pathref@ entries injectively; AccountFolders tells whether a path is inside, at or above an account folder (NFC- and case-folded); and a pure RealPathWalk state machine resolves a path one component at a time, never asking about a name inside an account folder (property test with a mutation check). Adds unicode-normalization (decision-37).
<!-- SECTION:FINAL_SUMMARY:END -->
