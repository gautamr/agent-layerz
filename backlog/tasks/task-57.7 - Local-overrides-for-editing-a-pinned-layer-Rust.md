---
id: TASK-57.7
title: Local overrides for editing a pinned layer (Rust)
status: To Do
assignee: []
created_date: '2026-11-22T14:00:00Z'
updated_date: '2026-11-22T14:00:00Z'
labels:
  - area:cli
  - area:resolve
  - area:security
milestone: m-3
dependencies:
  - TASK-57.1
  - TASK-55.9
parent_task_id: TASK-57
priority: medium
ordinal: 57390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Let a developer point a pinned source (for example github:her/agent-layer-personal@v1.2.0) at a local checkout through the machine-local config, so the committed manifest stays reproducible while she edits the layer: alz build uses the override and prints a warning naming it, layer.lock never records the override, alz check fails while any override is active, and alz explain shows it. Works with the cache, integrity and account rules unchanged for non-overridden layers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With an override active, build output reflects the local checkout, layer.lock is byte-identical to a build without it, and check fails naming the override
- [ ] #2 An override never applies to a source it does not name, and an unsafe override path (inside an account folder, not owned by the user) is refused
<!-- AC:END -->
