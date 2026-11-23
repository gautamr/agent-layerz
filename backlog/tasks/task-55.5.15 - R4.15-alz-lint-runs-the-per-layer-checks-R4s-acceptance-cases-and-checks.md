---
id: TASK-55.5.15
title: 'R4.15: alz lint runs the per-layer checks; R4''s acceptance cases and checks'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T12:28:00Z'
updated_date: '2026-11-23T12:39:00Z'
labels:
  - area:rust
  - area:lint
milestone: m-7
dependencies:
  - TASK-55.5.14
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 128390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step.
- alz lint, with --layer and in stack mode, runs every kind's per-layer reader on each layer alone and drops `unresolved`, as spec §10 and the reference do (owner answer 3 in TASK-55.5: no cross-layer merge in lint, no spec change; merge errors show in build, R5), plus the provides_slots and requires_slots checks.
- Acceptance: test/acceptance/37-settings-targets.test.ts with fixture test/fixtures/37-settings-targets and its §13 entry (R4.1): "a settings or MCP file whose top-level key is not a known target is refused, whatever the key", one row per key, through alz lint --layer and through alz build (no output written). Both groups use referenceDrift("TASK-55.5", ...); the build rows pass by R5.
- Run tests 02 to 11, 17 to 19, 26, 29 and 37 against ALZ_BIN and write into TASK-55.5's notes each merge case with the Rust test that gives its verdict (TASK-55.5 AC #1).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz lint reports the per-layer errors of every item kind and no unresolved one, for one layer and for each layer of a stack alone (assert_cmd tests)
- [ ] #2 Test 37's lint rows pass against the Rust binary and are marked reference drift against the TypeScript reference (mise run test:acceptance, with and without ALZ_BIN)
- [ ] #3 Every case of tests 02 to 11, 17 to 19, 26, 29 and 37 that drives alz lint passes against ALZ_BIN, the 84 of the 2026-11-23 baseline included, and TASK-55.5's notes list each merge case with its Rust test
- [ ] #4 mise run rust:ci, mise run lint and mise run alz:check pass, and src/ is unchanged (git diff --stat main -- src)
<!-- AC:END -->
