---
id: TASK-28.13
title: Add Given/When/Then scenarios to the Agent Lens use cases
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-19T18:08:00Z'
labels:
  - enabler
  - area:spec
milestone: m-5
dependencies:
  - TASK-28.12
documentation:
  - backlog/docs/doc-2 - Agent-Lens-plan.md
parent_task_id: TASK-28
priority: high
ordinal: 87
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Agent Lens design (task-28.4) should be written against scenario IDs from the start instead of retrofitting them, and the build tasks' criteria and tests should cite them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every use case UC-L1 to UC-L7 in doc-2 has at least one Given/When/Then scenario with an ID UC-L<n>.S<m>
- [ ] #2 Scenarios cover the edge cases the design must handle: narrow terminals, a repository without alz, a session without subagents, a full context
- [ ] #3 The Agent Lens build tasks cite the scenarios they implement in their acceptance criteria
- [ ] #4 mise run uc:check passes
<!-- AC:END -->
