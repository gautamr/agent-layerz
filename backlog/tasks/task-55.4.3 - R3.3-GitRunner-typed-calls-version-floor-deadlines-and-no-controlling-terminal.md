---
id: TASK-55.4.3
title: >-
  R3.3: GitRunner: typed calls, version floor, deadlines and no controlling
  terminal
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T08:20:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.4.2
parent_task_id: TASK-55.4
priority: high
type: task
ordinal: 104390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Fixes TASK-70 (AC #2). Adds rustix's `process` feature, which decision-27 allows.
- alz-io `git/runner.rs`: `GitRunner::new(&EnvSnapshot, Deadlines)` runs `git version` once, parses it with alz-core's `GitVersion` and refuses a git below 2.30 with error [git] naming the found and the minimum version (owner answer 4). `GitCall` is a typed operation, and no `&str` arguments come from callers:
  - Clone: `clone --quiet -- <arg> <temp>`, with `-c protocol.file.allow=never` when remote;
  - RevParseCommit: `rev-parse --verify --quiet --end-of-options <rev>^{commit}`;
  - Checkout: `-c advice.detachedHead=false checkout --quiet --end-of-options <sha>`;
  - ShowToplevel and ShowPrefix;
  - LsTreeAt and LsTreeRecursive (`-z`);
  - CatFileBatchCheck and CatFileBatch.
  Output is parsed into `ObjectId` and the like, never returned as text to print.
- `git/spawn.rs`:
  - `env_clear()` plus `child_environment` (R3.2), with git found through the child's PATH, as test/git-proxy.ts needs;
  - standard input null, except that the batch calls feed it;
  - stdout and stderr read on two threads with caps, stderr's first 64 KiB kept;
  - a deadline per call from `Deadlines`, which the binary sets to 10 minutes for a clone and 2 minutes for any other call (owner answer 5); when it passes, `kill_process_group` ends the session's group, and the result is a `GitFailure::TimedOut`;
  - the exit status checked.
- `git/detach.rs` (owner answer 1): the runner starts `current_exe()` with a fixed internal first argument. `run_if_requested()`, called first in `main`, sees that argument. It calls `rustix::process::setsid()` (the child is not a group leader, so setsid succeeds) and then `CommandExt::exec("git", args)`. That is safe code only, so `unsafe_code = forbid` holds. The internal argument runs only `git`, never another program.
- A SMOKE run, recorded in the notes and the spec, shows that a process started this way has no controlling terminal: opening /dev/tty fails with ENXIO.
The user's core.sshCommand and GIT_SSH_COMMAND are untouched, and ssh's graphical askpass stays allowed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every git call is a typed GitCall run with an argument array, --end-of-options before each revision, a cleared environment plus child_environment, and standard input null except for batch calls (unit and integration tests with a git stand-in that records argv and env)
- [x] #2 A fake ssh that opens /dev/tty, started by git through GitRunner while the test runs under script(1) with a controlling terminal, cannot open it, and nothing it writes reaches the pty (Rust test; TASK-70 AC #2)
- [x] #3 A call past its deadline is killed with its whole process group, ssh included, and the fetch fails; the binary's deadlines are 10 minutes for a clone and 2 minutes for other calls; a git below 2.30 is refused with error [git] naming the found and the minimum version before any other call (Rust tests)
- [x] #4 No unsafe code: unsafe_code = forbid still holds and mise run rust:ci passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (not committed, no AC checked). alz-core: git/version.rs (GitVersion, MINIMUM 2.30, VersionRefusal -> error [git] 'alz needs git 2.30 or later; found <v>' / 'an unreadable version' / 'none'), Code::Git, git/{tree_entry,batch,object_kind}.rs (pure parsers of ls-tree -z, cat-file --batch-check and --batch). alz-io: git/{call,spawn,detach,runner,deadlines,failure}.rs; GitRunner::new(&EnvSnapshot, Deadlines) / with_launcher; typed methods clone_source, rev_parse_commit(scope), checkout, show_toplevel, show_prefix, ls_tree_at, ls_tree_recursive, cat_file_batch_check, cat_file_batch; Deadlines::STANDARD = 10 min clone, 2 min other. alz main.rs calls alz_io::run_if_requested() first. rustix gains only the process feature. alz-test-support: ScratchDir::{write_executable,read,exists}, test_executable(). Spec: SMOKE sentence under 'No terminal'.
AC1: crates/alz/tests/git_runner.rs: every_call_runs_git_with_the_argument_array..., the_tree_and_object_calls_..., git_gets_a_cleared_environment_plus_the_child_environment_of_its_operation use a stand-in git on PATH recording argv, env (env_clear proven: only PATH/HOME/stand-in vars plus child_environment survive; GIT_DIR, GIT_INDEX_FILE, GIT_CONFIG_* removed, GIT_TERMINAL_PROMPT=0, tree reads add GIT_NO_REPLACE_OBJECTS/GIT_NO_LAZY_FETCH), cwd, and stdin (empty for non-batch, ids for batch). Pure argv unit tests: crates/alz-io/src/git/call/tests.rs (--end-of-options before every revision, -- only before the clone source).
AC2: a_fake_ssh_cannot_open_the_terminal_git_runs_under: the test re-runs itself under script(1) (a controlling pty). Inside, the same fake ssh started directly opens /dev/tty and its marker reaches the pty (control); started by real git through GitRunner (launcher = alz binary: setsid then exec git) it gets 'refused' with a non-empty error and its marker never reaches the pty. Mutation check: replacing setsid() with a no-op makes the test fail (ssh 'opened'). SMOKE by hand (Darwin 25.6, git 2.50.1): under script(1), 'alz --alz-internal-run-git-without-terminal -c alias.probe=!sh -c "cat /dev/tty" probe' prints 'cat: /dev/tty: Device not configured' (ENXIO); git run directly opens it. Note: setsid fails with EPERM if the launcher is a process group leader (e.g. 'sh -c alz ...' that exec-optimises under script); the runner starts it as an ordinary child so it is not one.
AC3: a_call_past_its_deadline_is_killed_with_its_whole_process_group_ssh_included (real git + fake ssh sleeping 300s, clone deadline 3 s: TimedOut{after:3s}, both ssh and its parent are dead afterwards); Deadlines::STANDARD unit test (600 s / 120 s); version floor tests: git 2.29.9 refused with 'error [git] alz needs git 2.30 or later; found 2.29.9 (Test Git-1)' and the stand-in log shows only 'version' was run; 2.30.0 accepted; unreadable; no git on PATH -> 'found none'. Real-git test real_git_clones_resolves_checks_out_lists_and_reads_objects covers every call against git 2.50.1.
AC4: unsafe_code = forbid unchanged, no unsafe; mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, msrv, deny, audit); mise run lint exit 0.
Open: (1) Ctrl-C: git now runs in its own session, so SIGINT to alz does not reach it; alz's exit leaves a running clone to finish or hit nothing (no kill-on-drop). Worth a decision for R3.8 (e.g. signal handling or kill on exit). (2) The ls-tree argv has no '--' (path follows --end-of-options as an operand); git 2.30's handling of '--' after --end-of-options is unverified, only 2.50.1 was run. (3) A stream held open by a descendant that left the group is bounded by the deadline, then the group is killed and the call is TimedOut. (4) Stdout caps: 64 KiB for one-line outputs, 64 MiB for listings and batches (OutputTooLarge); R3.9 may want its own cap. (5) GitRunner::new uses current_exe, which only works in the alz binary (test binaries use with_launcher). (6) R3.4's commit landed on the branch while I worked; I kept its FetchFailureLine export in git.rs.

Verified by the lead (2026-11-23): mise run rust:ci exits 0 (452 tests, incl. tests/git_runner.rs), lint passes, src/ unchanged. The /dev/tty proof failed under a setsid no-op mutation and passes with the fix. Lead decisions on open points: (1) Ctrl-C: R3.8 makes alz kill the git process group when the call is abandoned (alz interrupted or exiting), so a clone never outlives alz; recorded on TASK-55.4.8. (2) ls-tree argv on git 2.30 is unverified on 2.30 itself: R3.11 runs the git_runner tests against a git 2.30 build if one can be installed through mise, else records the gap. (3) Output caps (64 KiB / 64 MiB) stay; R3.9 may change the listing cap with a reason.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
GitRunner runs git only as typed calls with argument arrays (--end-of-options before revisions) in a cleared environment plus the R3.2 child environment, refuses git below 2.30 with error [git], kills the whole process group at a per-call deadline (10 min clone, 2 min other), and runs every call through a self re-exec child that calls rustix setsid() first, so git and ssh have no controlling terminal (proved with a fake ssh under a pseudo-terminal; no unsafe code). Adds rustix's process feature.
<!-- SECTION:FINAL_SUMMARY:END -->
