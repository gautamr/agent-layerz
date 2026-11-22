---
id: TASK-55.1.5
title: CI job for the Rust workspace
status: To Do
assignee: []
created_date: '2026-11-22T15:53:00Z'
updated_date: '2026-11-22T15:53:00Z'
labels:
  - area:ci
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.1.4
parent_task_id: TASK-55.1
priority: medium
ordinal: 63390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A GitHub Actions workflow for the Rust workspace on Linux and macOS: fmt check, clippy -D warnings, tests and doc tests, docs, cargo deny, and the MSRV job; pinned action versions; no secrets needed. It runs next to the TypeScript CI until cutover.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The workflow passes on a pull request that only touches crates/, and fails when clippy or rustfmt would fail
- [ ] #2 Every third-party action is pinned to a full commit SHA
<!-- AC:END -->
