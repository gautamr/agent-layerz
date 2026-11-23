---
id: TASK-61
title: A layer ref that is a list or a number is converted with String()
status: In Progress
assignee: []
created_date: '2026-11-22T16:27:00Z'
updated_date: '2026-11-23T06:52:00Z'
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
- [x] #1 A ref that is not a string is a lint and build error naming the entry
- [x] #2 An acceptance test covers ref as a list and ref: 1.10
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23, decision-30): src/ is never changed; this bug is fixed only in the Rust alz, in R2 (TASK-55.3, YAML with limits and the manifest model). Its acceptance cases assert the spec and are marked reference drift (test/drift.ts) where the TypeScript reference fails them. R3 depends on this task, so it closes within R2.

Architect (2026-11-23, R2 plan in TASK-55.3): fixed in the Rust model by R2.6 (TASK-55.3.6); closed by R2.11 (TASK-55.3.11), which adds the acceptance cases (referenceDrift where the reference fails) and checks these criteria with its run. This task now depends on TASK-55.3.11.

R2.11 evidence (2026-11-23): AC1 - lint half: test/acceptance/35-manifest-schema.test.ts 'alz lint refuses it as a manifest error naming the entry' for ref: [refmarker7] and ref: 1.10 passes against the Rust alz (error [manifest], stack[1].ref named, neither value printed); the reference answers with error [source] 'cannot resolve ref 1.1' (referenceDrift TASK-61). Build half (owner answer 5): the shared parser's unit tests (R2.6) check it; the 'alz build' twins are written now as referenceDrift TASK-61 and fail against the R2 Rust binary (no build command, exit 2) until R5 (TASK-55.6), where they must turn green. AC2 - the same file covers ref as a list and ref: 1.10, plus 'alz lint gets past a ref that is text' (ref: "1.10").
<!-- SECTION:NOTES:END -->
