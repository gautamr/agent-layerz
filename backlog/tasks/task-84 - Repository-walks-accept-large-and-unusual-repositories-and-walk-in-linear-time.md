---
id: TASK-84
title: >-
  Repository walks: accept large and unusual repositories, and walk in linear
  time
status: To Do
assignee: []
created_date: '2026-11-24T06:34:00Z'
updated_date: '2026-11-24T18:49:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies: []
priority: medium
ordinal: 155390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R5 review (2026-11-24): the AGENTS.override.md walk (crates/alz-io/src/fs/repo_root/list.rs) caps a folder at 10,000 entries, refuses a folder with a name that is not UTF-8 and a .GIT folder on Linux, and re-walks each folder from the root, so cost grows with depth squared. Keep the guarantees (never follow links, find every override) while accepting such repositories where safe and walking each folder once.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Large folders, unusual names and deep trees are handled as the spec states, with tests, and the walk visits each folder once
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented on fix/repository-walk-task-84. Cause: the override search walked every folder (gitignored build folders included, as the spec says every folder) but listing() refused a folder past 10,000 entries, refused any folder whose name was not UTF-8 or no SafeRelPath (.GIT, backslash), and re-opened each folder from the root. Fix: walk() in crates/alz-io/src/fs/repo_root/list.rs lists each folder once, streaming, entering each child by openat from its parent handle (O_NOFOLLOW|O_DIRECTORY), by raw bytes (CString), no entry/depth/path-length cap, d_type instead of lstat per entry, parent handle dropped with its last pending child. files_named now returns Stripped paths. TooManyEntries removed. Spec 4.3 search paragraph rewritten. Tests: crates/alz-io/tests/repository_walk.rs (10,001-entry folders, non-Unicode name where the file system allows, odd valid names, 40x200-char path, 1000 deep, links not followed, .git/node_modules skipped; run also with ulimit -n 64) and a proptest in list/tests.rs that each (folder,entry) is reported exactly once. rust:ci green, 2043 tests. Evidence: scratch repo with a 10,001-entry build folder: old binary 'error [write] build holds more than 10000 entries'; this branch's release binary 'check passed: 68 generated files match'.
<!-- SECTION:NOTES:END -->
