---
id: TASK-55.4.3
title: >-
  R3.3: GitRunner: typed calls, version floor, deadlines and no controlling
  terminal
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T07:59:00Z'
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
- [ ] #1 Every git call is a typed GitCall run with an argument array, --end-of-options before each revision, a cleared environment plus child_environment, and standard input null except for batch calls (unit and integration tests with a git stand-in that records argv and env)
- [ ] #2 A fake ssh that opens /dev/tty, started by git through GitRunner while the test runs under script(1) with a controlling terminal, cannot open it, and nothing it writes reaches the pty (Rust test; TASK-70 AC #2)
- [ ] #3 A call past its deadline is killed with its whole process group, ssh included, and the fetch fails; the binary's deadlines are 10 minutes for a clone and 2 minutes for other calls; a git below 2.30 is refused with error [git] naming the found and the minimum version before any other call (Rust tests)
- [ ] #4 No unsafe code: unsafe_code = forbid still holds and mise run rust:ci passes
<!-- AC:END -->
