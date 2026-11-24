---
id: TASK-55.8
title: 'R7: Distribute the Rust binary'
status: To Do
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-24T17:43:00Z'
labels:
  - area:rust
  - area:release
milestone: m-7
dependencies:
  - TASK-55.6
parent_task_id: TASK-55
priority: medium
ordinal: 40390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R7, re-scoped by decision-39: no npm package and no npm wrapper. Prebuilt binaries for the decision-25 targets (aarch64-apple-darwin, x86_64-apple-darwin, and static x86_64- and aarch64-unknown-linux-musl) are published as GitHub releases by CI only, Homebrew installs them, and the local install puts one regular file at ~/.local/bin/alz (R8 switches mise run install:local to the Rust alz). Replaces TASK-4, TASK-15 and TASK-16. The first release waits for R8 (TASK-55.9), so the released binary is the Rust-only alz proven by the spec tests. Steps only the owner can take are owner:human subtasks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A tagged release from CI publishes the four decision-25 binaries with SHA-256 sums and build attestations as a GitHub release, and no step publishes from a laptop or with a long-lived token (release run, workflow review)
- [ ] #2 Homebrew and the local install each give a working alz: alz --version prints the released version (runs in notes)
- [ ] #3 Every subtask is Done
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-24, replanned under decision-39 (architect): the npm wrapper is gone, and the subtasks are ordered by dependency; the owner-only steps carry owner:human.
<!-- SECTION:NOTES:END -->
