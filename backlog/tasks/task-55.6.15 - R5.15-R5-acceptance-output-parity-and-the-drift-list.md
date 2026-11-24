---
id: TASK-55.6.15
title: 'R5.15: R5 acceptance, output parity and the drift list'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-24T02:21:00Z'
labels:
  - area:rust
  - area:test
milestone: m-7
dependencies:
  - TASK-55.6.9
  - TASK-55.6.11
  - TASK-55.6.12
  - TASK-55.6.14
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 147390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step: close R5.
- Run the whole acceptance suite against ALZ_BIN; record pass and fail per file in TASK-55.6's notes; every file listed in TASK-55.6's plan passes; every case left for R6 is named in TASK-55.7's notes.
- The output parity list (TASK-73 AC #4): each reference drift case of spec §10.2 reachable through adoption has an acceptance case checking alz's own bytes, marked referenceDrift TASK-73; the golden trees list every drift sample.
- Acceptance cases for the writer rules of R5.1 (a passthrough .claude/Settings.json collides; a protected path in another case is left alone; a generated path under a link is refused; a lock entry alone does not make a file deletable), each referenceDrift where the reference fails it.
- test 37: every row, including the rows for Codex keys that duplicate first-class items, also through alz build, writing nothing; test 33's name-order drift cases.
- Carried criteria: TASK-52 #4, TASK-53 #4, TASK-63 #4, TASK-71 #3, TASK-66 #2, TASK-73 #4, and TASK-69 #5 where a Linux run exists; tick each with its evidence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every acceptance file the TASK-55.6 plan lists passes against ALZ_BIN=target/release/alz, and the suite against the reference has only the expected drift failures (mise run test and the ALZ_BIN run, counts in the notes)
- [ ] #2 The writer-rule and parity drift cases are in the suite and pass against the Rust binary (vitest run)
- [ ] #3 mise run rust:ci, mise run lint and mise run alz:check exit 0, and src/ is identical to main (commands in the notes)
- [ ] #4 The carried criteria listed in this task's description are checked with evidence, and the R6 remainder is in TASK-55.7's notes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R5.16 (2026-11-24): remove continue-on-error from the acceptance-rust job in .github/workflows/tests.yml and mark the R6 cases (identity, account) as expected failures there, which also closes R5.16's AC #2.
<!-- SECTION:NOTES:END -->
