---
id: TASK-55.7.6
title: 'R6.6: The home folder and the two account files in alz-io'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T14:36:00Z'
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
- [x] #1 TASK-72 #1 holds: with HOME unset and no user database entry (an injected lookup), resolution warns once that HOME is not an absolute path and gives the default account for every target (Rust test)
- [x] #2 Each ignore reason of 'Both files' has an alz-io integration test with a temporary HOME, and a FIFO in place of either file is reported unreadable without waiting (cargo test)
- [x] #3 Property: an ignored file contributes no entry, and no warning holds a value or an invalid name from the file (proptest)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (R6.6). alz-io account module: Home (HOME as given; unset -> std::env::home_dir by injected lookup; none/empty/relative -> NotAbsolute), AccountFiles (locate, 'Both files' checks in spec order, no-follow O_NONBLOCK capped 1 MiB read via ReadOnlyDir::read_absolute with owner and mode from fstat of the read handle, parse into alz-core UserConfig/AllowFile, recheck once real into ResolvedUserConfig/ResolvedAllowState). alz-core: AllowFile::List gained numbers (entry numbers in state order) so warnings can name positions. Evidence: AC1 (files part) crates/alz-io/tests/account_files.rs with_home_unset_and_no_user_database_entry_both_files_are_ignored_with_one_warning + src/account/home.rs unit tests; the 'default account for every target' half needs the resolver (R6.9) and is not shown yet. AC2: one test per IgnoreReason in account_files.rs (NoUserId, NotOwned, group/other writable file, writable directory and parent below HOME, outside-HOME folder, link target folder, inside account folder real and as written, inside repository, work tree for allowed.yaml and warn-only for accounts.yaml, XDG moved with mise/direnv, empty/relative XDG, loop of links, folder in place of file, FIFO read on a thread with a 10 s guard, size cap 1 MiB vs +1) plus unit test for a directory owned by another user. AC3: proptests an_ignored_file_contributes_nothing_and_no_warning_holds_its_text and an_invalid_name_is_never_repeated_in_a_warning. Checks: mise run rust:ci exit 0; mise run lint ok. Open: spec needs for R6.2 (code of the file-ignored and HOME warnings, work-tree and relative-XDG warning texts, the root-in-configured-folder rule kept for allowed.yaml, geteuid vs real user id).

Verified by the lead (2026-11-24): merged into the R6 branch (8fa584f); fmt, clippy and the workspace tests pass on the merged tree. AC #1's default-account half is shown by the resolver (R6.9/R6.10).

Architect (2026-11-24, R6.2, lead decision): the account warning codes follow the reference, not account-config and account-allowed. Spec 7.13 (User config, Allow state, Both files), 10.1 and 10.3 now list each code and text. Replace: accounts.yaml: top level or folders not a mapping -> account-config-shape; unknown key -> account-config-key; default_name not an account name -> account-config-default-name; default_name also an account folder name -> account-default-name-folder; ignored folders key or entry -> account-config-folder; two keys for one directory -> account-config-duplicate. allowed.yaml: top level not a list -> account-allowed-shape; ignored entry or entries with the same root -> account-allowed-entry. Either file ignored by the Both files checks or because it is not valid YAML -> account-file-ignored; XDG_CONFIG_HOME or XDG_STATE_HOME not absolute -> account-xdg-relative; accounts.yaml inside a git work tree -> account-file-in-work-tree; HOME not an absolute path -> account-home. Texts unchanged (they already match the reference).

R6.16 evidence (architect, 2026-11-24, branch feat/rust-r6-task-55.7 at b757f43, target/release/alz rebuilt, cargo said up to date). AC #1 (TASK-72 #1): Ran: cargo test --locked -q -p alz-io --test account_files home: 9 passed, among them with_home_unset_and_no_user_database_entry_both_files_are_ignored_with_one_warning (Home::locate_with(env without HOME, || None) is Home::NotAbsolute; one account-home warning; both files ignored with HomeNotAbsolute and no repeated warning; no folders, no entries). cargo test --locked -q -p alz-core with_home_not_absolute: 1 passed (with HomeFact::NotAbsolute a named claude account fails closed, codex default resolves). Acceptance 31 'a HOME that is not an absolute path' (relative HOME, the same Home::NotAbsolute downstream) passes in the full Rust run: exit 0, one warning, claude default; it asserts no codex line. Missing test (so #1 stays unchecked): no test composes the two halves through alz account which. facts::gather (crates/alz/src/command/account/facts.rs) calls Home::locate(env) itself, so no test can inject the empty lookup into the command. Needed: a seam such as gather_with(env, lookup) used by which::run, and a crates/alz test that runs account which with HOME unset and lookup || None, asserting exit 0, exactly one warning [account-home] line on stderr, 'claude: default (source: default) CLAUDE_CONFIG_DIR unset' and 'codex: default (source: default) CODEX_HOME unset' on stdout.

R6.16 follow-up (implementer, 2026-11-24): the missing composition test for AC #1 is in, see TASK-72: facts::gather_with and which::run_at take the located Home, and which.rs tests::with_home_unset_and_no_user_database_entry_every_target_resolves_to_default runs account which with HOME unset and Home::locate_with(env, || None): exit 0, one [account-home] warning on stderr, claude and codex default on stdout. Passed; mise run rust:ci exit 0. Also removed Blocker::ConfigOutside (alz-core identity/setup.rs, its reason text, one test line, and its arm in alz-io repo_root/git_config.rs verify), never constructed; spec words a link at .git/config as 'not a regular file'. AC #1 left unchecked for the owner.
<!-- SECTION:NOTES:END -->
