---
id: TASK-55.1.1
title: 'Spike: choose the YAML library for the Rust port'
status: To Do
assignee: []
created_date: '2026-11-22T15:52:00Z'
labels:
  - area:rust
  - area:spec
milestone: m-7
dependencies: []
parent_task_id: TASK-55.1
priority: high
ordinal: 59390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
alz parses layer.yaml, layer.lock, frontmatter and settings from untrusted repositories. serde_yaml is no longer maintained. Compare the maintained candidates (check current maintenance and soundness yourself; do not rely on memory) against: active maintenance and licence; serde support; limits on aliases, nesting depth and input size; error messages that do not quote the input; YAML 1.1 versus 1.2 behaviour compared with what the TypeScript reference accepts today (booleans like yes/no, numbers, merge keys); and the byte-for-byte output alz writes (YAML it generates). Write a small throwaway comparison, not production code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A written comparison of at least three maintained candidates against every criterion, with evidence from running them
- [ ] #2 A recommendation the owner can accept or reject, noting what changes for users if YAML parsing behaviour differs from the TypeScript reference
<!-- AC:END -->
