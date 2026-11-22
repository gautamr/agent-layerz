---
id: TASK-61
title: A layer ref that is a list or a number is converted with String()
status: To Do
assignee: []
created_date: '2026-11-22T16:27:00Z'
labels:
  - discovered
  - area:resolve
milestone: m-7
dependencies: []
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
