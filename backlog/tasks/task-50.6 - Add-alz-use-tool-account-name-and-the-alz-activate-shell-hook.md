---
id: TASK-50.6
title: Add alz use <tool> --account <name> and the alz activate shell hook
status: To Do
assignee: []
created_date: '2026-11-21T18:04:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.3
parent_task_id: TASK-50
priority: high
ordinal: 22390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The second way to switch: alz use overrides the account for the current terminal, with the same shape as alz run. A Node CLI cannot change its parent shell's environment, so like mise it needs the hook: eval "$(alz activate zsh)" defines an alz shell function and switches accounts automatically on cd.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz activate zsh and alz activate bash print a hook that, after cd into a configured repository, sets the account variables, and unsets them on leaving (shell test for both)
- [ ] #2 With the hook loaded, alz use <tool> --account <name> sets that tool's account for the current terminal until alz use <tool> --reset, and a cd does not undo it
- [ ] #3 Without the hook, alz use changes nothing, prints the eval line to add, and exits non-zero
- [ ] #4 When mise or direnv has already set a different value, the hook leaves it and prints one warning naming both values' sources
<!-- AC:END -->
