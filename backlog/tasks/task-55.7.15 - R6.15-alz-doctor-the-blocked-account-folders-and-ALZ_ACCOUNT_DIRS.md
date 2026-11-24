---
id: TASK-55.7.15
title: 'R6.15: alz doctor: the blocked account folders and ALZ_ACCOUNT_DIRS'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-24T08:16:00Z'
updated_date: '2026-11-24T11:14:00Z'
labels:
  - area:rust
  - area:accounts
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.7.2
  - TASK-55.7.7
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 174390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§10 doctor row; TASK-50.4 AC #5 only; owner questions 5 and 8).
- doctor's account-folder list (alz-io account_names) leaves out ~/.claude-alz-blocked and ~/.codex-alz-blocked and warns once when one exists, since Claude creates a missing folder at a blocked start.
- ALZ_ACCOUNT_DIRS as the owner decides (recommended: not read; the spec says doctor lists CLAUDE_CONFIG_DIR, ~/.claude and ~/.claude-* only; reference drift).
- The rest of TASK-50.4 stays in m-3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz doctor with ~/.claude-alz-blocked present does not list it as an account and prints one warning naming it (acceptance case, referenceDrift)
- [ ] #2 ALZ_ACCOUNT_DIRS is handled as the owner decided and the spec says so (acceptance case and spec diff)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (branch feat/rust-r6.15-doctor). alz-io account_names: claude_account_folders drops any candidate named .claude-alz-blocked (also when CLAUDE_CONFIG_DIR names it); new blocked_folders(env) lstat-only lookup of ~/.claude-alz-blocked then ~/.codex-alz-blocked. alz-core doctor: Facts.blocked, one 'warning: <folder> is a blocked folder (§7.13), not an account; delete it' line each, after the claude section, whatever the targets; not a problem, exit code unchanged. ALZ_ACCOUNT_DIRS is not read (it never was in Rust). AC1 evidence: unit tests a_blocked_folder_is_warned_about_once_and_is_not_a_problem and both_blocked_folders_get_a_warning_whatever_the_targets (alz-core); alz-io tests (blocked folder not an account, found by name without opening, mode 000); alz tests/doctor_live.rs doctor_warns_once_about_a_blocked_folder_and_does_not_list_it_as_an_account and ..._codex_blocked_folder_too_and_reads_nothing_inside_either; acceptance 12-account-independence 'doctor does not list a blocked folder as an account and warns once for each that exists' wrapped referenceDrift('TASK-55.7'): passes against target/release/alz, expected-fail against the reference. AC2 evidence: doctor_live.rs doctor_does_not_read_alz_account_dirs; acceptance 'doctor does not read ALZ_ACCOUNT_DIRS' (referenceDrift TASK-55.7); spec §10 doctor row now states the account folders are CLAUDE_CONFIG_DIR, ~/.claude, ~/.claude-* only and ALZ_ACCOUNT_DIRS is not read; §10.3 gives the warning position and exit code (the drift list rows for TASK-55.7 and TASK-50.4 already existed). Checks: mise run rust:ci exit 0; mise run lint exit 0; full acceptance project with ALZ_BIN=release binary: check-r6-pending exits 0 (247 failing, 247 listed). No r6-pending entry concerned doctor, so none deleted.
<!-- SECTION:NOTES:END -->
