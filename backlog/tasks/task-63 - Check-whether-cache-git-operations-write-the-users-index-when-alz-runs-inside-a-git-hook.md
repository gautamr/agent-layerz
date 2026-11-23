---
id: TASK-63
title: >-
  Check whether cache git operations write the user's index when alz runs inside
  a git hook
status: To Do
assignee: []
created_date: '2026-11-22T17:19:00Z'
updated_date: '2026-11-23T04:24:00Z'
labels:
  - discovered
  - area:security
  - area:resolve
  - area:rust
milestone: m-7
dependencies: []
references:
  - >-
    backlog/decisions/decision-24 -
    alz-runs-the-git-binary-with-argument-arrays-and-an-explicit-child-environment-no-git-library-owner-TASK-55.1.2.md
priority: medium
type: bug
ordinal: 69390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while amending decision-24 after the R0 security review; not reproduced yet. Git passes GIT_INDEX_FILE (and GIT_DIR in a linked worktree) to hooks. The TypeScript reference strips GIT_* variables only for identity calls, so its cache clone and checkout probably inherit them when alz check or alz build runs from a pre-commit hook, and could write the user's index or repository instead of the cache: a write outside insideRoot and the cache. decision-24 makes the Rust port remove these variables for cache and clone operations in R3; if the TypeScript reference has the bug, it needs the same fix so both stay one oracle.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A reproduction (acceptance test or recorded manual run) shows whether alz build or alz check inside a pre-commit hook changes the user's index or repository
- [ ] #2 The Rust alz removes GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE, GIT_OBJECT_DIRECTORY, GIT_ALTERNATE_OBJECT_DIRECTORIES, GIT_COMMON_DIR, GIT_NAMESPACE and GIT_CEILING_DIRECTORIES for cache and clone operations (decision-24), with an acceptance case that is marked reference drift if the TypeScript reference fails it
- [ ] #3 The spec section on git subprocesses states the rule
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23, decision-30): src/ is never changed; this bug is fixed only in the Rust alz, in R3 (TASK-55.4); AC #2 reworded from a TypeScript fix to the Rust rule. R4 depends on this task.
<!-- SECTION:NOTES:END -->
