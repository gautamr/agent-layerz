---
id: TASK-23
title: Decide on Windows support
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-19T17:14:00Z'
labels:
  - area:spec
milestone: m-3
dependencies: []
priority: low
ordinal: 350
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Paths, symlinks and hook commands differ on Windows, and nothing is tested there today.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either CI runs the suite on Windows and it passes, or the README states Windows is unsupported and why
- [ ] #2 The decision is recorded with backlog decision create
<!-- AC:END -->
