---
id: TASK-50.3
title: Add alz run <tool> [--account <name>]
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-21T18:04:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.2
parent_task_id: TASK-50
priority: high
ordinal: 19390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The first way to switch accounts, needing no shell hook: alz run starts one tool on the repository's resolved account, or on the account named with --account. It replaces the owner's claude-as for one-off runs, for example when an account hits its usage limit. The commands share one shape: alz run <tool> --account <name> now, alz use <tool> --account <name> in TASK-50.6.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz run <tool> [args] starts the tool with the resolved account and passes the arguments and exit code through unchanged
- [ ] #2 alz run <tool> --account <name> [args] uses that account for this run only, whatever the layers say; --account default uses the default account
- [ ] #3 An unknown account name or tool fails before starting anything, lists the account folders that exist by name only, and exits non-zero
- [ ] #4 Tests cover claude and codex, with and without --account
<!-- AC:END -->
