---
id: TASK-4
title: Add CI on Node 22 and 24, Ubuntu and macOS
status: To Do
assignee:
  - '@release-engineer'
created_date: '2026-11-19T17:13:00Z'
labels:
  - area:ci
milestone: m-1
dependencies:
  - TASK-2
priority: high
ordinal: 190
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Nothing checks pull requests today, and the release pipeline in Phase 2 builds on CI. Node 22 is the oldest supported line, Node 24 the current LTS.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Runs on pull requests and on pushes to main
- [ ] #2 Matrix covers Node 22 and 24 on ubuntu-latest and macos-latest
- [ ] #3 Runs lint, the test suite and the build
- [ ] #4 Packs the package, installs the tarball and runs `alz --version` and `alz init` plus `alz build` in a temporary repository
- [ ] #5 Actions are pinned to commit SHAs and the workflow has least-privilege permissions
- [ ] #6 README shows the CI status badge
<!-- AC:END -->
