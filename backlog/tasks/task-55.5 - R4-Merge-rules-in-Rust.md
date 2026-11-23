---
id: TASK-55.5
title: 'R4: Merge rules in Rust'
status: To Do
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-23T04:25:00Z'
labels:
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.4
  - TASK-52
  - TASK-53
  - TASK-63
parent_task_id: TASK-55
priority: medium
ordinal: 37390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R4: every merge rule, protected variables, who may declare.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The merge cases (tests 02 to 11, 17 to 19, 26, 29) pass against the Rust binary
- [ ] #2 Settings and MCP files accept only the known target names at the top level; any other top-level key is an error naming the file, and the merged settings are ordered maps of validated keys
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
decision-30: R3 closes TASK-52, TASK-53 and TASK-63 (fixed in Rust only).
<!-- SECTION:NOTES:END -->
