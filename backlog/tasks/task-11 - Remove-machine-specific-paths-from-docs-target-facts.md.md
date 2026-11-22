---
id: TASK-11
title: Remove machine-specific paths from docs/target-facts.md
status: Done
assignee:
  - '@claude'
created_date: '2026-11-19T17:13:00Z'
updated_date: '2026-11-22T03:11:00Z'
labels:
  - area:docs
milestone: m-1
dependencies: []
priority: low
ordinal: 240
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The verified facts are valuable public evidence, but the file still names scratch paths from the author's machine.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No absolute paths or usernames remain
- [x] #2 Every verified fact, its status and its date are kept
- [x] #3 Scratch fixtures are described generically
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
docs/target-facts.md §0.1 no longer names an absolute scratchpad path (it held the local username); the fixtures are described generically and every fact, status and date is unchanged. A repo-wide git grep finds no absolute machine paths in tracked files. Note: the old path remains in history already pushed to origin/main.
<!-- SECTION:FINAL_SUMMARY:END -->
