---
id: TASK-55.4
title: 'R3: Source resolution and the cache in Rust'
status: To Do
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-22T12:54:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.3
parent_task_id: TASK-55
priority: medium
ordinal: 36390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R3: path:, git: and github: sources with the guarded clone, the cache layout and integrity checks, safe reads with openat. Builds TASK-52 and TASK-53 properly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The resolve and cache-integrity cases (tests 16, 24, 28, 30) pass against the Rust binary, and the spec Limits that openat closes are narrowed
<!-- AC:END -->
