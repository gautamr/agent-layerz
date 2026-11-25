---
id: TASK-55.9.7
title: >-
  Write the missing Rust tests for spec sections 7.1 to 7.11, 8 and 11: merge
  rules and adapters
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
updated_date: '2026-11-25T14:06:00Z'
labels:
  - area:rust
  - area:test
milestone: m-7
dependencies:
  - TASK-55.9.5
parent_task_id: TASK-55.9
priority: medium
ordinal: 189390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-39: test/ may be deleted only when every requirement of docs/LAYERING-SPEC.md has a Rust test. This writes, from the spec alone, the tests that docs/spec-coverage.md marks as gap in §7.1 to §7.11 (merge rules per item type), §8 (target adapters) and §11 (native plugins): assert_cmd against the binary for command behaviour (isolated HOME, scratch folders, fixtures under fixtures/), library tests and proptest laws for rules. Each new test names in its doc comment the spec section it proves, and the PR touches only these sections' rows. Where a new test shows alz differing from the spec, alz is fixed here if the fix is small; otherwise a bug task is created and this task depends on it. Untrusted input: the tests feed untrusted layer.yaml, layer.lock and layer content, and never touch real account folders or the developer's home. Out of scope: other sections, new spec rules (a spec hole becomes its own task).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/spec-coverage.md has no gap row in §7.1 to §7.11, §8 or §11 (guard test and grep)
- [ ] #2 Every new test names in its doc comment the spec section it proves, and the row it fills names it (guard test)
- [ ] #3 mise run rust:check passes, and the rust workflow passes on ubuntu-latest and macos-latest with the Linux-only cases
- [ ] #4 Every difference from the spec that a new test found is fixed here or tracked by a bug task this task depends on (notes)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From docs/spec-coverage.md (TASK-55.9.5): 2 gap rows in §7.1 to §7.11, §8 and §11: '§8 What an adapter may rely on' (no test ties adapter output keys to target-facts) and '§8.2' (the Codex adapter is not implemented in Rust, so implement it or amend the spec; §13.13 depends on it). Fixture unnamed by tests: 37-settings-targets.

Correction (55.9.5): the gap rows in your sections are now 1, not 2: only '§8 What an adapter may rely on' is a gap (it concerns every adapter, not only Codex). §8.2 is planned under TASK-18 (the Codex adapter is not built yet).

Re-read the tests named by the covered rows of your sections and downgrade any row whose test does not assert the requirement (from 55.9.5).
<!-- SECTION:NOTES:END -->
