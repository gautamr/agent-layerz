---
id: TASK-56.3
title: Merge the review policy across layers (Rust)
status: To Do
assignee: []
created_date: '2026-11-22T13:49:00Z'
updated_date: '2026-11-22T13:50:00Z'
labels:
  - area:process
  - area:security
milestone: m-3
dependencies:
  - TASK-56.2
parent_task_id: TASK-56
priority: medium
ordinal: 45390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the merge rules from the spec: the most specific layer wins for unit and gate; reviewers combine by agent name; a locked reviewer or condition cannot be removed or narrowed by a later layer (a build error naming the layer that tried, as for locked hooks); later layers may add or widen. alz explain review lists every layer's contribution and the merged result. The default policy applies when no layer declares review.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tests cover: repo overrides unit; personal-layer locked security-reviewer cannot be removed or narrowed by a client or repo layer (build fails naming the layer); a later layer adds a reviewer; no declaration gives the default
- [ ] #2 alz explain review shows the contributing layers in stack order and the merged policy
<!-- AC:END -->
