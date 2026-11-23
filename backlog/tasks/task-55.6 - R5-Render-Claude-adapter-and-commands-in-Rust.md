---
id: TASK-55.6
title: 'R5: Render, Claude adapter and commands in Rust'
status: To Do
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-23T05:28:00Z'
labels:
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.5
  - TASK-66
parent_task_id: TASK-55
priority: medium
ordinal: 38390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R5: the Claude adapter, adoption, init, build, check, diff, ls, explain, doctor, lint.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Byte-identical generated output for every fixture, and the command cases pass against the Rust binary
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
decision-30: R5 implements name order (§6) for AGENTS.md, settings.json and .mcp.json; the reference-drift cases of test/acceptance/33-name-order.test.ts (TASK-64) must pass against the Rust binary. TASK-66 settles mapping-key order first.

Architect (2026-11-23, owner answer 6 in TASK-55.3's notes): R3 and R4 are judged by Rust tests plus the cases their CLI can reach, so the suite files they name (16, 24, 28, 30; 02 to 11, 17 to 19, 26, 29) must pass against the Rust binary in R5, together with this task's own command cases.
<!-- SECTION:NOTES:END -->
