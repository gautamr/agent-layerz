---
id: TASK-37
title: Move the projects still running agentic-harness copies onto alz
status: To Do
assignee:
  - '@gautamr'
created_date: '2026-11-21T10:09:00Z'
labels:
  - area:process
  - owner:human
milestone: m-6
dependencies:
  - TASK-36
priority: medium
ordinal: 3390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
One client hub and four sandbox projects still carry harness 1.2.x files (.harness/, harness.conf, ownership.conf, PROGRESS.md). The harness source is being retired after task-36; their copies keep working but get no upgrades. Names stay out of this public board.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each project has alz init and alz build --adopt run, stacking the personal layer at v0.3.0 or later
- [ ] #2 Harness files the layers now replace (.harness/, harness.conf, PROGRESS.md) are removed or archived; ownership.conf is kept for the role path guard
- [ ] #3 alz check passes in each
<!-- AC:END -->
