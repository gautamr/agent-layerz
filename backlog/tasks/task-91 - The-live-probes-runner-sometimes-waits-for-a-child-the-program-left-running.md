---
id: TASK-91
title: The live probe's runner sometimes waits for a child the program left running
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-24T08:23:00Z'
updated_date: '2026-11-24T08:23:00Z'
labels:
  - bug
  - area:rust
  - area:security
milestone: m-7
dependencies: []
priority: high
ordinal: 177390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Seen twice in the stop hook on a loaded machine (2026-11-24): crates/alz-io/tests/tool_runner.rs a_program_that_ends_by_itself_leaves_nothing_of_its_group_running failed with the readers finishing more than 4 s after a program whose background child sleeps 6 s (the margin was widened from 2 s/3 s to 4 s/6 s in R5; it still failed). It passes alone. Find whether this is load or a real ordering problem in crates/alz-io/src/tool.rs (the group must be ended as soon as the program ends, before waiting for the pipe readers), fix the cause, and make the test robust without hiding a real failure.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The cause is named with evidence (a reproduction under load, or the ordering in tool.rs)
- [ ] #2 The runner ends the program's group before it waits for the readers, and the test passes 200 times under artificial CPU load
<!-- AC:END -->
