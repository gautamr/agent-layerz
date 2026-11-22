---
id: TASK-56.6
title: Report the effective review policy in alz doctor and the board
status: To Do
assignee: []
created_date: '2026-11-22T13:49:00Z'
updated_date: '2026-11-22T13:50:00Z'
labels:
  - area:process
  - area:cli
milestone: m-3
dependencies:
  - TASK-56.3
parent_task_id: TASK-56
priority: low
ordinal: 48390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
alz doctor prints the effective review policy and which layer set each part. Optionally, for repositories that use Backlog.md, alz suggests (never writes) a matching definition-of-done item so the board shows the review step; alz does not edit backlog configuration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz doctor shows unit, reviewers with conditions and lock state, and gate, each with its source layer
- [ ] #2 When Backlog.md is present, doctor prints the suggested definition-of-done line and the backlog command to add it, without running it
<!-- AC:END -->
