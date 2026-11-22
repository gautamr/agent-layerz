---
id: TASK-54
title: Find the intermittent ~145 s stall of git subprocesses in the acceptance tests
status: To Do
assignee: []
created_date: '2026-11-22T09:02:00Z'
updated_date: '2026-11-22T12:55:00Z'
labels:
  - area:test
  - discovered
dependencies: []
priority: medium
ordinal: 31390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Seen three times on 2026-11-22 (implementer runs and a Stop-hook run): several tests that run git, in unrelated files (26, 28, 30, 31), all stall for about 146 seconds at the same moment and hit the 20 s timeout; an immediate rerun passes in about 25 s. It also happened on an unmodified HEAD checkout, so it is not caused by the account work. Tests use an isolated HOME and GIT_CONFIG_NOSYSTEM.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The cause is identified (for example a git or macOS service the subprocess waits on) and recorded in the task notes
- [ ] #2 The test helpers avoid it, or the tests fail fast with a clear message instead of waiting about 145 s
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22: paused by decision-21 (Rust port, doc-5). Done as part of TASK-55.2: one temp root per run through TMPDIR, removed by a global teardown, because the suite is the oracle. About 64,000 leftover alz-* folders were deleted from the temp folder on 2026-11-22 with the owner's approval.
<!-- SECTION:NOTES:END -->
