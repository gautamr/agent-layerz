---
id: TASK-13
title: 'Publish the personal layer and stack it through github:'
status: To Do
assignee:
  - '@gautamr'
created_date: '2026-11-19T17:13:00Z'
labels:
  - owner:human
  - area:repo
milestone: m-1
dependencies:
  - TASK-12
priority: medium
ordinal: 250
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
This repository stacks the personal layer as path:../agent-layer-personal, which contributors and CI cannot resolve.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The personal layer contains no client or private names
- [ ] #2 The personal layer is public on GitHub with a v0.1.0 or later tag
- [ ] #3 This repository's layer.yaml uses github:gautamr/agent-layer-personal@<tag>
- [ ] #4 alz check passes in a fresh clone
<!-- AC:END -->
