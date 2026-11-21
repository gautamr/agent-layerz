---
id: TASK-22
title: Report per-account trust for workspace directories
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-19T17:14:00Z'
updated_date: '2026-11-21T17:58:00Z'
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
Claude keeps the trust dialog result per account folder; an untrusted repository ignores permissions.additionalDirectories, so it silently loses access to its hub after an account switch. This is now part of account switching in alz (decision-15, TASK-50).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The behaviour is verified and recorded in docs/target-facts.md
- [ ] #2 alz doctor reports, for each account folder, whether the repository is trusted when it has workspace directories, or the spec records why it cannot without breaking the names-only rule
- [ ] #3 The trust check runs in alz doctor next to the account checks from TASK-50.4
<!-- AC:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @claude
created: 2026-11-21T17:58:00Z

---

decision-15 replaces decision-2: account switching moves into alz (TASK-50), so the workspace-accounts contract criterion is replaced.

---
<!-- COMMENTS:END -->
