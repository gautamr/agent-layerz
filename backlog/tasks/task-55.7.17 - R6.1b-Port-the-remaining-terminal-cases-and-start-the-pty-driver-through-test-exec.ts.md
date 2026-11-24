---
id: TASK-55.7.17
title: >-
  R6.1b: Port the remaining terminal cases, and start the pty driver through
  test/exec.ts
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-24T10:46:00Z'
updated_date: '2026-11-24T11:15:00Z'
labels:
  - area:test
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.7.1
parent_task_id: TASK-55.7
priority: high
ordinal: 178390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From R6.1 (2026-11-24): the pty driver (crates/alz-test-support pty-run, test/pty.ts) is in, and 18 terminal cases moved to test/acceptance/40-account-allow-terminal.test.ts. About 100 TypeScript-only cases in test/reference/32 that need a terminal are not yet ported: the state refusals, lock cases, the hint on build and check at a terminal, escaping, the link and state-folder cases, the one-chunk reader, and the eight-distinct-roots concurrency case (file 40 has a simplified twelve-runs-on-one-root form). Port them as acceptance cases (failing against the Rust binary until R6.11, R6.13 and R6.14, listed in test/ci/r6-pending.json with reasons, the case floor raised). Also: tests.md requires a test that runs anything but the binary under test to go through test/exec.ts with an allow-listed environment — route test/pty.ts's spawn of pty-run through test/exec.ts (or extend exec.ts with a pty mode) and give it the allow-listed environment.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every terminal case of test/reference/32 has an acceptance twin in file 40 (or another), listed in r6-pending.json until it passes, and the case floor is raised
- [ ] #2 test/pty.ts starts pty-run through test/exec.ts with an allow-listed environment, never ...process.env
- [ ] #3 mise run test and the r6-pending check exit 0
<!-- AC:END -->
