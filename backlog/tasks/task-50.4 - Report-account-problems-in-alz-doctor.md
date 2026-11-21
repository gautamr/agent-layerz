---
id: TASK-50.4
title: Report account problems in alz doctor
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-21T17:58:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.2
parent_task_id: TASK-50
priority: medium
ordinal: 20390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Covers the folder and trust checks. TASK-22 (per-account trust) folds into this work.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz doctor reports, per repository, a named account folder that does not exist and a variable conflict with mise or direnv
- [ ] #2 It reports the shared items (agents, skills, settings, projects) that are not linked in an account folder, by name only
- [ ] #3 Every check is tested with a fake HOME and never opens a file inside an account folder
<!-- AC:END -->
