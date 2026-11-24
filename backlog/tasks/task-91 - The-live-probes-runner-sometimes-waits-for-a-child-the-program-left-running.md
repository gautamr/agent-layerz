---
id: TASK-91
title: The live probe's runner sometimes waits for a child the program left running
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T08:23:00Z'
updated_date: '2026-11-24T10:10:00Z'
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
- [x] #1 The cause is named with evidence (a reproduction under load, or the ordering in tool.rs)
- [x] #2 The runner ends the program's group before it waits for the readers, and the test passes 200 times under artificial CPU load
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Done in PR #15 (merged 2026-11-24): the runner already ended the group before reading the pipes; the test counted the first exec of a fresh script on loaded macOS (4-5 s). It now measures from the program's start. Under load 39/100 failures before, 0/200 after.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The tool runner test measures from the program's start; the runner was correct. Merged as PR #15.
<!-- SECTION:FINAL_SUMMARY:END -->
