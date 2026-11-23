---
id: TASK-74
title: Find the cause of the intermittent git-version test failure on Linux CI
status: To Do
assignee: []
created_date: '2026-11-23T12:09:00Z'
labels:
  - discovered
  - area:rust
  - area:test
milestone: m-7
dependencies: []
priority: low
ordinal: 113390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Seen once on PR #8's ubuntu job (commit 4124a4e): crates/alz/tests/git_runner.rs a_git_at_2_30_is_accepted_and_asked_for_its_version_once failed (GitRunner::with_launcher refused a fake git printing 'git version 2.30.0'); the same test passed on the commit before and after, on macOS, and 5 of 5 runs locally. The assertion now prints the refusal (ba79e69), so the next failure shows its kind. Leading hypothesis: ETXTBSY — a parallel test forks while this test's fake git script is still open for writing, so exec fails with 'Text file busy' (a known Linux race with freshly written executables in multithreaded test processes). If confirmed, make alz-test-support write fake executables so that no forked child can hold them open (e.g. write and close in a subprocess, or retry exec on ETXTBSY in tests only).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The cause is named from a failing run's message, or 50 consecutive Linux CI runs of git_runner pass
- [ ] #2 If the cause is ETXTBSY or another test-harness race, the fix is in alz-test-support or the tests, not in product code, and a note says why
<!-- AC:END -->
