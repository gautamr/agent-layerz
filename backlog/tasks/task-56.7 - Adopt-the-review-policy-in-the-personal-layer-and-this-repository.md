---
id: TASK-56.7
title: Adopt the review policy in the personal layer and this repository
status: To Do
assignee: []
created_date: '2026-11-22T13:49:00Z'
updated_date: '2026-11-22T13:50:00Z'
labels:
  - area:process
  - area:docs
milestone: m-3
dependencies:
  - TASK-56.4
parent_task_id: TASK-56
priority: medium
ordinal: 49390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Owner action plus a repository change: the personal layer declares the default (unit: task, reviewer always, security-reviewer locked on the security conditions) in its next release; this repository chooses its cadence; .layer/commands/next-task.md uses the slot instead of the fixed sentence; alz:build and alz:check pass. Document the setting in the README once the command exists.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The personal layer release declares the default review policy with the security-reviewer locked
- [ ] #2 This repository's generated AGENTS.md and next-task skill show the chosen cadence, and alz:check passes
- [ ] #3 README documents the review setting with one example
<!-- AC:END -->
