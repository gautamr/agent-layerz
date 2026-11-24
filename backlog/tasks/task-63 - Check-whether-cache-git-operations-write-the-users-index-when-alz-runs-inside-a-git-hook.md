---
id: TASK-63
title: >-
  Check whether cache git operations write the user's index when alz runs inside
  a git hook
status: In Progress
assignee: []
created_date: '2026-11-22T17:19:00Z'
updated_date: '2026-11-24T03:36:00Z'
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
- [x] #1 A reproduction (acceptance test) shows whether alz lint in a repository with a git: layer, run inside a pre-commit hook in a main checkout and in a linked worktree, changes the user's index or repository (lint twin, R3.11)
- [x] #2 The Rust alz removes GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE, GIT_OBJECT_DIRECTORY, GIT_ALTERNATE_OBJECT_DIRECTORIES, GIT_COMMON_DIR, GIT_NAMESPACE and GIT_CEILING_DIRECTORIES for cache and clone operations (decision-24), with an acceptance case that is marked reference drift if the TypeScript reference fails it
- [x] #3 The spec section on git subprocesses states the rule
- [ ] #4 The alz check form of AC #1, run as a pre-commit hook, is in the acceptance suite (written in R3.11 where it is missing) and is checked again against the Rust alz in R5 (TASK-55.6)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23, decision-30): src/ is never changed; this bug is fixed only in the Rust alz, in R3 (TASK-55.4); AC #2 reworded from a TypeScript fix to the Rust rule. R4 depends on this task.

Lead (2026-11-23, owner asked the lead to decide): no row in docs/target-facts.md for --end-of-options or the hook environment, since that file holds Claude Code and Codex facts only; these git facts are recorded in decision-24 (including the observed GIT_INDEX_FILE in a pre-commit hook). AC #3's spec text should also name the oldest git that alz supports, at least 2.24, the first with --end-of-options, and what alz does with an older git.

Architect (2026-11-23, owner answer 7 in TASK-55.4, delegated by the lead): criteria reworded to the lint twins that R3.11 adds; the build, check and account-resolution forms are checked again in R5 (TASK-55.6) or R6 (TASK-55.7), as the last criterion says. The other criteria are unchanged. Correction to the lead's note: the oldest supported git is 2.30, not 2.24. `rev-parse` accepts --end-of-options from 2.30, and 2.24 is the floor for checkout only (decision-24's evidence). Owner answer 4: below 2.30, alz stops with error [git], naming the version it found and the minimum.

R3.11 (2026-11-23): AC1-3 checked. Reproduction: a pre-commit hook that snapshots every file under the main .git (index included) before and after running alz lint, in a main checkout and in a linked worktree. Against the Rust binary both leave the repository byte-identical and lint exits 0. Against the TypeScript reference the main-checkout case passes (the index is not changed there) and the linked-worktree case fails, so only the worktree case is referenceDrift TASK-63. Spec 'Git subprocesses' states the rule (R3.1). AC4 left unchecked: the alz check hook form is written and fails against Rust until R5 (check does not exist yet).

R5.8 (TASK-55.6.8, 2026-11-24): build form run against ALZ_BIN=target/release/alz in test 36 (16 passed, 4 Linux-only skipped) and test 24 (259 of 277; none of the 18 failures is this task's rows: --adopt R5.14, account which R6, agent mcpServers and nested JSON R4 follow-up). AC #4 left for the lead to check.
<!-- SECTION:NOTES:END -->
