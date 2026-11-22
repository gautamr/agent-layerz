---
id: TASK-55.2
title: 'R1: Make the acceptance suite language-neutral'
status: To Do
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-22T12:54:00Z'
labels:
  - area:rust
  - area:test
milestone: m-7
dependencies:
  - TASK-55.1
parent_task_id: TASK-55
priority: high
ordinal: 34390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R1. The suite runs against the binary named by ALZ_BIN (TypeScript bundle by default). The six source-importing files become CLI tests or move to TypeScript-only unit tests. Includes TASK-54: one temp root per run, removed at the end.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With ALZ_BIN pointing at the TypeScript bundle the whole suite passes as before, no test imports src/ through the acceptance path, and a run leaves no alz-* folder in the temp folder
<!-- AC:END -->
