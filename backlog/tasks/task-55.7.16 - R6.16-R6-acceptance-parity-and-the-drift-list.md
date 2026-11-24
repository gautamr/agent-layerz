---
id: TASK-55.7.16
title: 'R6.16: R6 acceptance, parity and the drift list'
status: To Do
assignee: []
created_date: '2026-11-24T08:16:00Z'
updated_date: '2026-11-24T10:51:00Z'
labels:
  - area:rust
  - area:test
  - needs:owner-approval
milestone: m-7
dependencies:
  - TASK-55.7.5
  - TASK-55.7.13
  - TASK-55.7.14
  - TASK-55.7.15
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 175390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (doc-5 R6 gate).
- Run the whole acceptance suite against the release binary with the JSON reporter; test/ci/r6-pending.json holds no entry. Per owner question 6, remove the file and the R6 allowance of test/ci/check-r6-pending.ts so the acceptance-rust job fails on any failing case (a CI change, by the release-engineer); raise case-floor.json.
- Record the counts per file in TASK-55.7's notes; complete §13's drift list; golden trees with identity lines.
- Carried criteria: TASK-72 #1, TASK-52 #4 and TASK-69 #5 (account forms), TASK-50.4 #5, TASK-73's escape case.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 test/ci/r6-pending.json lists no case and every acceptance case passes against ALZ_BIN (JSON report run; counts per file in TASK-55.7's notes)
- [ ] #2 Spec §13's drift list names every reference-drift case R6 added, each with a Rust case that passes (grep and vitest run)
- [ ] #3 mise run rust:ci, mise run lint, mise run test and mise run alz:check exit 0 on the R6 branch
- [ ] #4 TASK-72 #1 and the account forms of TASK-52 #4 and TASK-69 #5 are checked off with the evidence in their notes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R6.5 (2026-11-24) for the spec: (1) drop '.git/config resolves outside the repository' from §9.4 and §10.3 and the Blocker enum — a link at .git/config is always 'not a regular file' now; (2) state that after a Refused stop (the place stopped being writable between two keys) the refusal line is printed, then the findings; (3) init skips the identity step whenever the build has any error (render-time ones included).
<!-- SECTION:NOTES:END -->
