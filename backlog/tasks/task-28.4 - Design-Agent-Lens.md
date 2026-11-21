---
id: TASK-28.4
title: Design Agent Lens
status: To Do
assignee:
  - '@tui-designer'
created_date: '2026-11-19T18:00:00Z'
updated_date: '2026-11-19T18:09:00Z'
labels:
  - area:lens
  - area:spec
  - area:security
  - uc:L1
  - uc:L2
  - uc:L3
  - uc:L4
  - uc:L5
  - uc:L6
  - uc:L7
milestone: m-5
dependencies:
  - TASK-28.3
  - TASK-28.13
documentation:
  - backlog/docs/doc-2 - Agent-Lens-plan.md
parent_task_id: TASK-28
priority: high
ordinal: 100
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
One design reviewed before any code: what the pane shows for each use case, how it behaves at each width, and what the mod may read and must never do.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/agent-lens.md maps every element to a use case UC-L1 to UC-L7
- [ ] #2 Defines the layout at 80, 110 and 144+ columns, the status line entry and the /lens command
- [ ] #3 Defines the state model and the performance budget (redraw rate, memory)
- [ ] #4 Defines layer attribution and privacy: no network, reads only inside the repository, nothing leaves the session
- [ ] #5 Reviewed by the architect and the security-reviewer, then approved by the owner before any implementation task starts
<!-- AC:END -->
