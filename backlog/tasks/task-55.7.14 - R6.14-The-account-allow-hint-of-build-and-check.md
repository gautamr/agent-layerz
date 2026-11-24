---
id: TASK-55.7.14
title: 'R6.14: The account-allow hint of build and check'
status: To Do
assignee: []
created_date: '2026-11-24T08:16:00Z'
labels:
  - area:rust
  - area:accounts
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.7.10
  - TASK-55.7.11
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 173390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.13 "alz build and alz check ... print the alz account allow command").
- build and check print `note [account-allow]` on standard error only when standard input is a terminal and no agent-session marker is set (the gate's check, without a prompt): for a layer-named account in a repository that is not allowed; for a changed entry, with the old and new values; the --remove form when the layers no longer declare and this root has an entry; the ignored-file reason, then `then run: alz account allow`; only the miss reason on a cache miss, and only when the merged configuration names an account.
- None for a root at or above HOME, in an account folder or in a configured folder. check prints it only when its comparison passed, whatever the identity result. Standard output and generated files never change.
- Deletes the entries of the cases it makes pass from test/ci/r6-pending.json in the same change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test 32's hint cases pass against ALZ_BIN and their entries are gone from test/ci/r6-pending.json (vitest run)
- [ ] #2 Under the pty driver build prints the hint; with any marker set or without a terminal it prints none, and the files written are the same (pty acceptance case)
- [ ] #3 check prints no hint when its comparison failed (pty acceptance case or assert_cmd test)
<!-- AC:END -->
