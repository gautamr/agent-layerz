---
id: TASK-30.4
title: Add the thin commands and board-ordered /next-task
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-20T16:59:00Z'
labels:
  - area:process
milestone: m-6
dependencies:
  - TASK-30.3
documentation:
  - >-
    backlog/docs/doc-3 -
    Delivery-model-engineering-manager-sprints-triage-and-commands.md
parent_task_id: TASK-30
priority: high
ordinal: 45
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Daily use needs autocomplete-friendly entry points that delegate to skills and agents (decision-7). Board order is moved here from task-28.2.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 /plan-sprint, /triage, /retro, /standup and /bug exist as commands with argument hints, each a short entry point naming the skill and agent it uses
- [ ] #2 /next-task follows board order (ordinal), skips blocked and owner:human tasks, claims the task and routes it with task-routing
- [ ] #3 Running each command once in this repository works, and the results are recorded in the task notes
<!-- AC:END -->
