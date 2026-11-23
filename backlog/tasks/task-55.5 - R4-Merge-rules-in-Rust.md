---
id: TASK-55.5
title: 'R4: Merge rules in Rust'
status: To Do
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-23T11:58:00Z'
labels:
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.4
parent_task_id: TASK-55
priority: medium
ordinal: 37390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R4: every merge rule, protected variables, who may declare.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every merge rule of spec §7 has Rust tests, with property tests for its law (doc-6 §3.5), and each merge case of tests 02 to 11, 17 to 19, 26 and 29 has a Rust test with the same verdict (list in the notes)
- [ ] #2 The cases of tests 02 to 11, 17 to 19, 26 and 29 that alz lint can reach pass against the Rust binary; the rest of those files pass by R5 (TASK-55.6)
- [ ] #3 Settings and MCP files accept only the known target names at the top level; any other top-level key is an error naming the file, and the merged settings are ordered maps of validated keys
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
decision-30: R3 closes TASK-52, TASK-53 and TASK-63 (fixed in Rust only).

Architect (2026-11-23): criteria restated per the owner's answer 6 in TASK-55.3's notes: R4 is judged by Rust tests plus the acceptance cases its CLI can reach, and the full suite files must pass by R5. The known-target criterion is unchanged in wording and meaning; it is now #3.

Lead (2026-11-23): TASK-52, 53, 63, 69 and 71 keep one criterion each that only R5's alz build/check can meet (and TASK-69's Linux acceptance case needs a Linux run of the TypeScript suite, TASK-4), so R5 (TASK-55.6) depends on them instead of R4. TASK-68 and TASK-70 close with R3.
<!-- SECTION:NOTES:END -->
