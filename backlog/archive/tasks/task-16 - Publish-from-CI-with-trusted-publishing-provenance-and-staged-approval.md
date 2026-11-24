---
id: TASK-16
title: Publish from CI with trusted publishing, provenance and staged approval
status: To Do
assignee:
  - '@release-engineer'
created_date: '2026-11-19T17:13:00Z'
updated_date: '2026-11-22T12:55:00Z'
labels:
  - area:release
  - area:security
milestone: m-2
dependencies:
  - TASK-15
priority: high
ordinal: 280
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Publishing from a laptop with a long-lived login is the weakest link of the supply chain (decision-4). The owner configures the trusted publisher on npmjs.com; the workflow does the rest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The package trusts only this repository's release workflow through OIDC, and no npm token is stored in GitHub secrets
- [ ] #2 Published versions show provenance on npmjs.com
- [ ] #3 CI stages each release; it goes live only after the owner approves it with 2FA
- [ ] #4 CONTRIBUTING.md documents the release flow
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22: paused by decision-21 (Rust port, doc-5). Replaced by TASK-55.8 (Rust distribution).
<!-- SECTION:NOTES:END -->
