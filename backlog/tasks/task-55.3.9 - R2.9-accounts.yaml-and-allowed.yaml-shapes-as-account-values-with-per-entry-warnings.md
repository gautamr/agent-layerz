---
id: TASK-55.3.9
title: >-
  R2.9: accounts.yaml and allowed.yaml shapes as account values with per-entry
  warnings
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
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
- [ ] #1 Each rule of spec §7.13 'User config' and 'Allow state' that needs no file-system call has a unit test with the spec's outcome
- [ ] #2 An AllowState emitted and parsed again is equal, with entries in name order of root (unit test)
- [ ] #3 Property test: no warning holds a refused value
<!-- AC:END -->
