---
id: TASK-20
title: 'alz update: move stack refs to the newest matching version'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-19T17:14:00Z'
labels:
  - area:cli
milestone: m-3
dependencies: []
priority: medium
ordinal: 320
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The spec lists alz update, but the CLI does not have it; updating a client layer tag in several repositories is manual today.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Updates git sources and pinned path sources to the newest tag matching their prefix (v*, layer-v*)
- [ ] #2 --dry-run shows the changes without writing
- [ ] #3 Rebuilds and updates layer.lock
- [ ] #4 Spec section 10 and README are updated
<!-- AC:END -->
