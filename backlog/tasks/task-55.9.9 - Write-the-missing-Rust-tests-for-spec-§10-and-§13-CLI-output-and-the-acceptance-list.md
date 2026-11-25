---
id: TASK-55.9.9
title: >-
  Write the missing Rust tests for spec sections 10 and 13: CLI, output and the
  acceptance list
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
updated_date: '2026-11-25T14:15:00Z'
labels:
  - area:rust
  - area:test
milestone: m-7
dependencies:
  - TASK-55.9.5
parent_task_id: TASK-55.9
priority: medium
ordinal: 191390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-39: test/ may be deleted only when every requirement of docs/LAYERING-SPEC.md has a Rust test. This writes, from the spec alone, the tests that docs/spec-coverage.md marks as gap in §10 (CLI, output and exit codes, generated YAML, command output) and the §13 items no other section's rows cover, plus any remaining row in §1 to §3, §12, §14 or §15: assert_cmd against the binary for command behaviour (isolated HOME, scratch folders, fixtures under fixtures/), library tests and proptest laws for rules. Each new test names in its doc comment the spec section it proves, and the PR touches only these sections' rows. Where a new test shows alz differing from the spec, alz is fixed here if the fix is small; otherwise a bug task is created and this task depends on it. Untrusted input: the tests feed untrusted layer.yaml, layer.lock and layer content, and never touch real account folders or the developer's home. Out of scope: other sections, new spec rules (a spec hole becomes its own task).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/spec-coverage.md has no gap row in §10, §13 or any other section, so the whole matrix has none (guard test and grep)
- [ ] #2 Every new test names in its doc comment the spec section it proves, and the row it fills names it (guard test)
- [ ] #3 mise run rust:check passes, and the rust workflow passes on ubuntu-latest and macos-latest with the Linux-only cases
- [ ] #4 Every difference from the spec that a new test found is fixed here or tracked by a bug task this task depends on (notes)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
§10.3 alz init no longer states the scaffolded files' contents; state them in the spec and test them (from 55.9.4).

From docs/spec-coverage.md (TASK-55.9.5): 12 gap rows: §10.1 Merge diagnostics, §10.1 Build and command diagnostics, §10.3 alz build, §10.3 alz check, and §13.12, §13.13, §13.14, §13.18, §13.19, §13.20, §13.24, §13.28. §0 to §3, §12, §14, §15 have none. §13.21 to §13.23 are planned (TASK-50.3, TASK-50.6, TASK-50.5), not gaps. Fixtures unnamed by tests: 14-non-destructive, 38-emitter-drift, 31-account-which, 32-account-allow, 28-cache-integrity.

Correction (55.9.5): the gap rows in your sections are now 11, not 12: §13.13 (two targets) is planned under TASK-18 (the Codex adapter is not built yet).

Re-read the tests named by the covered rows of your sections and downgrade any row whose test does not assert the requirement (from 55.9.5).
<!-- SECTION:NOTES:END -->
