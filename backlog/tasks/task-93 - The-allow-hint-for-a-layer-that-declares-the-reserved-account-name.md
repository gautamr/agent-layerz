---
id: TASK-93
title: The allow hint for a layer that declares the reserved account name
status: To Do
assignee: []
created_date: '2026-11-24T14:52:00Z'
labels:
  - area:rust
  - area:spec
  - needs:owner-approval
milestone: m-7
dependencies: []
priority: low
ordinal: 180390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From the R6 unit review. When a layer declares the reserved name alz-blocked, build and check print the hint to run alz account allow, which then refuses the name as reserved. The spec does not cover this case. Decide (spec sentence, owner approval) whether the hint is left out or names the reserved name, and implement it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Spec §10.3 says what build and check print when the declared account is the reserved name (owner approved)
- [ ] #2 A pty acceptance case shows build and check follow that sentence and allow's refusal stays as it is
<!-- AC:END -->
