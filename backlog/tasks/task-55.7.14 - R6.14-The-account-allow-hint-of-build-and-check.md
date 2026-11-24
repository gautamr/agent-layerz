---
id: TASK-55.7.14
title: 'R6.14: The account-allow hint of build and check'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T08:16:00Z'
updated_date: '2026-11-24T13:51:00Z'
labels:
  - area:rust
  - area:accounts
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.7.10
  - TASK-55.7.11
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 173390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.13 "alz build and alz check ... print the alz account allow command").
- build and check print `note [account-allow]` on standard error only when standard input is a terminal and no agent-session marker is set (the gate's check, without a prompt): for a layer-named account in a repository that is not allowed; for a changed entry, with the old and new values; the --remove form when the layers no longer declare and this root has an entry; the ignored-file reason, then `then run: alz account allow`; only the miss reason on a cache miss, and only when the merged configuration names an account.
- None for a root at or above HOME, in an account folder or in a configured folder. check prints it only when its comparison passed, whatever the identity result. Standard output and generated files never change.
- Deletes the entries of the cases it makes pass from test/ci/r6-pending.json in the same change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Test 32's hint cases pass against ALZ_BIN and their entries are gone from test/ci/r6-pending.json (vitest run)
- [x] #2 Under the pty driver build prints the hint; with any marker set or without a terminal it prints none, and the files written are the same (pty acceptance case)
- [x] #3 check prints no hint when its comparison failed (pty acceptance case or assert_cmd test)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented (not pushed). Pure decision: alz-core account::allow_hint (HintFacts -> AllowHint -> note [account-allow] diagnostic, new Code::AccountAllow), 6 unit tests. Wiring: crates/alz/src/command/account/hint.rs reuses facts::gather (Gathered gained allow_problem) and CachedLayers::declaration; gate is PersonAtTerminal::check(env).is_ok(); build prints after its summary line, check only when its comparison passed (before the identity result). Exit code and stdout unchanged (a Note never fails). AC1: of 16 r6-pending entries in test 42, 6 hint-only cases now pass and were deleted (terminal-only print, check passes with identity difference, miss line under mise, mapping at top, ignored entry, two entries); the other 10 call 'alz account allow' (R6.13) and stay listed; no hint cases in 32 beyond those. AC2/AC3: crates/alz/tests/allow_hint.rs (7 pty-run tests: hint at terminal with files and stdout unchanged, piped none, all 6 markers none, allowed none, changed declaration shows both maps, check with failed comparison none, no-account cache miss none) plus acceptance 42. Results: mise run rust:ci exit 0 (1985 tests); mise run lint ok; mise run test 1307 passed; ALZ_BIN acceptance 1219 cases, 165 failing = 165 listed, check-r6-pending exit 0.

Merged into the R6 branch by the lead (2026-11-24) after R6.13: conflicts in account.rs, facts.rs (both sides kept: Gathered.files and allow_problem) and r6-pending.json (base minus both deletions). Test 42 passes 20 of 20 against the merged binary; the full acceptance run is 1220 cases, 0 failing, 0 listed; the R6 pending list is empty.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
build and check print one note [account-allow] line, only for a person at a terminal with no agent-session marker, when a declaration is not allowed or changed or allowed.yaml is unusable; never in an account folder, at HOME or a configured folder, or after check's own comparison failed; the exit code and stdout never change.
<!-- SECTION:FINAL_SUMMARY:END -->
