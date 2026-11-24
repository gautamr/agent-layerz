---
id: TASK-55.7
title: 'R6: Commit identity and accounts in Rust'
status: To Do
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-24T06:09:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.6
  - TASK-72
parent_task_id: TASK-55
priority: medium
ordinal: 39390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R6: commit identity, alz account which and alz account allow, with the same security rules.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tests 26, 30, 31 and 32 pass against the Rust binary
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R5.15 (implementer, 2026-11-24): the R6 remainder. test/ci/r6-pending.json lists 256 acceptance cases that fail against the Rust alz today, grouped by file: 24-untrusted-repo 3 (alz account which; the hub-folder escape case also needs a rewrite because the Rust alz refuses a layer.yaml with a control character after escapes, spec 5.2, so the folder name must come from allowed.yaml); 26-commit-identity 24 (commit identity setup by init and build --adopt, identity warnings, the identity part of check, schema rows; spec 7.12, 9.4); 28-cache-integrity 2 (init sets the git identity from a cached layer, spec 9.4); 29-account-layer 6 (alz account which and the account warnings of init, spec 7.13); 30-account-cache-read 63 (account resolution through alz account which and the user config); 31-account-which 103 (alz account which and alz account allow, spec 7.13, 9.1); 32-account-allow 55 (alz account allow and the allow hint of build, spec 7.13). R6 deletes each entry from test/ci/r6-pending.json as its case passes; test/ci/check-r6-pending.ts (the acceptance-rust CI job) fails on any other failing case and on a listed case that now passes. Also for R6: the escaped C1/U+2028/U+2029 bytes of spec 10.2 are reachable through an allowed.yaml root path; add an acceptance case with referenceDrift TASK-73 when alz account allow exists.
<!-- SECTION:NOTES:END -->
