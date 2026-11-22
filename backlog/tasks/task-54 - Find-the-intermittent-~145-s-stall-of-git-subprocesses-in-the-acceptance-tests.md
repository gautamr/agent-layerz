---
id: TASK-54
title: Find the intermittent ~145 s stall of git subprocesses in the acceptance tests
status: To Do
assignee: []
created_date: '2026-11-22T09:02:00Z'
updated_date: '2026-11-22T18:07:00Z'
labels:
  - area:test
  - discovered
dependencies:
  - TASK-55.2.7
priority: medium
ordinal: 31390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Seen three times on 2026-11-22 (implementer runs and a Stop-hook run): several tests that run git, in unrelated files (26, 28, 30, 31), all stall for about 146 seconds at the same moment and hit the 20 s timeout; an immediate rerun passes in about 25 s. It also happened on an unmodified HEAD checkout, so it is not caused by the account work. Tests use an isolated HOME and GIT_CONFIG_NOSYSTEM.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The cause is identified (for example a git or macOS service the subprocess waits on) and recorded in the task notes
- [ ] #2 The test helpers avoid it, or the tests fail fast with a clear message instead of waiting about 145 s
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22: paused by decision-21 (Rust port, doc-5). Done as part of TASK-55.2: one temp root per run through TMPDIR, removed by a global teardown, because the suite is the oracle. About 64,000 leftover alz-* folders were deleted from the temp folder on 2026-11-22 with the owner's approval.

2026-11-22 (architect, TASK-55.2 planning). TASK-55.2.7 (R1.7) implements this task's AC #2, with one temp root per run, TMPDIR for every child, and a spawn timeout that records a process listing of the stuck group. This task stays open for AC #1 (the cause). It closes when a recurrence is caught by that listing, or after the owner decides to close it once the stall has not recurred.

Hypotheses (no experiments run):
- H1, a shared temp folder. Every test makes its fixture directly in the user temp folder. That folder held about 64,000 alz-* entries before the clean-up on 2026-11-22 and 33,102 again at planning time. A sweep of that folder by macOS cleanup, indexing or a security agent, or the per-entry cost of a folder that size, could hold directory operations for every test at once. This fits "several unrelated files stall at the same moment". R1.7 removes the cause, because the folder gets one entry per run.
- H2, the xcrun git shim. git here is Apple's /usr/bin/git, an xcrun shim with a shared cache at $TMPDIR/xcrun_db. Every git call, from the tests and from alz, goes through it, and the children get an explicit env without TMPDIR. A shim that waits on its cache lock or on the developer-tools lookup would stall every concurrent git call together. If R1.7's listing shows xcrun or xcode-select under a stuck git, the follow-up is to resolve `xcrun --find git` once in setup and put that directory first on the children's PATH.
- H3, a network wait. About 146 s is close to two 75 s TCP connect timeouts. A case that lets alz or git reach a real host (a github: or https source that misses the cache) would wait on the network. The spawn timeout and listing would show git-remote-https or ssh.
- Unlikely: fsmonitor and maintenance or gc --auto, because the HOME is isolated, GIT_CONFIG_NOSYSTEM is set for the tests' own git, and the repositories are tiny. Note that alz's own git children do not get GIT_CONFIG_NOSYSTEM from the test env, so they read the system gitconfig, which includes the CommandLineTools credential helper.

2026-11-22: deleted 33102 leftover alz-* folders from $TMPDIR (owner approved), so the folder-size hypothesis (H1) can be checked: if the stall stops while R1.7 prevents new leaks, H1 is likely.
<!-- SECTION:NOTES:END -->
