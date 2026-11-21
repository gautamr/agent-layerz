---
id: TASK-28.7
title: Build the pane, status line entry and /lens command
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-19T18:00:00Z'
updated_date: '2026-11-19T18:06:00Z'
labels:
  - area:lens
  - uc:L1
  - uc:L2
  - uc:L3
  - uc:L4
  - uc:L5
  - uc:L6
milestone: m-5
dependencies:
  - TASK-28.5
  - TASK-28.6
documentation:
  - backlog/docs/doc-2 - Agent-Lens-plan.md
parent_task_id: TASK-28
priority: high
ordinal: 130
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The visible part, built to the approved design with the tui-designer.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The pane shows UC-L1 to UC-L5 as designed, with layer attribution
- [ ] #2 A compact view and the status line entry work below 144 columns (UC-L6)
- [ ] #3 /lens opens and closes the pane
- [ ] #4 The tui-designer has reviewed it at 80, 110 and 144+ columns
<!-- AC:END -->
