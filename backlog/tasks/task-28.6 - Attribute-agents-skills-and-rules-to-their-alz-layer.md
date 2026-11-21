---
id: TASK-28.6
title: Attribute agents, skills and rules to their alz layer
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-19T18:00:00Z'
updated_date: '2026-11-19T18:06:00Z'
labels:
  - area:lens
  - area:security
  - uc:L1
  - uc:L2
  - uc:L3
milestone: m-5
dependencies:
  - TASK-28.4
documentation:
  - backlog/docs/doc-2 - Agent-Lens-plan.md
parent_task_id: TASK-28
priority: medium
ordinal: 120
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What makes Agent Lens unique: alz-generated files name their source layer in a header, so the lens can show personal, client or repo for every item. Runs in parallel with the collector in its own worktree.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reads the generated Source: headers and layer.lock to map each agent, skill and rule name to its layer, and plugin items to their plugin
- [ ] #2 Reads only inside the repository; a repo without alz shows no attribution and no errors
- [ ] #3 Tests cover personal, client and repo layers, plugin items and a repo without layer.lock
<!-- AC:END -->
