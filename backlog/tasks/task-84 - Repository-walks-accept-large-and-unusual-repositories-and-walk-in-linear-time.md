---
id: TASK-84
title: >-
  Repository walks: accept large and unusual repositories, and walk in linear
  time
status: To Do
assignee: []
created_date: '2026-11-24T06:34:00Z'
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
