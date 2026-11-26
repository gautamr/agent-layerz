---
id: TASK-129
title: ScratchDir skips a taken name instead of failing the test
status: Done
assignee: []
created_date: '2026-11-26T08:07:00Z'
updated_date: '2026-11-26T08:11:00Z'
labels:
  - bug
  - area:tests
dependencies: []
priority: medium
ordinal: 230390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A test failed with 'a scratch directory: File exists' (git_sources.rs:86). ScratchDir names its folder alz-test-<pid>-<counter> and creates it exclusively, but a run that is cut off mid-test leaves its folder behind; a later process with the same pid then hits it and the test fails. Fix: try the next counter when the name is taken, still creating exclusively, so a stale or planted folder is skipped and never reused.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ScratchDir::new skips names that already exist and creates a new folder exclusively; a folder of a taken name is left untouched (test)
- [x] #2 It gives up with an error after a bounded number of taken names (no endless loop)
- [x] #3 mise run rust:check passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
ScratchDir::new tries up to 1000 names (the next counter values) and skips one that exists; creation stays exclusive, so a stale or planted folder is never reused or touched. Test: scratch_dir::tests::a_taken_name_is_skipped_and_the_folder_of_that_name_is_left_untouched plants the next three names with a file each. mise run rust:check exit 0 (79 test result lines ok). Cause found: three folders left by runs cut off mid-test (two 'deep' trees from cache_dir's 600-level test, one 'repo'); removed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
ScratchDir skips a taken name (bounded, exclusive creation), so a folder left by an interrupted run no longer fails a later test.
<!-- SECTION:FINAL_SUMMARY:END -->
