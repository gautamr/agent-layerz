---
id: TASK-55.2.3
title: 'R1.3: Move the TypeScript-internal cases of 21, 24, 26, 28 and 29 to reference'
status: To Do
assignee: []
created_date: '2026-11-22T17:55:00Z'
labels:
  - area:test
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.2.2
parent_task_id: TASK-55.2
priority: high
type: task
ordinal: 73390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rule for this subtask and R1.4 to R1.6: a case that calls a src/ function moves to test/reference/NN-<same name>.test.ts with its name, body and assertions unchanged. If the behaviour can be seen through the CLI today, an acceptance case is added that shows it through the CLI; the PR lists each moved case with its CLI counterpart, or with "not reachable by the CLI yet" and the Rust task that will test it. Coverage of the TypeScript code is never lost: the moved case still runs.

Cases:
- 21: `parseStackOption` table. Counterpart: `alz init --stack` with a bare path, `path:...@ref` and `github:...@v` writes those sources and refs into layer.yaml.
- 24: the two `resolveAccounts` cases ("a refused account declaration, at resolution"). Counterpart: `alz account which` shows the ignored declaration and its one `account-declaration-ignored` warning. The two fake-terminal cases stay for R1.6.
- 26: the `isInsideRepo` tables and the `guardedClone`/`git` control. Counterparts: a build whose stack names `git:host.invalid:client.git@v1` while a folder of that name exists in the repo is rejected as inside the repository; a build with `GIT_ALLOW_PROTOCOL=file` in the environment still refuses the local clone the guard refuses. The remote side of the table makes no network call and stays reference-only.
- 28: the `verifyCachedLayer` and `lockedEntryFor` units, and the `gitCacheKey`/`githubCloneUrl` unit. Counterpart: after a build from a git source, the cache holds the entry named by the R1.2 formula.
- 29: the `parseAccount`, `isAccountName`, `codexPatternMatches` and `evaluateDeclarations` tables, and every `config()` (computeBuild) use. Where a `config()` case asserts what reaches generated files, it is rewritten as a CLI case that reads those files.
- Out of scope: 30, 31, 32, and the harness.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 21, 24, 26, 28 and 29 under test/acceptance import nothing from src/ directly; the R1.1 guard list drops their direct imports (24 keeps only the fake-terminal cases until R1.6)
- [ ] #2 Every moved case runs under test/reference with its original name. The PR description lists each moved case with its CLI counterpart or the reason it has none
- [ ] #3 `mise run test` passes; the acceptance and reference case counts are at least the R1.1 baseline, recorded in the notes
<!-- AC:END -->
