---
id: TASK-55.6.14
title: 'R5.14: alz build --adopt'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
labels:
  - area:rust
  - area:adopt
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.6.13
  - TASK-55.6.8
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 146390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§4.3, §10).
- RepoRoot applies an accepted AdoptionPlan: layer files written create-only, moves as renameat inside the repository, removals, empty folders pruned, then the manifest edit as owner question 2 decides (keeping its comments and layout).
- build --adopt: notes `note [adopt]`, problems `error [adopt]` and the line that nothing was adopted, exit 1 with the repository unchanged; else the adopted lines, a second pipeline run from the manifest, and any collision left is an error.
- The repo layer must be a path: entry inside the repository and last in the stack, else the error adopt.
- Setting the local git identity on --adopt is R6.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test 20 passes against ALZ_BIN: the adopted repository builds the same settings and agents, the manifest keeps its comments, and check passes (vitest run)
- [ ] #2 When any file cannot be adopted the repository tree is byte-identical before and after (test 20 against ALZ_BIN and an alz-io test)
- [ ] #3 Nothing adoption writes, moves or deletes lies outside the repository or goes through a link (alz-io tests)
<!-- AC:END -->
