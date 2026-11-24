---
id: TASK-72
title: >-
  With HOME unset and no user database entry, account resolution fails closed
  instead of crashing
status: To Do
assignee: []
created_date: '2026-11-23T04:51:00Z'
updated_date: '2026-11-24T14:36:00Z'
labels:
  - discovered
  - area:rust
milestone: m-7
dependencies: []
references:
  - docs/LAYERING-SPEC.md
  - src/cli/io.ts
priority: low
type: bug
ordinal: 89390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found in the TASK-65 review. src/cli/io.ts takes io.env.HOME ?? homedir(). Node's os.homedir() looks up the effective user id when HOME is unset, and throws when the user database has no entry, so the command crashes. Spec §7.13 'Environment' says that case counts as a HOME that is not an absolute path: both files are ignored with one warning, no account folder is set, and a named account fails closed. Fixed only in Rust, in R6 (TASK-55.7), per decision-30.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 With HOME unset and an effective user id that has no user database entry, alz account which exits 0, warns once that HOME is not an absolute path, and reports the default account for every target (Rust test, since the setup needs a uid without an entry)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
R6.6 (TASK-55.7.6): the Home type and the HOME-not-absolute warning with both files ignored are in alz-io (tests in crates/alz-io/tests/account_files.rs, the lookup injected). The criterion's 'reports the default account for every target' part is shown once the resolver exists (R6.9/R6.10); not checked here.

R6.16 evidence (architect, 2026-11-24, branch feat/rust-r6-task-55.7 at b757f43, target/release/alz rebuilt, cargo said up to date). Ran: cargo test --locked -q -p alz-io --test account_files home: 9 passed, among them with_home_unset_and_no_user_database_entry_both_files_are_ignored_with_one_warning (Home::locate_with(env without HOME, || None) is Home::NotAbsolute; one account-home warning; both files ignored with HomeNotAbsolute and no repeated warning; no folders, no entries). cargo test --locked -q -p alz-core with_home_not_absolute: 1 passed (with HomeFact::NotAbsolute a named claude account fails closed, codex default resolves). Acceptance 31 'a HOME that is not an absolute path' (relative HOME, the same Home::NotAbsolute downstream) passes in the full Rust run: exit 0, one warning, claude default; it asserts no codex line. Missing test (so #1 stays unchecked): no test composes the two halves through alz account which. facts::gather (crates/alz/src/command/account/facts.rs) calls Home::locate(env) itself, so no test can inject the empty lookup into the command. Needed: a seam such as gather_with(env, lookup) used by which::run, and a crates/alz test that runs account which with HOME unset and lookup || None, asserting exit 0, exactly one warning [account-home] line on stderr, 'claude: default (source: default) CLAUDE_CONFIG_DIR unset' and 'codex: default (source: default) CODEX_HOME unset' on stdout.

R6.16 follow-up (implementer, 2026-11-24, branch feat/rust-r6-task-55.7 on b757f43, uncommitted). Seam: facts::gather_with(env, directory, choices, home: Home) in crates/alz/src/command/account/facts.rs, gather calls it with Home::locate(env); which::run calls which::run_at(args, surroundings, home, stdout, stderr) with Home::locate(surroundings.env). Test: crates/alz/src/command/account/which.rs tests::with_home_unset_and_no_user_database_entry_every_target_resolves_to_default. It sits in the binary crate's unit tests because alz has no lib and the real binary cannot be made to find no user database entry. It builds an EnvSnapshot with no HOME and XDG_CONFIG_HOME, XDG_STATE_HOME and XDG_CACHE_HOME under a ScratchDir, takes home = Home::locate_with(&env, || None) (asserted to be Home::NotAbsolute), and runs the real which code (gather_with, CachedLayers, resolve, which_report, emit_diagnostics) with Vec writers in a scratch repository: exit 0; stdout holds 'claude: default (source: default) CLAUDE_CONFIG_DIR unset' and 'codex: default (source: default) CODEX_HOME unset'; stderr is exactly one line, with one [account-home] warning. Result: ok (cargo test --locked -p alz --bin alz with_home_unset: 1 passed). mise run rust:ci exit 0 (run without a pipe); mise run lint ok; mise run test 56 files, 1307 passed, 295 expected fail, 5 skipped. AC #1 stays unchecked for the owner.
<!-- SECTION:NOTES:END -->
