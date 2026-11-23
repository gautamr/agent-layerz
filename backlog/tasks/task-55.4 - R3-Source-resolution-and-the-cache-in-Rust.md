---
id: TASK-55.4
title: 'R3: Source resolution and the cache in Rust'
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-23T07:30:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.3
parent_task_id: TASK-55
priority: medium
ordinal: 36390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R3: path:, git: and github: sources with the guarded clone, the cache layout and integrity checks, safe reads with openat. Builds TASK-52 and TASK-53 properly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Rust tests cover path:, git: and github: resolution with the guarded clone and its explicit child environment, the cache layout and integrity checks and the openat reads, and each case of tests 16, 24, 28 and 30 about them has a Rust test with the same verdict or a reachable CLI case (list in the notes)
- [ ] #2 The cases of tests 16, 24, 28 and 30 that alz lint can reach (stack mode now resolves the layers, and R2's error [not-ported] is gone) pass against the Rust binary; the rest of those files pass by R5 (TASK-55.6)
- [ ] #3 The spec Limits that openat closes are narrowed
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
decision-30: R2 closes TASK-59 to TASK-62 (YAML and ref bugs, fixed in Rust only).

Architect (2026-11-23): criteria restated per the owner's answer 6 in TASK-55.3's notes: R3 is judged by Rust tests plus the acceptance cases its CLI can reach, and the full suite files must pass by R5. The old single criterion named tests 16, 24, 28 and 30, which mostly run alz build (R5).

From R2.7 (2026-11-23): the Rust lock reader validates rev as a GitRef and requires visibility; confirm against what the resolver writes as rev (a ref or a 12-character sha prefix).

Lead (2026-11-23): TASK-68 to TASK-71 are fixed inside R3, so R4 (TASK-55.5) depends on them instead of R3, as TASK-59 to TASK-62 were for R2.
<!-- SECTION:NOTES:END -->
