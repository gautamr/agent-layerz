---
id: TASK-61
title: A layer ref that is a list or a number is converted with String()
status: To Do
assignee: []
created_date: '2026-11-22T16:27:00Z'
updated_date: '2026-11-23T05:19:00Z'
labels:
  - discovered
  - area:resolve
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.3.11
priority: low
type: bug
ordinal: 67390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A stack entry's ref is passed through String(), so ref: [x] becomes "x" and ref: 1.10 becomes "1.1" instead of failing or staying "1.10". Found by the YAML spike (TASK-55.1.1) against the frozen TypeScript reference. Decide per decision-21 whether TypeScript takes a fix (it must stay a correct oracle) or the spec records the intended Rust behaviour before R2 ports parsing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A ref that is not a string is a lint and build error naming the entry
- [ ] #2 An acceptance test covers ref as a list and ref: 1.10
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23, decision-30): src/ is never changed; this bug is fixed only in the Rust alz, in R2 (TASK-55.3, YAML with limits and the manifest model). Its acceptance cases assert the spec and are marked reference drift (test/drift.ts) where the TypeScript reference fails them. R3 depends on this task, so it closes within R2.

Architect (2026-11-23, R2 plan in TASK-55.3): fixed in the Rust model by R2.6 (TASK-55.3.6); closed by R2.11 (TASK-55.3.11), which adds the acceptance cases (referenceDrift where the reference fails) and checks these criteria with its run. This task now depends on TASK-55.3.11.
<!-- SECTION:NOTES:END -->
