---
id: TASK-55.9
title: 'R8: Cut over to the Rust binary'
status: To Do
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-24T08:19:00Z'
labels:
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.7
  - TASK-55.8
parent_task_id: TASK-55
priority: medium
ordinal: 41390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R8: a parity run on every fixture, the TypeScript implementation removed, docs and .layer updated.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The suite passes only against the Rust binary, src/ and the Node build are gone, and alz:check passes with the Rust binary
- [ ] #2 The TypeScript acceptance suite is replaced by Rust tests (unit, integration, assert_cmd, proptest) that cover the same spec sections
- [ ] #3 The pull request that makes the Rust binary the default describes every security fix it ships, in its body and in CHANGELOG.md
<!-- AC:END -->
