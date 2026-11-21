---
id: TASK-10
title: 'Add a runnable example: a personal layer and a sample repository'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-19T17:13:00Z'
labels:
  - area:docs
  - area:cli
milestone: m-1
dependencies:
  - TASK-4
priority: medium
ordinal: 230
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Newcomers should be able to try alz in two minutes without writing a layer first.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 examples/ contains a minimal public personal layer and a sample repository whose stack uses it
- [ ] #2 In a fresh clone, `alz build` then `alz check` succeed in the sample repository
- [ ] #3 CI builds and checks the example
- [ ] #4 The README quick start points to it
<!-- AC:END -->
