---
id: TASK-55.8.4
title: Release the Rust alz as 0.2.0
status: To Do
assignee:
  - '@release-engineer'
created_date: '2026-11-24T17:45:00Z'
updated_date: '2026-11-24T18:25:00Z'
labels:
  - area:release
  - area:docs
  - needs:owner-approval
milestone: m-7
dependencies:
  - TASK-55.8.3
  - TASK-55.9
parent_task_id: TASK-55.8
priority: medium
ordinal: 197390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The first Rust release replaces the npm 0.1.0 for users. Its layer checks are stricter and its generated output differs from 0.1.0 (CHANGELOG.md Unreleased), and the npm install path is gone, so in 0.x it is a minor bump (the owner confirms the number; TASK-17 writes the policy down). Its notes and CHANGELOG.md describe every security fix it ships since 0.1.0, in wording and at a time the owner approves. README's install section then names the release binaries, Homebrew and the local install. Waits for R8 (TASK-55.9), so the release is the Rust-only alz proven by the spec tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 crates/alz has the version the owner chose, and the tag's release run publishes the four binaries with sums and attestations (release page)
- [ ] #2 Homebrew and the local install each give an alz whose --version prints that version (runs in notes)
- [ ] #3 CHANGELOG.md and the release notes describe every security fix shipped since 0.1.0, with the owner's approval of wording and timing in a comment
- [ ] #4 README's install section names only the release binaries, Homebrew and the local install (diff)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-24): the private security tasks are disclosed with the 0.2.0 release (a Security section in CHANGELOG and the release notes, optionally GitHub advisories); until then they stay private, then their files are committed and closed.
<!-- SECTION:NOTES:END -->
