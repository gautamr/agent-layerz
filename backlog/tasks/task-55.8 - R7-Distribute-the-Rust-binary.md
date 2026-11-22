---
id: TASK-55.8
title: 'R7: Distribute the Rust binary'
status: To Do
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-22T12:54:00Z'
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
doc-5 phase R7: prebuilt binaries for macOS and Linux, GitHub releases, Homebrew, an npm wrapper so npm install -g agent-layerz keeps working, and a local install as one regular file. Replaces TASK-4, TASK-15 and TASK-16.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A tagged release from CI installs alz through Homebrew, npm and the local install, each as a working binary, with no publish from a laptop
<!-- AC:END -->
