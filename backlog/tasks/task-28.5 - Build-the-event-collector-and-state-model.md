---
id: TASK-28.5
title: Build the event collector and state model
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
milestone: m-5
dependencies:
  - TASK-28.4
documentation:
  - backlog/docs/doc-2 - Agent-Lens-plan.md
parent_task_id: TASK-28
priority: high
ordinal: 110
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The data layer every view reads from, built first so the UI and attribution work can proceed against a fixed contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Collects the events verified in the design into the state model declared in the mod's type contract
- [ ] #2 Tests written first cover each use case's data (UC-L1 to UC-L5) and pass with claude plugin test
- [ ] #3 Stays within the design's performance budget
<!-- AC:END -->
