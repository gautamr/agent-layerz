---
id: TASK-55
title: Port alz to Rust (decision-21)
status: In Progress
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-24T17:43:00Z'
labels:
  - area:rust
milestone: m-7
dependencies: []
priority: high
ordinal: 32390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rewrite alz in Rust against docs/LAYERING-SPEC.md, following doc-5 and doc-6. decision-39 (owner, 2026-11-24) makes the Rust port the project: the TypeScript implementation, its tests and the npm distribution are deleted, and the spec's requirements are proven by Rust tests written from the spec (unit, integration, assert_cmd against the binary, proptest). decision-39 supersedes the reference role of decision-30 and the TypeScript parts of decision-21.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every phase subtask is Done
- [ ] #2 Every requirement of docs/LAYERING-SPEC.md is proven by Rust tests: docs/spec-coverage.md has no gap row and its guard test passes, and git ls-files lists no TypeScript, JavaScript or Node package file
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-24, decision-39: the owner settled the former criterion #2 ('byte-identical to the TypeScript reference for every fixture') by that decision; it no longer applies. Criterion #2 now states the Rust-only measure: the spec, proven by Rust tests written from it, and no TypeScript left.
<!-- SECTION:NOTES:END -->
