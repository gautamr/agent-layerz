---
id: TASK-31
title: Promote the delivery kit to the personal layer
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-20T16:59:00Z'
labels:
  - area:process
  - needs:owner-approval
milestone: m-3
dependencies:
  - TASK-19
  - TASK-30.5
  - TASK-30.6
documentation:
  - >-
    backlog/docs/doc-3 -
    Delivery-model-engineering-manager-sprints-triage-and-commands.md
priority: medium
ordinal: 317
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Once proven here and in a client hub, every project should inherit the kit from the personal layer, for greenfield and existing projects alike.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The owner approves the change to the personal layer first
- [ ] #2 The agent, skills and commands are promoted with alz promote and the personal layer is released with a new tag
- [ ] #3 This repository and the pilot hub stack the new version and still pass alz check
<!-- AC:END -->
