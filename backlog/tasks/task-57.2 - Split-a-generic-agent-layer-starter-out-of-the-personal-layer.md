---
id: TASK-57.2
title: Split a generic agent-layer-starter out of the personal layer
status: To Do
assignee: []
created_date: '2026-11-22T14:00:00Z'
updated_date: '2026-11-22T14:00:00Z'
labels:
  - area:skills
  - area:process
milestone: m-1
dependencies:
  - TASK-57.1
parent_task_id: TASK-57
priority: high
ordinal: 52390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Work in the author's layer repositories (outside this repo): create agent-layer-starter with the generic parts of agent-layer-personal (role agents, delivery protocol and board skills, safe hooks, default permissions and models), rename anything personal (no gr- prefix), remove commit_identity and forbid_patterns. The personal layer then stacks on the starter and keeps only what is the author's own. Version both, with a CHANGELOG each. Owner reviews what counts as generic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 agent-layer-starter has no commit_identity, no forbid_patterns, no gr- names and no client, employer or private names (alz lint --layer passes with the leak guard)
- [ ] #2 The personal layer stacks on the starter and this repository still builds to byte-identical output (or the differences are reviewed and intended)
- [ ] #3 Both layers have a version, a tag and a CHANGELOG entry
<!-- AC:END -->
