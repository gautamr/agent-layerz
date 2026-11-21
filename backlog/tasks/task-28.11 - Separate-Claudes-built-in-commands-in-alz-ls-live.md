---
id: TASK-28.11
title: Separate Claude's built-in commands in alz ls --live
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-19T18:00:00Z'
updated_date: '2026-11-19T18:06:00Z'
labels:
  - area:cli
  - enabler
milestone: m-5
dependencies: []
documentation:
  - backlog/docs/doc-2 - Agent-Lens-plan.md
parent_task_id: TASK-28
priority: low
ordinal: 80
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
alz ls --live lists Claude's built-in skills and commands as 'not from layers', which buries the items that matter: user-level folders and plugins.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Verifies what Claude's init event reports for each source and records it in docs/target-facts.md
- [ ] #2 ls --live groups items into built from layers, plugins, user-level folders and Claude built-ins
- [ ] #3 An acceptance test with a fake claude binary covers each group
<!-- AC:END -->
