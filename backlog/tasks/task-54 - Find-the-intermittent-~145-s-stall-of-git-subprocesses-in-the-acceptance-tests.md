---
id: TASK-54
title: Find the intermittent ~145 s stall of git subprocesses in the acceptance tests
status: To Do
assignee: []
created_date: '2026-11-22T09:02:00Z'
updated_date: '2026-11-23T14:39:00Z'
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

2026-11-22: after the cleanup, three full suite runs left 3,203 new alz-* folders in $TMPDIR (~1,070 per run). R1.7 (one temp root per run, removed at the end) is what stops this.

2026-11-23 evidence for H1 (temp folder size): with 17,630 alz-* folders in $TMPDIR a full run took 2,128 s with 25 tests timing out at 20 s across 9 files (all TestTimeoutError, no assertion failures); git --version, init and commit each took 10-20 ms outside the suite. After deleting the folders the same tree passed in 58 s (958 passed). One run then left ~950 new folders. Next: R1.7 removes the per-run root; if the stall does not return once folders no longer pile up, close this with H1 as the cause.

2026-11-23, after R1.7 (TASK-55.2.7): a run no longer leaves alz-* folders, so the folder-count cause (H1) is removed. Two new pieces of evidence: (1) one 132,814 ms stall of the tests' own `git config --local user.name` in 26, with an empty temp folder, despite the 15 s timeout (the synchronous helper blocks the worker, so the kill and listing come late; the ps listing was not kept). (2) The verify-on-stop hook in .layer/hooks/hooks.yaml runs lint and the whole vitest suite at the end of every main-agent turn; seen with ps running alongside a manual `mise run test`, so two full suites can contend for the git subprocesses. Overlapping runs are a candidate cause for the remaining timeouts. AC #2 (fail fast with a clear message) is covered by R1.7's timeout helper.

2026-11-23: the R1 reviewer's theory that spawnSync waits on a grandchild holding the pipes after its timeout did not reproduce (passes on the old helper in 1 s; Node closes the pipes on timeout). The 132 s stall remains unexplained; the overlapping stop-hook suite is the open lead.

2026-11-23 20:01: a stop-hook run of the whole suite (508 s) failed one case: test/reference/32-account-allow.test.ts 'refuses when the layers name no account, and names --remove when this root has an entry' — test/exec.ts capture reported git -c commit.gpgsign=false init --quiet --initial-branch=main 'did not end within 438223 ms and was killed' although its timeout is 15 s; the process-group listing at the kill was empty (nothing left). A Rust implementer's builds and tests ran at the same time. Since spawnSync's timeout did not fire for ~7 min, a paused machine (sleep) is one explanation; another is that spawnSync's timer cannot run while the call blocks in a way the timeout does not cover. Next step: record wall-clock and monotonic time around capture to tell sleep from a hang.
<!-- SECTION:NOTES:END -->
