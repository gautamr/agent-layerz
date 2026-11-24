---
id: TASK-55.9.6
title: >-
  Write the missing Rust tests for spec sections 4 to 6: layout, manifests, lock
  and resolution
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
updated_date: '2026-11-24T17:45:00Z'
labels:
  - area:rust
  - area:test
milestone: m-7
dependencies:
  - TASK-55.9.5
parent_task_id: TASK-55.9
priority: medium
ordinal: 188390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-39: test/ may be deleted only when every requirement of docs/LAYERING-SPEC.md has a Rust test. This writes, from the spec alone, the tests that docs/spec-coverage.md marks as gap in §4 (file layout and protected paths), §5 (manifests and layer.lock) and §6 (resolution and the layer cache): assert_cmd against the binary for command behaviour (isolated HOME, scratch folders, fixtures under fixtures/), library tests and proptest laws for rules. Each new test names in its doc comment the spec section it proves, and the PR touches only these sections' rows. Where a new test shows alz differing from the spec, alz is fixed here if the fix is small; otherwise a bug task is created and this task depends on it. Untrusted input: the tests feed untrusted layer.yaml, layer.lock and layer content, and never touch real account folders or the developer's home. Out of scope: other sections, new spec rules (a spec hole becomes its own task).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/spec-coverage.md has no gap row in §4, §5 or §6 (guard test and grep)
- [ ] #2 Every new test names in its doc comment the spec section it proves, and the row it fills names it (guard test)
- [ ] #3 mise run rust:check passes, and the rust workflow passes on ubuntu-latest and macos-latest with the Linux-only cases
- [ ] #4 Every difference from the spec that a new test found is fixed here or tracked by a bug task this task depends on (notes)
<!-- AC:END -->
