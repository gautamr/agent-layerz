---
id: TASK-57.6
title: Machine-local default stack (Rust)
status: To Do
assignee: []
created_date: '2026-11-22T14:00:00Z'
updated_date: '2026-11-22T14:00:00Z'
labels:
  - area:cli
  - area:security
milestone: m-3
dependencies:
  - TASK-57.1
  - TASK-55.9
parent_task_id: TASK-57
priority: medium
ordinal: 56390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A machine-local config file next to accounts.yaml with default_stack, used by alz init when no --stack is given; read with the same safety rules as accounts.yaml (outside every repository, owned by the user, not group or world writable, never written by alz unless a command is asked to). alz doctor shows the effective default and where it comes from.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz init with no --stack uses default_stack and says so; without the file it offers the starter
- [ ] #2 The file is ignored with a warning in every unsafe state the spec lists, with tests
<!-- AC:END -->
