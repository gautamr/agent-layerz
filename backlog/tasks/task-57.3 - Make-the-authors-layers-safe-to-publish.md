---
id: TASK-57.3
title: Make the author's layers safe to publish
status: To Do
assignee: []
created_date: '2026-11-22T14:00:00Z'
updated_date: '2026-11-22T14:54:00Z'
labels:
  - area:security
  - area:repo
milestone: m-1
dependencies:
  - TASK-57.2
  - TASK-12
parent_task_id: TASK-57
priority: high
ordinal: 53390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Before any public push of agent-layerz-starter or agent-layerz-personal: finish TASK-12 so forbid patterns live outside public layers; scan each repository's full git history for client, employer and private names, emails and machine paths; if any are found, the owner decides whether to rewrite history (irreversible) or start a fresh public repository; add a licence, a README and a CI job that runs alz lint --layer. Then TASK-13 publishes and switches this repository to github: sources.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A recorded history scan of both repositories finds no client, employer or private names, emails or machine paths, or the owner's decision on how they were removed is recorded
- [ ] #2 Both repositories have a licence, a README and CI running alz lint --layer
- [ ] #3 TASK-12 is Done before either repository is pushed publicly
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22: the personal layer folder and repository are renamed agent-layerz-personal, and the planned starter is agent-layerz-starter; earlier notes keep the old names.
<!-- SECTION:NOTES:END -->
