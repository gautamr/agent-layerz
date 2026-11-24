---
id: TASK-74
title: Find the cause of the intermittent git-version test failure on Linux CI
status: Done
assignee: []
created_date: '2026-11-23T12:09:00Z'
updated_date: '2026-11-24T03:19:00Z'
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
- [x] #1 The cause is named from a failing run's message, or 50 consecutive Linux CI runs of git_runner pass
- [x] #2 If the cause is ETXTBSY or another test-harness race, the fix is in alz-test-support or the tests, not in product code, and a note says why
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Cause (2026-11-24): a parallel test forks while ScratchDir::write_executable still holds a stub script open for writing, so running the stub fails on Linux (ETXTBSY) and the version call sees no output ('found none'). Reproduced in a rust:1.100.0 Linux container (git_runner binary, --test-threads 32, 2 CPUs): the old code failed 7 of 300 runs across several stub tests (a_git_at_2_30..., a_git_below_2_30..., a_git_that_prints_no_version..., the_tree_and_object_calls...); with the fix 0 of 300. Fix in alz-test-support only (no product code): a short-lived /bin/sh child writes the file and sets its mode, so the test process never holds a writable handle that a forked child could inherit.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The git_runner flake was a test-harness ETXTBSY race; alz-test-support now writes stub executables through a child shell. Linux: 7/300 failures before, 0/300 after.
<!-- SECTION:FINAL_SUMMARY:END -->
