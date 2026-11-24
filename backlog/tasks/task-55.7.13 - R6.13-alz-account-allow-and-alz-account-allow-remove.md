---
id: TASK-55.7.13
title: 'R6.13: alz account allow and alz account allow --remove'
status: To Do
assignee: []
created_date: '2026-11-24T08:16:00Z'
labels:
  - area:rust
  - area:accounts
  - area:cli
  - area:security
milestone: m-7
dependencies:
  - TASK-55.7.10
  - TASK-55.7.11
  - TASK-55.7.12
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 172390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§10 `alz account allow`, §7.13).
- alz-core account/allow.rs: from the declaration read with R6.8's reader (as resolution reads it), the allowed state and the folder listing: AlreadyAllowed | NothingDeclared (names --remove when this root has an entry) | CacheMiss (run alz build) | Conflict | Reserved | Ask(Offer). The Offer shows the repository, the account per target and whether its folder exists, the declaring layer and its position, source and ref as written, revision, the source to record and the current entry when it differs; repository text escaped (class 2).
- One prompt per distinct name in target order; an answer without its surrounding spaces and tabs must equal the name; after the last answer the declaration is read again and a changed map, source, revision or layer is refused; then StateDir writes.
- --remove reads no stack: shows the entry, asks for its names, removes it; with no file or entry, `nothing to remove`, exit 0, nothing created.
- clap accepts only --remove; --yes, -y or anything else is a usage error (exit 2). The gate (R6.11) runs before any file system call.
- Deletes the entries of the cases it makes pass from test/ci/r6-pending.json in the same change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every case of test 32 that allow or --remove reaches, and every pty case R6.1 brought back, passes against ALZ_BIN, and their entries are gone from test/ci/r6-pending.json (vitest run; check-r6-pending exits 0)
- [ ] #2 Under the pty driver a repository whose real root holds U+0085, U+2028 and U+2029 is allowed, and alz account which then applies the layer's account (acceptance case, referenceDrift TASK-73)
- [ ] #3 A declaration changed between the offer and the last answer is refused and allowed.yaml stays byte-identical (pty acceptance case)
<!-- AC:END -->
