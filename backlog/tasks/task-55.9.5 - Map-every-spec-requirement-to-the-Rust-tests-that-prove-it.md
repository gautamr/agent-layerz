---
id: TASK-55.9.5
title: Map every spec requirement to the Rust tests that prove it
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
labels:
  - area:spec
  - area:test
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.9.3
  - TASK-55.9.4
parent_task_id: TASK-55.9
priority: medium
ordinal: 187390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-39: the spec's requirements are proven by Rust tests alone, and test/ may go only when no requirement lacks one. This builds docs/spec-coverage.md from docs/LAYERING-SPEC.md and the Rust tests, never from the TypeScript tests: one row per anchor of the spec (every ## and ### heading, every paragraph that opens with a bold lead, and every numbered §13 item), naming the Rust tests (file and function) that prove it, or n/a with a reason for text that states no behaviour, planned with a task ID for behaviour the spec marks as planned, or gap with the behaviour that lacks a test. A Rust guard test keeps the matrix true as the spec and the tests change. The architect reviews the rows. The four gap subtasks then fill the gaps by spec area. Out of scope: writing gap tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every anchor of the spec has exactly one row, and every row is covered, n/a, planned or gap (guard test)
- [ ] #2 The guard test, run by mise run rust:check, fails when an anchor has no row, a row names an anchor the spec no longer has, or a row names a test function that does not exist (each shown failing once, in notes)
- [ ] #3 The matrix states the gap count per spec section, and every gap row names the behaviour that lacks a test (file review)
- [ ] #4 mise run rust:check passes
<!-- AC:END -->
