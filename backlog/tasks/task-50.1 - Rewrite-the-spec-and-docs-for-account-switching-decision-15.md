---
id: TASK-50.1
title: Rewrite the spec and docs for account switching (decision-15)
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
labels:
  - area:accounts
  - area:docs
milestone: m-3
dependencies: []
parent_task_id: TASK-50
priority: high
ordinal: 17390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
LAYERING-SPEC §2 lists 'managing credentials or accounts' as a non-goal, and doc-1, FRAMEWORK-GAPS and the README name a separate workspace-accounts project. Settle the design in the spec before code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 LAYERING-SPEC states the goal (choose the account folder) and keeps the non-goal (credentials and logins), with the layer field, the user config file and its path, the resolution order and the fallback
- [ ] #2 doc-1, FRAMEWORK-GAPS.md and README.md no longer describe a separate account project, and each points to decision-15
- [ ] #3 The spec lists, per target adapter, the variable it sets and what is shared between account folders
<!-- AC:END -->
