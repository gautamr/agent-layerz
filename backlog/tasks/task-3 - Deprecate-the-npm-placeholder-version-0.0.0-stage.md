---
id: TASK-3
title: Deprecate the npm placeholder version 0.0.0-stage
status: To Do
assignee:
  - '@gautamr'
created_date: '2026-11-19T17:12:00Z'
labels:
  - owner:human
  - area:release
milestone: m-0
dependencies: []
priority: medium
ordinal: 30
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
npm staged publishing created 0.0.0-stage when the name was first published; it holds no code and should not be installed. Agents are denied npm deprecate, so the owner runs it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 npm view agent-layerz@0.0.0-stage deprecated prints a message that points to 0.1.0 or later
<!-- AC:END -->
