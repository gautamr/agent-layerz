---
id: TASK-55.9.1
title: Move the test fixtures to a workspace-level fixtures/ folder
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-24T17:43:00Z'
updated_date: '2026-11-24T17:49:00Z'
labels:
  - area:rust
  - area:test
milestone: m-7
dependencies: []
parent_task_id: TASK-55.9
priority: medium
ordinal: 183390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Rust tests read test/fixtures in about 116 places (include_str! paths and fixture roots in alz-core, alz-io and alz), and R8 deletes test/ (decision-39). Moving every fixture with git mv to fixtures/ now lets later subtasks write new fixtures in their final place. The vitest harness finds fixtures through one root in test/helpers.ts; repointing that line is the only edit to test/, so the acceptance suite keeps running as the safety net until the deletion subtask. Out of scope: fixture contents, deleting fixtures (the deletion subtask removes those no Rust test reads), and the spec text (the spec subtask).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 git grep -n 'test/fixtures' -- crates mise.toml .github .layer scripts prints nothing (grep)
- [ ] #2 Every fixture moved unchanged: git diff main -M --name-status -- test/fixtures fixtures lists only R100 lines (git diff)
- [ ] #3 mise run rust:check passes
<!-- AC:END -->
