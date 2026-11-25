---
id: TASK-55.9.8
title: >-
  Write the missing Rust tests for spec sections 7.12, 7.13 and 9: identity,
  accounts and safety
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
updated_date: '2026-11-25T14:36:00Z'
labels:
  - area:rust
  - area:test
  - area:security
milestone: m-7
dependencies:
  - TASK-55.9.5
parent_task_id: TASK-55.9
priority: medium
ordinal: 190390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-39: test/ may be deleted only when every requirement of docs/LAYERING-SPEC.md has a Rust test. This writes, from the spec alone, the tests that docs/spec-coverage.md marks as gap in §7.12 (commit identity), §7.13 (account selection) and §9 (safety rules and the repository git identity): assert_cmd against the binary for command behaviour (isolated HOME, scratch folders, fixtures under fixtures/), library tests and proptest laws for rules. Each new test names in its doc comment the spec section it proves, and the PR touches only these sections' rows. Where a new test shows alz differing from the spec, alz is fixed here if the fix is small; otherwise a bug task is created and this task depends on it. Untrusted input: the tests feed untrusted layer.yaml, layer.lock and layer content, and never touch real account folders or the developer's home. Out of scope: other sections, new spec rules (a spec hole becomes its own task).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/spec-coverage.md has no gap row in §7.12, §7.13 or §9 (guard test and grep)
- [ ] #2 Every new test names in its doc comment the spec section it proves, and the row it fills names it (guard test)
- [ ] #3 mise run rust:check passes, and the rust workflow passes on ubuntu-latest and macos-latest with the Linux-only cases
- [ ] #4 Every difference from the spec that a new test found is fixed here or tracked by a bug task this task depends on (notes)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From docs/spec-coverage.md (TASK-55.9.5): 1 gap row in §7.12, §7.13 and §9: '§7.13' (the whole account section is one anchor of about 740 lines with no sub-anchors, unaudited; add anchors in the spec, then name tests per bullet). §7.12 and §9 have no gap rows. Fixtures unnamed by tests: 10-leak-guard, 29-account-layer, 30-account-cache-read, 31-account-which, 32-account-allow, 34-yaml-limits.

First split §7.13 into anchors (### headings or bold leads, no change of meaning), then audit it (from 55.9.5).

Re-read the tests named by the covered rows of your sections and downgrade any row whose test does not assert the requirement (from 55.9.5).

Done on branch test/spec-7-12-9-task-55.9.8. Spec 7.13 split into 25 bold-lead anchors (Account names, Folders per target, Who may declare it, Layers cannot set the variables, User config, Allow state, Both files, Trust and alz account allow, Resolution, Paths, No fetch on resolve, YAML limits, What else makes a YAML file invalid, Environment, A named account that cannot apply fails closed, Values set outside alz, What a session passes on, alz use stays in the shell, Shell hook, Shared and per-account items, alz account init safety, Trust is per account, Logins kept outside the account folder, No generated output, Layer format.) plus the heading row; the bullets lost their dash and two spaces of indent only, the text is unchanged. New tests: crates/alz/tests/account_declaration.rs (6, fixture 29), yaml_limits.rs (4, fixture 34), account_root.rs (2, fixture 31), leak_guard_fields.rs (2, fixture 10), commit_identity_output.rs (2, fixture 26), and 2 end-to-end allow flow tests in account_allow.rs (fixture 32, pty). Matrix: 26 section 7.13 rows (20 covered, 4 planned: Shell hook, Shared and per-account items, account init safety, Trust is per account; 2 n/a), rows of 7.12, 9.2, 9.3 and 9.4 topped up with named tests; no gap row is left in 7.12, 7.13 or 9; spec_coverage guard passes. Not done: the rows are checked by test names and the assertions of the tests I re-read, not clause by clause for every older test; fixture 30-account-cache-read is still named by no test. AC1 and AC2 hold per the guard; AC3 (CI on both platforms) and AC4 not yet confirmed here.
<!-- SECTION:NOTES:END -->
