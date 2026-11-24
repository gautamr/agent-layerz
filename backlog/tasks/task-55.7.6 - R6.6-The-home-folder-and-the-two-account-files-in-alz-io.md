---
id: TASK-55.7.6
title: 'R6.6: The home folder and the two account files in alz-io'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T08:41:00Z'
labels:
  - area:rust
  - area:accounts
  - area:security
milestone: m-7
dependencies: []
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 165390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.13 "User config", "Allow state", "Both files", "Environment"; TASK-72; owner question 3).
- account/home.rs: HOME as given; unset: the user database lookup of std::env::home_dir (no new crate); no entry: not an absolute path, so both files are ignored with one warning and named accounts fail closed.
- XDG_CONFIG_HOME and XDG_STATE_HOME: empty or relative counts as unset, a relative one with a warning.
- account/files.rs: AccountFile<Located> -> Trusted | Ignored(reason): inside the repository, not owned by the user, writable by group or others (file, folder, parents below HOME), the folder of a link and of its target, an XDG folder moved while mise or direnv is active (presence only), allowed.yaml inside a git work tree by lstat of .git (accounts.yaml there only warned), a path that names an account folder as written (before any file system call), no user id.
- Reads are no-follow, O_NONBLOCK, regular by fstat, at most 1 MiB plus one byte, with the alias limit, into alz-core's UserConfig and AllowState; folder keys and allowed roots are checked again once real; two that resolve alike are both ignored.
- Every map read from the two files is an ordered map keyed by validated values; a key outside its grammar is ignored with a warning, whatever the key (table rows of unusual keys, without comment).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 TASK-72 #1 holds: with HOME unset and no user database entry (an injected lookup), resolution warns once that HOME is not an absolute path and gives the default account for every target (Rust test)
- [x] #2 Each ignore reason of 'Both files' has an alz-io integration test with a temporary HOME, and a FIFO in place of either file is reported unreadable without waiting (cargo test)
- [x] #3 Property: an ignored file contributes no entry, and no warning holds a value or an invalid name from the file (proptest)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (R6.6). alz-io account module: Home (HOME as given; unset -> std::env::home_dir by injected lookup; none/empty/relative -> NotAbsolute), AccountFiles (locate, 'Both files' checks in spec order, no-follow O_NONBLOCK capped 1 MiB read via ReadOnlyDir::read_absolute with owner and mode from fstat of the read handle, parse into alz-core UserConfig/AllowFile, recheck once real into ResolvedUserConfig/ResolvedAllowState). alz-core: AllowFile::List gained numbers (entry numbers in state order) so warnings can name positions. Evidence: AC1 (files part) crates/alz-io/tests/account_files.rs with_home_unset_and_no_user_database_entry_both_files_are_ignored_with_one_warning + src/account/home.rs unit tests; the 'default account for every target' half needs the resolver (R6.9) and is not shown yet. AC2: one test per IgnoreReason in account_files.rs (NoUserId, NotOwned, group/other writable file, writable directory and parent below HOME, outside-HOME folder, link target folder, inside account folder real and as written, inside repository, work tree for allowed.yaml and warn-only for accounts.yaml, XDG moved with mise/direnv, empty/relative XDG, loop of links, folder in place of file, FIFO read on a thread with a 10 s guard, size cap 1 MiB vs +1) plus unit test for a directory owned by another user. AC3: proptests an_ignored_file_contributes_nothing_and_no_warning_holds_its_text and an_invalid_name_is_never_repeated_in_a_warning. Checks: mise run rust:ci exit 0; mise run lint ok. Open: spec needs for R6.2 (code of the file-ignored and HOME warnings, work-tree and relative-XDG warning texts, the root-in-configured-folder rule kept for allowed.yaml, geteuid vs real user id).

Verified by the lead (2026-11-24): merged into the R6 branch (8fa584f); fmt, clippy and the workspace tests pass on the merged tree. AC #1's default-account half is shown by the resolver (R6.9/R6.10).
<!-- SECTION:NOTES:END -->
