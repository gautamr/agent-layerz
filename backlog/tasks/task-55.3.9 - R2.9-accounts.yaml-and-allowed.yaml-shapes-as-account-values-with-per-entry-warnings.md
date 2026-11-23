---
id: TASK-55.3.9
title: >-
  R2.9: accounts.yaml and allowed.yaml shapes as account values with per-entry
  warnings
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
updated_date: '2026-11-23T06:36:00Z'
labels:
  - area:rust
  - area:accounts
milestone: m-7
dependencies:
  - TASK-55.3.5
  - TASK-55.3.8
parent_task_id: TASK-55.3
priority: high
type: task
ordinal: 98390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Spec §7.13 "User config" and "Allow state": the shapes only, parsed into account model values. The trust checks of "Both files" (owner, mode, real paths, $HOME, account folders) and resolution are file-system work in R6 (TASK-55.7). Module `crates/alz-core/src/account/{user_config,allow_state}.rs`.

Types: `UserConfig { default_name: Option<AccountName>, folders: Vec<FolderRule { key: FolderKey (Absolute | Home), choice } > }`; `AllowState(Vec<AllowEntry { root: AbsolutePath, account: NonEmptyTargetMap<AccountName>, source: AllowedSource }>)`. Parsing returns the value plus warnings as diagnostics: the whole file ignored when its top level is wrong, each bad entry ignored with one warning that holds only its position or a quoted key, as §7.13 says. Both read through R2.4's entry point (the cap and budgets §7.13 states for them).

Writing `allowed.yaml` uses R2.8's emitter (the header comment, entries sorted by root in name order); the write itself, under the lock, is R6's.

Tests: each ignore rule of the two §7.13 bullets that needs no file-system call, as a table; name order of entries. Property test: no warning contains a refused value.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each rule of spec §7.13 'User config' and 'Allow state' that needs no file-system call has a unit test with the spec's outcome
- [x] #2 An AllowState emitted and parsed again is equal, with entries in name order of root (unit test)
- [x] #3 Property test: no warning holds a refused value
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented in an isolated worktree (2026-11-23), merged by the lead. New: path/absolute.rs (AbsolutePath), account/folder_key.rs, account/reading.rs (Reading<T> with warnings), account/user_config.rs (+tests), account/allow_state.rs (+tests, to_yaml through render::yaml). NonEmptyTargetMap::from_node made generic; Code gains account-config and account-allowed. AC1: unit tests for every §7.13 User config and Allow state rule that needs no file-system call (17-row bad-entry table for allowed.yaml). AC2: emit-then-parse equality, entries in name order of root (unit and property tests). AC3: no_warning_holds_a_refused_value property tests per file. Implementer's rust:ci in the worktree exit 0 (210 alz-core tests). Lead decisions on its open points: messages keep the reference phrasing; a non-mapping, non-null folders is one warning and ignored; an entry with a missing key is ignored as 'needs exactly root, account, source'; AbsolutePath is POSIX-only (Windows out of scope). For R2.11/spec: the warning codes account-config and account-allowed are not yet named in the spec (§7.13 or §10.1) — the architect names them there, or R2.11 records why not. Left to R6 on purpose: account-folder, real-path, duplicate-directory, longest-match and trust checks.

Verified by the lead after merging with R2.7 (2026-11-23): mise run rust:ci exits 0 (280 tests).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
accounts.yaml and allowed.yaml parse into account values (UserConfig with folder rules, AllowState with entries in name order of root) through the one YAML reader, with a warning per ignored entry or field that never holds a value but a quoted key, and a whole-file warning when the shape is wrong. AllowState writes back through alz's YAML emitter and reads back equal. File-system checks stay for R6.
<!-- SECTION:FINAL_SUMMARY:END -->
