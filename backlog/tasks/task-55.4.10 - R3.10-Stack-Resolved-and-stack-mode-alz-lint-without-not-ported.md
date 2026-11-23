---
id: TASK-55.4.10
title: 'R3.10: Stack<Resolved> and stack-mode alz lint, without [not-ported]'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T09:19:00Z'
labels:
  - area:rust
  - area:resolve
milestone: m-7
dependencies:
  - TASK-55.4.9
parent_task_id: TASK-55.4
priority: high
type: task
ordinal: 111390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Removes R2's `[not-ported]`, and fixes TASK-71 end to end through lint.
- alz-io `resolve/working_tree.rs`: an unpinned `path:` opens its `Dir` through the walk (TASK-52 refusal), scans it with `Purpose::Build`, and is not checked against the lock.
- alz-io `resolve.rs`: `stack(&Manifest, Option<&Lock>, &EnvSnapshot, &GitRunner, &CacheDir) -> Result<Stack<Resolved>, Diagnostics>`, for each entry in stack order:
  1. reads `layer.yaml` as a regular file through `Dir` (O_NOFOLLOW, 1 MiB);
  2. parses it with R2's `LayerMeta`;
  3. records `Pin { rev, sha }`.
  Then §6 step 2 in alz-core's `stack/validate.rs`: visibility order and duplicate names, with behaviour from src/resolve/layer-load.ts validateStack.
- alz `command/lint.rs`, stack mode:
  1. builds `EnvSnapshot`, `CacheLocation`, `CacheDir` and `GitRunner` only when a `git:`, `github:` or pinned `path:` entry needs them;
  2. resolves;
  3. lints each layer's meta;
  4. prints `lint ok: <names>` or `lint failed: …`, as in R2's layer mode (§10.1).
  `Code::NotPorted` and its message are removed.
- A `git:` source holding credentials stops at the manifest, before any cache or git access (R2.6's rule).
- assert_cmd tests in `crates/alz/tests/lint.rs`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz lint in a repository resolves every stack entry, validates the stack (visibility order, duplicate names, layer.yaml a regular file) and prints lint ok: <names>; no output holds not-ported (assert_cmd tests)
- [ ] #2 alz lint with a git:https://user:token@host/x.git@v1 source exits 1 with error [source] and the §5.2 message, runs no git, creates no cache directory, and stderr holds no token (assert_cmd test; TASK-71 AC #1 and AC #2 through lint)
- [ ] #3 An unpinned path: layer is read in place and never checked against the lock, while a cached git: or pinned layer is (assert_cmd tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R3.8 (2026-11-23): wire the origin refusals (OriginRefusal::into_diagnostic: account folder, '..' in a local git path, invalid file:// URL) into stack resolution.
<!-- SECTION:NOTES:END -->
