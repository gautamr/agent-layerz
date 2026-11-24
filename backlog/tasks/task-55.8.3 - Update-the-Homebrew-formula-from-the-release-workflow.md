---
id: TASK-55.8.3
title: Update the Homebrew formula from the release workflow
status: To Do
assignee:
  - '@release-engineer'
created_date: '2026-11-24T17:45:00Z'
labels:
  - area:release
  - area:ci
milestone: m-7
dependencies:
  - TASK-55.8.2
parent_task_id: TASK-55.8
priority: medium
ordinal: 196390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-39: Homebrew is one of the two install paths. After a release, the workflow writes a formula naming each archive's URL and SHA-256 to the owner's tap, through the credential scoped to that repository. The formula installs the one binary, with no wrapper or script. Untrusted input: none beyond the release's own assets, whose sums the formula pins. Out of scope: homebrew-core.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After a release run, the tap holds a formula with each archive's URL and SHA-256 (tap commit in notes)
- [ ] #2 brew install of that formula gives a working alz on macOS arm64 and Linux x86_64, and brew test runs alz --version (runs in notes)
- [ ] #3 mise run workflows:lint passes
<!-- AC:END -->
