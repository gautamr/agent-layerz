---
id: TASK-30.3
title: Add the engineering-manager agent and its skills
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-20T16:59:00Z'
labels:
  - area:process
  - area:skills
milestone: m-6
dependencies:
  - TASK-30.1
  - TASK-30.2
documentation:
  - >-
    backlog/docs/doc-3 -
    Delivery-model-engineering-manager-sprints-triage-and-commands.md
parent_task_id: TASK-30
priority: high
ordinal: 44
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The core of the kit, built in this repository's layer first (decision-8).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An engineering-manager agent in .layer/agents preloads sprint-planning, bug-triage, task-routing and retro, plans and assigns, and has no code-writing role
- [ ] #2 Skills sprint-planning, bug-triage, task-routing and retro exist; generic content is separated from project policy, which lives in a profile the skills read
- [ ] #3 The routing table moves from /next-task into task-routing
- [ ] #4 mise run alz:build, alz:check, lint and tests pass
<!-- AC:END -->
