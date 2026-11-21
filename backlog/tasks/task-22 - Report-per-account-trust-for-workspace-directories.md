---
id: TASK-22
title: Report per-account trust for workspace directories
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-19T17:14:00Z'
labels:
  - area:cli
  - area:security
milestone: m-3
dependencies: []
priority: medium
ordinal: 340
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Claude keeps the trust dialog result per account folder; an untrusted repository ignores permissions.additionalDirectories, so it silently loses access to its hub after an account switch. This is part of the contract with workspace-accounts (decision-2).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The behaviour is verified and recorded in docs/target-facts.md
- [ ] #2 alz doctor reports, for each account folder, whether the repository is trusted when it has workspace directories, or the spec records why it cannot without breaking the names-only rule
- [ ] #3 The workspace-accounts side of the contract is agreed and linked
<!-- AC:END -->
