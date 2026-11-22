---
id: TASK-57.5
title: alz layer new --from <source> (Rust)
status: To Do
assignee: []
created_date: '2026-11-22T14:00:00Z'
updated_date: '2026-11-22T14:00:00Z'
labels:
  - area:cli
milestone: m-3
dependencies:
  - TASK-57.1
  - TASK-55.9
parent_task_id: TASK-57
priority: medium
ordinal: 55390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create a new personal layer from a source (the starter by default): copy the layer at a pinned revision into a new folder or git repository the user owns, give it her name, drop the source's identity and history, and print the next steps (edit, commit, push, use it in a stack). Generalises today's alz init --kind layer.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The new layer has the user's chosen name, no commit_identity unless she sets one, and builds as the first layer of a stack
- [ ] #2 Nothing is written outside the chosen folder, and an existing non-empty folder is refused
<!-- AC:END -->
