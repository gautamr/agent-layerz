---
id: TASK-50.5
title: Add alz account init with a dry run and a backup
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-21T17:58:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.2
parent_task_id: TASK-50
priority: low
ordinal: 21390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Creating ~/.claude-<name> and linking the shared items moves the user's files, which makes it the riskiest part. It follows the dry-run-and-backup rule from TASK-48. It can ship after the rest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz account init <name> --dry-run lists every folder it would create and every item it would link or move, and changes nothing
- [ ] #2 Without --dry-run it backs up every file or folder it replaces, prints the backup's path, and never opens a login file
- [ ] #3 Rerunning it changes nothing (idempotence test)
<!-- AC:END -->
