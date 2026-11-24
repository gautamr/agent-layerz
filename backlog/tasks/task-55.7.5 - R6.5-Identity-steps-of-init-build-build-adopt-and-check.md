---
id: TASK-55.7.5
title: 'R6.5: Identity steps of init, build, build --adopt and check'
status: To Do
assignee: []
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T10:23:00Z'
labels:
  - area:rust
  - area:identity
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.7.2
  - TASK-55.7.4
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 164390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§9.4, §10, §10.3).
- init (repo kind): after the scaffold, resolve the stack as build does (fetching); stack and declaration problems are warnings, nothing more is written, exit 0; a refused account declaration also skips the identity step; with a declaration, set the identity (R6.4) and report.
- build warns about a mismatch; build --adopt also sets the identity; check reports it as an error after its own comparison unless --skip-identity.
- Outside a git repository a note; git missing or broken: check error, build and init warning. No declaration: no git command and no line.
- golden_trees.rs stops leaving out the [identity] lines.
- Deletes the entries of the cases it makes pass from test/ci/r6-pending.json in the same change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every case of tests 26 and 28 and test 29's init case passes against ALZ_BIN, and their entries are gone from test/ci/r6-pending.json (vitest run; check-r6-pending exits 0)
- [ ] #2 With no commit_identity declared, init, build and check run no git command for the identity and print no identity line (acceptance case with gitProxy, or an alz integration test)
- [ ] #3 golden_trees.rs compares the [identity] lines by level and code for every fixture (cargo test)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R6.4 (2026-11-24): turn RepositoryGit's SetupResult (Blocked, Planned { plan, written, stopped }), Stopped (Refused, Tampered { field, changed }) and the IdentityCheck pieces into the report lines of §10.3; Tampered is error [identity] and stops init and build --adopt with exit 1.
<!-- SECTION:NOTES:END -->
