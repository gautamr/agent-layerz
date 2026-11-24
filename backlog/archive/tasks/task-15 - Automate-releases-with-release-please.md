---
id: TASK-15
title: Automate releases with release-please
status: To Do
assignee:
  - '@release-engineer'
created_date: '2026-11-19T17:13:00Z'
updated_date: '2026-11-22T12:55:00Z'
labels:
  - area:release
milestone: m-2
dependencies:
  - TASK-4
priority: high
ordinal: 270
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
0.1.0 was published by hand. Versions, tags, the changelog and GitHub Releases should follow from merged Conventional Commits.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Merging to main opens or updates a release pull request with the version bump and CHANGELOG entries
- [ ] #2 Merging the release pull request creates a tag and a GitHub Release
- [ ] #3 package.json and CHANGELOG.md versions stay in sync
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22: paused by decision-21 (Rust port, doc-5). Replaced by TASK-55.8 (Rust distribution).
<!-- SECTION:NOTES:END -->
