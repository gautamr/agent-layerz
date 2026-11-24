---
id: TASK-55.6.9
title: 'R5.9: alz diff'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-24T03:42:00Z'
labels:
  - area:rust
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.6.8
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 141390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§10).
- alz-core diff.rs: a unified diff of two byte strings as owner question 3 decides, with three lines of context and `a/<path>` and `b/<path>` headers; content that is not valid UTF-8 or holds NUL is `Binary file <path> changes`.
- command/diff.rs: the pipeline, then one diff per change and for the lock, a note per collision on standard error, `no changes` when there is none; exit 1 only on diagnostics errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Applying the diff to the old text gives the new text, for generated text pairs (proptest)
- [ ] #2 alz diff after editing a generated file shows a diff of that file, and on a built repository prints no changes (assert_cmd tests)
- [ ] #3 The diff cases of test 28 pass against ALZ_BIN (vitest run)
<!-- AC:END -->
