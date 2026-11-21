---
id: TASK-14
title: Run alz check on this repository in CI
status: To Do
assignee:
  - '@release-engineer'
created_date: '2026-11-19T17:13:00Z'
labels:
  - area:ci
milestone: m-1
dependencies:
  - TASK-4
  - TASK-13
priority: medium
ordinal: 260
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The generated agent configuration of this repository must not drift from .layer/.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CI fails when the generated agent configuration or layer.lock is out of date
- [ ] #2 The check uses the CLI built from the same commit
<!-- AC:END -->
