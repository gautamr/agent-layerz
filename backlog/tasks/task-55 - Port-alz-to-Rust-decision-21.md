---
id: TASK-55
title: Port alz to Rust (decision-21)
status: In Progress
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-22T16:08:00Z'
labels:
  - area:rust
milestone: m-7
dependencies: []
priority: high
ordinal: 32390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rewrite alz in Rust against LAYERING-SPEC and the acceptance suite, following doc-5. The TypeScript implementation is the frozen reference until cutover.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every phase subtask is Done
- [ ] #2 The whole acceptance suite passes against the Rust binary and its generated output is byte-identical to the TypeScript reference for every fixture
<!-- AC:END -->
