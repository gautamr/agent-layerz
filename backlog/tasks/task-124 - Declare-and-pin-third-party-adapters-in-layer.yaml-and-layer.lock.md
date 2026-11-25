---
id: TASK-124
title: Declare and pin third-party adapters in layer.yaml and layer.lock
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:07:00Z'
labels:
  - area:adapter
  - needs:owner-approval
milestone: m-8
dependencies:
  - TASK-123
references:
  - decision-40
priority: low
ordinal: 225390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Where components come from is an owner question (decision-40 point 8). Proposal: the repository manifest names each adapter with a source, layer.lock pins its content hash, and the file lives in the layer cache only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 layer.yaml accepts an adapters list with id and source, and the spec states it (parse tests, grep)
- [ ] #2 layer.lock pins each component's hash, and a component whose hash differs fails closed (test)
- [ ] #3 Components are read only from the layer cache, and nothing is written outside the repository and the cache (test)
<!-- AC:END -->
